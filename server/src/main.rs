mod ai;
mod config;
mod db;
mod error;
mod fetcher;
mod models;
mod routes;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::ai::client::AnthropicClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub ai: std::sync::Arc<AnthropicClient>,
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

    let state = AppState { pool, ai };

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/chains", get(routes::chains::list_chains))
        .route("/api/locations", get(routes::locations::list_locations))
        .route(
            "/api/locations/search",
            get(routes::locations::search_locations),
        )
        .route(
            "/api/locations/resolve",
            post(routes::locations::resolve_location),
        )
        .route("/api/deals/:location_id", get(routes::deals::get_deals))
        .route(
            "/api/deals/:location_id/refresh",
            post(routes::deals::refresh_deals),
        )
        .route("/api/meals/:location_id", get(routes::meals::get_meals))
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

