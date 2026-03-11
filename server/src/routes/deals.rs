use std::collections::{HashMap, HashSet};

use axum::extract::{Path, State};
use axum::Json;
use chrono::Datelike;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::fetcher::hmart;
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

    let (deals, _week_id, cached) = ensure_current_deals(&state, &location).await?;
    Ok(Json(build_deals_response(chain, zip, deals, cached)))
}

pub async fn refresh_deals(
    State(state): State<AppState>,
    Path((chain, zip)): Path<(String, String)>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = resolve_or_create_location(&state, &chain, &zip).await?;

    queries::invalidate_all_deals_for_location(&state.pool, location.id).await?;

    let (deals, _week_id, _) = ensure_current_deals(&state, &location).await?;
    Ok(Json(build_deals_response(chain, zip, deals, false)))
}

fn build_deals_response(
    chain_id: String,
    zip_code: String,
    deals: Vec<Deal>,
    cached: bool,
) -> DealsResponse {
    let valid_from = deals.first().and_then(|deal| deal.valid_from.clone());
    let valid_to = deals.first().and_then(|deal| deal.valid_to.clone());
    DealsResponse { chain_id, zip_code, valid_from, valid_to, deals, cached }
}

/// Returns current deals for a location, fetching fresh if expired or missing.
/// Returns `(deals, week_id, was_from_cache)`.
async fn ensure_current_deals(
    state: &AppState,
    location: &StoreLocation,
) -> Result<(Vec<Deal>, String, bool), AppError> {
    let key = format!("deals:{}", location.id);

    loop {
        if let Some((deals, week_id)) =
            queries::get_current_deals(&state.pool, location.id).await?
        {
            // Filter out expired deals, keep valid ones
            let (valid_deals, expired_week_ids) = partition_expired_deals(deals);

            // Clean up expired batches
            for expired_id in &expired_week_ids {
                queries::invalidate_deals_cache(&state.pool, location.id, expired_id).await?;
            }

            if !valid_deals.is_empty() && expired_week_ids.is_empty() {
                state.resolve_deals_hash(location.id, &week_id, &valid_deals);
                return Ok((valid_deals, week_id, true));
            }

            if !valid_deals.is_empty() {
                // Some deals still valid but need to refetch expired ones
                tracing::info!(
                    "Some deal batches expired for location {}, refetching",
                    location.id
                );
            }
        }

        match state.deals_tracker.try_acquire(&key) {
            AcquireResult::Wait(notify) => {
                tracing::debug!("Deals fetch already in-flight for {key}, waiting");
                notify.notified().await;
            }
            AcquireResult::Lead(guard) => {
                let (new_deals, week_id) =
                    fetch_deals_from_source(state, location).await?;

                // Combine newly fetched deals with any still-valid cached deals
                let all_deals = if let Some((existing, _)) =
                    queries::get_current_deals(&state.pool, location.id).await?
                {
                    let existing_names: HashSet<String> =
                        existing.iter().map(|deal| deal.item_name.clone()).collect();
                    let mut combined = existing;
                    for deal in new_deals {
                        if !existing_names.contains(&deal.item_name) {
                            combined.push(deal);
                        }
                    }
                    combined
                } else {
                    new_deals
                };

                state.resolve_deals_hash(location.id, &week_id, &all_deals);
                drop(guard);
                return Ok((all_deals, week_id, false));
            }
        }
    }
}

/// Separate deals into valid and expired, returning the expired week_ids.
fn partition_expired_deals(deals: Vec<Deal>) -> (Vec<Deal>, Vec<String>) {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut valid = Vec::new();
    let mut expired_week_ids = std::collections::HashSet::new();

    for deal in deals {
        let is_expired = deal.valid_to.as_ref().map_or(false, |valid_to| {
            let date_part = valid_to.split('T').next().unwrap_or(valid_to);
            date_part < today.as_str()
        });

        if is_expired {
            expired_week_ids.insert(deal.week_id.clone());
        } else {
            valid.push(deal);
        }
    }

    (valid, expired_week_ids.into_iter().collect())
}

/// Dispatches to the appropriate fetch strategy (Flipp or Vision).
/// Returns `(deals, week_id)`.
async fn fetch_deals_from_source(
    state: &AppState,
    location: &StoreLocation,
) -> Result<(Vec<Deal>, String), AppError> {
    if location.flipp_merchant_id.is_some() {
        fetch_and_cache_flipp_deals(state, location).await
    } else if location.chain_id == "h-mart" {
        fetch_and_cache_hmart_deals(state, location).await
    } else {
        let week_id = queries::current_week_id();
        let deals = fetch_and_cache_vision_deals(state, location, &week_id).await?;
        Ok((deals, week_id))
    }
}

