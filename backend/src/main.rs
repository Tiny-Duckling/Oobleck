use axum::http::{header, Method};
use axum::routing::{get, post};
use axum::Router;
use oobleck_backend::config::{AppState, Config};
use oobleck_backend::routes;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oobleck_backend=debug,tower_http=info".into()),
        )
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    // Run migrations on startup.
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = AppState {
        db: pool.clone(),
        jwt_secret: config.jwt_secret,
    };

    let cors = CorsLayer::new()
        .allow_origin(config.cors_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        // Cache preflight results in the browser for a day instead of
        // re-running OPTIONS before every cross-origin request.
        .max_age(std::time::Duration::from_secs(86_400));

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
        .route(
            "/api/quizzes/{id}/attempt",
            post(routes::quizzes::start_attempt),
        )
        .route("/api/me/history", get(routes::history::my_history))
        .route("/api/attempts/{id}", get(routes::history::get_attempt))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
