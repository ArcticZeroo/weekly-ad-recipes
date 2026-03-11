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

pub struct HmartDealsResult {
    pub weekly_deals: Vec<DealTuple>,
    pub weekly_week_id: String,
    pub weekly_valid_from: String,
    pub weekly_valid_to: String,
    pub monthly_deals: Vec<DealTuple>,
    pub monthly_week_id: Option<String>,
}

/// Fetch H Mart deals via the vision pipeline.
///
/// Fetches weekly and monthly ads separately. Monthly ads are cached with a
/// month-based week_id to avoid re-running Vision every week.
pub async fn fetch_hmart_deals(
    state: &AppState,
    _location: &StoreLocation,
) -> Result<HmartDealsResult, AppError> {
    // Check if we already have H Mart weekly deals cached from a sibling
    if let Some(existing_week_id) = current_hmart_week_id(&state.pool).await? {
        if let Some(sibling_deals) = fetch_sibling_deals(&state.pool, &existing_week_id).await? {
            tracing::info!(
                "Copying {} deals from sibling H Mart location (week: {})",
                sibling_deals.len(),
                existing_week_id
            );
            let valid_from = sibling_deals.first().and_then(|d| d.valid_from.clone()).unwrap_or_default();
            let valid_to = sibling_deals.first().and_then(|d| d.valid_to.clone()).unwrap_or_default();
            return Ok(HmartDealsResult {
                weekly_deals: sibling_deals_to_tuples(&sibling_deals),
                weekly_week_id: existing_week_id,
                weekly_valid_from: valid_from,
                weekly_valid_to: valid_to,
                monthly_deals: vec![],
                monthly_week_id: None,
            });
        }
    }

    let client = reqwest::Client::new();
    let (weekly_image_url, page_monthly_urls) = fetch_hmart_ad_image_urls(&client).await?;

    // Run weekly and monthly pipelines in parallel
    let weekly_future = async {
        tracing::info!("Downloading H Mart weekly flyer: {}", weekly_image_url);
        let weekly_bytes = download_image(&client, &weekly_image_url).await?;

        let vision_result =
            extract_hmart_deals_with_dates(&state.ai, &[weekly_bytes.clone()]).await?;

        let week_id = week_id_from_valid_dates(&vision_result.valid_from, &vision_result.valid_to);
        tracing::info!(
            "H Mart weekly flyer valid {} to {} (week_id: {week_id}), extracted {} raw deals",
            vision_result.valid_from,
            vision_result.valid_to,
            vision_result.deals.len()
        );

        // Crop thumbnails from the flyer image
        let thumbnails = crop_and_save_thumbnails(
            &weekly_bytes,
            vision_result.grid_rows,
            vision_result.grid_cols,
            &vision_result.deal_positions,
        );

        // Assign thumbnail URLs to deals
        let mut weekly_deals = vision_result.deals;
        for (deal_index, thumbnail_url) in &thumbnails {
            if let Some(deal) = weekly_deals.get_mut(*deal_index) {
                deal.4 = Some(thumbnail_url.clone());
            }
        }

        categorize_deal_tuples(&state.ai, &mut weekly_deals).await;

        Ok::<_, AppError>((weekly_deals, week_id, vision_result.valid_from, vision_result.valid_to))
    };

    let monthly_future = fetch_monthly_deals_if_needed(state, &client, &page_monthly_urls);

    let (weekly_result, (monthly_deals, monthly_week_id)) =
        tokio::join!(weekly_future, monthly_future);

    let (weekly_deals, week_id, valid_from, valid_to) = weekly_result?;

    Ok(HmartDealsResult {
        weekly_deals,
        weekly_week_id: week_id,
        weekly_valid_from: valid_from,
        weekly_valid_to: valid_to,
        monthly_deals: monthly_deals.unwrap_or_default(),
        monthly_week_id,
    })
}

