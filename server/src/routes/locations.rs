use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::models::location::{CreateLocationRequest, StoreLocation};
use crate::AppState;

pub async fn list_locations(
    State(state): State<AppState>,
) -> Result<Json<Vec<StoreLocation>>, AppError> {
    let locations = queries::list_locations(&state.pool).await?;
    Ok(Json(locations))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub zip: String,
}

/// Search for stores by zip code. Returns lightweight results without DB writes.
pub async fn search_locations(
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<flipp::FlippStoreMatch>>, AppError> {
    let client = reqwest::Client::new();
    let mut matches = flipp::search_flyers_by_zip(&client, &query.zip).await?;

    // Whole Foods uses structured scraping instead of Flipp
    matches.push(flipp::FlippStoreMatch {
        chain_id: "whole-foods".to_string(),
        chain_name: "Whole Foods".to_string(),
        flyer_id: None,
        merchant_id: None,
        merchant_name: "Whole Foods Market".to_string(),
        store_name: Some("Redmond".to_string()),
        valid_from: None,
        valid_to: None,
    });

    Ok(Json(matches))
}

#[derive(Deserialize)]
pub struct ResolveRequest {
    pub chain_id: String,
    pub chain_name: String,
    pub zip_code: String,
    pub flipp_merchant_id: Option<i64>,
    pub flipp_merchant_name: Option<String>,
}

/// Find-or-create a location record, returning the stable ID.
/// Keyed on (chain_id, zip_code) — same chain in different regions
/// may have slightly different flyers.
pub async fn resolve_location(
    State(state): State<AppState>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<StoreLocation>, AppError> {
    if let Some(existing) =
        queries::find_location_by_chain_zip(&state.pool, &req.chain_id, &req.zip_code).await?
    {
        return Ok(Json(existing));
    }

    // For Whole Foods, set the weekly_ad_url with the hardcoded store ID
    let weekly_ad_url = if req.chain_id == "whole-foods" {
        // TODO: Look up WFM store ID dynamically based on zip
        Some("https://www.wholefoodsmarket.com/sales-flyer?store-id=10260".to_string())
    } else {
        None
    };

    let create_req = CreateLocationRequest {
        chain_id: req.chain_id.clone(),
        name: format!("{} - {}", req.chain_name, req.zip_code),
        address: None,
        zip_code: req.zip_code,
        flipp_merchant_id: req.flipp_merchant_id,
        flipp_merchant_name: req.flipp_merchant_name,
        weekly_ad_url,
    };

    let location = queries::create_location(&state.pool, &create_req).await?;
    Ok(Json(location))
}
