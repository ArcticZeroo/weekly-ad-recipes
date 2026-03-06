use axum::Json;

use crate::models::chain::{supported_chains, StoreChain};

pub async fn list_chains() -> Json<Vec<StoreChain>> {
    Json(supported_chains())
}