/// Fetch monthly ad deals, using cached results if available.
/// Returns `(deals, week_id)` if a monthly ad was found.
async fn fetch_monthly_deals_if_needed(
    state: &AppState,
    client: &reqwest::Client,
    page_monthly_urls: &[String],
) -> (Option<Vec<DealTuple>>, Option<String>) {
    let monthly_week_id = current_monthly_id();

    // Check if monthly deals are already cached for any h-mart location
    if let Ok(Some(cached)) = fetch_sibling_deals(&state.pool, &monthly_week_id).await {
        tracing::info!(
            "Using {} cached monthly deals ({})",
            cached.len(),
            monthly_week_id
        );
        return (Some(sibling_deals_to_tuples(&cached)), Some(monthly_week_id));
    }

    // Try to find a monthly ad image: first from the page, then from the popup API
    let mut monthly_url = page_monthly_urls.first().cloned();

    if monthly_url.is_none() {
        if let Ok(Some(popup_url)) = fetch_hmart_popup_image_url(client).await {
            monthly_url = Some(popup_url);
        }
    }

    let monthly_url = match monthly_url {
        Some(url) => url,
        None => return (None, None),
    };

    tracing::info!("Downloading H Mart monthly ad: {}", monthly_url);
    let monthly_bytes = match download_image(client, &monthly_url).await {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!("Failed to download monthly ad: {error}");
            return (None, None);
        }
    };

    let monthly_deals = match extract_hmart_monthly_deals(&state.ai, &monthly_bytes).await {
        Ok(deals) => deals,
        Err(error) => {
            tracing::warn!("Failed to extract monthly ad deals: {error}");
            return (None, None);
        }
    };

    let mut categorized = monthly_deals;
    categorize_deal_tuples(&state.ai, &mut categorized).await;

    (Some(categorized), Some(monthly_week_id))
}

fn current_monthly_id() -> String {
    let now = chrono::Utc::now();
    format!("hmart-monthly-{}", now.format("%Y%m"))
}

/// Extract deals from a monthly ad image (no date extraction needed).
async fn extract_hmart_monthly_deals(
    ai: &crate::ai::client::AnthropicClient,
    image_bytes: &[u8],
) -> Result<Vec<DealTuple>, AppError> {
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        image_bytes,
    );
    let media_type = detect_image_media_type(image_bytes);

    let content_blocks = vec![
        serde_json::json!({
            "type": "text",
            "text": "This is an H Mart monthly deals advertisement:"
        }),
        serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": b64
            }
        }),
        serde_json::json!({
            "type": "text",
            "text": "Extract all grocery deals from this monthly ad image.\n\
                     For each deal, provide the item name, brand (if shown), and deal description.\n\n\
                     For deal_description, always include the dollar sign for prices (e.g., \"$2.99/lb\", \"$5.99\", \"2 for $5\").\n\
                     Read prices carefully from the image.\n\n\
                     Respond with ONLY a JSON array of objects:\n\
                     [{\"item_name\": \"...\", \"brand\": \"...\", \"deal_description\": \"...\", \"category\": \"...\"}]\n\
                     Categories: produce, meat, dairy, bakery, frozen, pantry, beverages, snacks, deli, seafood.\n\
                     If brand is unknown, use null. Output only the JSON array."
        }),
    ];

    let response = ai
        .send_with_images("claude-sonnet-4-20250514", 4096, content_blocks)
        .await?;

    let json_str = extract_json_array(&response);

    let items: Vec<std::collections::HashMap<String, serde_json::Value>> =
        serde_json::from_str(json_str).map_err(|error| {
            tracing::warn!("Failed to parse monthly deals response: {error}\nResponse: {response}");
            AppError::Ai(format!("Failed to parse monthly deals: {error}"))
        })?;

    let deals = items
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

    Ok(deals)
}

fn extract_json_array(text: &str) -> &str {
    let text = text.trim();
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return &text[start..=end];
        }
    }
    text
}

