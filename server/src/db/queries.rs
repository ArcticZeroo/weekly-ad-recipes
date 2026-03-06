use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::deal::Deal;
use crate::models::location::{CreateLocationRequest, StoreLocation};
use crate::models::meal::MealIdea;

// ---- Store Locations ----

pub async fn list_locations(pool: &SqlitePool) -> Result<Vec<StoreLocation>, AppError> {
    let locations = sqlx::query_as!(
        StoreLocation,
        r#"SELECT id as "id!", chain_id, name, address, zip_code,
           flipp_merchant_id, flipp_merchant_name, weekly_ad_url, created_at
           FROM store_locations ORDER BY created_at DESC"#
    )
    .fetch_all(pool)
    .await?;

    Ok(locations)
}

pub async fn get_location(pool: &SqlitePool, id: i64) -> Result<StoreLocation, AppError> {
    sqlx::query_as!(
        StoreLocation,
        r#"SELECT id as "id!", chain_id, name, address, zip_code,
           flipp_merchant_id, flipp_merchant_name, weekly_ad_url, created_at
           FROM store_locations WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Location {id} not found")))
}

pub async fn create_location(
    pool: &SqlitePool,
    req: &CreateLocationRequest,
) -> Result<StoreLocation, AppError> {
    let result = sqlx::query!(
        r#"INSERT INTO store_locations (chain_id, name, address, zip_code,
           flipp_merchant_id, flipp_merchant_name, weekly_ad_url)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        req.chain_id,
        req.name,
        req.address,
        req.zip_code,
        req.flipp_merchant_id,
        req.flipp_merchant_name,
        req.weekly_ad_url,
    )
    .execute(pool)
    .await
    .map_err(|err| match &err {
        sqlx::Error::Database(db_err) if db_err.message().contains("UNIQUE") => {
            AppError::BadRequest(format!(
                "A {} location for zip {} already exists",
                req.chain_id, req.zip_code
            ))
        }
        _ => AppError::Database(err),
    })?;

    get_location(pool, result.last_insert_rowid()).await
}

pub async fn delete_location(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    // Delete cascaded data first
    sqlx::query!("DELETE FROM deals WHERE location_id = ?", id)
        .execute(pool)
        .await?;
    sqlx::query!("DELETE FROM meal_ideas WHERE location_id = ?", id)
        .execute(pool)
        .await?;

    let result = sqlx::query!("DELETE FROM store_locations WHERE id = ?", id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Location {id} not found")));
    }

    Ok(())
}

// ---- Deals ----

pub async fn get_cached_deals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
) -> Result<Option<Vec<Deal>>, AppError> {
    let deals = sqlx::query_as!(
        Deal,
        r#"SELECT id as "id!", location_id as "location_id!", week_id, item_name, brand,
           deal_description, category, image_url, fetched_at
           FROM deals WHERE location_id = ? AND week_id = ?
           ORDER BY category, item_name"#,
        location_id,
        week_id
    )
    .fetch_all(pool)
    .await?;

    if deals.is_empty() {
        Ok(None)
    } else {
        Ok(Some(deals))
    }
}

pub async fn save_deals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
    deals: &[(String, Option<String>, String, String, Option<String>)],
) -> Result<(), AppError> {
    sqlx::query!(
        "DELETE FROM deals WHERE location_id = ? AND week_id = ?",
        location_id,
        week_id
    )
    .execute(pool)
    .await?;

    for (item_name, brand, deal_description, category, image_url) in deals {
        sqlx::query!(
            r#"INSERT INTO deals (location_id, week_id, item_name, brand,
               deal_description, category, image_url)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            location_id,
            week_id,
            item_name,
            brand,
            deal_description,
            category,
            image_url,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

// ---- Meal Ideas ----

pub async fn get_cached_meals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
) -> Result<Option<Vec<MealIdea>>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT id as "id!", location_id as "location_id!", week_id, name, description,
           on_sale_ingredients, additional_ingredients, estimated_savings, fetched_at
           FROM meal_ideas WHERE location_id = ? AND week_id = ?"#,
        location_id,
        week_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let meals = rows
        .into_iter()
        .map(|row| {
            let on_sale: Vec<String> =
                serde_json::from_str(&row.on_sale_ingredients).unwrap_or_default();
            let additional: Vec<String> =
                serde_json::from_str(&row.additional_ingredients).unwrap_or_default();

            MealIdea {
                id: row.id,
                location_id: row.location_id,
                week_id: row.week_id,
                name: row.name,
                description: row.description,
                on_sale_ingredients: on_sale,
                additional_ingredients: additional,
                estimated_savings: row.estimated_savings,
                fetched_at: row.fetched_at,
            }
        })
        .collect();

    Ok(Some(meals))
}

pub async fn save_meals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
    meals: &[(String, String, Vec<String>, Vec<String>, String)],
) -> Result<(), AppError> {
    sqlx::query!(
        "DELETE FROM meal_ideas WHERE location_id = ? AND week_id = ?",
        location_id,
        week_id
    )
    .execute(pool)
    .await?;

    for (name, description, on_sale, additional, savings) in meals {
        let on_sale_json = serde_json::to_string(on_sale).unwrap_or_default();
        let additional_json = serde_json::to_string(additional).unwrap_or_default();

        sqlx::query!(
            r#"INSERT INTO meal_ideas (location_id, week_id, name, description,
               on_sale_ingredients, additional_ingredients, estimated_savings)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            location_id,
            week_id,
            name,
            description,
            on_sale_json,
            additional_json,
            savings,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Get the current ISO week ID (e.g., "2026-W10")
pub fn current_week_id() -> String {
    let now = chrono::Utc::now();
    format!("{}-W{:02}", now.format("%G"), now.format("%V"))
}
