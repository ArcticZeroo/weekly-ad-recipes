use std::collections::HashSet;

use crate::ai::client::AnthropicClient;
use crate::error::AppError;
use crate::models::deal::Deal;
use crate::models::meal::SaleIngredient;

const MODEL: &str = "claude-sonnet-4-20250514";
const MAX_TOKENS: u32 = 4096;

#[derive(Debug, serde::Deserialize)]
struct IngredientRef {
    ingredient: String,
    deal_id: i64,
}

#[derive(Debug, serde::Deserialize)]
struct MealSuggestion {
    name: String,
    description: String,
    on_sale_ingredients: Vec<IngredientRef>,
    additional_ingredients: Vec<String>,
    estimated_savings: String,
}

/// Generate meal ideas from a list of deals on sale.
/// Returns tuples of (name, description, on_sale_ingredients, additional_ingredients, estimated_savings).
pub async fn generate_meal_ideas(
    client: &AnthropicClient,
    deals: &[Deal],
) -> Result<Vec<(String, String, Vec<SaleIngredient>, Vec<String>, String)>, AppError> {
    if deals.is_empty() {
        return Ok(vec![]);
    }

    let deals_list = format_deals_list(deals);

    let prompt = format!(
        r#"You are a helpful meal planning assistant. Based on the grocery deals below, suggest 5-8 meal ideas that take advantage of items currently on sale.

Current deals:
{deals_list}

For each meal, respond with a JSON array of objects with these fields:
- "name": short meal name
- "description": 1-2 sentence description of the dish
- "on_sale_ingredients": array of objects, each with "ingredient" (string) and "deal_id" (number matching the ID from the deals list above)
- "additional_ingredients": array of common pantry/fridge items needed that aren't on sale
- "estimated_savings": approximate savings compared to regular prices (e.g., "$5-8")

Focus on practical, delicious meals. Prefer meals that use multiple on-sale items. Output ONLY the JSON array, no other text."#
    );

    let response = client.send_message(MODEL, MAX_TOKENS, &prompt).await?;

    let json_str = extract_json_array(&response);

    let suggestions: Vec<MealSuggestion> =
        serde_json::from_str(json_str).map_err(|error| {
            tracing::warn!("Failed to parse meal suggestions: {error}\nResponse: {response}");
            AppError::Ai(format!("Failed to parse meal ideas: {error}"))
        })?;

    let valid_ids: HashSet<i64> = deals.iter().map(|deal| deal.id).collect();

    Ok(suggestions
        .into_iter()
        .map(|suggestion| {
            let validated_ingredients = validate_ingredient_ids(
                suggestion.on_sale_ingredients,
                &valid_ids,
            );
            (
                suggestion.name,
                suggestion.description,
                validated_ingredients,
                suggestion.additional_ingredients,
                suggestion.estimated_savings,
            )
        })
        .collect())
}

fn format_deals_list(deals: &[Deal]) -> String {
    deals
        .iter()
        .map(|deal| {
            let brand = deal
                .brand
                .as_deref()
                .map(|b| format!(" ({})", b))
                .unwrap_or_default();
            format!(
                "- [ID:{}] {}{}: {} [{}]",
                deal.id, deal.item_name, brand, deal.deal_description, deal.category
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_ingredient_ids(
    ingredients: Vec<IngredientRef>,
    valid_ids: &HashSet<i64>,
) -> Vec<SaleIngredient> {
    ingredients
        .into_iter()
        .filter(|ingredient_ref| {
            let is_valid = valid_ids.contains(&ingredient_ref.deal_id);
            if !is_valid {
                tracing::warn!(
                    "Filtering out ingredient '{}' with invalid deal_id {}",
                    ingredient_ref.ingredient,
                    ingredient_ref.deal_id
                );
            }
            is_valid
        })
        .map(|ingredient_ref| SaleIngredient {
            ingredient: ingredient_ref.ingredient,
            deal_id: ingredient_ref.deal_id,
        })
        .collect()
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
