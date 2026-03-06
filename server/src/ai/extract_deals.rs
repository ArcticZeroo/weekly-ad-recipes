use std::collections::HashMap;

use serde_json::json;

use crate::ai::client::AnthropicClient;
use crate::error::AppError;

const MODEL: &str = "claude-haiku-4-5-20251001";
const MAX_TOKENS: u32 = 2048;

/// Download images and send them to Claude Vision to extract deal descriptions
/// for items where Flipp didn't provide price/deal info.
/// Returns a map from item name → extracted deal description.
pub async fn extract_deals_from_images(
    ai: &AnthropicClient,
    http: &reqwest::Client,
    items: &[(String, String)], // (name, image_url)
) -> Result<HashMap<String, String>, AppError> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    // Download all images and convert to base64
    let mut content_blocks: Vec<serde_json::Value> = Vec::new();
    let mut item_names: Vec<String> = Vec::new();

    for (name, image_url) in items {
        let image_bytes = match http.get(image_url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => bytes,
                Err(err) => {
                    tracing::warn!("Failed to download image for {name}: {err}");
                    continue;
                }
            },
            Err(err) => {
                tracing::warn!("Failed to fetch image for {name}: {err}");
                continue;
            }
        };

        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &image_bytes,
        );

        // Determine media type from URL or default to JPEG
        let media_type = if image_url.contains(".png") {
            "image/png"
        } else {
            "image/jpeg"
        };

        content_blocks.push(json!({
            "type": "text",
            "text": format!("Item {}: {}", item_names.len() + 1, name)
        }));

        content_blocks.push(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": b64
            }
        }));

        item_names.push(name.clone());
    }

    if item_names.is_empty() {
        return Ok(HashMap::new());
    }

    content_blocks.push(json!({
        "type": "text",
        "text": format!(
            "For each item image above, read the deal/sale text visible in the image \
             (e.g. \"Buy 1 Get 1 Free\", \"2 for $5\", \"$3.99/lb\", \"Save $2\", etc). \
             Respond with ONLY a JSON object mapping item names (exactly as given) to the deal text. \
             If you can't determine the deal, use \"On Sale\". Example:\n\
             {{\"Coca-Cola\": \"Buy 2 Get 1 Free\", \"Doritos\": \"2 for $7\"}}"
        )
    }));

    tracing::info!(
        "Sending {} item images to Vision for deal extraction",
        item_names.len()
    );

    let response = ai
        .send_with_images(MODEL, MAX_TOKENS, content_blocks)
        .await?;

    let json_str = extract_json(&response);

    let deals: HashMap<String, String> = serde_json::from_str(json_str).map_err(|err| {
        tracing::warn!("Failed to parse vision deal response: {err}\nResponse: {response}");
        AppError::Ai(format!("Failed to parse vision deals: {err}"))
    })?;

    tracing::info!("Vision extracted deals for {} items", deals.len());
    Ok(deals)
}

fn extract_json(text: &str) -> &str {
    let text = text.trim();
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return &text[start..=end];
        }
    }
    text
}
