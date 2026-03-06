use axum::extract::{Path, State};
use axum::Json;
use sqlx::SqlitePool;

use crate::db::queries;
use crate::error::AppError;
use crate::models::deal::DealsResponse;

pub async fn get_deals(
    State(pool): State<SqlitePool>,
    Path(location_id): Path<i64>,
) -> Result<Json<DealsResponse>, AppError> {
    // Verify location exists
    let _location = queries::get_location(&pool, location_id).await?;
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

    // Placeholder: will be implemented in Phase 3/4 (Flipp + AI)
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
    let _location = queries::get_location(&pool, location_id).await?;
    let week_id = queries::current_week_id();

    // Placeholder: force re-fetch will be implemented in Phase 3/4
    Ok(Json(DealsResponse {
        location_id,
        week_id,
        deals: vec![],
        cached: false,
    }))
}
