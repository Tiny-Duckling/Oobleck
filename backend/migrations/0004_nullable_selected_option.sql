-- Unanswered questions must be distinguishable from answered ones in stored
-- attempts, so allow selected_option_id to be NULL.
ALTER TABLE attempt_answers ALTER COLUMN selected_option_id DROP NOT NULL;
