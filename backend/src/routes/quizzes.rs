use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::config::AppState;
use crate::error::{AppError, AppResult};

#[derive(Serialize, sqlx::FromRow)]
pub struct QuizSummary {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub question_count: i64,
    /// Distinct subjects of the quiz's questions (may be empty).
    pub subjects: Vec<String>,
}

#[derive(Serialize)]
pub struct OptionDto {
    pub id: Uuid,
    pub text: String,
}

#[derive(Serialize)]
pub struct QuestionDto {
    pub id: Uuid,
    pub text: String,
    pub subject: Option<String>,
    pub options: Vec<OptionDto>,
}

#[derive(Serialize)]
pub struct QuizDetail {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub questions: Vec<QuestionDto>,
}

#[derive(Deserialize)]
pub struct SubmitRequest {
    pub answers: Vec<AnswerSubmission>,
}

#[derive(Deserialize)]
pub struct AnswerSubmission {
    pub question_id: Uuid,
    pub selected_option_id: Uuid,
}

#[derive(Serialize)]
pub struct AnswerResult {
    pub question_id: Uuid,
    /// `None` when the question was not answered.
    pub selected_option_id: Option<Uuid>,
    pub correct_option_id: Uuid,
    pub is_correct: bool,
}

#[derive(Serialize)]
pub struct SubmitResponse {
    pub attempt_id: Uuid,
    pub score: i32,
    pub total_questions: i32,
    pub results: Vec<AnswerResult>,
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub subject: String,
    /// Number of questions to include; clamped to 1..=50.
    pub count: i64,
}

#[derive(Serialize)]
pub struct GenerateResponse {
    pub quiz_id: Uuid,
}

pub async fn list_quizzes(State(state): State<AppState>) -> AppResult<Json<Vec<QuizSummary>>> {
    let quizzes = sqlx::query_as::<_, QuizSummary>(
        r#"
        SELECT q.id, q.title, q.description, COUNT(DISTINCT qs.id)::bigint AS question_count,
               COALESCE(ARRAY_AGG(DISTINCT s.name) FILTER (WHERE s.name IS NOT NULL),
                        ARRAY[]::text[]) AS subjects
        FROM quizzes q
        LEFT JOIN questions qs ON qs.quiz_id = q.id
        LEFT JOIN subjects s ON s.id = qs.subject_id
        WHERE NOT q.is_generated
        GROUP BY q.id, q.title, q.description
        ORDER BY q.created_at
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(quizzes))
}

pub async fn get_quiz(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(quiz_id): Path<Uuid>,
) -> AppResult<Json<QuizDetail>> {
    let quiz = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, title, description FROM quizzes WHERE id = $1",
    )
    .bind(quiz_id)
    .fetch_optional(&state.db)
    .await?;

    let (id, title, description) =
        quiz.ok_or_else(|| AppError::NotFound("quiz not found".into()))?;

    let questions = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        r#"
        SELECT qs.id, qs.text, s.name
        FROM questions qs
        LEFT JOIN subjects s ON s.id = qs.subject_id
        WHERE qs.quiz_id = $1
        ORDER BY qs.position
        "#,
    )
    .bind(quiz_id)
    .fetch_all(&state.db)
    .await?;

    let mut question_dtos = Vec::with_capacity(questions.len());
    for (qid, qtext, qsubject) in questions {
        // NOTE: is_correct is intentionally not selected — answers must not leak.
        let options = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, text FROM options WHERE question_id = $1 ORDER BY position",
        )
        .bind(qid)
        .fetch_all(&state.db)
        .await?;

        question_dtos.push(QuestionDto {
            id: qid,
            text: qtext,
            subject: qsubject,
            options: options
                .into_iter()
                .map(|(oid, otext)| OptionDto { id: oid, text: otext })
                .collect(),
        });
    }

    Ok(Json(QuizDetail {
        id,
        title,
        description,
        questions: question_dtos,
    }))
}

