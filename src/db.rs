use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions}, Pool, Sqlite, ConnectOptions};
use chrono::{DateTime, Utc, Duration};
use crate::data::{KANA_DATA, VOCAB_DATA};
use std::str::FromStr;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuizCard {
    // Unique ID logic:
    // For Kana: "kana:<char>"
    // For Vocab: "vocab:<id>"
    pub id: String,
    pub card_type: String, // "kana" or "vocab"
    pub question: String, // Display text (e.g., "あ" or "猫 (ねこ)\nCat")
    pub answer: String,   // Romaji to type
    pub interval: i64,
    pub easiness: f64,
    pub repetitions: i64,
    #[allow(dead_code)]
    pub next_review_date: DateTime<Utc>,
    // Extra fields to help with feedback/logic, optional but useful
    pub extra_info: Option<String>,
}

#[derive(Clone)]
pub struct Db {
    pool: Pool<Sqlite>,
}

impl Db {
    pub async fn new() -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite://kana.db?mode=rwc")?
            .log_statements(log::LevelFilter::Trace);
        let pool = SqlitePool::connect_with(options).await?;

        let db = Db { pool };
        db.migrate().await?;
        db.seed_database_if_empty().await?;
        db.seed_vocabulary_if_empty().await?;

        Ok(db)
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS progress (
                kana_char TEXT PRIMARY KEY,
                romaji TEXT NOT NULL,
                interval INTEGER DEFAULT 0,
                easiness REAL DEFAULT 2.5,
                repetitions INTEGER DEFAULT 0,
                next_review_date DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS vocabulary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word_kanji TEXT,
                word_kana TEXT NOT NULL,
                meaning TEXT NOT NULL,
                romaji TEXT NOT NULL,
                interval INTEGER DEFAULT 0,
                easiness REAL DEFAULT 2.5,
                repetitions INTEGER DEFAULT 0,
                next_review_date DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_database_if_empty(&self) -> anyhow::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM progress")
            .fetch_one(&self.pool)
            .await?;

        if count == 0 {
            for k in KANA_DATA {
                sqlx::query("INSERT OR IGNORE INTO progress (kana_char, romaji) VALUES (?, ?)")
                    .bind(k.kana)
                    .bind(k.romaji)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn seed_vocabulary_if_empty(&self) -> anyhow::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM vocabulary")
            .fetch_one(&self.pool)
            .await?;

        if count == 0 {
            for v in VOCAB_DATA {
                // Ensure unique inserts. Kanji is good check if exists, or kana.
                sqlx::query(
                    "INSERT INTO vocabulary (word_kanji, word_kana, meaning, romaji) VALUES (?, ?, ?, ?)"
                )
                .bind(v.kanji)
                .bind(v.kana)
                .bind(v.meaning)
                .bind(v.romaji)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_due_cards(&self) -> anyhow::Result<Vec<QuizCard>> {
        // We use UNION ALL to combine Kana and Vocab items into a unified list
        // Note: For ID, we construct a string.
        // For Kana question, we just use the char.
        // For Vocab question, we format it as "Kanji (Kana)\nMeaning".
        let query = r#"
            SELECT
                'kana:' || kana_char as id,
                'kana' as card_type,
                kana_char as question,
                romaji as answer,
                interval,
                easiness,
                repetitions,
                next_review_date,
                NULL as extra_info
            FROM progress
            WHERE next_review_date <= CURRENT_TIMESTAMP

            UNION ALL

            SELECT
                'vocab:' || id as id,
                'vocab' as card_type,
                word_kanji || ' (' || word_kana || ')' || char(10) || meaning as question,
                romaji as answer,
                interval,
                easiness,
                repetitions,
                next_review_date,
                word_kana as extra_info
            FROM vocabulary
            WHERE next_review_date <= CURRENT_TIMESTAMP
        "#;

        let cards = sqlx::query_as::<_, QuizCard>(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(cards)
    }

    pub async fn update_card(&self, card: &QuizCard, correct: bool) -> anyhow::Result<i64> {
        // Simplified SuperMemo-2 Logic (Common for both)
        let quality = if correct { 5 } else { 0 };

        let mut next_easiness = card.easiness + (0.1 - (5.0 - quality as f64) * (0.08 + (5.0 - quality as f64) * 0.02));
        if next_easiness < 1.3 {
            next_easiness = 1.3;
        }

        let next_reps;
        let next_interval;

        if quality >= 3 {
             next_reps = card.repetitions + 1;
             if next_reps == 1 {
                 next_interval = 1;
             } else if next_reps == 2 {
                 next_interval = 6;
             } else {
                 next_interval = (card.interval as f64 * next_easiness).ceil() as i64;
             }
        } else {
             next_reps = 0;
             next_interval = 1;
        }

        let next_date = Utc::now() + Duration::days(next_interval);

        // Determine which table to update based on card_type
        if card.card_type == "kana" {
            // ID format "kana:char", strip "kana:"
            let kana_char = card.id.strip_prefix("kana:").unwrap_or(&card.question); // Fallback if parse fails, though shouldn't
            sqlx::query(
                "UPDATE progress SET interval = ?, easiness = ?, repetitions = ?, next_review_date = ? WHERE kana_char = ?"
            )
            .bind(next_interval)
            .bind(next_easiness)
            .bind(next_reps)
            .bind(next_date)
            .bind(kana_char)
            .execute(&self.pool)
            .await?;
        } else if card.card_type == "vocab" {
             // ID format "vocab:integer"
             if let Some(id_str) = card.id.strip_prefix("vocab:") {
                 if let Ok(id_int) = id_str.parse::<i64>() {
                     sqlx::query(
                        "UPDATE vocabulary SET interval = ?, easiness = ?, repetitions = ?, next_review_date = ? WHERE id = ?"
                     )
                     .bind(next_interval)
                     .bind(next_easiness)
                     .bind(next_reps)
                     .bind(next_date)
                     .bind(id_int)
                     .execute(&self.pool)
                     .await?;
                 }
             }
        }

        Ok(next_interval)
    }

    pub async fn get_count_due(&self) -> anyhow::Result<i64> {
         let count_kana: i64 = sqlx::query_scalar("SELECT count(*) FROM progress WHERE next_review_date <= CURRENT_TIMESTAMP")
            .fetch_one(&self.pool)
            .await?;
         let count_vocab: i64 = sqlx::query_scalar("SELECT count(*) FROM vocabulary WHERE next_review_date <= CURRENT_TIMESTAMP")
            .fetch_one(&self.pool)
            .await?;
         Ok(count_kana + count_vocab)
    }
}
