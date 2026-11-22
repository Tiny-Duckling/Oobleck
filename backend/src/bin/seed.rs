use sqlx::postgres::PgPool;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

// Each quiz: (title, subject, description, questions), where each question is
// (text, options), and options is a list of (text, is_correct).
type SeedQuiz = (
    &'static str,
    &'static str,
    &'static str,
    Vec<(&'static str, Vec<(&'static str, bool)>)>,
);

fn quizzes() -> Vec<SeedQuiz> {
    vec![
        (
            "General Knowledge Basics",
            "General Knowledge",
            "A quick warm-up quiz covering everyday general knowledge.",
            vec![
                (
                    "What is the capital of France?",
                    vec![
                        ("Paris", true),
                        ("London", false),
                        ("Berlin", false),
                        ("Madrid", false),
                    ],
                ),
                (
                    "Which planet is known as the Red Planet?",
                    vec![
                        ("Venus", false),
                        ("Mars", true),
                        ("Jupiter", false),
                        ("Saturn", false),
                    ],
                ),
                (
                    "How many continents are there on Earth?",
                    vec![("5", false), ("6", false), ("7", true), ("8", false)],
                ),
                (
                    "What is the largest ocean on Earth?",
                    vec![
                        ("Atlantic", false),
                        ("Indian", false),
                        ("Arctic", false),
                        ("Pacific", true),
                    ],
                ),
            ],
        ),
        (
            "Web Development Fundamentals",
            "Web Development",
            "Test your knowledge of HTML, CSS, and the web platform.",
            vec![
                (
                    "What does HTML stand for?",
                    vec![
                        ("HyperText Markup Language", true),
                        ("HighText Machine Language", false),
                        ("Hyperlink Text Management Language", false),
                        ("Home Tool Markup Language", false),
                    ],
                ),
                (
                    "Which HTTP status code means 'Not Found'?",
                    vec![
                        ("200", false),
                        ("301", false),
                        ("404", true),
                        ("500", false),
                    ],
                ),
                (
                    "In CSS, which property controls text color?",
                    vec![
                        ("font-style", false),
                        ("color", true),
                        ("text-color", false),
                        ("background", false),
                    ],
                ),
                (
                    "Which tag is used for the largest heading in HTML?",
                    vec![
                        ("<head>", false),
                        ("<h6>", false),
                        ("<h1>", true),
                        ("<title>", false),
                    ],
                ),
                (
                    "What does CORS stand for?",
                    vec![
                        ("Cross-Origin Resource Sharing", true),
                        ("Central Online Request System", false),
                        ("Client-Origin Response Standard", false),
                        ("Cached Object Retrieval Service", false),
                    ],
                ),
            ],
        ),
        (
            "Rust Programming",
            "Rust",
            "Questions about ownership, borrowing, and the Rust ecosystem.",
            vec![
                (
                    "Which keyword declares an immutable variable binding?",
                    vec![
                        ("var", false),
                        ("let", true),
                        ("const", false),
                        ("mut", false),
                    ],
                ),
                (
                    "What is the size of a Rust char type in bytes?",
                    vec![("1", false), ("2", false), ("4", true), ("8", false)],
                ),
                (
                    "Which tool is Rust's official package manager?",
                    vec![
                        ("npm", false),
                        ("pip", false),
                        ("cargo", true),
                        ("gradle", false),
                    ],
                ),
                (
                    "What does the Result<T, E> type represent?",
                    vec![
                        ("A lazy iterator", false),
                        ("Success or failure", true),
                        ("A thread handle", false),
                        ("A raw pointer", false),
                    ],
                ),
            ],
        ),
    ]
}

/// Insert the subject if missing and return its id.
async fn ensure_subject(pool: &PgPool, name: &str) -> Result<Uuid, Box<dyn std::error::Error>> {
    sqlx::query("INSERT INTO subjects (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
        .bind(name)
        .execute(pool)
        .await?;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM subjects WHERE name = $1")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(id)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL is not set (copy backend/.env.example to backend/.env)")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // Skip quizzes that already exist so this can run on every container start.
    let existing: std::collections::HashSet<String> =
        sqlx::query_scalar("SELECT title FROM quizzes")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .collect();

    let mut created = 0usize;
    for (title, subject, description, questions) in quizzes() {
        let subject_id = ensure_subject(&pool, subject).await?;
        if existing.contains(title) {
            // Quiz already seeded: just make sure its questions carry the subject.
            sqlx::query(
                r#"
                UPDATE questions SET subject_id = $1
                WHERE quiz_id = (SELECT id FROM quizzes WHERE title = $2)
                  AND (subject_id IS NULL OR subject_id <> $1)
                "#,
            )
            .bind(subject_id)
            .bind(title)
            .execute(&pool)
            .await?;
            println!("skip: {title:?} already exists");
            continue;
        }
        created += 1;
        let quiz_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO quizzes (id, title, description, total_questions) VALUES ($1, $2, $3, $4)",
        )
        .bind(quiz_id)
        .bind(title)
        .bind(description)
        .bind(questions.len() as i32)
        .execute(&pool)
        .await?;

        for (q_pos, (q_text, options)) in questions.iter().enumerate() {
            let correct_count = options.iter().filter(|(_, is_correct)| *is_correct).count();
            if correct_count != 1 {
                panic!(
                    "question {q_text:?} must have exactly 1 correct option, found {correct_count}"
                );
            }

            let question_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO questions (id, quiz_id, text, position, subject_id) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(question_id)
            .bind(quiz_id)
            .bind(q_text)
            .bind(q_pos as i32)
            .bind(subject_id)
            .execute(&pool)
            .await?;

            for (o_pos, (o_text, is_correct)) in options.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO options (id, question_id, text, is_correct, position) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(Uuid::new_v4())
                .bind(question_id)
                .bind(o_text)
                .bind(is_correct)
                .bind(o_pos as i32)
                .execute(&pool)
                .await?;
            }
        }

        println!("seeded quiz: {title}");
    }

    println!(
        "done. {} quizzes seeded ({} skipped).",
        created,
        quizzes().len() - created
    );
    Ok(())
}
