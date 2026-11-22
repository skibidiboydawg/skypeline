mod auth;
mod db;
mod hub;

use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
    pub hub: hub::RelayHub,
}

#[tokio::main]
async fn main() {
    // Local dev: load .env, then Neon's .env.local (both git-ignored).
    // Production (Render/HF): real environment variables take precedence.
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_path(".env.local");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "skypeline_server=info,tower_http=info".into()),
        )
        .init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set (run `neon env pull` or see README)");
    // Neon writes the direct/non-pooled URL too. Use it for DDL migrations;
    // pooled connections are for application traffic only.
    let migration_url = env::var("DATABASE_URL_UNPOOLED").ok();
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set (see README)");
    if jwt_secret.len() < 16 || jwt_secret == "change_me_to_random_secret" {
        panic!("JWT_SECRET is too weak; set a long random secret");
    }
    let port: u16 = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(7860);

    tracing::info!("connecting to PostgreSQL and running migrations...");
    let pool = db::connect_and_migrate(&database_url, migration_url.as_deref())
        .await
        .expect("failed to connect to PostgreSQL");

    let state = Arc::new(AppState {
        db: pool,
        jwt_secret,
        hub: hub::RelayHub::new(),
    });

    let app = Router::new()
        .route("/", get(|| async { "Skypeline relay server is running" }))
        .route("/health", get(|| async { "ok" }))
        .route("/api/register", post(auth::register))
        .route("/api/login", post(auth::login))
        .route("/api/channels", get(auth::list_channels))
        .route("/api/messages/{channel_id}", get(auth::list_messages))
        .route("/api/dm/{user_id}", post(auth::open_dm))
        .route("/ws", get(hub::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Skypeline relay server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind port");
    axum::serve(listener, app).await.expect("server exited unexpectedly");
}
