use axum::extract::{Path, State};
use axum::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::models::deal::DealsResponse;
use crate::AppState;

pub async fn get_deals(
    State(state): State<AppState>,
    Path(location_id): Path<i64>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = queries::get_location(&state.pool, location_id).await?;
    let week_id = queries::current_week_id();

    // Check cache
    if let Some(deals) = queries::get_cached_deals(&state.pool, location_id, &week_id).await? {
        return Ok(Json(DealsResponse {
            location_id,
            week_id,
            deals,
            cached: true,
        }));
    }

    // Fetch from Flipp if this location has a merchant ID
    if location.flipp_merchant_id.is_some() {
        let deals =
            fetch_and_cache_flipp_deals(&state, &location, &week_id).await?;
        return Ok(Json(DealsResponse {
            location_id,
            week_id,
            deals,
            cached: false,
        }));
    }

    Ok(Json(DealsResponse {
        location_id,
        week_id,
        deals: vec![],
        cached: false,
    }))
}

pub async fn refresh_deals(
    State(state): State<AppState>,
    Path(location_id): Path<i64>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = queries::get_location(&state.pool, location_id).await?;
    let week_id = queries::current_week_id();

    if location.flipp_merchant_id.is_some() {
        let deals =
            fetch_and_cache_flipp_deals(&state, &location, &week_id).await?;
        return Ok(Json(DealsResponse {
            location_id,
            week_id,
            deals,
            cached: false,
        }));
    }

    Ok(Json(DealsResponse {
        location_id,
        week_id,
        deals: vec![],
        cached: false,
    }))
}

async fn fetch_and_cache_flipp_deals(
    state: &AppState,
    location: &crate::models::location::StoreLocation,
    week_id: &str,
) -> Result<Vec<crate::models::deal::Deal>, AppError> {
    let client = reqwest::Client::new();

    let flyers = flipp::search_flyers_by_zip(&client, &location.zip_code).await?;

    let flyer = flyers.iter().find(|f| {
        f.merchant_id == location.flipp_merchant_id
            || f.chain_id == location.chain_id
    });

    let flyer_id = match flyer {
        Some(f) => f.flyer_id,
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
                tracing::info!("Vision extracted deals for {} items", extracted.len());
            }
            Err(err) => {
                tracing::warn!("Vision deal extraction failed: {err}");
            }
        }
    }

    // AI categorization
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
            tracing::info!("AI categorized {} items", categories.len());
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
