use std::collections::{HashMap, HashSet};

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

/// GregMat GRE vocabulary data (backend/data/gregmat_words.json), sourced from
/// the community mirror https://github.com/Xatta-Trone/gregmatgrewordsdb.
const WORDS_JSON: &str = include_str!("../../data/gregmat_words.json");

/// Subject assigned to every GRE question.
const SUBJECT: &str = "GregMat GRE Vocabulary";

// ---------------------------------------------------------------------------
// Input data types (serde_json parsed, tolerant of missing fields)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Word {
    group: i64,
    word: String,
    /// Lower-cased synonym set (used for overlap filtering).
    syn_set: HashSet<String>,
    /// Display-ready synonyms (duplicates removed, casing tidied).
    synonyms: Vec<String>,
    /// "(part of speech) definition; definition" strings.
    meanings: Vec<String>,
    /// Part-of-speech bucket of the primary meaning ("noun"/"verb"/"adj"/"adv"/"other").
    pos: Option<&'static str>,
    /// Example sentences.
    examples: Vec<String>,
}

/// Minimal deterministic RNG so re-seeding produces identical quizzes.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn shuffle<T>(&mut self, items: &mut Vec<T>) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Tidy a synonym for display: trim, dedupe, and normalize a leading capital
/// letter ("Nebulous" -> "nebulous") while leaving mixed/phrase forms alone.
fn tidy_synonym(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    let rest: String = chars.collect();
    let tidy = if first.is_uppercase() && rest.chars().all(|c| c.is_lowercase() || c == ' ') {
        first.to_lowercase().to_string() + &rest
    } else {
        s.to_string()
    };
    Some(tidy)
}

/// Build "(pos) def1; def2" meaning strings from the meanings blob.
/// Each entry is {"pos": <part of speech>, "defs": [<definitions>]}.
fn meaning_text(defs: &[serde_json::Value]) -> Vec<String> {
    let mut out = Vec::new();
    for entry in defs {
        let pos = entry["pos"].as_str().unwrap_or("").trim();
        let list: Vec<String> = entry["defs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|d| d.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        if list.is_empty() {
            continue;
        }
        let body = list.join("; ");
        out.push(if pos.is_empty() {
            body
        } else {
            let abbr = match pos {
                "noun" => "n.",
                "verb" | "transitive verb" | "intransitive verb" => "v.",
                "adjective" => "adj.",
                "adverb" => "adv.",
                _ => pos,
            };
            format!("({abbr}) {body}")
        });
    }
    out
}

/// Collapse a part-of-speech label into a coarse bucket used to keep
/// distractors grammatically plausible.
fn pos_bucket(pos: &str) -> &'static str {
    match pos {
        "noun" => "noun",
        "adjective" => "adj",
        "adverb" => "adv",
        "verb" | "transitive verb" | "intransitive verb" => "verb",
        _ => "other",
    }
}

/// Loose POS comparison: unknown buckets always match (permissive fallback).
fn same_pos(a: Option<&'static str>, b: Option<&'static str>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    }
}

/// True if neither word lists the other as a synonym and their synonym sets
/// don't overlap — i.e. `dw` is safe to use as a distractor for `w`.
fn not_synonymous(w: &Word, dw: &Word) -> bool {
    let dw_word = dw.word.to_lowercase();
    let w_word = w.word.to_lowercase();
    !w.syn_set.contains(&dw_word)
        && !dw.syn_set.contains(&w_word)
        && w.syn_set.intersection(&dw.syn_set).next().is_none()
}

/// Blank out the first whole-word occurrence of `word` in `sentence`.
/// Returns None when the word does not appear as a standalone word.
fn make_cloze(sentence: &str, word: &str) -> Option<String> {
    let s_lower = sentence.to_lowercase();
    let w_lower = word.to_lowercase();
    for (idx, _) in s_lower.match_indices(&w_lower) {
        let before_ok = s_lower[..idx]
            .chars()
            .next_back()
            .map_or(true, |c| !c.is_alphanumeric());
        let after_ok = s_lower[idx + w_lower.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric());
        if before_ok && after_ok {
            return Some(format!(
                "{}_____{}",
                &sentence[..idx],
                &sentence[idx + word.len()..]
            ));
        }
    }
    None
}

