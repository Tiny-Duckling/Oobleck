//! Unit and integration tests for attempt and history serialization.

use chrono::Utc;
use oobleck_backend::routes::history::{AttemptDetailDto, AttemptDto};
use uuid::Uuid;

#[test]
fn in_progress_attempt_dto_has_total_questions_and_none_score() {
    let attempt = AttemptDto {
        id: Uuid::new_v4(),
        quiz_id: Uuid::new_v4(),
        quiz_title: "GRE Vocabulary Set 1".to_string(),
        score: None,
        total_questions: 10,
        started_at: Utc::now(),
        submitted_at: None,
    };

    let json = serde_json::to_value(&attempt).expect("serialization failed");
    assert_eq!(json["total_questions"], 10);
    assert!(json["score"].is_null());
    assert!(json["submitted_at"].is_null());
}

#[test]
fn completed_attempt_dto_has_score_and_total_questions() {
    let attempt = AttemptDto {
        id: Uuid::new_v4(),
        quiz_id: Uuid::new_v4(),
        quiz_title: "Math Basics".to_string(),
        score: Some(8),
        total_questions: 10,
        started_at: Utc::now(),
        submitted_at: Some(Utc::now()),
    };

    let json = serde_json::to_value(&attempt).expect("serialization failed");
    assert_eq!(json["score"], 8);
    assert_eq!(json["total_questions"], 10);
    assert!(json["submitted_at"].is_string());
}

#[test]
fn attempt_detail_dto_has_non_optional_total_questions() {
    let detail = AttemptDetailDto {
        id: Uuid::new_v4(),
        quiz_id: Uuid::new_v4(),
        quiz_title: "Chemistry 101".to_string(),
        score: None,
        total_questions: 25,
        submitted_at: None,
        started_at: Utc::now(),
        results: vec![],
    };

    let json = serde_json::to_value(&detail).expect("serialization failed");
    assert_eq!(json["total_questions"], 25);
    assert!(json["score"].is_null());
}
