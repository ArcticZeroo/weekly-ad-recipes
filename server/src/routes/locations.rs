use axum::extract::{Path, Query, State};
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

pub async fn create_location(
    State(state): State<AppState>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<StoreLocation>, AppError> {
    let location = queries::create_location(&state.pool, &req).await?;
    Ok(Json(location))
}

pub async fn delete_location(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    queries::delete_location(&state.pool, id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub zip: String,
}

pub async fn search_locations(
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<flipp::FlippStoreMatch>>, AppError> {
    let client = reqwest::Client::new();
    let matches = flipp::search_flyers_by_zip(&client, &query.zip).await?;
    Ok(Json(matches))
}
