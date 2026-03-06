pub mod browser;
pub mod stores;

use std::collections::HashMap;

use crate::ai::client::AnthropicClient;
use crate::error::AppError;

const MODEL: &str = "claude-sonnet-4-20250514";
const MAX_TOKENS: u32 = 4096;

/// Extract deals from screenshots of a weekly ad page.
/// Returns tuples of (item_name, brand, deal_description, category, image_url).
pub async fn extract_deals_from_screenshots(
    ai: &AnthropicClient,
    screenshots: &[Vec<u8>],
) -> Result<Vec<(String, Option<String>, String, String, Option<String>)>, AppError> {
    if screenshots.is_empty() {
        return Ok(vec![]);
    }

    let mut content_blocks: Vec<serde_json::Value> = Vec::new();

    for (i, screenshot) in screenshots.iter().enumerate() {
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            screenshot,
        );

        content_blocks.push(serde_json::json!({
            "type": "text",
            "text": format!("Screenshot {} of the weekly ad:", i + 1)
        }));

        content_blocks.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": b64
            }
        }));
    }

    content_blocks.push(serde_json::json!({
        "type": "text",
        "text": "Extract all grocery deals from these weekly ad screenshots. \
                 For each deal, provide the item name, deal description (price or discount), \
                 and a category (produce, meat, dairy, bakery, frozen, pantry, beverages, snacks, deli, seafood). \
                 Skip non-food items. \
                 Respond with ONLY a JSON array of objects: \
                 [{\"item_name\": \"...\", \"brand\": \"...\", \"deal_description\": \"...\", \"category\": \"...\"}] \
                 If brand is unknown, use null. Output only the JSON array."
    }));

    let response = ai
        .send_with_images(MODEL, MAX_TOKENS, content_blocks)
        .await?;

    let json_str = extract_json_array(&response);

    let items: Vec<HashMap<String, serde_json::Value>> =
        serde_json::from_str(json_str).map_err(|err| {
            tracing::warn!("Failed to parse vision deal response: {err}\nResponse: {response}");
            AppError::Ai(format!("Failed to parse vision deals: {err}"))
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
                .unwrap_or("other")
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
