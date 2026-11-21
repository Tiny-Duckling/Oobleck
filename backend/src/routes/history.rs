use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::config::AppState;
use crate::error::AppResult;

#[derive(Serialize, sqlx::FromRow)]
pub struct AttemptDto {
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub quiz_title: String,
    /// `None` while the attempt has not been submitted.
    pub score: Option<i32>,
    pub total_questions: Option<i32>,
    /// `None` while the attempt has not been submitted.
    pub submitted_at: Option<DateTime<Utc>>,
    /// When the quiz was opened (also the submit time for legacy attempts).
    pub started_at: DateTime<Utc>,
}

/// One reviewed answer of a past attempt. Option texts are included so the
/// review page works without re-fetching the quiz (which may be a generated
/// quiz that could later be deleted).
#[derive(Serialize, sqlx::FromRow)]
pub struct AttemptAnswerDto {
    pub question_id: Uuid,
    pub question_text: String,
    pub position: i32,
    /// `None` for questions the user did not answer.
    pub selected_option_id: Option<Uuid>,
    pub selected_option_text: Option<String>,
    pub correct_option_id: Uuid,
    pub correct_option_text: String,
    pub is_correct: bool,
}

#[derive(Serialize)]
pub struct AttemptDetailDto {
    pub id: Uuid,
    pub quiz_id: Uuid,
    pub quiz_title: String,
    /// `None` while the attempt has not been submitted.
    pub score: Option<i32>,
    pub total_questions: Option<i32>,
    /// `None` while the attempt has not been submitted.
    pub submitted_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    pub results: Vec<AttemptAnswerDto>,
}

pub async fn my_history(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
) -> AppResult<Json<Vec<AttemptDto>>> {
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| crate::error::AppError::Unauthorized("invalid token subject".into()))?;

    let history = sqlx::query_as::<_, AttemptDto>(
        r#"
        SELECT a.id, a.quiz_id, q.title AS quiz_title, a.score, a.total_questions,
               a.submitted_at, a.started_at
        FROM attempts a
        JOIN quizzes q ON q.id = a.quiz_id
        WHERE a.user_id = $1
        ORDER BY COALESCE(a.submitted_at, a.started_at) DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(history))
}

/// Review a single past attempt: score plus every question with the selected
/// and correct answers. Only the attempt's owner may view it.
pub async fn get_attempt(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
    Path(attempt_id): Path<Uuid>,
) -> AppResult<Json<AttemptDetailDto>> {
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| crate::error::AppError::Unauthorized("invalid token subject".into()))?;

    // NotFound for both missing and foreign attempts so attempt ids aren't
    // enumerable across users.
    let (quiz_id, quiz_title, score, total_questions, submitted_at, started_at) =
        sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                Option<i32>,
                Option<i32>,
                Option<DateTime<Utc>>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT a.quiz_id, q.title, a.score, a.total_questions, a.submitted_at, a.started_at
            FROM attempts a
            JOIN quizzes q ON q.id = a.quiz_id
            WHERE a.id = $1 AND a.user_id = $2
            "#,
        )
        .bind(attempt_id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("attempt not found".into()))?;

    let results = sqlx::query_as::<_, AttemptAnswerDto>(
        r#"
        SELECT aa.question_id, qs.text AS question_text, qs.position,
               aa.selected_option_id, sel.text AS selected_option_text,
               o.id AS correct_option_id, o.text AS correct_option_text,
               aa.is_correct
        FROM attempt_answers aa
        JOIN questions qs ON qs.id = aa.question_id
        LEFT JOIN options sel ON sel.id = aa.selected_option_id
        JOIN LATERAL (
            SELECT id, text FROM options
            WHERE question_id = aa.question_id AND is_correct
            ORDER BY position
            LIMIT 1
        ) o ON true
        WHERE aa.attempt_id = $1
        ORDER BY qs.position
        "#,
    )
    .bind(attempt_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(AttemptDetailDto {
        id: attempt_id,
        quiz_id,
        quiz_title,
        score,
        total_questions,
        submitted_at,
        started_at,
        results,
    }))
}
