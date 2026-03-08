use axum::extract::{Path, State};
use axum::Json;

use crate::db::queries;
use crate::error::AppError;
use crate::inflight::AcquireResult;
use crate::models::meal::MealsResponse;
use crate::routes::locations::resolve_or_create_location;
use crate::AppState;

pub async fn get_meals(
    State(state): State<AppState>,
    Path((chain, zip)): Path<(String, String)>,
) -> Result<Json<MealsResponse>, AppError> {
    let location = resolve_or_create_location(&state, &chain, &zip).await?;
    let week_id = queries::current_week_id();
    let key = format!("{}:{}", location.id, week_id);

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

    let deals_hash = state.resolve_deals_hash(location.id, &week_id, &deals);

    loop {
        if let Some((meals, stored_hash)) =
            queries::get_cached_meals(&state.pool, location.id, &week_id).await?
        {
            if stored_hash == deals_hash {
                return Ok(Json(MealsResponse {
                    chain_id: chain,
                    zip_code: zip,
                    week_id,
                    meals,
                    cached: true,
                }));
            }
            tracing::info!(
                "Deals hash mismatch for {}/{} — invalidating stale meals",
                chain,
                zip
            );
            queries::invalidate_meals_cache(&state.pool, location.id, &week_id).await?;
        }

        match state.meals_tracker.try_acquire(&key) {
            AcquireResult::Wait(notify) => {
                tracing::debug!("Meals fetch already in-flight for {key}, waiting");
                notify.notified().await;
            }
            AcquireResult::Lead(guard) => {
                let meal_tuples =
                    crate::ai::meals::generate_meal_ideas(&state.ai, &deals).await?;

                tracing::info!(
                    "AI generated {} meal ideas for {}/{}",
                    meal_tuples.len(),
                    chain,
                    zip
                );

                queries::save_meals(
                    &state.pool,
                    location.id,
                    &week_id,
                    &meal_tuples,
                    &deals_hash,
                )
                .await?;

                let meals = queries::get_cached_meals(&state.pool, location.id, &week_id)
                    .await?
                    .map(|(meals, _)| meals)
                    .unwrap_or_default();

                drop(guard);
                return Ok(Json(MealsResponse {
                    chain_id: chain,
                    zip_code: zip,
                    week_id,
                    meals,
                    cached: false,
                }));
            }
        }
    }
}
