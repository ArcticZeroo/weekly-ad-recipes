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

    let (deals, week_id) = match queries::get_current_deals(&state.pool, location.id).await? {
        Some((deals, week_id)) if !queries::are_deals_expired(&deals) => (deals, week_id),
        _ => {
            return Ok(Json(MealsResponse {
                chain_id: chain,
                zip_code: zip,
                valid_from: None,
                valid_to: None,
                meals: vec![],
                deals: vec![],
                cached: false,
            }));
        }
    };

    let valid_from = deals.first().and_then(|deal| deal.valid_from.clone());
    let valid_to = deals.first().and_then(|deal| deal.valid_to.clone());

    let key = format!("{}:{}", location.id, week_id);

    let deals_hash = state.resolve_deals_hash(location.id, &week_id, &deals);

    loop {
        if let Some((meals, stored_hash)) =
            queries::get_cached_meals(&state.pool, location.id, &week_id).await?
        {
            if stored_hash == deals_hash {
                return Ok(Json(MealsResponse {
                    chain_id: chain,
                    zip_code: zip,
                    valid_from: valid_from.clone(),
                    valid_to: valid_to.clone(),
                    meals,
                    deals,
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
                    valid_from,
                    valid_to,
                    meals,
                    deals,
                    cached: false,
                }));
            }
        }
    }
}
