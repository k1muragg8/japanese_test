use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous}, Pool, Sqlite, ConnectOptions, Row};
use chrono::{Utc, DateTime};
use crate::data::get_all_kana;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String, // kana_char
    pub kana_char: String,
    pub romaji: String,
    // FSRS Fields
    pub stability: f64,
    pub difficulty: f64,
    pub last_review: Option<DateTime<Utc>>,
    // State Machine Fields
    pub state: i64, // 0: New, 1: Learning, 2: Review, 3: Relearning
    pub step: i64,
    // Calculated interval (seconds) for display/logic, mostly virtual or stored for short-term scheduling
    pub interval: i64,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Card {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        let kana_char: String = row.try_get("kana_char")?;
        let romaji: String = row.try_get("romaji")?;

        let stability: f64 = row.try_get("stability").unwrap_or(86400.0);
        let difficulty: f64 = row.try_get("difficulty").unwrap_or(5.0);
        let last_review: Option<DateTime<Utc>> = row.try_get("last_review").ok();

        let state: i64 = row.try_get("state").unwrap_or(0);
        let step: i64 = row.try_get("step").unwrap_or(0);
        let interval: i64 = row.try_get("interval").unwrap_or(0);

        Ok(Card {
            id: kana_char.clone(),
            kana_char,
            romaji,
            stability,
            difficulty,
            last_review,
            state,
            step,
            interval,
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
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
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
                next_review_date DATETIME DEFAULT CURRENT_TIMESTAMP,
                stability REAL DEFAULT 86400.0,
                difficulty REAL DEFAULT 5.0,
                last_review DATETIME,
                state INTEGER DEFAULT 0,
                step INTEGER DEFAULT 0
            );
            "#
        )
        .execute(&self.pool)
        .await?;

        // FSRS & State Machine Migration: Add columns if they don't exist
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN stability REAL DEFAULT 86400.0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN difficulty REAL DEFAULT 5.0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN last_review DATETIME").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN state INTEGER DEFAULT 0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN step INTEGER DEFAULT 0").execute(&self.pool).await;

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
        // Fetch Priority:
        // 1. Learning/Relearning (State 1, 3) where DUE.
        //    DUE means: (now - last_review) >= interval (in seconds).
        // 2. Review (State 2) where DUE.
        //    DUE means: (now - last_review) >= interval (in seconds, derived from stability).
        // 3. New (State 0).

        let limit = 20;
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT * FROM progress
            WHERE
                -- Priority 1 & 2: Due Cards (Learning/Relearning/Review)
                (
                    state IN (1, 2, 3)
                    AND
                    (strftime('%s', 'now') - strftime('%s', last_review)) >= interval
                )
                OR
                -- Priority 3: New Cards
                (state = 0)
            ORDER BY
                -- Order Logic:
                -- 1. Due Learning/Relearning (States 1, 3) first
                CASE WHEN state IN (1, 3) THEN 0 ELSE 1 END ASC,

                -- 2. Due Review (State 2) next
                CASE WHEN state = 2 THEN 0 ELSE 1 END ASC,

                -- 3. New (State 0) last
                CASE WHEN state = 0 THEN 0 ELSE 1 END ASC,

                -- Tie-breaker for due cards: most overdue first
                (strftime('%s', 'now') - strftime('%s', last_review)) DESC
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    pub async fn update_card(&self, id: &str, correct: bool) -> anyhow::Result<i64> {
        let mut tx = self.pool.begin().await?;

        // Read current state
        let card = sqlx::query_as::<_, Card>("SELECT * FROM progress WHERE kana_char = ?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;

        let now = Utc::now();
        let mut s = card.stability; // Seconds
        let mut d = card.difficulty;
        let mut state = card.state;
        let mut step = card.step;
        let mut interval = 0; // Will be set logic below

        if state == 0 || state == 1 {
            // --- New (0) & Learning (1) ---
            if correct {
                if step == 0 {
                    interval = 60; // 1 min
                    state = 1;
                    step = 1;
                } else if step == 1 {
                    interval = 600; // 10 min
                    state = 1;
                    step = 2;
                } else {
                    // Graduate
                    state = 2; // Review
                    step = 0;
                    s = 86400.0; // 1 day stability
                    interval = 86400; // 1 day
                }
            } else {
                // Wrong: Reset
                state = 1;
                step = 0;
                interval = 60; // 1 min
            }
        } else if state == 2 {
            // --- Review (2) ---
            if correct {
                // FSRS Logic
                // Update Stability: S_new = S * (1 + factor * difficulty_weight)
                let growth_multiplier = 1.0 + (d * 0.2);
                s = s * growth_multiplier;

                // Update Difficulty
                d = d - 0.2;

                interval = s as i64;
            } else {
                // Wrong (Lapse) -> Downgrade to Relearning
                state = 3; // Relearning
                step = 0;
                interval = 600; // 10 min

                // Slash Stability
                s = s * 0.5;

                // Update Difficulty (Harder)
                d = d + 0.5;
            }
        } else if state == 3 {
            // --- Relearning (3) ---
            if correct {
                // Re-Graduate
                state = 2; // Back to Review
                interval = s as i64; // Restore stability as interval
            } else {
                // Wrong: Reset Relearning step
                interval = 600; // 10 min
            }
        }

        // Clamp D
        if d < 1.0 { d = 1.0; }
        if d > 10.0 { d = 10.0; }

        // Ensure min interval 60s
        if interval < 60 { interval = 60; }

        sqlx::query(
            "UPDATE progress SET stability = ?, difficulty = ?, last_review = ?, state = ?, step = ?, interval = ? WHERE kana_char = ?"
        )
        .bind(s)
        .bind(d)
        .bind(now)
        .bind(state)
        .bind(step)
        .bind(interval)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(interval)
    }

    pub async fn get_count_due(&self) -> anyhow::Result<i64> {
         let count: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM progress
            WHERE
                (state IN (1, 2, 3) AND (strftime('%s', 'now') - strftime('%s', last_review)) >= interval)
                OR (state = 0)
            "#
         )
            .fetch_one(&self.pool)
            .await?;
         Ok(count)
    }
}
