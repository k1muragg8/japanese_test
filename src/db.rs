use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions}, Pool, Sqlite, ConnectOptions};
use chrono::{DateTime, Utc, Duration};
use crate::data::KANA_DATA;
use std::str::FromStr;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Card {
    pub kana_char: String,
    pub romaji: String,
    pub interval: i64,
    pub easiness: f64,
    pub repetitions: i64,
    #[allow(dead_code)]
    pub next_review_date: DateTime<Utc>,
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
        Ok(())
    }

    async fn seed_database_if_empty(&self) -> anyhow::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM progress")
            .fetch_one(&self.pool)
            .await?;

        if count == 0 {
            for k in KANA_DATA {
                // Ensure we don't duplicate keys if partial seed exists (though count==0 checks that)
                // Using INSERT OR IGNORE just in case.
                sqlx::query("INSERT OR IGNORE INTO progress (kana_char, romaji) VALUES (?, ?)")
                    .bind(k.kana)
                    .bind(k.romaji)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_due_cards(&self) -> anyhow::Result<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            "SELECT * FROM progress WHERE next_review_date <= CURRENT_TIMESTAMP"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(cards)
    }

    pub async fn update_card(&self, kana: &str, correct: bool) -> anyhow::Result<i64> {
        let card = sqlx::query_as::<_, Card>("SELECT * FROM progress WHERE kana_char = ?")
            .bind(kana)
            .fetch_one(&self.pool)
            .await?;

        // Simplified SuperMemo-2
        // Quality: 5 if correct, 0 if incorrect.
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

        sqlx::query(
            "UPDATE progress SET interval = ?, easiness = ?, repetitions = ?, next_review_date = ? WHERE kana_char = ?"
        )
        .bind(next_interval)
        .bind(next_easiness)
        .bind(next_reps)
        .bind(next_date)
        .bind(kana)
        .execute(&self.pool)
        .await?;

        Ok(next_interval)
    }

    pub async fn get_count_due(&self) -> anyhow::Result<i64> {
         let count: i64 = sqlx::query_scalar("SELECT count(*) FROM progress WHERE next_review_date <= CURRENT_TIMESTAMP")
            .fetch_one(&self.pool)
            .await?;
         Ok(count)
    }
}
