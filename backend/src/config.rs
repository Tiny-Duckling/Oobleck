use axum::http::HeaderValue;
use sqlx::PgPool;
use thiserror::Error;
use url::Url;

/// Configuration errors, surfaced with actionable messages at startup.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0} is not set (copy backend/.env.example to backend/.env)")]
    MissingEnv(&'static str),

    #[error("JWT_SECRET is too short ({0} chars, need >= 32). Generate one with: openssl rand -base64 48")]
    WeakJwtSecret(usize),

    #[error(
        "invalid CORS origin {0:?} (expected scheme://host[:port], e.g. http://localhost:5173)"
    )]
    InvalidCorsOrigin(String),
}

impl ConfigError {
    fn var(name: &'static str) -> Result<String, Self> {
        std::env::var(name).map_err(|_| Self::MissingEnv(name))
    }
}

/// Parse and normalize a CORS origin to the exact form browsers send in the
/// `Origin` header: `scheme://host[:port]` — no path, query, credentials, or
/// default ports (80/443). Browsers match `Origin` strictly, so a value like
/// `localhost:5173` or `http://localhost:5173/` would silently never match;
/// rejecting those here turns a typo into a startup error instead.
pub fn parse_origin(raw: &str) -> Result<HeaderValue, ConfigError> {
    let invalid = || ConfigError::InvalidCorsOrigin(raw.to_string());

    let url = Url::parse(raw).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }

    let host = url.host_str().ok_or_else(invalid)?;
    let mut origin = format!("{}://{}", url.scheme(), host);
    if let Some(port) = url.port() {
        let is_default =
            (url.scheme() == "http" && port == 80) || (url.scheme() == "https" && port == 443);
        if !is_default {
            origin.push_str(&format!(":{port}"));
        }
    }

    HeaderValue::from_str(&origin).map_err(|_| invalid())
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

        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4000);

        let cors_origins = std::env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".to_string())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(parse_origin)
            .collect::<Result<Vec<_>, ConfigError>>()?;

        Ok(Self {
            database_url,
            jwt_secret,
            port,
            cors_origins,
        })
    }
}
