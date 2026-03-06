mod config;
mod db;
mod error;
mod fetcher;
mod models;
mod routes;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

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

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/chains", get(routes::chains::list_chains))
        .route("/api/locations", get(routes::locations::list_locations))
        .route("/api/locations", post(routes::locations::create_location))
        .route(
            "/api/locations/:id",
            delete(routes::locations::delete_location),
        )
        .route(
            "/api/locations/search",
            get(routes::locations::search_locations),
        )
        .route("/api/deals/:location_id", get(routes::deals::get_deals))
        .route(
            "/api/deals/:location_id/refresh",
            post(routes::deals::refresh_deals),
        )
        .route("/api/meals/:location_id", get(routes::meals::get_meals))
        .layer(CorsLayer::permissive())
        .with_state(pool);

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

