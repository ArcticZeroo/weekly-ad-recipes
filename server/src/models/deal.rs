use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct Deal {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub location_id: i64,
    pub week_id: String,
    pub item_name: String,
    pub brand: Option<String>,
    pub deal_description: String,
    pub category: String,
    pub image_url: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct DealsResponse {
    pub chain_id: String,
    pub zip_code: String,
    pub week_id: String,
    pub deals: Vec<Deal>,
    pub cached: bool,
}
