use regex::Regex;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::fetcher::zip_geo::{self, ZipGeo};
use crate::models::deal::Deal;
use crate::models::location::StoreLocation;
use crate::AppState;

const HMART_WA_STORE_ZIPS: &[(&str, &str)] = &[
    ("98101", "H Mart - Seattle"),
    ("98003", "H Mart - Federal Way"),
    ("98052", "H Mart - Redmond"),
    ("98037", "H Mart - Lynnwood"),
    ("98105", "H Mart - UW"),
    ("98499", "H Mart - Tacoma"),
    ("98109", "H Mart - District H"),
    ("98107", "H Mart - Ballard"),
    ("98004", "H Mart - Bellevue"),
];

const HMART_WA_WEEKLY_AD_URL: &str = "https://www.hmartus.com/weekly-sale-wa";
const MAX_STORE_DISTANCE_KM: f64 = 80.0;

type DealTuple = (String, Option<String>, String, String, Option<String>);

/// Find the nearest H Mart WA store to the given zip code.
/// Returns `(store_name, weekly_ad_url)` if one is within range.
pub fn find_nearest_hmart_wa_store(zip_geo: &ZipGeo, zip: &str) -> Option<(String, String)> {
    let (user_latitude, user_longitude) = zip_geo.lookup(zip)?;

    let nearest = HMART_WA_STORE_ZIPS
        .iter()
        .filter_map(|(store_zip, store_name)| {
            let (store_latitude, store_longitude) = zip_geo.lookup(store_zip)?;
            let distance = zip_geo::haversine_distance_km(
                user_latitude,
                user_longitude,
                store_latitude,
                store_longitude,
            );
            Some((store_name, distance))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    match nearest {
        Some((name, distance)) if distance <= MAX_STORE_DISTANCE_KM => {
            Some((name.to_string(), HMART_WA_WEEKLY_AD_URL.to_string()))
        }
        _ => None,
    }
}

/// Compute the H Mart week ID by looking at what's currently cached.
/// If no cached deals exist, returns None — the caller should fetch fresh deals
/// and use the valid dates extracted from the flyer image.
pub async fn current_hmart_week_id(pool: &SqlitePool) -> Result<Option<String>, AppError> {
    let row = sqlx::query_scalar!(
        "SELECT DISTINCT d.week_id FROM deals d \
         JOIN store_locations sl ON d.location_id = sl.id \
         WHERE sl.chain_id = 'h-mart' \
         ORDER BY d.week_id DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Build a week_id from the valid dates extracted by Vision from the flyer.
/// Format: "hmart-YYYYMMDD-YYYYMMDD" (e.g., "hmart-20260306-20260312")
pub fn week_id_from_valid_dates(valid_from: &str, valid_to: &str) -> String {
    let from_clean = valid_from.replace(['/', '-', '.'], "");
    let to_clean = valid_to.replace(['/', '-', '.'], "");
    format!("hmart-{}-{}", from_clean, to_clean)
}

/// Fetch H Mart deals via the vision pipeline.
///
/// If a sibling H Mart location already has deals for this ad period, copies them
/// instead of making another Vision API call.
///
/// Returns `(deals, week_id)` — the week_id is derived from the flyer's valid dates.
pub async fn fetch_hmart_deals(
    state: &AppState,
    location: &StoreLocation,
) -> Result<(Vec<DealTuple>, String), AppError> {
    // Check if we already have H Mart deals cached
    if let Some(existing_week_id) = current_hmart_week_id(&state.pool).await? {
        if let Some(sibling_deals) = fetch_sibling_deals(&state.pool, &existing_week_id).await? {
            tracing::info!(
                "Copying {} deals from sibling H Mart location (week: {})",
                sibling_deals.len(),
                existing_week_id
            );
            return Ok((sibling_deals_to_tuples(&sibling_deals), existing_week_id));
        }
    }

    // Fetch the flyer image and extract deals + valid dates via Vision
    let image_bytes = fetch_hmart_flyer_image().await?;

    tracing::info!(
        "Sending H Mart flyer image ({} bytes) to Vision AI for location {}",
        image_bytes.len(),
        location.id
    );

    let (mut deal_tuples, valid_from, valid_to) =
        extract_hmart_deals_with_dates(&state.ai, &image_bytes).await?;

    let week_id = week_id_from_valid_dates(&valid_from, &valid_to);
    tracing::info!(
        "H Mart flyer valid {valid_from} to {valid_to} (week_id: {week_id}), extracted {} raw deals",
        deal_tuples.len()
    );

    // Check if we already have deals for this specific ad period (from a previous fetch)
    if let Some(sibling_deals) = fetch_sibling_deals(&state.pool, &week_id).await? {
        tracing::info!("Deals already cached for {week_id}, reusing");
        return Ok((sibling_deals_to_tuples(&sibling_deals), week_id));
    }

    // Categorize
    let items_for_categorization: Vec<(String, Option<String>)> = deal_tuples
        .iter()
        .map(|(name, brand, _, _, _)| (name.clone(), brand.clone()))
        .collect();

    match crate::ai::categorize::categorize_items(&state.ai, &items_for_categorization).await {
        Ok(categories) => {
            for deal in &mut deal_tuples {
                if let Some(category) = categories.get(&deal.0) {
                    deal.3 = category.clone();
                }
            }
            let before = deal_tuples.len();
            deal_tuples.retain(|deal| deal.3 != "not_food");
            tracing::info!(
                "Categorized {} H Mart deals, filtered {} non-food",
                categories.len(),
                before - deal_tuples.len()
            );
        }
        Err(error) => {
            tracing::warn!("AI categorization failed for H Mart deals: {error}");
        }
    }

    Ok((deal_tuples, week_id))
}

/// Extract deals AND valid dates from the H Mart flyer image in a single Vision call.
async fn extract_hmart_deals_with_dates(
    ai: &crate::ai::client::AnthropicClient,
    image_bytes: &[u8],
) -> Result<(Vec<DealTuple>, String, String), AppError> {
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        image_bytes,
    );

    let content_blocks = vec![
        serde_json::json!({
            "type": "text",
            "text": "This is an H Mart grocery weekly ad flyer:"
        }),
        serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/jpeg",
                "data": b64
            }
        }),
        serde_json::json!({
            "type": "text",
            "text": "Extract two things from this flyer:\n\
                     1. The valid date range (e.g., \"03/06\" to \"03/12\", or \"March 6\" to \"March 12\"). \
                        Include the year if shown, otherwise assume the current year.\n\
                     2. All grocery deals with item name, brand (if shown), and deal description (price or discount).\n\n\
                     Respond with ONLY a JSON object in this format:\n\
                     {\n\
                       \"valid_from\": \"YYYYMMDD\",\n\
                       \"valid_to\": \"YYYYMMDD\",\n\
                       \"deals\": [{\"item_name\": \"...\", \"brand\": \"...\", \"deal_description\": \"...\", \"category\": \"...\"}]\n\
                     }\n\
                     Categories: produce, meat, dairy, bakery, frozen, pantry, beverages, snacks, deli, seafood.\n\
                     If brand is unknown, use null. Output only the JSON object."
        }),
    ];

    let response = ai
        .send_with_images(
            "claude-sonnet-4-20250514",
            4096,
            content_blocks,
        )
        .await?;

    let json_str = extract_json_object(&response);

    let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|error| {
        tracing::warn!("Failed to parse H Mart vision response: {error}\nResponse: {response}");
        AppError::Ai(format!("Failed to parse H Mart vision response: {error}"))
    })?;

    let valid_from = parsed
        .get("valid_from")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let valid_to = parsed
        .get("valid_to")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let deals_array = parsed
        .get("deals")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let deal_tuples = deals_array
        .into_iter()
        .filter_map(|item| {
            let name = item.get("item_name")?.as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let brand = item
                .get("brand")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let description = item
                .get("deal_description")
                .and_then(|v| v.as_str())
                .unwrap_or("On Sale")
                .trim()
                .to_string();
            let category = item
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("uncategorized")
                .trim()
                .to_lowercase();

            Some((name, brand, description, category, None))
        })
        .collect();

    Ok((deal_tuples, valid_from, valid_to))
}

