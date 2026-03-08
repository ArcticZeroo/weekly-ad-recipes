use axum::extract::{Path, State};
use axum::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::inflight::AcquireResult;
use crate::models::deal::{Deal, DealsResponse};
use crate::models::location::StoreLocation;
use crate::routes::locations::resolve_or_create_location;
use crate::AppState;

pub async fn get_deals(
    State(state): State<AppState>,
    Path((chain, zip)): Path<(String, String)>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = resolve_or_create_location(&state, &chain, &zip).await?;
    let week_id = queries::current_week_id();

    let (deals, cached) = ensure_deals(&state, &location, &week_id).await?;
    Ok(Json(DealsResponse { chain_id: chain, zip_code: zip, week_id, deals, cached }))
}

pub async fn refresh_deals(
    State(state): State<AppState>,
    Path((chain, zip)): Path<(String, String)>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = resolve_or_create_location(&state, &chain, &zip).await?;
    let week_id = queries::current_week_id();

    // Invalidate cache so the loop below is forced to re-fetch.
    // Concurrent refreshes race to delete (idempotent), then one becomes
    // the leader and the rest wait and read the freshly cached result.
    queries::invalidate_deals_cache(&state.pool, location.id, &week_id).await?;
    state.invalidate_deals_hash(location.id, &week_id);

    let (deals, _) = ensure_deals(&state, &location, &week_id).await?;
    Ok(Json(DealsResponse { chain_id: chain, zip_code: zip, week_id, deals, cached: false }))
}

/// Returns deals from cache or fetches them, deduplicating concurrent requests
/// so only one fetch runs at a time for a given location+week.
///
/// Returns `(deals, was_from_cache)`.
async fn ensure_deals(
    state: &AppState,
    location: &StoreLocation,
    week_id: &str,
) -> Result<(Vec<Deal>, bool), AppError> {
    let key = format!("{}:{}", location.id, week_id);

    loop {
        if let Some(deals) = queries::get_cached_deals(&state.pool, location.id, week_id).await? {
            state.resolve_deals_hash(location.id, week_id, &deals);
            return Ok((deals, true));
        }

        match state.deals_tracker.try_acquire(&key) {
            AcquireResult::Wait(notify) => {
                tracing::debug!("Deals fetch already in-flight for {key}, waiting");
                notify.notified().await;
            }
            AcquireResult::Lead(guard) => {
                let deals = fetch_deals_from_source(state, location, week_id).await?;
                state.resolve_deals_hash(location.id, week_id, &deals);
                drop(guard);
                return Ok((deals, false));
            }
        }
    }
}

/// Dispatches to the appropriate fetch strategy (Flipp or Vision).
async fn fetch_deals_from_source(
    state: &AppState,
    location: &StoreLocation,
    week_id: &str,
) -> Result<Vec<Deal>, AppError> {
    if location.flipp_merchant_id.is_some() {
        fetch_and_cache_flipp_deals(state, location, week_id).await
    } else {
        fetch_and_cache_vision_deals(state, location, week_id).await
    }
}

async fn fetch_and_cache_flipp_deals(
    state: &AppState,
    location: &StoreLocation,
    week_id: &str,
) -> Result<Vec<Deal>, AppError> {
    let client = reqwest::Client::new();

    let flyers = flipp::search_flyers_by_zip(&client, &location.zip_code).await?;

    let flyer = flyers.iter().find(|f| {
        f.merchant_id == location.flipp_merchant_id
            || f.chain_id == location.chain_id
    });

    let flyer_id = match flyer.and_then(|f| f.flyer_id) {
        Some(id) => id,
        None => {
            tracing::warn!(
                "No current flyer found for location {} (chain: {})",
                location.id,
                location.chain_id
            );
            return Ok(vec![]);
        }
    };

    let items = flipp::fetch_flyer_items(&client, flyer_id).await?;
    let mut deal_tuples = flipp::items_to_deal_tuples(&items);

    tracing::info!(
        "Fetched {} items from Flipp flyer {} for location {}",
        deal_tuples.len(),
        flyer_id,
        location.id
    );

    // Extract deal descriptions from images for items with no price info
    let vision_items = flipp::items_needing_vision(&items);
    if !vision_items.is_empty() {
        tracing::info!("{} items need vision extraction", vision_items.len());
        match crate::ai::extract_deals::extract_deals_from_images(
            &state.ai,
            &client,
            &vision_items,
        )
        .await
        {
            Ok(extracted) => {
                for deal in &mut deal_tuples {
                    if deal.2 == "On Sale" {
                        if let Some(description) = extracted.get(&deal.0) {
                            deal.2 = description.clone();
                        }
                    }
                }
                // Remove items the AI identified as not actual deals
                deal_tuples.retain(|deal| deal.2 != "NOT_A_DEAL");
                tracing::info!("Vision extracted deals for {} items", extracted.len());
            }
            Err(err) => {
                tracing::warn!("Vision deal extraction failed: {err}");
            }
        }
    }

    // AI categorization — also filters out non-food items
    let items_for_categorization: Vec<(String, Option<String>)> = deal_tuples
        .iter()
        .map(|(name, brand, _, _, _)| (name.clone(), brand.clone()))
        .collect();

    match crate::ai::categorize::categorize_items(&state.ai, &items_for_categorization).await {
        Ok(categories) => {
            for deal in &mut deal_tuples {
                if let Some(category) = categories.get(&deal.0) {
                    deal.3 = category.clone();
                }
            }
            let before = deal_tuples.len();
            deal_tuples.retain(|deal| deal.3 != "not_food");
            tracing::info!(
                "AI categorized {} items, filtered {} non-food",
                categories.len(),
                before - deal_tuples.len()
            );
        }
        Err(err) => {
            tracing::warn!("AI categorization failed, using 'uncategorized': {err}");
        }
    }

    queries::save_deals(&state.pool, location.id, week_id, &deal_tuples).await?;

    let deals = queries::get_cached_deals(&state.pool, location.id, week_id)
        .await?
        .unwrap_or_default();

    Ok(deals)
}

