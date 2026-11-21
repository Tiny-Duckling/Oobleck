-- Allow attempts to exist before they are submitted, so history includes
-- quizzes the user started but never submitted.

ALTER TABLE attempts ALTER COLUMN score DROP NOT NULL;
ALTER TABLE attempts ALTER COLUMN total_questions DROP NOT NULL;
ALTER TABLE attempts ALTER COLUMN submitted_at DROP NOT NULL;
ALTER TABLE attempts ALTER COLUMN submitted_at DROP DEFAULT;

ALTER TABLE attempts ADD COLUMN started_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Backfill: every pre-existing attempt was created at submit time.
UPDATE attempts SET started_at = submitted_at;

-- At most one unsubmitted attempt per user and quiz; a repeated attempt id
-- (e.g. the same quiz opened in a second tab) reuses it.
CREATE UNIQUE INDEX IF NOT EXISTS idx_attempts_one_in_progress
    ON attempts (user_id, quiz_id) WHERE submitted_at IS NULL;
