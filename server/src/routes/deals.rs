use axum::extract::{Path, State};
use axum::Json;
use sqlx::SqlitePool;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::models::deal::DealsResponse;

pub async fn get_deals(
    State(pool): State<SqlitePool>,
    Path(location_id): Path<i64>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = queries::get_location(&pool, location_id).await?;
    let week_id = queries::current_week_id();

    // Check cache
    if let Some(deals) = queries::get_cached_deals(&pool, location_id, &week_id).await? {
        return Ok(Json(DealsResponse {
            location_id,
            week_id,
            deals,
            cached: true,
        }));
    }

    // Fetch from Flipp if this location has a merchant ID
    if let Some(_merchant_id) = location.flipp_merchant_id {
        let deals = fetch_and_cache_flipp_deals(&pool, &location, &week_id).await?;
        return Ok(Json(DealsResponse {
            location_id,
            week_id,
            deals,
            cached: false,
        }));
    }

    // No Flipp merchant and no cache - return empty (Vision fallback in Phase 5)
    Ok(Json(DealsResponse {
        location_id,
        week_id,
        deals: vec![],
        cached: false,
    }))
}

pub async fn refresh_deals(
    State(pool): State<SqlitePool>,
    Path(location_id): Path<i64>,
) -> Result<Json<DealsResponse>, AppError> {
    let location = queries::get_location(&pool, location_id).await?;
    let week_id = queries::current_week_id();

    if location.flipp_merchant_id.is_some() {
        let deals = fetch_and_cache_flipp_deals(&pool, &location, &week_id).await?;
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
    pool: &SqlitePool,
    location: &crate::models::location::StoreLocation,
    week_id: &str,
) -> Result<Vec<crate::models::deal::Deal>, AppError> {
    let client = reqwest::Client::new();

    // We need the flyer ID. For now, search for the current flyer by zip + merchant
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
    let deal_tuples = flipp::items_to_deal_tuples(&items);

    tracing::info!(
        "Fetched {} items from Flipp flyer {} for location {}",
        deal_tuples.len(),
        flyer_id,
        location.id
    );

    // Save to cache
    queries::save_deals(pool, location.id, week_id, &deal_tuples).await?;

    // Return cached deals (they now have IDs)
    let deals = queries::get_cached_deals(pool, location.id, week_id)
        .await?
        .unwrap_or_default();

    Ok(deals)
}
