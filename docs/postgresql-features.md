# Advanced PostgreSQL Features in Oobleck

This document catalogs the PostgreSQL-specific features and SQL patterns used throughout the Oobleck codebase, explaining the architectural problem each pattern solves and where it is implemented.

---

## Table of Contents

1. [Correlated Subqueries via `JOIN LATERAL`](#1-correlated-subqueries-via-join-lateral)
2. [Aggregate `FILTER` Clauses & Native Array Aggregation](#2-aggregate-filter-clauses--native-array-aggregation)
3. [Pessimistic Row-Level Locking (`SELECT ... FOR UPDATE`)](#3-pessimistic-row-level-locking-select--for-update)
4. [Partial / Conditional Unique Indexes](#4-partial--conditional-unique-indexes)
5. [Multi-Table Deduplication (`DELETE ... USING`)](#5-multi-table-deduplication-delete--using)
6. [Atomic Copying (`INSERT INTO ... SELECT ... RETURNING`)](#6-atomic-copying-insert-into--select--returning)
7. [Native Cryptographic UUID Defaults (`gen_random_uuid()`)](#7-native-cryptographic-uuid-defaults-gen_random_uuid)

---

## 1. Correlated Subqueries via `JOIN LATERAL`

### Locations
- `backend/src/routes/quizzes.rs` (`submit_quiz`)
- `backend/src/routes/history.rs` (`get_attempt`)

### Query Snippet
```sql
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
```

### Explanation
In standard SQL, a subquery inside a `JOIN` cannot reference columns defined in preceding tables in the `FROM` list. PostgreSQL's `LATERAL` keyword acts like a `for-each` loop in SQL, evaluating the subquery independently for every row of `questions qs`.

### Why It Was Used in Oobleck
When grading a quiz or generating an attempt review, the system must resolve the single correct option for each question.
- Prior to migration `0008`, if data corruption or a faulty seed script marked multiple options as `is_correct = true`, a standard `JOIN options o ON o.question_id = qs.id AND o.is_correct` would duplicate the question row, inflating the question count and breaking score calculations.
- Combining `JOIN LATERAL` with `ORDER BY position LIMIT 1` was used to guarantee that **at most one** correct option row was matched per question.
- In migration `0008`, this query-level defense was promoted to a database-enforced guarantee via the partial unique index `idx_options_one_correct` (see [Section 4](#4-partial--conditional-unique-indexes)), allowing simpler direct joins while preserving the mathematical guarantee.

---

## 2. Aggregate `FILTER` Clauses & Native Array Aggregation

### Location
- `backend/src/routes/quizzes.rs` (`list_quizzes`)

### Query Snippet
```sql
SELECT q.id, q.title, q.description, q.total_questions::bigint AS question_count,
       COALESCE(ARRAY_AGG(DISTINCT s.name) FILTER (WHERE s.name IS NOT NULL),
                ARRAY[]::text[]) AS subjects
FROM quizzes q
LEFT JOIN questions qs ON qs.quiz_id = q.id
LEFT JOIN subjects s ON s.id = qs.subject_id
WHERE NOT q.is_generated
GROUP BY q.id, q.title, q.description, q.total_questions
ORDER BY q.created_at
```

### Explanation
- **`ARRAY_AGG(DISTINCT s.name)`**: Aggregates joined subject names into a native PostgreSQL array (`text[]`). SQLx decodes `text[]` directly into Rust's `Vec<String>` without requiring string splitting, JSON parsing, or subqueries.
- **`FILTER (WHERE s.name IS NOT NULL)`**: A SQL standard aggregate filter feature supported natively by PostgreSQL. For quizzes that have questions with no assigned subject (`NULL`), standard `ARRAY_AGG` would yield an array containing `[NULL]`. The `FILTER` clause discards `NULL` values prior to aggregation.
- **`COALESCE(..., ARRAY[]::text[])`**: When a quiz has zero subjects, `ARRAY_AGG` evaluates to `NULL`. `COALESCE` safely falls back to a typed, empty array (`ARRAY[]::text[]`), mapping cleanly to an empty Rust vector `vec![]`.

---

## 3. Pessimistic Row-Level Locking (`SELECT ... FOR UPDATE`)

### Location
- `backend/src/routes/quizzes.rs` (`submit_quiz`)

### Query Snippet
```sql
SELECT id, submitted_at IS NOT NULL
FROM attempts
WHERE id = $1 AND user_id = $2 AND quiz_id = $3
FOR UPDATE
```

### Explanation
`SELECT ... FOR UPDATE` locks the matching row against concurrent reads or writes by other transactions until the current transaction commits or rolls back.

### Why It's Used in Oobleck
To protect against double-submit race conditions:
1. When a user clicks "Submit" (or double-clicks rapidly), two requests may reach the server simultaneously.
2. The transaction acquires an exclusive row lock on the attempt row.
3. The first transaction checks `submitted_at IS NOT NULL` (false), grades the quiz, updates the attempt, sets `submitted_at = now()`, and commits.
4. The second transaction, which was queued waiting on the row lock, unblocks and re-evaluates the query. It now observes `submitted_at IS NOT NULL` (true) and immediately aborts with `409 Conflict`, preventing duplicate scoring and orphaned answer writes.

---

## 4. Partial / Conditional Unique Indexes

### Locations
- `backend/migrations/0005_in_progress_attempts.sql` (`idx_attempts_one_in_progress`)
- `backend/migrations/0008_unique_correct_option.sql` (`idx_options_one_correct`)

### DDL Snippets
```sql
-- At most one in-progress attempt per user and quiz:
CREATE UNIQUE INDEX IF NOT EXISTS idx_attempts_one_in_progress
    ON attempts (user_id, quiz_id) WHERE submitted_at IS NULL;

-- At most one correct option per question:
CREATE UNIQUE INDEX IF NOT EXISTS idx_options_one_correct
    ON options (question_id) WHERE is_correct;
```

### Explanation
PostgreSQL supports partial indexes—indexes built over a subset of a table defined by a conditional `WHERE` clause. When marked `UNIQUE`, uniqueness is enforced **only** among rows satisfying the predicate.

### Why They Are Used in Oobleck
1. **At most one in-progress attempt (`idx_attempts_one_in_progress`)**:
   - A user can submit the same quiz multiple times across days or weeks, producing multiple completed attempt records (`submitted_at IS NOT NULL`).
   - A user should have **at most one** unfinished / in-progress attempt per quiz (`submitted_at IS NULL`).
   - This index guarantees at the database level that multiple tabs or concurrent start requests cannot create duplicate in-progress attempts.

2. **At most one correct option per question (`idx_options_one_correct`)**:
   - Multiple incorrect options (`is_correct = false`) must be allowed per question.
   - At most one option with `is_correct = true` may exist per question.
   - This structural guarantee eliminates duplicate question joins and prevents score inflation at the database level, allowing queries in `submit_quiz` and `get_attempt` to perform clean, direct joins on `is_correct`.

---

## 5. Multi-Table Deduplication (`DELETE ... USING`)

### Location
- `backend/migrations/0006_unique_attempt_answers.sql`

### DDL Snippet
```sql
DELETE FROM attempt_answers a
USING attempt_answers b
WHERE a.attempt_id = b.attempt_id
  AND a.question_id = b.question_id
  AND a.id < b.id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_attempt_answers_attempt_question
    ON attempt_answers (attempt_id, question_id);
```

### Explanation
PostgreSQL's `DELETE ... USING` syntax allows joining other tables (or the same table) directly into a `DELETE` statement.

### Why It's Used in Oobleck
Migration `0006` enforced that a question can only be answered once per attempt. Before applying the `UNIQUE INDEX`, any historical duplicates caused by earlier concurrency bugs had to be cleaned up. The self-join `USING` clause deletes all but the most recent row (`a.id < b.id`) in a single set-based operation without requiring temporary staging tables or procedural cursors.

---

## 6. Atomic Copying (`INSERT INTO ... SELECT ... RETURNING`)

### Location
- `backend/src/routes/quizzes.rs` (`generate_quiz`)

### Query Snippet
```sql
INSERT INTO questions (quiz_id, text, position, subject_id)
SELECT $1, text, $2, subject_id FROM questions WHERE id = $3
RETURNING id
```

### Explanation
PostgreSQL allows combining `INSERT INTO ... SELECT` with a `RETURNING` clause to perform an atomic copy of rows while returning the newly generated primary keys in the same network round-trip.

### Why It's Used in Oobleck
When generating a randomized practice quiz from a question pool:
1. The question text and subject associations are copied directly inside PostgreSQL without deserializing them into Rust application memory.
2. The generated UUID is returned immediately so child option records can be copied next.

---

## 7. Native Cryptographic UUID Defaults (`gen_random_uuid()`)

### Location
- `backend/migrations/0001_init.sql`

### DDL Snippet
```sql
CREATE TABLE IF NOT EXISTS quizzes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### Explanation
Starting with PostgreSQL 13, `gen_random_uuid()` is built directly into core PostgreSQL without requiring the external `uuid-ossp` or `pgcrypto` extensions.

### Why It's Used in Oobleck
- Produces cryptographically strong Version 4 UUIDs natively.
- Keeps tables decoupled from auto-increment sequence coordination.
- Eliminates migration setup dependencies on superuser privileges for `CREATE EXTENSION`.
