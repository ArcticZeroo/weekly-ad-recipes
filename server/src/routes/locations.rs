use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::flipp;
use crate::models::location::{CreateLocationRequest, StoreLocation};

pub async fn list_locations(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<StoreLocation>>, AppError> {
    let locations = queries::list_locations(&pool).await?;
    Ok(Json(locations))
}

pub async fn create_location(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateLocationRequest>,
) -> Result<Json<StoreLocation>, AppError> {
    let location = queries::create_location(&pool, &req).await?;
    Ok(Json(location))
}

pub async fn delete_location(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    queries::delete_location(&pool, id).await?;
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
