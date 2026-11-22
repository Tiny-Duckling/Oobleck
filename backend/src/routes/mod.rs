pub mod auth;
pub mod history;
pub mod quizzes;

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
}

pub async fn health() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}
