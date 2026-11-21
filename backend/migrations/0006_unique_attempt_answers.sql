-- A question may only be answered once per attempt. Concurrent double-submits
-- could previously insert duplicate rows for the same question; delete any
-- existing duplicates before enforcing uniqueness.
DELETE FROM attempt_answers a
USING attempt_answers b
WHERE a.attempt_id = b.attempt_id
  AND a.question_id = b.question_id
  AND a.id < b.id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_attempt_answers_attempt_question
    ON attempt_answers (attempt_id, question_id);
