mod auth;
mod config;
mod error;
mod routes;

use axum::http::{header, Method};
use axum::routing::{get, post};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "quiz_backend=debug,tower_http=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let jwt_secret = std::env::var("JWT_SECRET")?;
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4000);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // Run migrations on startup.
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        db: pool.clone(),
        jwt_secret,
    };

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse()?,
            "http://127.0.0.1:5173".parse()?,
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/api/auth/register", post(routes::auth::register))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/quizzes", get(routes::quizzes::list_quizzes))
        .route("/api/quizzes/generate", post(routes::quizzes::generate_quiz))
        .route("/api/quizzes/{id}", get(routes::quizzes::get_quiz))
        .route(
            "/api/quizzes/{id}/submit",
            post(routes::quizzes::submit_quiz),
        )
        .route("/api/me/history", get(routes::history::my_history))
        .route("/api/attempts/{id}", get(routes::history::get_attempt))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
