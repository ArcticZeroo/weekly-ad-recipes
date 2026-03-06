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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct FlippItem {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub display_type: Option<i32>,
    pub brand: Option<String>,
    pub description: Option<String>,
    pub price: Option<String>,
    pub pre_price_text: Option<String>,
    pub cutout_image_url: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

/// Result from searching for stores near a zip code
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlippStoreMatch {
    pub chain_id: String,
    pub chain_name: String,
    pub flyer_id: Option<i64>,
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

/// Search Flipp for weekly ad flyers near a zip code, returning matches for supported chains.
/// Deduplicates by chain, keeping only the best grocery flyer per chain.
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

    let mut best_by_chain: std::collections::HashMap<String, (FlippStoreMatch, i32)> =
        std::collections::HashMap::new();

    for flyer in &response.flyers {
        let merchant_name = match &flyer.merchant {
            Some(name) => name.trim(),
            None => continue,
        };

        for &(chain_id, chain_display) in SUPPORTED_FLIPP_CHAINS {
            if merchant_name.eq_ignore_ascii_case(chain_display)
                || merchant_name
                    .to_lowercase()
                    .contains(&chain_display.to_lowercase())
            {
                let flyer_name = flyer.name.as_deref().unwrap_or("").to_lowercase();
                let priority = flyer_priority(&flyer_name);

                let candidate = FlippStoreMatch {
                    chain_id: chain_id.to_string(),
                    chain_name: chain_display.to_string(),
                    flyer_id: Some(flyer.id),
                    merchant_id: flyer.merchant_id,
                    merchant_name: merchant_name.to_string(),
                    store_name: flyer.name.clone(),
                    valid_from: flyer.valid_from.clone(),
                    valid_to: flyer.valid_to.clone(),
                };

                let key = chain_id.to_string();
                if let Some((_existing, existing_priority)) = best_by_chain.get(&key) {
                    if priority > *existing_priority {
                        best_by_chain.insert(key, (candidate, priority));
                    }
                } else {
                    best_by_chain.insert(key, (candidate, priority));
                }

                break;
            }
        }
    }

    let matches: Vec<FlippStoreMatch> = best_by_chain.into_values().map(|(m, _)| m).collect();
    Ok(matches)
}

/// Higher priority = more likely to be the main grocery weekly ad
fn flyer_priority(flyer_name: &str) -> i32 {
    if flyer_name.contains("weekly ad") || flyer_name.contains("weekly circular") {
        10
    } else if flyer_name.contains("weekly") {
        5
    } else if flyer_name.contains("flyer") || flyer_name.contains("circular") {
        3
    } else {
        // "Home & Apparel", "Big Book of Savings", etc.
        1
    }
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

/// Convert Flipp items into the tuple format expected by save_deals.
/// Filters out non-deal items (display_type 5 = logos/promos).
pub fn items_to_deal_tuples(
    items: &[FlippItem],
) -> Vec<(String, Option<String>, String, String, Option<String>)> {
    items
        .iter()
        .filter_map(|item| {
            // display_type 5 = non-deal items (store logos, promo banners)
            if item.display_type == Some(5) {
                return None;
            }

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

/// Returns items that have "On Sale" as their description but have a cutout image
/// that likely contains the actual deal text (e.g., "Buy 1 Get 1 Free").
pub fn items_needing_vision(
    items: &[FlippItem],
) -> Vec<(String, String)> {
    items
        .iter()
        .filter_map(|item| {
            if item.display_type == Some(5) {
                return None;
            }
            let name = item.name.as_ref()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let price = item.price.as_deref().unwrap_or("").trim();
            let has_price_info = !price.is_empty()
                || item.pre_price_text.as_deref().unwrap_or("").trim().len() > 0
                || item.description.as_deref().unwrap_or("").trim().len() > 0;
            if has_price_info {
                return None;
            }
            let image_url = item.cutout_image_url.as_ref()?.trim().to_string();
            if image_url.is_empty() {
                return None;
            }
            Some((name, image_url))
        })
        .collect()
}
