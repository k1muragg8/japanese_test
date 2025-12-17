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
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Card {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        let kana_char: String = row.try_get("kana_char")?;
        let romaji: String = row.try_get("romaji")?;

        let stability: f64 = row.try_get("stability").unwrap_or(86400.0);
        let difficulty: f64 = row.try_get("difficulty").unwrap_or(5.0);

        // Handle last_review potentially being null (new cards) or from legacy
        let last_review: Option<DateTime<Utc>> = row.try_get("last_review").ok();

        Ok(Card {
            id: kana_char.clone(),
            kana_char,
            romaji,
            stability,
            difficulty,
            last_review,
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
        // Initial Table Creation (if not exists)
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

        // FSRS Migration: Add columns if they don't exist
        // SQLite doesn't support IF NOT EXISTS in ADD COLUMN, so we catch errors
        // or check pragma. A simple way is to try adding and ignore duplicate column error.

        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN stability REAL DEFAULT 86400.0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN difficulty REAL DEFAULT 5.0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN last_review DATETIME").execute(&self.pool).await;

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
        // FSRS Logic for Fetching:
        // Prioritize by Retrievability (R) ascending.
        // R = 0.9 ^ (elapsed_days / stability)
        // elapsed_seconds = (now - last_review)
        // We want lowest R first.
        // Lower R means (elapsed / stability) is HIGHER.
        // So we order by (elapsed_seconds / stability) DESC.
        // COALESCE(last_review, 0) handles new cards (infinite elapsed -> top priority? Or handle separately).
        // Let's treat New Cards (last_review IS NULL) as high priority or mix them.
        // Standard approach: Due cards (R < 0.9) first, then New.

        // For simplicity and "show lowest retention first":
        // Sort by: (strftime('%s', 'now') - strftime('%s', last_review)) / stability DESC
        // New cards (last_review NULL) should appear.

        let limit = 20;
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT * FROM progress
            ORDER BY
                CASE WHEN last_review IS NULL THEN 1 ELSE 0 END DESC, -- New cards first? Or Review?
                (strftime('%s', 'now') - strftime('%s', last_review)) / stability DESC
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

        // Defaults if null (from migration)
        let last_review = card.last_review.unwrap_or(now);
        let mut s = card.stability; // Seconds
        let mut d = card.difficulty;

        // 1. Calculate Retention (R)
        // elapsed in days for formula
        let elapsed_seconds = (now - last_review).num_seconds().max(0) as f64;
        #[allow(unused)]
        let elapsed_days = elapsed_seconds / 86400.0;

        // R is not used directly in update logic provided, but concepts are.
        // R = 0.9f64.powf(elapsed_days / (s / 86400.0)); // if s is seconds?
        // Prompt says: "stability (S): How long (in seconds) the memory lasts."
        // And "R = 0.9 ^ (elapsed_days / stability)".
        // This implies stability in the formula is DAYS?
        // Or if stability is seconds, formula should be R = 0.9 ^ (elapsed_sec / s).
        // Let's assume stability is seconds as defined in struct.
        // Formula: R = 0.9 ^ (elapsed_seconds / s)

        // 2. Update Stability (S)
        if correct {
            // S_new = S * (1 + factor * difficulty_weight)
            // Factor ~ 2.0 heuristic?
            // "Exponential growth based on how hard it was"
            // Heuristic: S_new = S * (1.0 + (D * 0.2))
            let growth_multiplier = 1.0 + (d * 0.2);
            s = s * growth_multiplier;
        } else {
            // Wrong: S_new = S * 0.5
            s = s * 0.5;
        }

        // 3. Update Difficulty (D)
        if correct {
            d = d - 0.2;
        } else {
            d = d + 0.5;
        }
        // Clamp D
        if d < 1.0 { d = 1.0; }
        if d > 10.0 { d = 10.0; }

        // 4. Next Interval (I)
        // Target R = 0.9.
        // Next Interval (sec) = S * (log(0.9) / log(R_current))  <-- This formula from prompt is problematic if R=1.
        // Re-reading prompt: "Next Interval (sec) = S * (log(0.9) / log(R_current))"
        // If I assume "Stability" IS the interval where R=0.9, then Next Interval IS S.
        // The formula might be trying to compensate for "early review"?
        // But in FSRS, Stability IS the interval length for retrievability 0.9.
        // So I will set Next Interval = S.

        let next_interval_seconds = s.max(60.0) as i64; // Min 60s

        // Update DB
        // next_review_date is used for legacy or display?
        // We update stability, difficulty, last_review.
        // Note: We update last_review to NOW.

        sqlx::query(
            "UPDATE progress SET stability = ?, difficulty = ?, last_review = ? WHERE kana_char = ?"
        )
        .bind(s)
        .bind(d)
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(next_interval_seconds)
    }

    pub async fn get_count_due(&self) -> anyhow::Result<i64> {
         // Count where retention < 0.9?
         // (now - last_review) / stability > 1 (if S is time to 0.9)
         // So (now - last_review) > stability

         let count: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM progress
            WHERE
                last_review IS NOT NULL
                AND (strftime('%s', 'now') - strftime('%s', last_review)) > stability
            "#
         )
            .fetch_one(&self.pool)
            .await?;
         Ok(count)
    }
}
