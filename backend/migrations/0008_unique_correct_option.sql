-- Ensure at most one correct option per question.
-- Deduplicate any existing invalid data where multiple options are marked correct,
-- keeping only the first correct option per question (ordered by position, id).

UPDATE options o
SET is_correct = false
WHERE is_correct
  AND id NOT IN (
      SELECT DISTINCT ON (question_id) id
      FROM options
      WHERE is_correct
      ORDER BY question_id, position, id
  );

CREATE UNIQUE INDEX IF NOT EXISTS idx_options_one_correct
    ON options (question_id)
    WHERE is_correct;
