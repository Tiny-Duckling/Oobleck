use axum::http::HeaderValue;
use sqlx::PgPool;
use thiserror::Error;

/// Placeholder JWT secret in `.env.example` / the old committed `.env`.
/// Refused at startup so an example value never reaches production.
const JWT_PLACEHOLDER: &str = "change-me-to-a-long-random-string";

/// Configuration errors, surfaced with actionable messages at startup.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0} is not set (copy backend/.env.example to backend/.env)")]
    MissingEnv(&'static str),

    #[error("JWT_SECRET is too short ({0} chars, need >= 32). Generate one with: openssl rand -base64 48")]
    WeakJwtSecret(usize),

    #[error("JWT_SECRET is still the example placeholder. Generate one with: openssl rand -base64 48")]
    PlaceholderJwtSecret,

    #[error("invalid CORS origin {0:?}")]
    InvalidCorsOrigin(String),
}

impl ConfigError {
    fn var(name: &'static str) -> Result<String, Self> {
        std::env::var(name).map_err(|_| Self::MissingEnv(name))
    }
}

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
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = ConfigError::var("DATABASE_URL")?;
        let jwt_secret = ConfigError::var("JWT_SECRET")?;

        if jwt_secret.len() < 32 {
            return Err(ConfigError::WeakJwtSecret(jwt_secret.len()));
        }
        if jwt_secret == JWT_PLACEHOLDER {
            return Err(ConfigError::PlaceholderJwtSecret);
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
            .map(|s| HeaderValue::from_str(s).map_err(|_| ConfigError::InvalidCorsOrigin(s.to_string())))
            .collect::<Result<Vec<_>, ConfigError>>()?;

        Ok(Self {
            database_url,
            jwt_secret,
            port,
            cors_origins,
        })
    }
}
