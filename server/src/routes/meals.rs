use axum::extract::{Path, State};
use axum::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::models::meal::MealsResponse;
use crate::routes::locations::resolve_or_create_location;
use crate::AppState;

pub async fn get_meals(
    State(state): State<AppState>,
    Path((chain, zip)): Path<(String, String)>,
) -> Result<Json<MealsResponse>, AppError> {
    let location = resolve_or_create_location(&state, &chain, &zip).await?;
    let week_id = queries::current_week_id();

    // Check cache
    if let Some(meals) = queries::get_cached_meals(&state.pool, location.id, &week_id).await? {
        return Ok(Json(MealsResponse {
            chain_id: chain,
            zip_code: zip,
            week_id,
            meals,
            cached: true,
        }));
    }

    // Ensure we have deals first
    let deals = queries::get_cached_deals(&state.pool, location.id, &week_id)
        .await?
        .unwrap_or_default();

    if deals.is_empty() {
        return Ok(Json(MealsResponse {
            chain_id: chain,
            zip_code: zip,
            week_id,
            meals: vec![],
            cached: false,
        }));
    }

    // Generate meal ideas via AI
    let meal_tuples = crate::ai::meals::generate_meal_ideas(&state.ai, &deals).await?;

    tracing::info!(
        "AI generated {} meal ideas for {}/{}",
        meal_tuples.len(),
        chain,
        zip
    );

    queries::save_meals(&state.pool, location.id, &week_id, &meal_tuples).await?;

    let meals = queries::get_cached_meals(&state.pool, location.id, &week_id)
        .await?
        .unwrap_or_default();

    Ok(Json(MealsResponse {
        chain_id: chain,
        zip_code: zip,
        week_id,
        meals,
        cached: false,
    }))
}
