use std::collections::HashSet;

use regex::Regex;
use sqlx::SqlitePool;

use crate::db::queries;
use crate::error::AppError;
use crate::fetcher::zip_geo::{self, ZipGeo};

const SITEMAP_URL: &str = "https://www.wholefoodsmarket.com/sitemap/sitemap-stores.xml";
const STORE_BASE_URL: &str = "https://www.wholefoodsmarket.com/stores";
const SCRAPE_CONCURRENCY: usize = 10;

struct WfmStoreInfo {
    store_id: String,
    slug: String,
    name: String,
    city: Option<String>,
    state: Option<String>,
    zip_code: Option<String>,
    latitude: f64,
    longitude: f64,
}

pub async fn ensure_wfm_catalog(pool: &SqlitePool) -> Result<(), AppError> {
    let client = reqwest::Client::new();

    let sitemap_xml = fetch_sitemap(&client).await?;
    let sitemap_slugs: HashSet<String> = parse_sitemap_slugs(&sitemap_xml).into_iter().collect();

    let known_slugs = queries::get_known_wfm_slugs(pool).await?;
    let new_slugs: Vec<String> = sitemap_slugs.difference(&known_slugs).cloned().collect();

    if new_slugs.is_empty() {
        tracing::info!("WFM catalog is up to date ({} stores known)", known_slugs.len());
        return Ok(());
    }

    tracing::info!(
        "Found {} new WFM store slugs to scrape ({} already known)",
        new_slugs.len(),
        known_slugs.len()
    );

    let results = scrape_stores_in_batches(&client, &new_slugs).await;

    let mut inserted_count = 0;
    for result in results {
        match result {
            Ok(store) => {
                if let Err(error) = queries::insert_wfm_store(
                    pool,
                    &store.store_id,
                    &store.slug,
                    &store.name,
                    store.city.as_deref(),
                    store.state.as_deref(),
                    store.zip_code.as_deref(),
                    store.latitude,
                    store.longitude,
                )
                .await
                {
                    tracing::warn!("Failed to insert WFM store '{}': {}", store.slug, error);
                } else {
                    inserted_count += 1;
                }
            }
            Err(error) => {
                tracing::warn!("Failed to scrape WFM store page: {}", error);
            }
        }
    }

    if inserted_count > 0 {
        tracing::info!("Inserted {} new WFM stores, clearing lookup cache", inserted_count);
        queries::clear_wfm_store_lookups(pool).await?;
    }

    Ok(())
}

pub async fn find_nearest_wfm_store(
    pool: &SqlitePool,
    zip_geo: &ZipGeo,
    zip_code: &str,
) -> Result<Option<(String, String)>, AppError> {
    if let Some(cached_store_id) = queries::get_wfm_store_lookup(pool, zip_code).await? {
        return lookup_store_by_id(pool, &cached_store_id).await;
    }

    let (zip_latitude, zip_longitude) = match zip_geo.lookup(zip_code) {
        Some(coordinates) => coordinates,
        None => return Ok(None),
    };

    let stores = queries::get_all_wfm_stores(pool).await?;
    if stores.is_empty() {
        return Ok(None);
    }

    let nearest = find_closest_store(&stores, zip_latitude, zip_longitude);

    if let Some((store_id, store_name)) = &nearest {
        queries::save_wfm_store_lookup(pool, zip_code, store_id).await?;
        return Ok(Some((store_id.clone(), store_name.clone())));
    }

    Ok(None)
}

pub fn parse_sitemap_slugs(xml: &str) -> Vec<String> {
    let pattern =
        Regex::new(r#"<loc>https://www\.wholefoodsmarket\.com/stores/([^<]+)</loc>"#).unwrap();

    pattern
        .captures_iter(xml)
        .filter_map(|capture| capture.get(1).map(|matched| matched.as_str().to_string()))
        .collect()
}

// --- Helpers ---

async fn fetch_sitemap(client: &reqwest::Client) -> Result<String, AppError> {
    let response = client.get(SITEMAP_URL).send().await?.text().await?;
    Ok(response)
}

async fn scrape_stores_in_batches(
    client: &reqwest::Client,
    slugs: &[String],
) -> Vec<Result<WfmStoreInfo, AppError>> {
    let mut results = Vec::new();
    for chunk in slugs.chunks(SCRAPE_CONCURRENCY) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|slug| scrape_store_page(client, slug))
            .collect();
        let batch_results = futures::future::join_all(futures).await;
        results.extend(batch_results);
    }
    results
}

