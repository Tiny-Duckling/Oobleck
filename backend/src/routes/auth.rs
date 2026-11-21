use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::create_token;
use crate::config::AppState;
use crate::error::{AppError, AppResult};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub username: String,
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AppError::Internal("failed to hash password".into()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

fn validate_credentials(username: &str, password: &str) -> AppResult<()> {
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::BadRequest("username must be 3-32 characters".into()));
    }
    if password.len() < 6 {
        return Err(AppError::BadRequest("password must be at least 6 characters".into()));
    }
    Ok(())
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    validate_credentials(&body.username, &body.password)?;

    let hash = hash_password(&body.password)?;

    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE username = $1",
    )
    .bind(&body.username)
    .fetch_one(&state.db)
    .await?;

    if exists > 0 {
        return Err(AppError::Conflict("username already taken".into()));
    }

    let user_id: String = sqlx::query_scalar(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id::text",
    )
    .bind(&body.username)
    .bind(&hash)
    .fetch_one(&state.db)
    .await?;

    let token = create_token(&user_id, &body.username, &state.jwt_secret)
        .map_err(|_| AppError::Internal("failed to create token".into()))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            username: body.username,
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id::text, password_hash FROM users WHERE username = $1",
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await?;

    let (user_id, hash) = match row {
        Some(r) => r,
        None => return Err(AppError::Unauthorized("invalid username or password".into())),
    };

    if !verify_password(&body.password, &hash) {
        return Err(AppError::Unauthorized("invalid username or password".into()));
    }

    let token = create_token(&user_id, &body.username, &state.jwt_secret)
        .map_err(|_| AppError::Internal("failed to create token".into()))?;

    Ok(Json(AuthResponse {
        token,
        username: body.username,
    }))
}

// Re-export PgPool usage marker to avoid unused import warnings in some builds.
#[allow(dead_code)]
fn _pool_type_check(_: PgPool) {}
