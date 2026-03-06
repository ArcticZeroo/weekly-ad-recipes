use serde::Deserialize;

use crate::error::AppError;

const SALES_FLYER_URL: &str = "https://www.wholefoodsmarket.com/sales-flyer";

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct WfmPromotion {
    #[serde(rename = "promotionId")]
    promotion_id: Option<String>,
    #[serde(rename = "productName")]
    product_name: Option<String>,
    #[serde(rename = "originBrandName")]
    origin_brand_name: Option<String>,
    #[serde(rename = "regularPrice")]
    regular_price: Option<String>,
    #[serde(rename = "salePrice")]
    sale_price: Option<String>,
    #[serde(rename = "primePrice")]
    prime_price: Option<String>,
    #[serde(rename = "productImage")]
    product_image: Option<String>,
    #[serde(rename = "romanceCopy")]
    romance_copy: Option<String>,
    headline: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NextDataProps {
    #[serde(rename = "pageProps")]
    page_props: PageProps,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PageProps {
    promotions: Vec<WfmPromotion>,
    #[serde(rename = "storeId")]
    store_id: Option<serde_json::Value>,
    #[serde(rename = "storeName")]
    store_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NextData {
    props: NextDataProps,
}

/// Fetch Whole Foods deals by loading the sales flyer page with headless Chrome
/// and extracting the structured promotion data from __NEXT_DATA__.
pub async fn fetch_deals(
    wfm_store_id: &str,
) -> Result<Vec<(String, Option<String>, String, String, Option<String>)>, AppError> {
    let url = format!("{SALES_FLYER_URL}?store-id={wfm_store_id}");

    tracing::info!("Fetching Whole Foods deals from {url}");

    let html = crate::fetcher::vision::browser::dump_dom(&url).await?;

    // Extract __NEXT_DATA__ JSON
    let next_data_str = extract_next_data(&html)
        .ok_or_else(|| AppError::Internal("No __NEXT_DATA__ found in Whole Foods page".into()))?;

    let next_data: NextData = serde_json::from_str(next_data_str).map_err(|err| {
        tracing::warn!("Failed to parse Whole Foods __NEXT_DATA__: {err}");
        AppError::Internal(format!("Failed to parse Whole Foods data: {err}"))
    })?;

    let promotions = next_data.props.page_props.promotions;
    tracing::info!(
        "Found {} promotions for store {} ({})",
        promotions.len(),
        wfm_store_id,
        next_data.props.page_props.store_name.as_deref().unwrap_or("unknown")
    );

    let deals = promotions
        .into_iter()
        .filter_map(|promo| {
            let name = promo.product_name.as_deref()?.trim().to_string();
            if name.is_empty() {
                return None;
            }

            let brand = promo
                .origin_brand_name
                .filter(|b| !b.trim().is_empty());

            let deal_description = build_deal_description(&promo.regular_price, &promo.sale_price, &promo.prime_price);
            let image_url = promo.product_image.filter(|u| !u.trim().is_empty());

            // Use "uncategorized" — the categorization AI step will handle it
            Some((name, brand, deal_description, "uncategorized".to_string(), image_url))
        })
        .collect();

    Ok(deals)
}

fn build_deal_description(
    regular: &Option<String>,
    sale: &Option<String>,
    prime: &Option<String>,
) -> String {
    let mut parts = Vec::new();

    if let Some(sale) = sale {
        let sale = sale.trim();
        if !sale.is_empty() {
            parts.push(format!("Sale: {sale}"));
        }
    }

    if let Some(prime) = prime {
        let prime = prime.trim();
        if !prime.is_empty() {
            parts.push(format!("Prime: {prime}"));
        }
    }

    if let Some(regular) = regular {
        let regular = regular.trim();
        if !regular.is_empty() {
            parts.push(format!("Reg: {regular}"));
        }
    }

    if parts.is_empty() {
        "On Sale".to_string()
    } else {
        parts.join(" · ")
    }
}

fn extract_next_data(html: &str) -> Option<&str> {
    let marker = "__NEXT_DATA__";
    let start_tag = html.find(marker)?;
    let json_start = html[start_tag..].find('>')?;
    let json_begin = start_tag + json_start + 1;
    let json_end = html[json_begin..].find("</script>")?;
    Some(&html[json_begin..json_begin + json_end])
}