async fn scrape_store_page(
    client: &reqwest::Client,
    slug: &str,
) -> Result<WfmStoreInfo, AppError> {
    let url = format!("{}/{}", STORE_BASE_URL, slug);
    let html = client.get(&url).send().await?.text().await?;

    let (store_id, name) = extract_store_selector_attrs(&html, slug)?;
    let json_ld = extract_json_ld(&html);

    let (mut latitude, mut longitude) = (0.0, 0.0);
    let mut city = None;
    let mut state = None;
    let mut zip_code = None;

    if let Some(ref data) = json_ld {
        if let Some(geo) = data.get("geo") {
            latitude = parse_json_f64(geo.get("latitude"));
            longitude = parse_json_f64(geo.get("longitude"));
        }
        if let Some(address) = data.get("address") {
            city = json_string(address.get("addressLocality"));
            state = json_string(address.get("addressRegion"));
            zip_code = json_string(address.get("postalCode"));
        }
    }

    if latitude == 0.0 && longitude == 0.0 {
        if let Some((fallback_latitude, fallback_longitude)) =
            extract_store_geometry_fallback(&html)
        {
            latitude = fallback_latitude;
            longitude = fallback_longitude;
        }
    }

    Ok(WfmStoreInfo {
        store_id,
        slug: slug.to_string(),
        name,
        city,
        state,
        zip_code,
        latitude,
        longitude,
    })
}

fn extract_store_selector_attrs(html: &str, slug: &str) -> Result<(String, String), AppError> {
    let store_id_pattern = Regex::new(r#"store-id="([^"]+)""#).unwrap();
    let store_name_pattern = Regex::new(r#"store-name="([^"]+)""#).unwrap();

    let store_id = store_id_pattern
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|matched| matched.as_str().to_string())
        .ok_or_else(|| AppError::Internal(format!("No store-id found for slug '{}'", slug)))?;

    let name = store_name_pattern
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|matched| matched.as_str().to_string())
        .ok_or_else(|| AppError::Internal(format!("No store-name found for slug '{}'", slug)))?;

    Ok((store_id, name))
}

fn extract_json_ld(html: &str) -> Option<serde_json::Value> {
    let pattern =
        Regex::new(r#"(?s)<script type="application/ld\+json">\s*(\{.*?\})\s*</script>"#).unwrap();

    pattern
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|matched| serde_json::from_str(matched.as_str()).ok())
}

fn extract_store_geometry_fallback(html: &str) -> Option<(f64, f64)> {
    let pattern = Regex::new(r#"store-geometry="([^"]+)""#).unwrap();

    pattern
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|matched| {
            let parts: Vec<&str> = matched.as_str().split(',').collect();
            if parts.len() == 2 {
                let latitude = parts[0].trim().parse::<f64>().ok()?;
                let longitude = parts[1].trim().parse::<f64>().ok()?;
                Some((latitude, longitude))
            } else {
                None
            }
        })
}

fn parse_json_f64(value: Option<&serde_json::Value>) -> f64 {
    value
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .unwrap_or(0.0)
}

fn json_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|v| v.as_str().map(|s| s.to_string()))
}

async fn lookup_store_by_id(
    pool: &SqlitePool,
    store_id: &str,
) -> Result<Option<(String, String)>, AppError> {
    match queries::get_wfm_store_by_id(pool, store_id).await? {
        Some((id, name, _, _)) => Ok(Some((id, name))),
        None => Ok(None),
    }
}

fn find_closest_store(
    stores: &[(String, String, f64, f64)],
    target_latitude: f64,
    target_longitude: f64,
) -> Option<(String, String)> {
    stores
        .iter()
        .map(|(store_id, name, latitude, longitude)| {
            let distance =
                zip_geo::haversine_distance_km(target_latitude, target_longitude, *latitude, *longitude);
            (store_id, name, distance)
        })
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(store_id, name, _)| (store_id.clone(), name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sitemap_slugs_extracts_store_slugs() {
        let xml = r#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url><loc>https://www.wholefoodsmarket.com/stores/interbay</loc><lastmod>2025-10-29</lastmod></url>
<url><loc>https://www.wholefoodsmarket.com/stores/bellevue</loc><lastmod>2025-10-29</lastmod></url>
</urlset>"#;

        let slugs = parse_sitemap_slugs(xml);
        assert_eq!(slugs, vec!["interbay", "bellevue"]);
    }

    #[test]
    fn parse_sitemap_slugs_empty_xml_returns_empty() {
        let xml = r#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
</urlset>"#;

        let slugs = parse_sitemap_slugs(xml);
        assert!(slugs.is_empty());
    }
}
