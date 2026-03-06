use axum::extract::{Path, State};
use axum::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::models::meal::MealsResponse;
use crate::AppState;

pub async fn get_meals(
    State(state): State<AppState>,
    Path(location_id): Path<i64>,
) -> Result<Json<MealsResponse>, AppError> {
    let _location = queries::get_location(&state.pool, location_id).await?;
    let week_id = queries::current_week_id();

    // Check cache
    if let Some(meals) = queries::get_cached_meals(&state.pool, location_id, &week_id).await? {
        return Ok(Json(MealsResponse {
            location_id,
            week_id,
            meals,
            cached: true,
        }));
    }

    // Ensure we have deals first
    let deals = queries::get_cached_deals(&state.pool, location_id, &week_id)
        .await?
        .unwrap_or_default();

    if deals.is_empty() {
        return Ok(Json(MealsResponse {
            location_id,
            week_id,
            meals: vec![],
            cached: false,
        }));
    }

    // Generate meal ideas via AI
    let meal_tuples = crate::ai::meals::generate_meal_ideas(&state.ai, &deals).await?;

    tracing::info!(
        "AI generated {} meal ideas for location {}",
        meal_tuples.len(),
        location_id
    );

    queries::save_meals(&state.pool, location_id, &week_id, &meal_tuples).await?;

    let meals = queries::get_cached_meals(&state.pool, location_id, &week_id)
        .await?
        .unwrap_or_default();

    Ok(Json(MealsResponse {
        location_id,
        week_id,
        meals,
        cached: false,
    }))
}