pub async fn submit_quiz(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
    Path(quiz_id): Path<Uuid>,
    Json(body): Json<SubmitRequest>,
) -> AppResult<Json<SubmitResponse>> {
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::Unauthorized("invalid token subject".into()))?;

    // Load correct answers for this quiz (question_id -> correct option_id).
    let correct: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT qs.id, o.id
        FROM questions qs
        JOIN options o ON o.question_id = qs.id AND o.is_correct
        WHERE qs.quiz_id = $1
        ORDER BY qs.position
        "#,
    )
    .bind(quiz_id)
    .fetch_all(&state.db)
    .await?;

    if correct.is_empty() {
        return Err(AppError::NotFound("quiz not found or has no questions".into()));
    }

    let total = correct.len() as i32;
    let mut results = Vec::with_capacity(correct.len());

    for (question_id, correct_option_id) in &correct {
        let selected = body
            .answers
            .iter()
            .find(|a| a.question_id == *question_id)
            .map(|a| a.selected_option_id);

        let is_correct = selected == Some(*correct_option_id);
        results.push(AnswerResult {
            question_id: *question_id,
            selected_option_id: selected,
            correct_option_id: *correct_option_id,
            is_correct,
        });
    }

    let score = results.iter().filter(|r| r.is_correct).count() as i32;

    let mut tx = state.db.begin().await?;

    let attempt_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO attempts (user_id, quiz_id, score, total_questions)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(quiz_id)
    .bind(score)
    .bind(total)
    .fetch_one(&mut *tx)
    .await?;

    for r in &results {
        sqlx::query(
            "INSERT INTO attempt_answers (attempt_id, question_id, selected_option_id, is_correct) VALUES ($1, $2, $3, $4)",
        )
        .bind(attempt_id)
        .bind(r.question_id)
        .bind(r.selected_option_id)
        .bind(r.is_correct)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(SubmitResponse {
        attempt_id,
        score,
        total_questions: total,
        results,
    }))
}

/// Create a practice quiz with `count` random questions from a subject.
/// Questions (and their options) are copied into a new, private quiz so the
/// normal take/submit/history flow works unchanged.
pub async fn generate_quiz(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(body): Json<GenerateRequest>,
) -> AppResult<Json<GenerateResponse>> {
    let count = body.count.clamp(1, 50);

    let subject_id: Uuid =
        sqlx::query_scalar("SELECT id FROM subjects WHERE lower(name) = lower($1)")
            .bind(&body.subject)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("subject not found".into()))?;

    let selected: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM questions WHERE subject_id = $1 ORDER BY random() LIMIT $2",
    )
    .bind(subject_id)
    .bind(count)
    .fetch_all(&state.db)
    .await?;

    if selected.is_empty() {
        return Err(AppError::NotFound("subject has no questions".into()));
    }

    let n = selected.len();
    let title = format!("Random Mix \u{2014} {} ({n} questions)", body.subject);
    let description = format!(
        "Practice quiz generated from {n} random question(s) in \u{201c}{}\u{201d}.",
        body.subject
    );

    let mut tx = state.db.begin().await?;

    let quiz_id: Uuid = sqlx::query_scalar(
        "INSERT INTO quizzes (title, description, is_generated) VALUES ($1, $2, true) RETURNING id",
    )
    .bind(&title)
    .bind(&description)
    .fetch_one(&mut *tx)
    .await?;

    for (pos, source_question_id) in selected.iter().enumerate() {
        // Copy the question, then its options (correct answers stay server-side).
        let new_question_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO questions (quiz_id, text, position, subject_id)
            SELECT $1, text, $2, subject_id FROM questions WHERE id = $3
            RETURNING id
            "#,
        )
        .bind(quiz_id)
        .bind(pos as i32)
        .bind(source_question_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO options (question_id, text, is_correct, position)
            SELECT $1, text, is_correct, position FROM options WHERE question_id = $2
            "#,
        )
        .bind(new_question_id)
        .bind(source_question_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(GenerateResponse { quiz_id }))
}

#[allow(dead_code)]
fn _pool_type_check(_: PgPool) {}
