use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions}, Pool, Sqlite, ConnectOptions, Row};
use chrono::{DateTime, Utc, Duration};
use crate::data::{KANA_DATA, VOCAB_DATA};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Card {
    pub id: String, // kana_char for Kana, stringified int ID for Vocab
    pub kana_char: String, // Is used as 'primary text' in quiz (kana for Kana, kanji/kana for Vocab)
    pub romaji: String,
    pub interval: i64,
    pub easiness: f64,
    pub repetitions: i64,
    pub next_review_date: DateTime<Utc>,

    // Extra fields for Vocabulary
    pub is_vocab: bool,
    pub sub_text: Option<String>, // word_kana for Vocab
    pub meaning: Option<String>,
}

// Manual implementation because we are fetching from two tables with different schemas
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Card {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        // We expect the query to return consistent columns for both tables
        let id: String = row.try_get("id")?;
        let kana_char: String = row.try_get("kana_char")?;
        let romaji: String = row.try_get("romaji")?;
        let interval: i64 = row.try_get("interval")?;
        let easiness: f64 = row.try_get("easiness")?;
        let repetitions: i64 = row.try_get("repetitions")?;
        let next_review_date: DateTime<Utc> = row.try_get("next_review_date")?;
        let is_vocab: bool = row.try_get("is_vocab")?;
        let sub_text: Option<String> = row.try_get("sub_text")?;
        let meaning: Option<String> = row.try_get("meaning")?;

        Ok(Card {
            id,
            kana_char,
            romaji,
            interval,
            easiness,
            repetitions,
            next_review_date,
            is_vocab,
            sub_text,
            meaning,
        })
    }
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
            "#
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
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
                sqlx::query(
                    r#"
                    INSERT INTO vocabulary (word_kanji, word_kana, meaning, romaji)
                    VALUES (?, ?, ?, ?)
                    "#
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

    pub async fn get_due_cards(&self, mode: &str) -> anyhow::Result<Vec<Card>> {
        let mut cards = Vec::new();

        // Mode: "kana", "vocab", "mixed"
        let fetch_kana = mode == "kana" || mode == "mixed";
        let fetch_vocab = mode == "vocab" || mode == "mixed";

        if fetch_kana {
            let kana_cards = sqlx::query_as::<_, Card>(
                r#"
                SELECT
                    kana_char as id,
                    kana_char,
                    romaji,
                    interval,
                    easiness,
                    repetitions,
                    next_review_date,
                    0 as is_vocab,
                    NULL as sub_text,
                    NULL as meaning
                FROM progress
                WHERE next_review_date <= CURRENT_TIMESTAMP
                "#
            )
            .fetch_all(&self.pool)
            .await?;
            cards.extend(kana_cards);
        }

        if fetch_vocab {
            let vocab_cards = sqlx::query_as::<_, Card>(
                r#"
                SELECT
                    CAST(id AS TEXT) as id,
                    COALESCE(word_kanji, word_kana) as kana_char,
                    romaji,
                    interval,
                    easiness,
                    repetitions,
                    next_review_date,
                    1 as is_vocab,
                    word_kana as sub_text,
                    meaning
                FROM vocabulary
                WHERE next_review_date <= CURRENT_TIMESTAMP
                "#
            )
            .fetch_all(&self.pool)
            .await?;
            cards.extend(vocab_cards);
        }

        Ok(cards)
    }

    pub async fn update_card(&self, id: &str, is_vocab: bool, correct: bool) -> anyhow::Result<i64> {
        let card = if is_vocab {
            let id_int: i64 = id.parse().unwrap_or(0);
            sqlx::query_as::<_, Card>(
                r#"
                SELECT
                    CAST(id AS TEXT) as id,
                    COALESCE(word_kanji, word_kana) as kana_char,
                    romaji,
                    interval,
                    easiness,
                    repetitions,
                    next_review_date,
                    1 as is_vocab,
                    word_kana as sub_text,
                    meaning
                FROM vocabulary WHERE id = ?
                "#
            )
            .bind(id_int)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Card>(
                r#"
                SELECT
                    kana_char as id,
                    kana_char,
                    romaji,
                    interval,
                    easiness,
                    repetitions,
                    next_review_date,
                    0 as is_vocab,
                    NULL as sub_text,
                    NULL as meaning
                FROM progress WHERE kana_char = ?
                "#
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?
        };

        // Simplified SuperMemo-2
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

        if is_vocab {
            let id_int: i64 = id.parse().unwrap_or(0);
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
        } else {
            sqlx::query(
                "UPDATE progress SET interval = ?, easiness = ?, repetitions = ?, next_review_date = ? WHERE kana_char = ?"
            )
            .bind(next_interval)
            .bind(next_easiness)
            .bind(next_reps)
            .bind(next_date)
            .bind(id)
            .execute(&self.pool)
            .await?;
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
