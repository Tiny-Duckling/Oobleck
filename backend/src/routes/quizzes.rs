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
    /// Optional id of the in-progress attempt created when the quiz was
    /// opened. When absent, the user's unsubmitted attempt for this quiz is
    /// submitted (if any) or a new attempt is created.
    #[serde(default)]
    pub attempt_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct StartAttemptResponse {
    pub attempt_id: Uuid,
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
                .map(|(oid, otext)| OptionDto {
                    id: oid,
                    text: otext,
                })
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

/// Create (or reuse) the caller's in-progress attempt for a quiz. Called when
/// a quiz is opened so that history includes quizzes that are never submitted.
pub async fn start_attempt(
    State(state): State<AppState>,
    AuthUser { user_id, .. }: AuthUser,
    Path(quiz_id): Path<Uuid>,
) -> AppResult<Json<StartAttemptResponse>> {
    let user_id = Uuid::parse_str(&user_id)
        .map_err(|_| AppError::Unauthorized("invalid token subject".into()))?;

    let quiz_exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM quizzes WHERE id = $1")
        .bind(quiz_id)
        .fetch_optional(&state.db)
        .await?;
    if quiz_exists.is_none() {
        return Err(AppError::NotFound("quiz not found".into()));
    }

    // Reuse the user's unsubmitted attempt for this quiz, if any, so repeated
    // visits (or a second tab) don't create duplicate history entries.
    if let Some(attempt_id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM attempts WHERE user_id = $1 AND quiz_id = $2 AND submitted_at IS NULL",
    )
    .bind(user_id)
    .bind(quiz_id)
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(Json(StartAttemptResponse { attempt_id }));
    }

    let attempt_id: Uuid =
        match sqlx::query_scalar("INSERT INTO attempts (user_id, quiz_id) VALUES ($1, $2) RETURNING id")
            .bind(user_id)
            .bind(quiz_id)
            .fetch_one(&state.db)
            .await
        {
            Ok(id) => id,
            // A concurrent request created the in-progress attempt first;
            // reuse it (idx_attempts_one_in_progress).
            Err(err)
                if err
                    .as_database_error()
                    .is_some_and(|e| e.is_unique_violation()) =>
            {
                sqlx::query_scalar::<_, Uuid>(
                    "SELECT id FROM attempts WHERE user_id = $1 AND quiz_id = $2 AND submitted_at IS NULL",
                )
                .bind(user_id)
                .bind(quiz_id)
                .fetch_one(&state.db)
                .await?
            }
            Err(err) => return Err(err.into()),
        };

    Ok(Json(StartAttemptResponse { attempt_id }))
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
    // LATERAL + LIMIT 1 keeps exactly one row per question even if bad data
    // marks multiple options correct; attempt_answers is unique per question.
    let correct: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT qs.id, o.id
        FROM questions qs
        JOIN LATERAL (
            SELECT id FROM options
            WHERE question_id = qs.id AND is_correct
            ORDER BY position
            LIMIT 1
        ) o ON true
        WHERE qs.quiz_id = $1
        ORDER BY qs.position
        "#,
    )
    .bind(quiz_id)
    .fetch_all(&state.db)
    .await?;

    if correct.is_empty() {
        return Err(AppError::NotFound(
            "quiz not found or has no questions".into(),
        ));
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

    // Resolve and lock the attempt inside the transaction. The row lock makes
    // concurrent submits serialize: the second one sees `submitted_at` set and
    // returns 409 instead of inserting duplicate attempt_answers rows.
    let existing_attempt_id: Option<Uuid> = if let Some(id) = body.attempt_id {
        let (id, submitted) = sqlx::query_as::<_, (Uuid, bool)>(
            r#"
            SELECT id, submitted_at IS NOT NULL
            FROM attempts
            WHERE id = $1 AND user_id = $2 AND quiz_id = $3
            FOR UPDATE
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(quiz_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("attempt not found".into()))?;

        if submitted {
            return Err(AppError::Conflict("attempt already submitted".into()));
        }
        Some(id)
    } else {
        // No explicit id: reuse the user's unsubmitted attempt for this quiz.
        sqlx::query_scalar(
            r#"
            SELECT id FROM attempts
            WHERE user_id = $1 AND quiz_id = $2 AND submitted_at IS NULL
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .bind(quiz_id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let attempt_id: Uuid = match existing_attempt_id {
        Some(id) => {
            // Submitting an attempt that was created when the quiz was opened.
            // Old answer rows shouldn't exist, but clear them defensively.
            sqlx::query("DELETE FROM attempt_answers WHERE attempt_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            let updated = sqlx::query(
                r#"
                UPDATE attempts
                SET score = $2, total_questions = $3, submitted_at = now()
                WHERE id = $1 AND submitted_at IS NULL
                "#,
            )
            .bind(id)
            .bind(score)
            .bind(total)
            .execute(&mut *tx)
            .await?;
            if updated.rows_affected() == 0 {
                return Err(AppError::Conflict("attempt already submitted".into()));
            }
            id
        }
        None => {
            sqlx::query_scalar(
                r#"
                INSERT INTO attempts (user_id, quiz_id, score, total_questions, submitted_at)
                VALUES ($1, $2, $3, $4, now())
                RETURNING id
                "#,
            )
            .bind(user_id)
            .bind(quiz_id)
            .bind(score)
            .bind(total)
            .fetch_one(&mut *tx)
            .await?
        }
    };

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
