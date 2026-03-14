mod auth;
mod db;
mod llm;
mod projects;
mod chat;
mod ideation;
mod models;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::{CorsLayer, AllowOrigin};
use tower_http::set_header::SetResponseHeaderLayer;
use axum::http::HeaderValue;
use std::sync::Arc;

pub struct AppState {
    pub db: db::Db,
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8099);

    let data_dir = std::env::var("SF_DATA_DIR")
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| format!("{}/Library/Application Support/SimpleSF/data", h))
                .unwrap_or_else(|_| "/tmp/simple-sf".to_string())
        });

    std::fs::create_dir_all(&data_dir).ok();
    let db_path = format!("{}/simple-sf.db", data_dir);

    let db = db::Db::open(&db_path)?;
    db.migrate()?;

    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET env var is required — generate with: openssl rand -hex 32");

    let state = Arc::new(AppState { db, jwt_secret });

    let allowed_origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:8099".to_string());
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    // Security headers middleware
    let security_headers = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/me", get(auth::me))
        .route("/api/projects", get(projects::list).post(projects::create))
        .route("/api/projects/:id", get(projects::get_one).put(projects::update).delete(projects::remove))
        .route("/api/projects/:id/start", post(projects::start))
        .route("/api/projects/:id/stop", post(projects::stop))
        .route("/api/projects/:id/pause", post(projects::pause))
        .route("/api/chat/sessions", post(chat::create_session))
        .route("/api/chat/sessions/:id/message", post(chat::send_message))
        .route("/api/chat/sessions/:id/stream", post(chat::stream_message))
        .route("/api/chat/sessions/:id/history", get(chat::get_history))
        .route("/api/ideation/sessions", get(ideation::list).post(ideation::create))
        .route("/api/ideation/sessions/:id", get(ideation::get_one))
        .route("/api/ideation/sessions/:id/start", post(ideation::start))
        .route("/api/ideation/sessions/:id/stream", get(ideation::stream))
        .route("/api/providers", get(llm::list_providers))
        .route("/api/providers/:name/test", post(llm::test_provider))
        .with_state(state)
        .layer(cors)
        .layer(security_headers);

    let addr = format!("127.0.0.1:{}", port);
    tracing::info!("Simple SF Server running on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0",
        "engine": "rust"
    }))
}