fn parse_words() -> Vec<Word> {
    let raw: Vec<serde_json::Value> = serde_json::from_str(WORDS_JSON).expect("parse gregmat data");

    let mut words = Vec::new();
    let mut seen = HashSet::new();
    for entry in raw {
        let word = entry["word"].as_str().unwrap_or("").trim().to_string();
        if word.is_empty() || !seen.insert(word.to_lowercase()) {
            continue;
        }

        let mut syn_set = HashSet::new();
        let mut synonyms = Vec::new();
        for s in entry["synonyms"].as_array().unwrap_or(&vec![]) {
            if let Some(text) = s.as_str().and_then(tidy_synonym) {
                if syn_set.insert(text.to_lowercase()) {
                    synonyms.push(text);
                }
            }
        }

        words.push(Word {
            group: entry["group"].as_i64().unwrap_or(0),
            word,
            syn_set,
            synonyms,
            meanings: meaning_text(entry["meanings"].as_array().unwrap_or(&vec![])),
            pos: entry["meanings"]
                .as_array()
                .and_then(|m| m.first())
                .and_then(|m| m["pos"].as_str())
                .map(pos_bucket),
            examples: entry["examples"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|e| e.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    words.sort_by(|a, b| a.group.cmp(&b.group).then(a.word.cmp(&b.word)));
    words
}

// ---------------------------------------------------------------------------
// Question generation
// ---------------------------------------------------------------------------

struct Question {
    text: String,
    /// (text, is_correct) in shuffled order.
    options: Vec<(String, bool)>,
}

fn build_questions(words: &[Word]) -> Vec<(i64, Vec<Question>)> {
    // Group words by GregMat group for group-scoped distractor pools.
    let mut by_group: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, w) in words.iter().enumerate() {
        by_group.entry(w.group).or_default().push(i);
    }

    let mut rng = Rng::new(42);
    let mut quizzes = Vec::new();

    for (group, idxs) in &by_group {
        let group = *group;
        let group_words: Vec<usize> = idxs.clone();
        let mut questions = Vec::new();

        for &i in idxs {
            questions.extend(questions_for_word(&words, i, &group_words, &mut rng));
        }

        if questions.is_empty() {
            continue;
        }
        quizzes.push((group, questions));
    }

    quizzes.sort_by_key(|(g, _)| *g);
    quizzes
}

/// Take up to `n` distractor words from the pre-shuffled pool, preferring
/// same-POS candidates, deduped, excluding the target word itself.
/// Returns None when the pool can't supply enough distinct options.
fn pick_word_options(
    w: &Word,
    pool: &[usize],
    words: &[Word],
    n: usize,
) -> Option<Vec<(String, bool)>> {
    let mut options: Vec<(String, bool)> = Vec::new();
    let mut seen = HashSet::new();
    seen.insert(w.word.to_lowercase());
    for pos_matched in [true, false] {
        for &j in pool {
            if options.len() == n {
                break;
            }
            if pos_matched && !same_pos(w.pos, words[j].pos) {
                continue;
            }
            if seen.insert(words[j].word.to_lowercase()) {
                options.push((words[j].word.clone(), false));
            }
        }
        if options.len() == n {
            break;
        }
    }
    (options.len() == n).then_some(options)
}

/// Generate every supported question type for one word.
fn questions_for_word(
    words: &[Word],
    i: usize,
    group_words: &[usize],
    rng: &mut Rng,
) -> Vec<Question> {
    let w = &words[i];
    let mut out = Vec::new();

    // Candidate word distractors: same group first, then the rest of the list,
    // all guaranteed non-synonymous with the target. Shuffled once; consumers
    // that need POS matching take a prefix subset of this order.
    let mut word_pool: Vec<usize> = group_words
        .iter()
        .copied()
        .filter(|&j| j != i && not_synonymous(w, &words[j]))
        .collect();
    if word_pool.len() < 8 {
        word_pool.extend(
            (0..words.len())
                .filter(|&j| j != i && !group_words.contains(&j) && not_synonymous(w, &words[j])),
        );
    }
    rng.shuffle(&mut word_pool);

    // 1. Meaning MCQ — distractor definitions now POS-matched to the target.
    if let Some(correct) = w.meanings.first().cloned() {
        let mut seen_defs = HashSet::new();
        seen_defs.insert(correct.to_lowercase());
        let mut pool: Vec<String> = group_words
            .iter()
            .copied()
            .filter(|&j| j != i && same_pos(w.pos, words[j].pos))
            .filter_map(|j| {
                let m = words[j].meanings.first().cloned()?;
                (!m.is_empty() && seen_defs.insert(m.to_lowercase())).then_some(m)
            })
            .collect();
        if pool.len() < 3 {
            pool.extend(
                (0..words.len())
                    .filter(|&j| j != i && !group_words.contains(&j) && same_pos(w.pos, words[j].pos))
                    .filter_map(|j| {
                        let m = words[j].meanings.first().cloned()?;
                        (!m.is_empty() && seen_defs.insert(m.to_lowercase())).then_some(m)
                    }),
            );
        }
        if pool.len() < 3 {
            pool.extend(
                word_pool
                    .iter()
                    .filter_map(|&j| words[j].meanings.first().cloned())
                    .filter(|m| !m.is_empty() && seen_defs.insert(m.to_lowercase())),
            );
        }
        rng.shuffle(&mut pool);
        let mut options: Vec<(String, bool)> =
            pool.into_iter().take(3).map(|t| (t, false)).collect();
        options.push((correct, true));
        rng.shuffle(&mut options);
        if options.len() == 4 {
            out.push(Question {
                text: format!("What does \u{201c}{}\u{201d} mean?", w.word),
                options,
            });
        }
    }

    // 2. Definition -> word (production recall: meaning on top, word below).
    if let Some(correct) = w.meanings.first().cloned() {
        if let Some(mut options) = pick_word_options(w, &word_pool, &words, 3) {
            options.push((w.word.clone(), true));
            rng.shuffle(&mut options);
            out.push(Question {
                text: format!("Which word means: \u{201c}{}\u{201d}?", correct),
                options,
            });
        }
    }

    // 3. Synonym MCQ (word on top, closest word below).
    if !w.synonyms.is_empty() {
        if let Some(mut options) = pick_word_options(w, &word_pool, &words, 3) {
            options.push((w.synonyms[0].clone(), true));
            rng.shuffle(&mut options);
            out.push(Question {
                text: format!("Which word is closest in meaning to \u{201c}{}\u{201d}?", w.word),
                options,
            });
        }
    }

    // 4. Cloze — blank the word inside its example sentence (context recall).
    if let Some(sentence) = w.examples.iter().find_map(|e| make_cloze(e, &w.word)) {
        if let Some(mut options) = pick_word_options(w, &word_pool, &words, 3) {
            options.push((w.word.clone(), true));
            rng.shuffle(&mut options);
            out.push(Question {
                text: format!("Fill in the blank: \u{201c}{}\u{201d}", sentence),
                options,
            });
        }
    }

    // 5. Odd one out — three synonyms plus one intruder (discrimination).
    if w.synonyms.len() >= 3 && !word_pool.is_empty() {
        let intruder = word_pool[0];
        let mut options: Vec<(String, bool)> = w
            .synonyms
            .iter()
            .take(3)
            .map(|s| (s.clone(), false))
            .collect();
        options.push((words[intruder].word.clone(), true));
        rng.shuffle(&mut options);
        out.push(Question {
            text: "Three of these words are synonyms. Which one does NOT belong?".to_string(),
            options,
        });
    }

    out
}

// ---------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------

/// Insert the subject if missing and return its id.
async fn ensure_subject(pool: &sqlx::PgPool, name: &str) -> Result<Uuid, Box<dyn std::error::Error>> {
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

    // --reset: wipe existing GRE quizzes (and attempts on them) so the current
    // question-generation logic can rebuild everything from scratch.
    let reset = std::env::args().any(|a| a == "--reset");
    if reset {
        println!("reset: deleting existing GRE vocabulary quizzes and their attempts...");
        sqlx::query(
            "DELETE FROM attempt_answers WHERE attempt_id IN (
                SELECT id FROM attempts WHERE quiz_id IN (
                    SELECT id FROM quizzes WHERE title LIKE 'GRE Vocabulary%'))",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "DELETE FROM attempts WHERE quiz_id IN (
                SELECT id FROM quizzes WHERE title LIKE 'GRE Vocabulary%')",
        )
        .execute(&pool)
        .await?;
        sqlx::query("DELETE FROM quizzes WHERE title LIKE 'GRE Vocabulary%'")
            .execute(&pool)
            .await?;
    }

    let words = parse_words();
    println!(
        "parsed {} words ({} with meanings, {} with synonyms)",
        words.len(),
        words.iter().filter(|w| !w.meanings.is_empty()).count(),
        words.iter().filter(|w| !w.synonyms.is_empty()).count()
    );

    let quizzes = build_questions(&words);
    println!(
        "generated {} group quizzes, {} questions",
        quizzes.len(),
        quizzes.iter().map(|(_, q)| q.len()).sum::<usize>()
    );

    // Skip groups already seeded (quiz titles are the idempotency marker).
    let existing: HashSet<String> = sqlx::query_scalar("SELECT title FROM quizzes")
        .fetch_all(&pool)
        .await?
        .into_iter()
        .collect();

    let subject_id = ensure_subject(&pool, SUBJECT).await?;
    let mut tx = pool.begin().await?;
    let mut created = 0usize;

    for (group, questions) in &quizzes {
        let first = words
            .iter()
            .find(|w| w.group == *group)
            .map(|w| w.word.clone())
            .unwrap_or_default();
        let title = format!("GRE Vocabulary \u{2014} GregMat Group {group}");
        let description = format!(
            "GregMat vocabulary group {group}: meaning, definition, synonym, cloze and \
             odd-one-out questions covering words from \u{201c}{first}\u{201d} onward."
        );

        if existing.contains(&title) {
            // Quiz already seeded: make sure its questions carry the subject.
            sqlx::query(
                "UPDATE questions SET subject_id = $1
                 WHERE quiz_id = (SELECT id FROM quizzes WHERE title = $2) AND subject_id IS NULL",
            )
            .bind(subject_id)
            .bind(&title)
            .execute(&mut *tx)
            .await?;
            println!("skip: {title:?} already exists");
            continue;
        }

        let quiz_id = Uuid::new_v4();
        sqlx::query("INSERT INTO quizzes (id, title, description) VALUES ($1, $2, $3)")
            .bind(&quiz_id)
            .bind(&title)
            .bind(&description)
            .execute(&mut *tx)
            .await?;

        for (q_pos, q) in questions.iter().enumerate() {
            let question_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO questions (id, quiz_id, text, position, subject_id) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&question_id)
            .bind(&quiz_id)
            .bind(&q.text)
            .bind(q_pos as i32)
            .bind(subject_id)
            .execute(&mut *tx)
            .await?;

            for (o_pos, (text, is_correct)) in q.options.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO options (id, question_id, text, is_correct, position) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(Uuid::new_v4())
                .bind(&question_id)
                .bind(text)
                .bind(is_correct)
                .bind(o_pos as i32)
                .execute(&mut *tx)
                .await?;
            }
        }

        created += 1;
    }

    tx.commit().await?;
    println!("done: {created} GRE quizzes inserted");
    Ok(())
}
