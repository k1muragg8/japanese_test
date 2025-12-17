use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions}, Pool, Sqlite, ConnectOptions, Row};
use chrono::{Utc, Duration};
use crate::data::get_all_kana;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String, // kana_char
    pub kana_char: String,
    pub romaji: String,
    pub interval: i64,
    pub easiness: f64,    // Acts as 'ease_factor'
    pub repetitions: i64, // Acts as 'streak'
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Card {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        let kana_char: String = row.try_get("kana_char")?;
        let romaji: String = row.try_get("romaji")?;
        let interval: i64 = row.try_get("interval")?;
        let easiness: f64 = row.try_get("easiness")?;
        let repetitions: i64 = row.try_get("repetitions")?;

        Ok(Card {
            id: kana_char.clone(),
            kana_char,
            romaji,
            interval,
            easiness,
            repetitions
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

        Ok(db)
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        // Schema uses 'easiness' for ease_factor and 'repetitions' for streak
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
            let all_kana = get_all_kana();
            for (kana, romaji) in all_kana {
                sqlx::query("INSERT OR IGNORE INTO progress (kana_char, romaji) VALUES (?, ?)")
                    .bind(kana)
                    .bind(romaji)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_next_batch(&self) -> anyhow::Result<Vec<Card>> {
        let mut cards = Vec::new();
        let limit = 20;

        let select_clause = r#"
            SELECT
                kana_char,
                romaji,
                interval,
                easiness,
                repetitions,
                next_review_date
            FROM progress
        "#;

        // Priority 1: Due Reviews (repetitions > 0 AND due)
        let p1_query = format!("{} WHERE next_review_date <= CURRENT_TIMESTAMP AND repetitions > 0 LIMIT ?", select_clause);
        let p1_cards = sqlx::query_as::<_, Card>(&p1_query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        cards.extend(p1_cards);

        if cards.len() < limit as usize {
            let needed = limit - cards.len() as i64;
            // Priority 2: New Cards (repetitions = 0)
            let p2_query = format!("{} WHERE repetitions = 0 ORDER BY RANDOM() LIMIT ?", select_clause);
            let p2_cards = sqlx::query_as::<_, Card>(&p2_query)
                .bind(needed)
                .fetch_all(&self.pool)
                .await?;
            cards.extend(p2_cards);
        }

        if cards.len() < limit as usize {
            let needed = limit - cards.len() as i64;
            // Priority 3: Review Ahead (Future)
            let p3_query = format!("{} WHERE next_review_date > CURRENT_TIMESTAMP AND repetitions > 0 ORDER BY RANDOM() LIMIT ?", select_clause);
            let p3_cards = sqlx::query_as::<_, Card>(&p3_query)
                .bind(needed)
                .fetch_all(&self.pool)
                .await?;
            cards.extend(p3_cards);
        }

        Ok(cards)
    }

    pub async fn update_card(&self, id: &str, correct: bool) -> anyhow::Result<i64> {
        let card = sqlx::query_as::<_, Card>(
            r#"
            SELECT
                kana_char,
                romaji,
                interval,
                easiness,
                repetitions,
                next_review_date
            FROM progress WHERE kana_char = ?
            "#
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let mut next_interval: f64;
        let mut next_easiness = card.easiness;
        let mut next_reps = card.repetitions;

        if correct {
            // Correct: Increase Interval
            if card.interval == 0 {
                next_interval = 1.0;
            } else {
                next_interval = (card.interval as f64) * card.easiness;
            }

            // Min interval constraint
            if next_interval < 1.0 {
                next_interval = 1.0;
            }

            // Update Ease Factor: +0.1, Max 3.0
            next_easiness += 0.1;
            if next_easiness > 3.0 {
                next_easiness = 3.0;
            }

            // Update Streak
            next_reps += 1;

            // Fuzzing: +/- 5%
            let mut rng = rand::thread_rng();
            let fuzz_factor: f64 = rng.gen_range(0.95..1.05);
            next_interval *= fuzz_factor;

        } else {
            // Wrong: Reset
            next_interval = 1.0; // Reset to 1 day

            // Punish Ease Factor: -0.2, Min 1.3
            next_easiness -= 0.2;
            if next_easiness < 1.3 {
                next_easiness = 1.3;
            }

            // Reset Streak
            next_reps = 0;
        }

        let next_interval_int = next_interval.round() as i64;
        // Ensure strictly positive interval even after fuzzing rounding
        let final_interval = if next_interval_int < 1 { 1 } else { next_interval_int };

        let next_date = Utc::now() + Duration::days(final_interval);

        sqlx::query(
            "UPDATE progress SET interval = ?, easiness = ?, repetitions = ?, next_review_date = ? WHERE kana_char = ?"
        )
        .bind(final_interval)
        .bind(next_easiness)
        .bind(next_reps)
        .bind(next_date)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(final_interval)
    }

    pub async fn get_count_due(&self) -> anyhow::Result<i64> {
         let count: i64 = sqlx::query_scalar("SELECT count(*) FROM progress WHERE next_review_date <= CURRENT_TIMESTAMP")
            .fetch_one(&self.pool)
            .await?;
         Ok(count)
    }
}
