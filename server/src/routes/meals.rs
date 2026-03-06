use axum::extract::{Path, State};
use axum::Json;
use sqlx::SqlitePool;

use crate::db::queries;
use crate::error::AppError;
use crate::models::meal::MealsResponse;

pub async fn get_meals(
    State(pool): State<SqlitePool>,
    Path(location_id): Path<i64>,
) -> Result<Json<MealsResponse>, AppError> {
    let _location = queries::get_location(&pool, location_id).await?;
    let week_id = queries::current_week_id();

    // Check cache
    if let Some(meals) = queries::get_cached_meals(&pool, location_id, &week_id).await? {
        return Ok(Json(MealsResponse {
            location_id,
            week_id,
            meals,
            cached: true,
        }));
    }

    // Placeholder: will be implemented in Phase 4 (AI integration)
    Ok(Json(MealsResponse {
        location_id,
        week_id,
        meals: vec![],
        cached: false,
    }))
}
