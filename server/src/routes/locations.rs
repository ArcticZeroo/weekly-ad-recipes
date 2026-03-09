use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::fetcher::hmart;
use crate::fetcher::wfm_stores;
use crate::models::location::{CreateLocationRequest, StoreLocation};
use crate::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub zip: String,
}

/// Search for stores by zip code. Returns lightweight results without DB writes.
pub async fn search_locations(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<flipp::FlippStoreMatch>>, AppError> {
    let client = reqwest::Client::new();
    let mut matches = flipp::search_flyers_by_zip(&client, &query.zip).await?;

    let wfm_store_name = match wfm_stores::find_nearest_wfm_store(
        &state.pool,
        &state.zip_geo,
        &query.zip,
    )
    .await
    {
        Ok(Some((_, name))) => Some(name),
        _ => None,
    };

    matches.push(flipp::FlippStoreMatch {
        chain_id: "whole-foods".to_string(),
        chain_name: "Whole Foods".to_string(),
        flyer_id: None,
        merchant_id: None,
        merchant_name: "Whole Foods Market".to_string(),
        store_name: wfm_store_name,
        valid_from: None,
        valid_to: None,
    });

    let hmart_store_name =
        hmart::find_nearest_hmart_wa_store(&state.zip_geo, &query.zip).map(|(name, _)| name);

    matches.push(flipp::FlippStoreMatch {
        chain_id: "h-mart".to_string(),
        chain_name: "H Mart".to_string(),
        flyer_id: None,
        merchant_id: None,
        merchant_name: "H Mart".to_string(),
        store_name: hmart_store_name,
        valid_from: None,
        valid_to: None,
    });

    Ok(Json(matches))
}

/// Resolve a chain+zip to a StoreLocation, auto-creating if one doesn't exist yet.
/// Accepts any chain ID — Flipp merchants are auto-discovered, Whole Foods uses its own scraper.
pub async fn resolve_or_create_location(
    state: &AppState,
    chain: &str,
    zip: &str,
) -> Result<StoreLocation, AppError> {
    if let Some(existing) = queries::find_location_by_chain_zip(&state.pool, chain, zip).await? {
        return Ok(existing);
    }

    if chain == "h-mart" {
        let (store_name, weekly_ad_url) = hmart::find_nearest_hmart_wa_store(&state.zip_geo, zip)
            .ok_or_else(|| AppError::NotFound("No H Mart stores near this zip code".into()))?;

        let create_request = CreateLocationRequest {
            chain_id: "h-mart".to_string(),
            name: store_name,
            address: None,
            zip_code: zip.to_string(),
            flipp_merchant_id: None,
            flipp_merchant_name: None,
            weekly_ad_url: Some(weekly_ad_url),
        };
        return queries::create_location(&state.pool, &create_request).await;
    }

    if chain == "whole-foods" {
        let (weekly_ad_url, location_name) =
            resolve_whole_foods_location(state, zip, "Whole Foods").await?;

        let create_req = CreateLocationRequest {
            chain_id: chain.to_string(),
            name: location_name,
            address: None,
            zip_code: zip.to_string(),
            flipp_merchant_id: None,
            flipp_merchant_name: None,
            weekly_ad_url,
        };
        return queries::create_location(&state.pool, &create_req).await;
    }

    // Flipp-based chain: look up the merchant info from a search
    let client = reqwest::Client::new();
    let results = flipp::search_flyers_by_zip(&client, zip).await?;
    let found = results.iter().find(|m| m.chain_id == chain);

    let display_name = found
        .map(|m| m.chain_name.clone())
        .unwrap_or_else(|| chain.to_string());

    let create_req = CreateLocationRequest {
        chain_id: chain.to_string(),
        name: format!("{} - {}", display_name, zip),
        address: None,
        zip_code: zip.to_string(),
        flipp_merchant_id: found.and_then(|m| m.merchant_id),
        flipp_merchant_name: found.map(|m| m.merchant_name.clone()),
        weekly_ad_url: None,
    };

    queries::create_location(&state.pool, &create_req).await
}

async fn resolve_whole_foods_location(
    state: &AppState,
    zip: &str,
    chain_display_name: &str,
) -> Result<(Option<String>, String), AppError> {
    let (store_id, store_name) = wfm_stores::find_nearest_wfm_store(
        &state.pool,
        &state.zip_geo,
        zip,
    )
    .await?
    .ok_or_else(|| {
        AppError::Internal(
            "Whole Foods store catalog is not yet available. Please try again shortly.".into(),
        )
    })?;

    let url = format!(
        "https://www.wholefoodsmarket.com/sales-flyer?store-id={}",
        store_id
    );
    let name = format!("{} - {}", chain_display_name, store_name);
    Ok((Some(url), name))
}
