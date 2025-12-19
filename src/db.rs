use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous}, Pool, Sqlite, ConnectOptions, Row};
use chrono::{Utc, DateTime};
use crate::data::get_all_kana;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use rand::Rng;
use rand::seq::SliceRandom;

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
    // Leech Fields
    pub lapses: i64,
    pub suspended: bool,
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

        let lapses: i64 = row.try_get("lapses").unwrap_or(0);
        let suspended: bool = row.try_get("suspended").unwrap_or(false);

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
            lapses,
            suspended,
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
                step INTEGER DEFAULT 0,
                lapses INTEGER DEFAULT 0,
                suspended BOOLEAN DEFAULT 0
            );
            "#
        )
        .execute(&self.pool)
        .await?;

        // FSRS & State Machine & Leech Migration: Add columns if they don't exist
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN stability REAL DEFAULT 86400.0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN difficulty REAL DEFAULT 5.0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN last_review DATETIME").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN state INTEGER DEFAULT 0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN step INTEGER DEFAULT 0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN lapses INTEGER DEFAULT 0").execute(&self.pool).await;
        let _ = sqlx::query("ALTER TABLE progress ADD COLUMN suspended BOOLEAN DEFAULT 0").execute(&self.pool).await;

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
        let batch_size: usize = 20;
        let new_card_quota: usize = 5;

        // 1. Calculate Limits
        let limit_due = batch_size - new_card_quota; // 15

        // 2. Query A (Fetch Due Cards)
        // Condition: state IN (1, 2, 3) AND is_due
        let due_cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT * FROM progress
            WHERE
                state IN (1, 2, 3)
                AND (strftime('%s', 'now') - strftime('%s', last_review)) >= interval
            ORDER BY
                -- Priority: Learning/Relearning > Review
                CASE
                    WHEN state IN (1, 3) THEN 1
                    WHEN state = 2 THEN 2
                    ELSE 3
                END ASC,
                -- Overdue: Most overdue first
                (strftime('%s', 'now') - strftime('%s', last_review)) DESC
            LIMIT ?
            "#
        )
        .bind(limit_due as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut all_cards = due_cards;

        // 3. Query B (Fetch New Cards)
        // Smart Backfilling: If we fetched fewer due cards, increase new card limit.
        let slots_remaining = batch_size - all_cards.len();
        let limit_new = slots_remaining;

        if limit_new > 0 {
            let new_cards = sqlx::query_as::<_, Card>(
                r#"
                SELECT * FROM progress
                WHERE state = 0
                ORDER BY RANDOM()
                LIMIT ?
                "#
            )
            .bind(limit_new as i64)
            .fetch_all(&self.pool)
            .await?;

            all_cards.extend(new_cards);
        }

        // 4. Backfill (if still not full)
        // This happens if we ran out of New cards (Query B returned < limit_new).
        // We need to fetch more Due cards (if Query A was capped) or Review Ahead cards.
        if all_cards.len() < batch_size {
            let needed = batch_size - all_cards.len();

            // Query C: Fetch generic non-new cards, ordered by importance.
            // We fetch a bit more (needed + 30) and filter in Rust to avoid complex NOT IN SQL.
            let backfill_candidates = sqlx::query_as::<_, Card>(
                r#"
                SELECT * FROM progress
                WHERE
                    state IN (1, 2, 3)
                ORDER BY
                    -- Due cards first (is_due)
                    CASE
                        WHEN (strftime('%s', 'now') - strftime('%s', last_review)) >= interval THEN 0
                        ELSE 1
                    END ASC,
                    -- Priority
                    CASE
                        WHEN state IN (1, 3) THEN 1
                        ELSE 2
                    END ASC,
                    -- Overdue / Due Soonest
                    CASE
                        WHEN (strftime('%s', 'now') - strftime('%s', last_review)) >= interval THEN (strftime('%s', 'now') - strftime('%s', last_review))
                        ELSE 0
                    END DESC,
                    -- For non-due, earliest due date
                    (strftime('%s', last_review) + interval) ASC
                LIMIT ?
                "#
            )
            .bind((needed + 30) as i64)
            .fetch_all(&self.pool)
            .await?;

            let existing_ids: std::collections::HashSet<String> = all_cards.iter().map(|c| c.id.clone()).collect();

            for c in backfill_candidates {
                if all_cards.len() >= batch_size {
                    break;
                }
                if !existing_ids.contains(&c.id) {
                    all_cards.push(c);
                }
            }
        }

        // 5. Shuffle
        all_cards.shuffle(&mut rand::thread_rng());

        Ok(all_cards)
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
        let mut lapses = card.lapses;
        let suspended = card.suspended;

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
                let mut growth_multiplier = 1.0 + (d * 0.2);

                // --- Interval Factors (Early/Overdue) ---
                if let Some(last_rev) = card.last_review {
                    let actual_interval = (now - last_rev).num_seconds().max(1) as f64;
                    let scheduled_interval = card.interval.max(1) as f64;
                    let delay_factor = actual_interval / scheduled_interval;

                    if delay_factor > 1.0 {
                        // Bonus for Overdue: 1.0 + 0.5 * log(delay_factor)
                        growth_multiplier *= 1.0 + 0.5 * delay_factor.ln();
                    } else {
                        // Dampener for Early Review
                        // Linear interpolation: delay 0 => growth 1.0; delay 1 => full growth
                        growth_multiplier = 1.0 + (growth_multiplier - 1.0) * delay_factor;
                    }
                }

                s = s * growth_multiplier;

                // Update Difficulty
                d = d - 0.2;

                // --- Target Retention Tuning (85%) ---
                // New Interval = S * 1.6
                let base_interval = s * 1.6;

                // Fuzzing for State 2 (Long-term Review)
                let mut rng = rand::thread_rng();
                let fuzz_factor: f64 = rng.gen_range(0.95..1.05);
                interval = (base_interval * fuzz_factor) as i64;

            } else {
                // Wrong (Lapse) -> Downgrade to Relearning
                lapses += 1;

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
                // For re-graduation, we can also apply the 85% retention factor if we trust S is accurate now
                // Current S is the slashed stability.
                // Let's use S * 1.6 to be consistent with 85% retention target for Review state.
                interval = (s * 1.6) as i64;
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
            "UPDATE progress SET stability = ?, difficulty = ?, last_review = ?, state = ?, step = ?, interval = ?, lapses = ?, suspended = ? WHERE kana_char = ?"
        )
        .bind(s)
        .bind(d)
        .bind(now)
        .bind(state)
        .bind(step)
        .bind(interval)
        .bind(lapses)
        .bind(suspended)
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
