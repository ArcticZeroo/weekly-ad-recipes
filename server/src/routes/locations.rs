use axum::extract::Query;
use axum::Json;
use serde::Deserialize;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::models::chain::supported_chains;
use crate::models::location::{CreateLocationRequest, StoreLocation};
use crate::AppState;

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

/// Resolve a chain+zip to a StoreLocation, auto-creating if one doesn't exist yet.
pub async fn resolve_or_create_location(
    state: &AppState,
    chain: &str,
    zip: &str,
) -> Result<StoreLocation, AppError> {
    if let Some(existing) = queries::find_location_by_chain_zip(&state.pool, chain, zip).await? {
        return Ok(existing);
    }

    let chain_info = supported_chains()
        .into_iter()
        .find(|c| c.id == chain)
        .ok_or_else(|| AppError::NotFound(format!("Unsupported chain: {chain}")))?;

    let (merchant_id, merchant_name) = if chain_info.uses_flipp {
        let client = reqwest::Client::new();
        let results = flipp::search_flyers_by_zip(&client, zip).await?;
        let found = results.iter().find(|m| m.chain_id == chain);
        (
            found.and_then(|m| m.merchant_id),
            found.map(|m| m.merchant_name.clone()),
        )
    } else {
        (None, None)
    };

    let weekly_ad_url = if chain == "whole-foods" {
        Some("https://www.wholefoodsmarket.com/sales-flyer?store-id=10260".to_string())
    } else {
        None
    };

    let create_req = CreateLocationRequest {
        chain_id: chain.to_string(),
        name: format!("{} - {}", chain_info.name, zip),
        address: None,
        zip_code: zip.to_string(),
        flipp_merchant_id: merchant_id,
        flipp_merchant_name: merchant_name,
        weekly_ad_url,
    };

    let location = queries::create_location(&state.pool, &create_req).await?;
    Ok(location)
}
