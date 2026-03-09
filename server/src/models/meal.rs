use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::deal::Deal;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct SaleIngredient {
    pub ingredient: String,
    #[ts(type = "number")]
    pub deal_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct MealIdea {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub location_id: i64,
    pub week_id: String,
    pub name: String,
    pub description: String,
    pub on_sale_ingredients: Vec<SaleIngredient>,
    pub additional_ingredients: Vec<String>,
    pub estimated_savings: String,
    pub fetched_at: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct MealsResponse {
    pub chain_id: String,
    pub zip_code: String,
    pub week_id: String,
    pub meals: Vec<MealIdea>,
    pub deals: Vec<Deal>,
    pub cached: bool,
}
