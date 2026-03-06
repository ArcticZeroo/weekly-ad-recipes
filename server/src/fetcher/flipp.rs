use rand::Rng;
use serde::Deserialize;

use crate::error::AppError;

const FLIPP_FLYERS_URL: &str = "https://backflipp.wishabi.com/flipp/flyers";
const FLIPP_ITEMS_URL: &str = "https://flyers-ng.flippback.com/api/flipp/flyers";

/// Known chain names as they appear in the Flipp API
const SUPPORTED_FLIPP_CHAINS: &[(&str, &str)] = &[
    ("qfc", "QFC"),
    ("safeway", "Safeway"),
    ("fred-meyer", "Fred Meyer"),
];

#[derive(Debug, Deserialize)]
struct FlippFlyerResponse {
    #[serde(default)]
    flyers: Vec<FlippFlyer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlippFlyer {
    pub id: i64,
    pub merchant_id: Option<i64>,
    pub merchant: Option<String>,
    pub name: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub categories_csv: Option<String>,
    pub store_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlippItem {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub price: Option<String>,
    pub pre_price_text: Option<String>,
    pub cutout_image_url: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

/// Result from searching Flipp for stores near a zip code
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlippStoreMatch {
    pub chain_id: String,
    pub chain_name: String,
    pub flyer_id: i64,
    pub merchant_id: Option<i64>,
    pub merchant_name: String,
    pub store_name: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

fn generate_session_id() -> String {
    let mut rng = rand::thread_rng();
    let digits: String = (0..16).map(|_| rng.gen_range(0..10).to_string()).collect();
    digits
}

/// Search Flipp for weekly ad flyers near a zip code, returning matches for supported chains
pub async fn search_flyers_by_zip(
    client: &reqwest::Client,
    zip: &str,
) -> Result<Vec<FlippStoreMatch>, AppError> {
    let response: FlippFlyerResponse = client
        .get(FLIPP_FLYERS_URL)
        .query(&[("postal_code", zip), ("locale", "en")])
        .send()
        .await?
        .json()
        .await?;

    let mut matches = Vec::new();

    for flyer in &response.flyers {
        let merchant_name = match &flyer.merchant {
            Some(name) => name,
            None => continue,
        };

        for &(chain_id, chain_display) in SUPPORTED_FLIPP_CHAINS {
            if merchant_name.eq_ignore_ascii_case(chain_display)
                || merchant_name
                    .to_lowercase()
                    .contains(&chain_display.to_lowercase())
            {
                matches.push(FlippStoreMatch {
                    chain_id: chain_id.to_string(),
                    chain_name: chain_display.to_string(),
                    flyer_id: flyer.id,
                    merchant_id: flyer.merchant_id,
                    merchant_name: merchant_name.clone(),
                    store_name: flyer.name.clone(),
                    valid_from: flyer.valid_from.clone(),
                    valid_to: flyer.valid_to.clone(),
                });
                break;
            }
        }
    }

    Ok(matches)
}

/// Fetch all items from a specific Flipp flyer
pub async fn fetch_flyer_items(
    client: &reqwest::Client,
    flyer_id: i64,
) -> Result<Vec<FlippItem>, AppError> {
    let sid = generate_session_id();
    let url = format!("{FLIPP_ITEMS_URL}/{flyer_id}/flyer_items");

    let items: Vec<FlippItem> = client
        .get(&url)
        .query(&[("locale", "en"), ("sid", &sid)])
        .send()
        .await?
        .json()
        .await?;

    Ok(items)
}

/// Convert Flipp items into the tuple format expected by save_deals
pub fn items_to_deal_tuples(
    items: &[FlippItem],
) -> Vec<(String, Option<String>, String, String, Option<String>)> {
    items
        .iter()
        .filter_map(|item| {
            let name = item.name.as_ref()?.trim().to_string();
            if name.is_empty() {
                return None;
            }

            let deal_description = build_deal_description(item);
            let brand = item.brand.clone().filter(|b| !b.trim().is_empty());
            let image_url = item.cutout_image_url.clone().filter(|u| !u.trim().is_empty());

            Some((
                name,
                brand,
                deal_description,
                "uncategorized".to_string(),
                image_url,
            ))
        })
        .collect()
}

fn build_deal_description(item: &FlippItem) -> String {
    let mut parts = Vec::new();

    if let Some(pre) = &item.pre_price_text {
        let pre = pre.trim();
        if !pre.is_empty() {
            parts.push(pre.to_string());
        }
    }

    if let Some(price) = &item.price {
        let price = price.trim();
        if !price.is_empty() {
            parts.push(format!("${price}"));
        }
    }

    if let Some(description) = &item.description {
        let description = description.trim();
        if !description.is_empty() && !parts.iter().any(|p| p.contains(description)) {
            parts.push(description.to_string());
        }
    }

    if parts.is_empty() {
        "On Sale".to_string()
    } else {
        parts.join(" - ")
    }
}
