use anyhow::{bail, Context, Result};
use axum::http::HeaderValue;
use sqlx::PgPool;

/// Placeholder JWT secret in `.env.example` / the old committed `.env`.
/// Refused at startup so an example value never reaches production.
const JWT_PLACEHOLDER: &str = "change-me-to-a-long-random-string";

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub jwt_secret: String,
}

/// Runtime configuration, loaded from the environment (see `backend/.env.example`).
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub cors_origins: Vec<HeaderValue>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL is not set (copy backend/.env.example to backend/.env)")?;
        let jwt_secret = std::env::var("JWT_SECRET")
            .context("JWT_SECRET is not set (copy backend/.env.example to backend/.env)")?;

        if jwt_secret.len() < 32 {
            bail!(
                "JWT_SECRET is too short ({} chars, need >= 32). \
                 Generate one with: openssl rand -base64 48",
                jwt_secret.len()
            );
        }
        if jwt_secret == JWT_PLACEHOLDER {
            bail!(
                "JWT_SECRET is still the example placeholder. \
                 Generate one with: openssl rand -base64 48"
            );
        }

        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4000);

        let cors_origins = std::env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".to_string())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                HeaderValue::from_str(s).with_context(|| format!("invalid CORS origin {s:?}"))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            database_url,
            jwt_secret,
            port,
            cors_origins,
        })
    }
}
