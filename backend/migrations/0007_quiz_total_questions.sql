-- Move total_questions from attempts to quizzes.
-- Quizzes are immutable, so total_questions is a fixed property of the quiz,
-- not of individual attempts.

ALTER TABLE quizzes ADD COLUMN IF NOT EXISTS total_questions INT NOT NULL DEFAULT 0;

-- Backfill total_questions for existing quizzes from their questions count.
UPDATE quizzes q
SET total_questions = (
    SELECT COUNT(*)::INT
    FROM questions qs
    WHERE qs.quiz_id = q.id
);

-- Drop total_questions from attempts table.
ALTER TABLE attempts DROP COLUMN IF EXISTS total_questions;