async fn categorize_deal_tuples(
    ai: &crate::ai::client::AnthropicClient,
    deal_tuples: &mut Vec<DealTuple>,
) {
    let items_for_categorization: Vec<(String, Option<String>)> = deal_tuples
        .iter()
        .map(|(name, brand, _, _, _)| (name.clone(), brand.clone()))
        .collect();

    match crate::ai::categorize::categorize_items(ai, &items_for_categorization).await {
        Ok(categories) => {
            for deal in deal_tuples.iter_mut() {
                if let Some(category) = categories.get(&deal.0) {
                    deal.3 = category.clone();
                }
            }
            let before = deal_tuples.len();
            deal_tuples.retain(|deal| deal.3 != "not_food");
            tracing::info!(
                "Categorized {} deals, filtered {} non-food",
                categories.len(),
                before - deal_tuples.len()
            );
        }
        Err(error) => {
            tracing::warn!("AI categorization failed: {error}");
        }
    }
}

struct VisionResult {
    deals: Vec<DealTuple>,
    valid_from: String,
    valid_to: String,
    grid_rows: u32,
    grid_cols: u32,
    deal_positions: Vec<(usize, u32, u32)>, // (deal_index, row, col)
}

/// Extract deals AND valid dates from H Mart flyer images in a single Vision call.
async fn extract_hmart_deals_with_dates(
    ai: &crate::ai::client::AnthropicClient,
    images: &[Vec<u8>],
) -> Result<VisionResult, AppError> {
    let mut content_blocks: Vec<serde_json::Value> = Vec::new();

    let label = if images.len() > 1 {
        "These are H Mart grocery ad flyer images (weekly and/or monthly deals):"
    } else {
        "This is an H Mart grocery weekly ad flyer:"
    };

    content_blocks.push(serde_json::json!({
        "type": "text",
        "text": label
    }));

    for (index, image_bytes) in images.iter().enumerate() {
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            image_bytes,
        );
        let media_type = detect_image_media_type(image_bytes);

        if images.len() > 1 {
            content_blocks.push(serde_json::json!({
                "type": "text",
                "text": format!("Image {} of {}:", index + 1, images.len())
            }));
        }

        content_blocks.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": b64
            }
        }));
    }

    content_blocks.push(serde_json::json!({
            "type": "text",
            "text": "Extract two things from this flyer:\n\
                     1. The valid date range (e.g., \"03/06\" to \"03/12\", or \"March 6\" to \"March 12\"). \
                        Include the year if shown, otherwise assume the current year.\n\
                     2. All grocery deals with item name, brand (if shown), and deal description (price or discount).\n\n\
                     For deal_description, always include the dollar sign for prices (e.g., \"$2.99/lb\", \"$5.99\", \"2 for $5\", \"Buy 1 Get 1 Free\"). \
                     Read prices carefully from the image — make sure you're reading the correct price for each item.\n\n\
                     For each deal, also estimate its grid position in the flyer as row and column (0-indexed, \
                     reading left-to-right, top-to-bottom). The flyer is typically laid out in a grid of product tiles.\n\n\
                     Respond with ONLY a JSON object in this format:\n\
                     {\n\
                       \"valid_from\": \"YYYYMMDD\",\n\
                       \"valid_to\": \"YYYYMMDD\",\n\
                       \"grid_rows\": 4,\n\
                       \"grid_cols\": 5,\n\
                       \"deals\": [{\"item_name\": \"...\", \"brand\": \"...\", \"deal_description\": \"...\", \"category\": \"...\", \"row\": 0, \"col\": 0}]\n\
                     }\n\
                     Categories: produce, meat, dairy, bakery, frozen, pantry, beverages, snacks, deli, seafood.\n\
                     If brand is unknown, use null. Output only the JSON object."
    }));

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
    let grid_rows = parsed.get("grid_rows").and_then(|v| v.as_u64()).unwrap_or(4) as u32;
    let grid_cols = parsed.get("grid_cols").and_then(|v| v.as_u64()).unwrap_or(5) as u32;

    let deals_array = parsed
        .get("deals")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut deal_tuples = Vec::new();
    let mut deal_positions = Vec::new();

    for item in &deals_array {
        let name = match item.get("item_name").and_then(|v| v.as_str()) {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => continue,
        };
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

        let row = item.get("row").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let col = item.get("col").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let deal_index = deal_tuples.len();
        deal_tuples.push((name, brand, description, category, None));
        deal_positions.push((deal_index, row, col));
    }

    Ok(VisionResult {
        deals: deal_tuples,
        valid_from,
        valid_to,
        grid_rows,
        grid_cols,
        deal_positions,
    })
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

