use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id (UUID)
    pub username: String,
    pub exp: i64,
}

pub fn create_token(
    user_id: &str,
    username: &str,
    secret: &str,
) -> jsonwebtoken::errors::Result<String> {
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: Utc::now().timestamp() + 7 * 24 * 3600,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn verify_token(token: &str, secret: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .map(|data| data.claims)
}

/// Extractor that requires a valid `Authorization: Bearer <token>` header
/// and provides the authenticated user's id and username.
pub struct AuthUser {
    pub user_id: String,
    #[allow(dead_code)]
    pub username: String,
}

impl FromRequestParts<crate::config::AppState> for AuthUser {
    type Rejection = crate::error::AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::config::AppState,
    ) -> Result<Self, Self::Rejection> {
        let secret = state.jwt_secret.clone();

        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| crate::error::AppError::Unauthorized("missing token".into()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| crate::error::AppError::Unauthorized("invalid token format".into()))?;

        let claims = verify_token(token, &secret).ok_or_else(|| {
            crate::error::AppError::Unauthorized("invalid or expired token".into())
        })?;

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
        })
    }
}
