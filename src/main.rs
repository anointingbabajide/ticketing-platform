mod auth;
mod config;
mod email;
mod error;
mod jwks;
mod models;
mod routes;

use axum::{
    extract::FromRef,
    routing::get,
    Json, Router,
};
use jsonwebtoken::jwk::Jwk;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashMap, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use auth::AuthUser;
use config::Config;

async fn me(AuthUser(profile): AuthUser) -> Json<models::Profile> {
    Json(profile)
}

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Config,
    pub jwks: Arc<HashMap<String, Jwk>>,
}

impl FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let config = Config::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate::Migrator::new(std::path::Path::new("./migrations"))
        .await?
        .run(&pool)
        .await?;

    let jwk_set = jwks::fetch_jwks(&config.supabase_url).await?;
    let jwks_by_kid = Arc::new(jwks::index_by_kid(&jwk_set));

    let state = AppState {
        pool,
        config: config.clone(),
        jwks: jwks_by_kid,
    };

    let app = Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/me", get(me))
        .merge(routes::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}