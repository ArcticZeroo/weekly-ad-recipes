mod ai;
mod config;
mod db;
mod error;
mod fetcher;
mod inflight;
mod models;
mod routes;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::ai::client::AnthropicClient;
use crate::db::queries;
use crate::fetcher::zip_geo::ZipGeo;
use crate::models::deal::Deal;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub ai: std::sync::Arc<AnthropicClient>,
    pub deals_tracker: inflight::InFlightTracker,
    pub meals_tracker: inflight::InFlightTracker,
    pub zip_geo: std::sync::Arc<ZipGeo>,
    deals_hash_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
}

impl AppState {
    fn deals_hash_cache_key(location_id: i64, week_id: &str) -> String {
        format!("{location_id}:{week_id}")
    }

    /// Returns the cached deals hash, or computes it from the provided deals and caches it.
    pub fn resolve_deals_hash(&self, location_id: i64, week_id: &str, deals: &[Deal]) -> String {
        let key = Self::deals_hash_cache_key(location_id, week_id);
        let mut cache = self.deals_hash_cache.lock().unwrap();
        if let Some(hash) = cache.get(&key) {
            return hash.clone();
        }
        let hash = queries::compute_deals_hash(deals);
        cache.insert(key, hash.clone());
        hash
    }

    pub fn invalidate_deals_hash(&self, location_id: i64, week_id: &str) {
        let key = Self::deals_hash_cache_key(location_id, week_id);
        self.deals_hash_cache.lock().unwrap().remove(&key);
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "weekly_ad_recipes_server=debug,tower_http=debug".into()),
        )
        .init();

    let config = config::Config::from_env();

    let cwd = std::env::current_dir().unwrap_or_default();
    let static_dir = cwd.join("../client/dist");
    tracing::info!("Working directory: {}", cwd.display());
    tracing::info!(
        "Static files dir: {} (exists: {}, index.html exists: {})",
        static_dir.display(),
        static_dir.is_dir(),
        static_dir.join("index.html").is_file(),
    );

    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    let ai = std::sync::Arc::new(AnthropicClient::new(config.anthropic_api_key));
    let zip_geo = std::sync::Arc::new(ZipGeo::load().await);
    tracing::info!("Loaded {} zip code centroids", zip_geo.len());

    let state = AppState {
        pool,
        ai,
        zip_geo,
        deals_tracker: inflight::InFlightTracker::new(),
        meals_tracker: inflight::InFlightTracker::new(),
        deals_hash_cache: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
    };

    // Scrape WFM store catalog if empty, then re-check daily at 3 AM
    {
        let pool = state.pool.clone();
        tokio::spawn(async move {
            let known = crate::db::queries::get_known_wfm_slugs(&pool)
                .await
                .map(|slugs| slugs.len())
                .unwrap_or(0);

            if known == 0 {
                tracing::info!("No WFM stores in database, scraping catalog now");
                if let Err(error) = fetcher::wfm_stores::ensure_wfm_catalog(&pool).await {
                    tracing::warn!("WFM catalog scrape failed: {error}");
                }
            } else {
                tracing::info!("{known} WFM stores already cached, skipping boot scrape");
            }

            loop {
                let now = chrono::Local::now();
                let next_3am = (now + chrono::Duration::days(1))
                    .date_naive()
                    .and_hms_opt(3, 0, 0)
                    .unwrap();
                let next_3am = next_3am
                    .and_local_timezone(now.timezone())
                    .single()
                    .unwrap_or_else(|| now + chrono::Duration::days(1));
                let sleep_duration = (next_3am - now)
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(86400));

                tracing::info!(
                    "Next WFM catalog check in {:.1} hours",
                    sleep_duration.as_secs_f64() / 3600.0
                );
                tokio::time::sleep(sleep_duration).await;

                if let Err(error) = fetcher::wfm_stores::ensure_wfm_catalog(&pool).await {
                    tracing::warn!("WFM catalog scrape failed: {error}");
                }
            }
        });
    }

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/chains", get(routes::chains::list_chains))
        .route(
            "/api/locations/search",
            get(routes::locations::search_locations),
        )
        .route("/api/deals/:chain/:zip", get(routes::deals::get_deals))
        .route(
            "/api/deals/:chain/:zip/refresh",
            post(routes::deals::refresh_deals),
        )
        .route("/api/meals/:chain/:zip", get(routes::meals::get_meals))
        .nest_service("/api/thumbnails", ServeDir::new("data/thumbnails"))
        .fallback_service(
            ServeDir::new("../client/dist")
                .not_found_service(ServeFile::new("../client/dist/index.html")),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

async fn health() -> &'static str {
    "ok"
}