fn extract_json_object(text: &str) -> &str {
    let text = text.trim();
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return &text[start..=end];
        }
    }
    text
}

async fn fetch_hmart_flyer_image() -> Result<Vec<u8>, AppError> {
    let client = reqwest::Client::new();
    let image_url = fetch_hmart_deal_image_url(&client).await?;

    tracing::info!("Downloading H Mart flyer image: {}", image_url);

    let image_bytes = client
        .get(&image_url)
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();

    Ok(image_bytes)
}

async fn fetch_hmart_deal_image_url(client: &reqwest::Client) -> Result<String, AppError> {
    let html = client
        .get(HMART_WA_WEEKLY_AD_URL)
        .send()
        .await?
        .text()
        .await?;

    let patterns = [
        Regex::new(r#"data-image="([^"]*Weekly_eng[^"]*)""#).unwrap(),
        Regex::new(r#"data-src="([^"]*Weekly_eng[^"]*)""#).unwrap(),
        Regex::new(r#"data-image="([^"]*HMart\+Weekly_eng[^"]*)""#).unwrap(),
        Regex::new(r#"data-src="([^"]*HMart\+Weekly_eng[^"]*)""#).unwrap(),
    ];

    for pattern in &patterns {
        if let Some(capture) = pattern.captures(&html) {
            if let Some(matched) = capture.get(1) {
                let url = matched.as_str().to_string();
                let url = if url.contains('?') {
                    url
                } else {
                    format!("{}?format=1500w", url)
                };
                return Ok(url);
            }
        }
    }

    Err(AppError::Internal(
        "Could not find English weekly ad image on H Mart WA page".into(),
    ))
}

async fn fetch_sibling_deals(
    pool: &SqlitePool,
    week_id: &str,
) -> Result<Option<Vec<Deal>>, AppError> {
    let deals = sqlx::query_as!(
        Deal,
        r#"SELECT d.id as "id!", d.location_id as "location_id!", d.week_id, d.item_name,
           d.brand, d.deal_description, d.category, d.image_url, d.fetched_at
           FROM deals d
           JOIN store_locations sl ON d.location_id = sl.id
           WHERE sl.chain_id = 'h-mart' AND d.week_id = ?
           ORDER BY d.category, d.item_name"#,
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

fn sibling_deals_to_tuples(deals: &[Deal]) -> Vec<DealTuple> {
    deals
        .iter()
        .map(|deal| {
            (
                deal.item_name.clone(),
                deal.brand.clone(),
                deal.deal_description.clone(),
                deal.category.clone(),
                deal.image_url.clone(),
            )
        })
        .collect()
}
