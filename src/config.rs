use serde::Deserialize;
use std::{env, fs};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub supabase_url: String,
    pub supabase_jwt_secret: String,
    pub resend_api_key: String,
        pub resend_webhook_secret: String,
    pub from_email: String,
    pub app_url: String,
    pub port: u16,
}

#[derive(Deserialize)]
struct FileConfig {
    from_email: String,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 {
    8080
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let raw = fs::read_to_string("config.toml")
            .unwrap_or_else(|_| panic!("missing config.toml"));
        let file: FileConfig =
            toml::from_str(&raw).unwrap_or_else(|e| panic!("invalid config.toml: {e}"));

        Self {
            database_url: must("DATABASE_URL"),
            supabase_url: must("SUPABASE_URL"),
            supabase_jwt_secret: must("SUPABASE_JWT_SECRET"),
            resend_api_key: must("RESEND_API_KEY"),
            resend_webhook_secret: must("RESEND_WEBHOOK_SECRET"),
            from_email: file.from_email,
            app_url: env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
            port: env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(file.port),
        }
    }
}

fn must(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("missing required env var: {key}"))
}