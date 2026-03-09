use std::env;

pub struct Config {
    pub database_url: String,
    pub anthropic_api_key: String,
    pub port: u16,
    pub static_dir: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db".into()),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY")
                .unwrap_or_default(),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3001),
            static_dir: env::var("STATIC_DIR")
                .unwrap_or_else(|_| "../client/dist".into()),
        }
    }
}
