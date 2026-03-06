use std::collections::HashMap;

use crate::ai::client::AnthropicClient;
use crate::error::AppError;

const MODEL: &str = "claude-haiku-4-5-20251001";
const MAX_TOKENS: u32 = 4096;

const CATEGORIES: &[&str] = &[
    "produce",
    "meat",
    "dairy",
    "bakery",
    "frozen",
    "pantry",
    "beverages",
    "snacks",
    "deli",
    "seafood",
    "household",
    "other",
];

/// Batch-categorize a list of item names into grocery categories.
/// Returns a map from item name → category.
pub async fn categorize_items(
    client: &AnthropicClient,
    items: &[(String, Option<String>)],
) -> Result<HashMap<String, String>, AppError> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    let item_list: String = items
        .iter()
        .enumerate()
        .map(|(i, (name, brand))| {
            if let Some(brand) = brand {
                format!("{}. {} ({})", i + 1, name, brand)
            } else {
                format!("{}. {}", i + 1, name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let categories = CATEGORIES.join(", ");

    let prompt = format!(
        r#"Categorize each grocery item into exactly one category. Categories: {categories}

Items:
{item_list}

Respond with ONLY a JSON object mapping item names (exactly as given, without the number prefix) to their category. Example:
{{"Bananas": "produce", "Chicken Breast": "meat"}}

Important: Use the exact item names from the list. Output only the JSON object, no other text."#
    );

    let response = client.send_message(MODEL, MAX_TOKENS, &prompt).await?;

    // Parse JSON from response, handling potential markdown code blocks
    let json_str = extract_json(&response);

    let categories: HashMap<String, String> =
        serde_json::from_str(json_str).map_err(|err| {
            tracing::warn!("Failed to parse categorization response: {err}\nResponse: {response}");
            AppError::Ai(format!("Failed to parse categories: {err}"))
        })?;

    Ok(categories)
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