fn detect_image_media_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png"
    } else if bytes.starts_with(b"GIF") {
        "image/gif"
    } else {
        "image/jpeg"
    }
}

/// Crop deal thumbnails from the flyer image based on grid positions from Vision.
/// Saves each crop to `{base_path}/{hash}.jpg` and returns a map of deal_index → URL path.
fn crop_and_save_thumbnails(
    image_bytes: &[u8],
    grid_rows: u32,
    grid_cols: u32,
    deal_positions: &[(usize, u32, u32)],
) -> std::collections::HashMap<usize, String> {
    crop_and_save_thumbnails_to(
        image_bytes,
        grid_rows,
        grid_cols,
        deal_positions,
        std::path::Path::new("data/thumbnails"),
    )
}

fn crop_and_save_thumbnails_to(
    image_bytes: &[u8],
    grid_rows: u32,
    grid_cols: u32,
    deal_positions: &[(usize, u32, u32)],
    thumbnail_directory: &std::path::Path,
) -> std::collections::HashMap<usize, String> {
    let mut result = std::collections::HashMap::new();

    let image = match image::load_from_memory(image_bytes) {
        Ok(image) => image,
        Err(error) => {
            tracing::warn!("Failed to load flyer image for thumbnail cropping: {error}");
            return result;
        }
    };

    let width = image.width();
    let height = image.height();
    let cell_width = width / grid_cols;
    let cell_height = height / grid_rows;

    if let Err(error) = std::fs::create_dir_all(thumbnail_directory) {
        tracing::warn!("Failed to create thumbnail directory: {error}");
        return result;
    }

    for &(deal_index, row, col) in deal_positions {
        if row >= grid_rows || col >= grid_cols {
            continue;
        }

        let x = col * cell_width;
        let y = row * cell_height;
        let crop = image.crop_imm(x, y, cell_width, cell_height);

        let thumbnail = crop.resize(300, 300, image::imageops::FilterType::Lanczos3);

        let filename = format!("{:016x}.jpg", {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            (row, col, width, height).hash(&mut hasher);
            hasher.finish()
        });

        let file_path = thumbnail_directory.join(&filename);
        match thumbnail.save(&file_path) {
            Ok(_) => {
                let url_path = format!("/api/thumbnails/{}", filename);
                result.insert(deal_index, url_path);
            }
            Err(error) => {
                tracing::warn!("Failed to save thumbnail {}: {error}", file_path.display());
            }
        }
    }

    tracing::info!(
        "Cropped {} thumbnails from {}x{} flyer ({}x{} grid)",
        result.len(),
        width,
        height,
        grid_rows,
        grid_cols
    );

    result
}

