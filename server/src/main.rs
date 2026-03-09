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
use crate::models::deal::Deal;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub ai: std::sync::Arc<AnthropicClient>,
    pub deals_tracker: inflight::InFlightTracker,
    pub meals_tracker: inflight::InFlightTracker,
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
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    let ai = std::sync::Arc::new(AnthropicClient::new(config.anthropic_api_key));

    let state = AppState {
        pool,
        ai,
        deals_tracker: inflight::InFlightTracker::new(),
        meals_tracker: inflight::InFlightTracker::new(),
        deals_hash_cache: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
    };

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

