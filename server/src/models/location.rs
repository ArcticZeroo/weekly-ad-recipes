use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct StoreLocation {
    #[ts(type = "number")]
    pub id: i64,
    pub chain_id: String,
    pub name: String,
    pub address: Option<String>,
    pub zip_code: String,
    #[ts(type = "number | null")]
    pub flipp_merchant_id: Option<i64>,
    pub flipp_merchant_name: Option<String>,
    pub weekly_ad_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateLocationRequest {
    pub chain_id: String,
    pub name: String,
    pub address: Option<String>,
    pub zip_code: String,
    pub flipp_merchant_id: Option<i64>,
    pub flipp_merchant_name: Option<String>,
    pub weekly_ad_url: Option<String>,
}