async fn download_image(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, AppError> {
    let image_bytes = client
        .get(url)
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();
    Ok(image_bytes)
}

const HMART_POPUP_API_URL: &str =
    "https://www.hmartus.com/api/popup-overlay/render?currentUrl=%2Fweekly-sale-wa";

/// Fetch all H Mart ad image URLs from the weekly sale page.
/// Returns `(weekly_url, additional_urls)` where additional_urls may include monthly ads.
async fn fetch_hmart_ad_image_urls(
    client: &reqwest::Client,
) -> Result<(String, Vec<String>), AppError> {
    let html = client
        .get(HMART_WA_WEEKLY_AD_URL)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await?
        .text()
        .await?;

    tracing::info!("H Mart page HTML length: {} chars", html.len());

    // Find the weekly English flyer image
    let weekly_patterns = [
        Regex::new(r#"src="(https://images\.squarespace-cdn\.com/[^"]*Weekly_eng[^"]*)""#).unwrap(),
        Regex::new(r#"data-src="([^"]*Weekly_eng[^"]*)""#).unwrap(),
        Regex::new(r#"data-image="([^"]*Weekly_eng[^"]*)""#).unwrap(),
    ];

    let mut weekly_url = None;
    for pattern in &weekly_patterns {
        if let Some(capture) = pattern.captures(&html) {
            if let Some(matched) = capture.get(1) {
                let url = append_format_if_needed(matched.as_str(), "2500w");
                tracing::info!("Found H Mart weekly flyer via: {}", pattern.as_str());
                weekly_url = Some(url);
                break;
            }
        }
    }

    let weekly_url = match weekly_url {
        Some(url) => url,
        None => {
            let all_images = Regex::new(r#"data-image="([^"]*)""#).unwrap();
            let found: Vec<&str> = all_images
                .captures_iter(&html)
                .filter_map(|c| c.get(1).map(|m| m.as_str()))
                .collect();
            tracing::warn!(
                "Could not find weekly ad image. Found {} data-image URLs. HTML starts with: {}",
                found.len(),
                &html[..html.len().min(500)]
            );
            for (index, url) in found.iter().enumerate() {
                tracing::warn!("  [{}] {}", index, url);
            }
            return Err(AppError::Internal(
                "Could not find English weekly ad image on H Mart WA page".into(),
            ));
        }
    };

    // Check for monthly ad images on the page itself
    let mut additional_urls = Vec::new();
    let monthly_pattern = Regex::new(
        r#"(?:src|data-src|data-image)="(https://images\.squarespace-cdn\.com/[^"]*[Mm]onthly[^"]*)""#
    ).unwrap();
    for capture in monthly_pattern.captures_iter(&html) {
        if let Some(matched) = capture.get(1) {
            let url = append_format_if_needed(matched.as_str(), "2500w");
            if !additional_urls.contains(&url) {
                tracing::info!("Found monthly ad on page: {}", url);
                additional_urls.push(url);
            }
        }
    }

    Ok((weekly_url, additional_urls))
}

/// Check the Squarespace popup overlay API for a monthly ad image.
async fn fetch_hmart_popup_image_url(
    client: &reqwest::Client,
) -> Result<Option<String>, AppError> {
    let response = client
        .get(HMART_POPUP_API_URL)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await?
        .text()
        .await?;

    let parsed: serde_json::Value = match serde_json::from_str(&response) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let rendered_html = parsed
        .get("renderedHtml")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let image_pattern = Regex::new(
        r#"https://images\.squarespace-cdn\.com/content/[^"\\]*(?:[Mm]onthly|[Aa]d)[^"\\]*\.(?:jpg|jpeg|png|webp)"#
    ).unwrap();

    if let Some(matched) = image_pattern.find(rendered_html) {
        let url = append_format_if_needed(matched.as_str(), "2500w");
        tracing::info!("Found ad image in popup overlay: {}", url);
        return Ok(Some(url));
    }

    Ok(None)
}

fn append_format_if_needed(url: &str, format: &str) -> String {
    if url.contains('?') {
        url.to_string()
    } else {
        format!("{}?format={}", url, format)
    }
}

async fn fetch_sibling_deals(
    pool: &SqlitePool,
    week_id: &str,
) -> Result<Option<Vec<Deal>>, AppError> {
    let deals = sqlx::query_as!(
        Deal,
        r#"SELECT d.id as "id!", d.location_id as "location_id!", d.week_id, d.item_name,
           d.brand, d.deal_description, d.category, d.image_url, d.valid_from, d.valid_to, d.fetched_at
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_zip_geo() -> ZipGeo {
        ZipGeo::from_entries(vec![
            ("98052".into(), 47.6740, -122.1215),
            ("98101".into(), 47.6114, -122.3378),
            ("98004".into(), 47.6101, -122.2015),
            ("90210".into(), 34.0901, -118.4065),
        ])
    }

    #[test]
    fn find_nearest_hmart_redmond_zip() {
        let zip_geo = test_zip_geo();
        let result = find_nearest_hmart_wa_store(&zip_geo, "98052");
        assert!(result.is_some());
        let (store_name, url) = result.unwrap();
        assert_eq!(store_name, "H Mart - Redmond");
        assert_eq!(url, HMART_WA_WEEKLY_AD_URL);
    }

    #[test]
    fn find_nearest_hmart_far_away_zip_returns_none() {
        let zip_geo = test_zip_geo();
        let result = find_nearest_hmart_wa_store(&zip_geo, "90210");
        assert!(result.is_none());
    }

    #[test]
    fn find_nearest_hmart_unknown_zip_returns_none() {
        let zip_geo = test_zip_geo();
        let result = find_nearest_hmart_wa_store(&zip_geo, "00000");
        assert!(result.is_none());
    }

    #[test]
    fn detect_jpeg_media_type() {
        assert_eq!(detect_image_media_type(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
    }

    #[test]
    fn detect_png_media_type() {
        assert_eq!(detect_image_media_type(&[0x89, 0x50, 0x4E, 0x47, 0x0D]), "image/png");
    }

    #[test]
    fn detect_webp_media_type() {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&[0x00; 4]); // file size
        bytes.extend_from_slice(b"WEBP");
        assert_eq!(detect_image_media_type(&bytes), "image/webp");
    }

    #[test]
    fn detect_unknown_defaults_to_jpeg() {
        assert_eq!(detect_image_media_type(&[0x00, 0x01, 0x02]), "image/jpeg");
    }

    #[test]
    fn crop_thumbnails_from_synthetic_image() {
        let image = image::RgbImage::from_fn(400, 300, |x, y| {
            let row = if y < 150 { 0 } else { 1 };
            let col = if x < 200 { 0 } else { 1 };
            match (row, col) {
                (0, 0) => image::Rgb([255, 0, 0]),
                (0, 1) => image::Rgb([0, 255, 0]),
                (1, 0) => image::Rgb([0, 0, 255]),
                _ => image::Rgb([255, 255, 0]),
            }
        });

        let mut png_bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .unwrap();

        let positions = vec![
            (0, 0, 0),
            (1, 0, 1),
            (2, 1, 0),
        ];

        let temp_directory = std::path::PathBuf::from("target/test_thumbnails_synthetic");
        let _ = std::fs::remove_dir_all(&temp_directory);

        let result = crop_and_save_thumbnails_to(&png_bytes, 2, 2, &positions, &temp_directory);

        let _ = std::fs::remove_dir_all(&temp_directory);

        assert_eq!(result.len(), 3, "Should produce 3 thumbnails");
        assert!(result.contains_key(&0));
        assert!(result.contains_key(&1));
        assert!(result.contains_key(&2));

        for url in result.values() {
            assert!(url.starts_with("/api/thumbnails/"), "URL should start with /api/thumbnails/, got: {url}");
            assert!(url.ends_with(".jpg"), "URL should end with .jpg, got: {url}");
        }
    }

    #[test]
    fn crop_thumbnails_skips_out_of_bounds_positions() {
        let image = image::RgbImage::from_fn(200, 200, |_, _| image::Rgb([128, 128, 128]));
        let mut png_bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .unwrap();

        let positions = vec![
            (0, 0, 0),
            (1, 10, 10),
        ];

        let temp_directory = std::path::PathBuf::from("target/test_thumbnails_oob");
        let _ = std::fs::remove_dir_all(&temp_directory);

        let result = crop_and_save_thumbnails_to(&png_bytes, 2, 2, &positions, &temp_directory);

        let _ = std::fs::remove_dir_all(&temp_directory);

        assert_eq!(result.len(), 1, "Should only produce 1 thumbnail (out-of-bounds skipped)");
        assert!(result.contains_key(&0));
        assert!(!result.contains_key(&1));
    }

    #[test]
    fn crop_thumbnails_handles_invalid_image() {
        let result = crop_and_save_thumbnails(b"not an image", 2, 2, &[(0, 0, 0)]);
        assert!(result.is_empty(), "Should return empty map for invalid image");
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
