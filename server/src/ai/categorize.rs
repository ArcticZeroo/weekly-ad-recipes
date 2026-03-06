use std::collections::HashMap;

use crate::ai::client::AnthropicClient;
use crate::error::AppError;

const MODEL: &str = "claude-haiku-4-5-20251001";
const MAX_TOKENS: u32 = 4096;
const BATCH_SIZE: usize = 40;

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
    "not_food",
];

/// Batch-categorize a list of item names into grocery categories.
/// Splits into chunks to avoid truncated responses.
/// Feeds back discovered categories to keep naming consistent across batches.
/// Returns a map from item name → category.
pub async fn categorize_items(
    client: &AnthropicClient,
    items: &[(String, Option<String>)],
) -> Result<HashMap<String, String>, AppError> {
    if items.is_empty() {
        return Ok(HashMap::new());
    }

    let mut all_categories = HashMap::new();
    let mut seen_categories: Vec<String> = CATEGORIES.iter().map(|s| s.to_string()).collect();

    for chunk in items.chunks(BATCH_SIZE) {
        match categorize_batch(client, chunk, &seen_categories).await {
            Ok(batch_result) => {
                for category in batch_result.values() {
                    if !seen_categories.contains(category) {
                        seen_categories.push(category.clone());
                    }
                }
                all_categories.extend(batch_result);
            }
            Err(err) => {
                tracing::warn!(
                    "Categorization batch failed ({}), skipping: {err}",
                    chunk.len()
                );
            }
        }
    }

    Ok(all_categories)
}

async fn categorize_batch(
    client: &AnthropicClient,
    items: &[(String, Option<String>)],
    known_categories: &[String],
) -> Result<HashMap<String, String>, AppError> {
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

    let categories = known_categories.join(", ");

    let prompt = format!(
        r#"Categorize each grocery item into exactly one category. Categories: {categories}

This is for a meal planning app, so focus on food and drink items.
Use "not_food" for non-food items like baby supplies, cleaning products, toiletries, pet food, 
Easter decorations, flowers, household items, etc. Alcohol and beverages should stay as "beverages".

Items:
{item_list}

Respond with ONLY a JSON object mapping each item number to its category. Example:
{{"1": "produce", "2": "meat", "3": "not_food"}}

You MUST include an entry for every item number from 1 to {count}. Output only the JSON object."#,
        count = items.len()
    );

    let response = client.send_message(MODEL, MAX_TOKENS, &prompt).await?;

    let json_str = extract_json(&response);

    let by_number: HashMap<String, String> =
        serde_json::from_str(json_str).map_err(|err| {
            tracing::warn!("Failed to parse categorization response: {err}\nResponse: {response}");
            AppError::Ai(format!("Failed to parse categories: {err}"))
        })?;

    // Map from number-based keys back to item names
    let mut result = HashMap::new();
    for (i, (name, _)) in items.iter().enumerate() {
        let key = (i + 1).to_string();
        if let Some(category) = by_number.get(&key) {
            result.insert(name.clone(), category.clone());
        }
    }

    Ok(result)
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