async fn fetch_and_cache_vision_deals(
    state: &AppState,
    location: &StoreLocation,
    week_id: &str,
) -> Result<Vec<Deal>, AppError> {
    let deal_tuples = match location.chain_id.as_str() {
        "whole-foods" => fetch_whole_foods_deals(state, location).await?,
        _ => fetch_generic_vision_deals(state, location).await?,
    };

    if deal_tuples.is_empty() {
        return Ok(vec![]);
    }

    queries::save_deals(&state.pool, location.id, week_id, &deal_tuples).await?;

    let deals = queries::get_cached_deals(&state.pool, location.id, week_id)
        .await?
        .unwrap_or_default();

    Ok(deals)
}

/// Try structured __NEXT_DATA__ scrape first, fall back to Vision screenshots.
async fn fetch_whole_foods_deals(
    state: &AppState,
    location: &StoreLocation,
) -> Result<Vec<(String, Option<String>, String, String, Option<String>)>, AppError> {
    // Extract WFM store ID from weekly_ad_url or use a default
    let wfm_store_id = location
        .weekly_ad_url
        .as_deref()
        .and_then(|url| {
            url.split("store-id=").nth(1).map(|s| {
                s.split('&').next().unwrap_or(s).to_string()
            })
        })
        .unwrap_or_else(|| "10260".to_string()); // Default to Redmond

    tracing::info!(
        "Trying structured scrape for Whole Foods store {}",
        wfm_store_id
    );

    match crate::fetcher::vision::stores::whole_foods::fetch_deals(&wfm_store_id).await {
        Ok(deals) if !deals.is_empty() => {
            tracing::info!(
                "Structured scrape got {} deals from Whole Foods",
                deals.len()
            );

            // Still need AI categorization
            let mut deal_tuples = deals;
            let items_for_categorization: Vec<(String, Option<String>)> = deal_tuples
                .iter()
                .map(|(name, brand, _, _, _)| (name.clone(), brand.clone()))
                .collect();

            match crate::ai::categorize::categorize_items(
                &state.ai,
                &items_for_categorization,
            )
            .await
            {
                Ok(categories) => {
                    for deal in &mut deal_tuples {
                        if let Some(category) = categories.get(&deal.0) {
                            deal.3 = category.clone();
                        }
                    }
                    deal_tuples.retain(|deal| deal.3 != "not_food");
                }
                Err(err) => {
                    tracing::warn!("Categorization failed for WF deals: {err}");
                }
            }

            Ok(deal_tuples)
        }
        Ok(_) => {
            tracing::warn!("Structured scrape returned empty, falling back to Vision");
            fetch_generic_vision_deals(state, location).await
        }
        Err(err) => {
            tracing::warn!("Structured scrape failed: {err}, falling back to Vision");
            fetch_generic_vision_deals(state, location).await
        }
    }
}

async fn fetch_generic_vision_deals(
    state: &AppState,
    location: &StoreLocation,
) -> Result<Vec<(String, Option<String>, String, String, Option<String>)>, AppError> {
    let url = match location.weekly_ad_url.as_deref() {
        Some(url) => url.to_string(),
        None => {
            tracing::warn!("No weekly ad URL for chain: {}", location.chain_id);
            return Ok(vec![]);
        }
    };

    tracing::info!(
        "Taking screenshots of {} for location {}",
        url,
        location.id
    );

    let screenshots =
        crate::fetcher::vision::browser::screenshot_page(&url).await?;

    tracing::info!("Captured {} screenshots, sending to Vision AI", screenshots.len());

    let deal_tuples =
        crate::fetcher::vision::extract_deals_from_screenshots(&state.ai, &screenshots).await?;

    tracing::info!(
        "Vision extracted {} deals for location {}",
        deal_tuples.len(),
        location.id
    );

    Ok(deal_tuples)
}