async fn fetch_and_cache_flipp_deals(
    state: &AppState,
    location: &StoreLocation,
) -> Result<(Vec<Deal>, String), AppError> {
    let client = reqwest::Client::new();

    let flyers = flipp::search_flyers_by_zip(&client, &location.zip_code).await?;

    let flyer = flyers.iter().find(|f| {
        f.merchant_id == location.flipp_merchant_id
            || f.chain_id == location.chain_id
    });

    let valid_from = flyer.and_then(|f| f.valid_from.clone());
    let valid_to = flyer.and_then(|f| f.valid_to.clone());
    let week_id = valid_from
        .as_deref()
        .map(flipp::week_id_from_valid_from)
        .unwrap_or_else(queries::current_week_id);

    let flyer_id = match flyer.and_then(|f| f.flyer_id) {
        Some(id) => id,
        None => {
            tracing::warn!(
                "No current flyer found for location {} (chain: {})",
                location.id,
                location.chain_id
            );
            return Ok((vec![], week_id));
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

    let mut all_categories: HashMap<String, String> = HashMap::new();

    if !vision_items.is_empty() {
        tracing::info!("{} items need vision extraction", vision_items.len());

        let vision_item_names: HashSet<&str> = vision_items
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();

        let ready_items: Vec<(String, Option<String>)> = deal_tuples
            .iter()
            .filter(|(name, _, _, _, _)| !vision_item_names.contains(name.as_str()))
            .map(|(name, brand, _, _, _)| (name.clone(), brand.clone()))
            .collect();

        // Pre-categorize enough items that the remainder + vision items
        // fit in a single final batch
        let keep_for_final = crate::ai::categorize::BATCH_SIZE
            .saturating_sub(vision_items.len());
        let pre_categorize_count = ready_items.len().saturating_sub(keep_for_final);

        if pre_categorize_count > 0 {
            tracing::info!(
                "Pre-categorizing {} items concurrently with vision extraction",
                pre_categorize_count
            );

            let (vision_result, pre_cat_result) = tokio::join!(
                crate::ai::extract_deals::extract_deals_from_images(
                    &state.ai, &client, &vision_items
                ),
                crate::ai::categorize::categorize_items(
                    &state.ai, &ready_items[..pre_categorize_count]
                ),
            );

            apply_vision_results(&mut deal_tuples, vision_result);

            match pre_cat_result {
                Ok(categories) => all_categories.extend(categories),
                Err(error) => tracing::warn!("Pre-categorization failed: {error}"),
            }
        } else {
            let vision_result = crate::ai::extract_deals::extract_deals_from_images(
                &state.ai, &client, &vision_items,
            )
            .await;
            apply_vision_results(&mut deal_tuples, vision_result);
        }
    }

    // Categorize remaining uncategorized items
    let remaining_items: Vec<(String, Option<String>)> = deal_tuples
        .iter()
        .filter(|(name, _, _, _, _)| !all_categories.contains_key(name))
        .map(|(name, brand, _, _, _)| (name.clone(), brand.clone()))
        .collect();

    if !remaining_items.is_empty() {
        match crate::ai::categorize::categorize_items(&state.ai, &remaining_items).await {
            Ok(categories) => all_categories.extend(categories),
            Err(error) => {
                tracing::warn!("AI categorization failed, using 'uncategorized': {error}");
            }
        }
    }

    for deal in &mut deal_tuples {
        if let Some(category) = all_categories.get(&deal.0) {
            deal.3 = category.clone();
        }
    }
    let before = deal_tuples.len();
    deal_tuples.retain(|deal| deal.3 != "not_food");
    tracing::info!(
        "AI categorized {} items, filtered {} non-food",
        all_categories.len(),
        before - deal_tuples.len()
    );

    queries::save_deals(
        &state.pool,
        location.id,
        &week_id,
        &deal_tuples,
        valid_from.as_deref(),
        valid_to.as_deref(),
    )
    .await?;

    let deals = queries::get_cached_deals(&state.pool, location.id, &week_id)
        .await?
        .unwrap_or_default();

    Ok((deals, week_id))
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

    queries::save_deals(&state.pool, location.id, week_id, &deal_tuples, None, None).await?;

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

fn apply_vision_results(
    deal_tuples: &mut Vec<(String, Option<String>, String, String, Option<String>)>,
    result: Result<HashMap<String, String>, AppError>,
) {
    match result {
        Ok(extracted) => {
            for deal in deal_tuples.iter_mut() {
                if deal.2 == "On Sale" {
                    if let Some(description) = extracted.get(&deal.0) {
                        deal.2 = description.clone();
                    }
                }
            }
            deal_tuples.retain(|deal| deal.2 != "NOT_A_DEAL");
            tracing::info!("Vision extracted deals for {} items", extracted.len());
        }
        Err(error) => {
            tracing::warn!("Vision deal extraction failed: {error}");
        }
    }
}

async fn fetch_and_cache_hmart_deals(
    state: &AppState,
    location: &StoreLocation,
) -> Result<(Vec<Deal>, String), AppError> {
    let result = hmart::fetch_hmart_deals(state, location).await?;

    // Save weekly deals
    if !result.weekly_deals.is_empty() {
        queries::save_deals(
            &state.pool,
            location.id,
            &result.weekly_week_id,
            &result.weekly_deals,
            Some(&result.weekly_valid_from),
            Some(&result.weekly_valid_to),
        )
        .await?;
    }

    // Save monthly deals separately (if any)
    if !result.monthly_deals.is_empty() {
        if let Some(monthly_week_id) = &result.monthly_week_id {
            let today = chrono::Utc::now().date_naive();
            let month_start = today
                .with_day(1)
                .map(|date: chrono::NaiveDate| date.format("%Y-%m-%dT00:00:00").to_string());
            let month_end = today
                .with_day(1)
                .and_then(|date: chrono::NaiveDate| date.checked_add_months(chrono::Months::new(1)))
                .map(|date| date.pred_opt().unwrap_or(date))
                .map(|date| date.format("%Y-%m-%dT23:59:59").to_string());

            queries::save_deals(
                &state.pool,
                location.id,
                monthly_week_id,
                &result.monthly_deals,
                month_start.as_deref(),
                month_end.as_deref(),
            )
            .await?;
        }
    }

    // Return all deals combined
    let all_deals = queries::get_current_deals(&state.pool, location.id)
        .await?
        .map(|(deals, _)| deals)
        .unwrap_or_default();

    Ok((all_deals, result.weekly_week_id))
}
