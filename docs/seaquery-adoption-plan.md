# SeaQuery Adoption Plan & Prerequisites

This document outlines the architectural roadmap and technical prerequisites required before introducing **SeaQuery** into the Oobleck backend.

---

## 1. Executive Summary & Philosophy

SeaQuery is a dynamic SQL query builder for Rust that compiles queries into parameterized SQL strings at runtime.

### Current State
100% of Oobleck’s queries today are **static raw SQL** executed via SQLx (`query`, `query_as`). For static queries, raw SQL is cleaner, faster to compile, and has zero boilerplate.

### Why Adopt SeaQuery Later, Not Now?
Introducing SeaQuery without a dynamic querying requirement adds substantial identifier boilerplate and extra compile-time macros without providing tangible benefits. 

We will introduce SeaQuery **only when a feature demands runtime-constructed SQL** (e.g. multi-field filtering, dynamic sorting, or variable-length bulk inserts). When introduced, we will use a **hybrid architecture**—keeping raw SQL for advanced PostgreSQL-specific queries.

---

## 2. Adoption Trigger (When to Adopt)

Do not introduce SeaQuery until at least one of the following requirements is prioritized:

1. **Dynamic Quiz Search & Filtering:**
   - E.g. `GET /api/quizzes?search=gre&subject=math&min_questions=10&sort_by=created_at&order=desc&page=1`
   - Managing conditional `WHERE` clauses and dynamically numbering `$1, $2, $3...` parameters in raw strings becomes bug-prone.
2. **Dynamic Attempt History Filtering:**
   - E.g. `GET /api/me/history?quiz_id=...&status=in_progress|submitted&from=...&to=...`
3. **Dynamic Batch Operations:**
   - Variable-length bulk inserts/upserts where the number of values varies per request.

---

## 3. Coexistence Strategy (Hybrid Architecture)

SeaQuery will **not** replace all existing SQLx queries. We maintain a strict boundary:

### Use SeaQuery for:
- Dynamic `SELECT` queries with conditional filters, search terms, and sorting.
- Dynamic pagination and count wrappers.
- Runtime-constructed multi-row batch inserts.

### Keep Raw SQLx for:
- **Advanced PostgreSQL features:**
  - Aggregate filter clauses: `ARRAY_AGG(...) FILTER (WHERE s.name IS NOT NULL)` in `list_quizzes`.
  - Correlated joins: `JOIN LATERAL (...)` or complex subqueries.
- **Pessimistic row locking:**
  - `SELECT ... FOR UPDATE` in `submit_quiz`.
- **Atomic database-internal copying:**
  - `INSERT INTO ... SELECT ... RETURNING` in `generate_quiz`.
- **Simple static queries:**
  - 1-to-2 line queries like `SELECT title, description FROM quizzes WHERE id = $1`.

---

## 4. Technical Prerequisites Checklist

Before writing the first SeaQuery route handler, complete the following setup steps:

### Step 1: Align Dependencies & Feature Flags
Add SeaQuery to `backend/Cargo.toml` with feature flags matching Oobleck’s existing type ecosystem:

```toml
[dependencies]
sea-query = { version = "0.32", features = ["derive", "attr"] }
sea-query-binder = { version = "0.7", features = ["sqlx-postgres", "with-uuid", "with-chrono"] }
```

*Pre-flight check:* Ensure `with-uuid` and `with-chrono` align with the `uuid` (v1) and `chrono` (v0.4) versions in `Cargo.lock` to prevent type-mismatch compilation errors.

---

### Step 2: Centralize Schema Identifiers (`Iden`)
Create a dedicated schema module (`backend/src/schema.rs` or `backend/src/db/schema.rs`) containing `Iden` enums for all database tables and columns:

```rust
use sea_query::Iden;

#[derive(Iden)]
pub enum Quizzes {
    Table,
    Id,
    Title,
    Description,
    TotalQuestions,
    IsGenerated,
    CreatedAt,
}

#[derive(Iden)]
pub enum Questions {
    Table,
    Id,
    QuizId,
    Text,
    Position,
    SubjectId,
}

#[derive(Iden)]
pub enum Options {
    Table,
    Id,
    QuestionId,
    Text,
    IsCorrect,
    Position,
}

#[derive(Iden)]
pub enum Attempts {
    Table,
    Id,
    UserId,
    QuizId,
    Score,
    StartedAt,
    SubmittedAt,
}

#[derive(Iden)]
pub enum AttemptAnswers {
    Table,
    Id,
    AttemptId,
    QuestionId,
    SelectedOptionId,
    IsCorrect,
}

#[derive(Iden)]
pub enum Subjects {
    Table,
    Id,
    Name,
    CreatedAt,
}
```

*Naming rule:* SeaQuery derives convert PascalCase variants (`TotalQuestions`) to snake_case (`total_questions`) automatically. Verify that every variant matches the PostgreSQL column name.

---

### Step 3: Implement the SQLx Execution Helper
To avoid repeating `.build_sqlx(PostgresQueryBuilder)` and `sqlx::query_as_with` in every route handler, write a shared database bridge (e.g. `backend/src/db/mod.rs`):

```rust
use sea_query::{PostgresQueryBuilder, SelectStatement};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool};
use crate::error::AppResult;

pub async fn fetch_all<T>(
    query: &SelectStatement,
    pool: &PgPool,
) -> AppResult<Vec<T>>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
    let rows = sqlx::query_as_with::<_, T, _>(&sql, values)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn fetch_optional<T>(
    query: &SelectStatement,
    pool: &PgPool,
) -> AppResult<Option<T>>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
    let row = sqlx::query_as_with::<_, T, _>(&sql, values)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}
```

---

### Step 4: Define Filtering & Pagination DTO Standards
Establish shared parameter extractors with strict validation to prevent SQL injection and unbounded queries:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_page() -> u64 { 1 }
fn default_limit() -> u64 { 20 }

impl PaginationParams {
    pub fn offset(&self) -> u64 {
        (self.page.max(1) - 1) * self.clamped_limit()
    }

    pub fn clamped_limit(&self) -> u64 {
        self.limit.clamp(1, 100)
    }
}
```

---

## 5. Summary Table

| Step | Action Item | Priority |
| :--- | :--- | :--- |
| **Trigger** | Prioritize dynamic search/filter or dynamic batch insert feature | Required first |
| **Dependencies** | Add `sea-query` and `sea-query-binder` with `with-uuid` and `with-chrono` | Upon trigger |
| **Schema** | Create `src/schema.rs` with `#[derive(Iden)]` enums | Upon trigger |
| **Helper** | Write `fetch_all` / `fetch_optional` bridge for SQLx | Upon trigger |
| **DTOs** | Standardize validated pagination and whitelisted sort enums | Upon trigger |
| **Boundary** | Retain raw SQL for `FOR UPDATE`, lateral joins, and aggregate filters | Ongoing invariant |
