use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::deal::Deal;
use crate::models::location::{CreateLocationRequest, StoreLocation};
use crate::models::meal::{MealIdea, SaleIngredient};

// ---- Store Locations ----

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

pub async fn find_location_by_chain_zip(
    pool: &SqlitePool,
    chain_id: &str,
    zip_code: &str,
) -> Result<Option<StoreLocation>, AppError> {
    let location = sqlx::query_as!(
        StoreLocation,
        r#"SELECT id as "id!", chain_id, name, address, zip_code,
           flipp_merchant_id, flipp_merchant_name, weekly_ad_url, created_at
           FROM store_locations WHERE chain_id = ? AND zip_code = ?"#,
        chain_id,
        zip_code
    )
    .fetch_optional(pool)
    .await?;

    Ok(location)
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

// ---- Deals ----

pub async fn get_cached_deals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
) -> Result<Option<Vec<Deal>>, AppError> {
    let deals = sqlx::query_as!(
        Deal,
        r#"SELECT id as "id!", location_id as "location_id!", week_id, item_name, brand,
           deal_description, category, image_url, valid_from, valid_to, fetched_at
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

pub async fn invalidate_deals_cache(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        "DELETE FROM deals WHERE location_id = ? AND week_id = ?",
        location_id,
        week_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn invalidate_all_deals_for_location(
    pool: &SqlitePool,
    location_id: i64,
) -> Result<(), AppError> {
    sqlx::query!(
        "DELETE FROM deals WHERE location_id = ?",
        location_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all current (non-expired) deals for a location across all week_ids.
/// Returns `Some((deals, primary_week_id))` if any exist, `None` if empty.
/// The primary_week_id is from the most recently fetched batch (used for hash caching).
pub async fn get_current_deals(
    pool: &SqlitePool,
    location_id: i64,
) -> Result<Option<(Vec<Deal>, String)>, AppError> {
    let all_deals = sqlx::query_as!(
        Deal,
        r#"SELECT id as "id!", location_id as "location_id!", week_id, item_name, brand,
           deal_description, category, image_url, valid_from, valid_to, fetched_at
           FROM deals WHERE location_id = ?
           ORDER BY category, item_name"#,
        location_id
    )
    .fetch_all(pool)
    .await?;

    if all_deals.is_empty() {
        return Ok(None);
    }

    let primary_week_id = all_deals
        .first()
        .map(|deal| deal.week_id.clone())
        .unwrap_or_default();

    Ok(Some((all_deals, primary_week_id)))
}

/// Check if ALL deals in the slice are expired based on their valid_to date.
/// Returns true only if every deal with a valid_to has expired.
/// Deals without valid_to are considered non-expired.
pub fn are_deals_expired(deals: &[Deal]) -> bool {
    if deals.is_empty() {
        return false;
    }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    deals.iter().all(|deal| {
        deal.valid_to.as_ref().map_or(false, |valid_to| {
            let date_part = valid_to.split('T').next().unwrap_or(valid_to);
            date_part < today.as_str()
        })
    })
}

pub async fn save_deals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
    deals: &[(String, Option<String>, String, String, Option<String>)],
    valid_from: Option<&str>,
    valid_to: Option<&str>,
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
               deal_description, category, image_url, valid_from, valid_to)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            location_id,
            week_id,
            item_name,
            brand,
            deal_description,
            category,
            image_url,
            valid_from,
            valid_to,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

// ---- Deals Hashing ----

pub fn compute_deals_hash(deals: &[Deal]) -> String {
    let mut sorted: Vec<(&str, &str)> = deals
        .iter()
        .map(|deal| (deal.item_name.as_str(), deal.deal_description.as_str()))
        .collect();
    sorted.sort();

    let mut hasher = DefaultHasher::new();
    for (name, description) in &sorted {
        name.hash(&mut hasher);
        description.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

// ---- Meal Ideas ----

pub async fn get_cached_meals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
) -> Result<Option<(Vec<MealIdea>, String)>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT id as "id!", location_id as "location_id!", week_id, name, description,
           on_sale_ingredients, additional_ingredients, estimated_savings, deals_hash, fetched_at
           FROM meal_ideas WHERE location_id = ? AND week_id = ?"#,
        location_id,
        week_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let deals_hash = rows[0].deals_hash.clone();

    let meals = rows
        .into_iter()
        .map(|row| {
            let on_sale: Vec<SaleIngredient> =
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

    Ok(Some((meals, deals_hash)))
}

pub async fn invalidate_meals_cache(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        "DELETE FROM meal_ideas WHERE location_id = ? AND week_id = ?",
        location_id,
        week_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn save_meals(
    pool: &SqlitePool,
    location_id: i64,
    week_id: &str,
    meals: &[(String, String, Vec<SaleIngredient>, Vec<String>, String)],
    deals_hash: &str,
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
               on_sale_ingredients, additional_ingredients, estimated_savings, deals_hash)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            location_id,
            week_id,
            name,
            description,
            on_sale_json,
            additional_json,
            savings,
            deals_hash,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

// ---- WFM Stores ----

pub async fn get_known_wfm_slugs(pool: &SqlitePool) -> Result<HashSet<String>, AppError> {
    let rows = sqlx::query_scalar!("SELECT slug FROM wfm_stores")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().collect())
}

pub async fn insert_wfm_store(
    pool: &SqlitePool,
    store_id: &str,
    slug: &str,
    name: &str,
    city: Option<&str>,
    state: Option<&str>,
    zip_code: Option<&str>,
    latitude: f64,
    longitude: f64,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"INSERT OR REPLACE INTO wfm_stores
           (store_id, slug, name, city, state, zip_code, latitude, longitude)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        store_id,
        slug,
        name,
        city,
        state,
        zip_code,
        latitude,
        longitude,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_all_wfm_stores(
    pool: &SqlitePool,
) -> Result<Vec<(String, String, f64, f64)>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT store_id as "store_id!", name, latitude, longitude FROM wfm_stores"#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.store_id, row.name, row.latitude, row.longitude))
        .collect())
}

pub async fn get_wfm_store_lookup(
    pool: &SqlitePool,
    zip_code: &str,
) -> Result<Option<String>, AppError> {
    let store_id = sqlx::query_scalar!(
        "SELECT store_id FROM wfm_store_lookups WHERE zip_code = ?",
        zip_code
    )
    .fetch_optional(pool)
    .await?;
    Ok(store_id)
}

pub async fn save_wfm_store_lookup(
    pool: &SqlitePool,
    zip_code: &str,
    store_id: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        "INSERT OR REPLACE INTO wfm_store_lookups (zip_code, store_id) VALUES (?, ?)",
        zip_code,
        store_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear_wfm_store_lookups(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM wfm_store_lookups")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_wfm_store_by_id(
    pool: &SqlitePool,
    store_id: &str,
) -> Result<Option<(String, String, f64, f64)>, AppError> {
    let row = sqlx::query!(
        r#"SELECT store_id as "store_id!", name, latitude, longitude
           FROM wfm_stores WHERE store_id = ?"#,
        store_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| (row.store_id, row.name, row.latitude, row.longitude)))
}

/// Get the current ISO week ID (e.g., "2026-W10")
pub fn current_week_id() -> String {
    let now = chrono::Utc::now();
    format!("{}-W{:02}", now.format("%G"), now.format("%V"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_deal(item_name: &str, deal_description: &str, valid_to: Option<&str>) -> Deal {
        Deal {
            id: 0,
            location_id: 0,
            week_id: String::new(),
            item_name: item_name.to_string(),
            brand: None,
            deal_description: deal_description.to_string(),
            category: String::new(),
            image_url: None,
            valid_from: None,
            valid_to: valid_to.map(|s| s.to_string()),
            fetched_at: String::new(),
        }
    }

    #[test]
    fn compute_deals_hash_is_sort_independent() {
        let deals_a = vec![
            make_deal("Bananas", "$0.59/lb", None),
            make_deal("Apples", "$1.99/lb", None),
        ];
        let deals_b = vec![
            make_deal("Apples", "$1.99/lb", None),
            make_deal("Bananas", "$0.59/lb", None),
        ];
        assert_eq!(compute_deals_hash(&deals_a), compute_deals_hash(&deals_b));
    }

    #[test]
    fn compute_deals_hash_differs_for_different_deals() {
        let deals_a = vec![make_deal("Bananas", "$0.59/lb", None)];
        let deals_b = vec![make_deal("Oranges", "$2.99/lb", None)];
        assert_ne!(compute_deals_hash(&deals_a), compute_deals_hash(&deals_b));
    }

    #[test]
    fn compute_deals_hash_empty_is_deterministic() {
        let empty: Vec<Deal> = vec![];
        let hash_a = compute_deals_hash(&empty);
        let hash_b = compute_deals_hash(&empty);
        assert_eq!(hash_a, hash_b);
        assert_eq!(hash_a.len(), 16);
    }

    #[test]
    fn are_deals_expired_past_date_returns_true() {
        let deals = vec![make_deal("Milk", "$3.99", Some("2020-01-01T23:59:59-05:00"))];
        assert!(are_deals_expired(&deals));
    }

    #[test]
    fn are_deals_expired_future_date_returns_false() {
        let deals = vec![make_deal("Milk", "$3.99", Some("2099-12-31T23:59:59-05:00"))];
        assert!(!are_deals_expired(&deals));
    }

    #[test]
    fn are_deals_expired_today_returns_false() {
        let today = chrono::Utc::now().format("%Y-%m-%dT23:59:59-05:00").to_string();
        let deals = vec![make_deal("Milk", "$3.99", Some(&today))];
        assert!(!are_deals_expired(&deals));
    }

    #[test]
    fn are_deals_expired_none_valid_to_returns_false() {
        let deals = vec![make_deal("Milk", "$3.99", None)];
        assert!(!are_deals_expired(&deals));
    }

    #[test]
    fn are_deals_expired_empty_slice_returns_false() {
        let deals: Vec<Deal> = vec![];
        assert!(!are_deals_expired(&deals));
    }
}
