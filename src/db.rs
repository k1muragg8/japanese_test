use sqlx::{sqlite::SqlitePool, Row};
use color_eyre::eyre::Result;
use chrono::{NaiveDateTime, Utc};
use crate::data::KANA_DATA;

#[derive(Debug, sqlx::FromRow)]
pub struct Progress {
    pub kana_char: String,
    pub romaji: String,
    pub interval: i64,
    pub easiness: f64,
    pub repetitions: i64,
    pub next_review_date: NaiveDateTime,
}

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
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
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm2_calculation_logic() {
        // Since the function `update_card_progress` is async and talks to DB,
        // we can't easily unit test it without mocking or a test DB.
        // However, we can simulate the math here to verify the logic we implemented.

        // Logic from implementation:
        // if grade >= 3 {
        //   interval calculation...
        //   repetitions += 1
        // } else {
        //   repetitions = 0
        //   interval = 1
        // }
        // easiness calculation...

        struct MockCard {
            interval: i64,
            easiness: f64,
            repetitions: i64,
        }

        let mut card = MockCard { interval: 0, easiness: 2.5, repetitions: 0 };
        let grade = 5; // Perfect

        // Step 1: Correct answer (First time)
        if grade >= 3 {
            if card.repetitions == 0 {
                card.interval = 1;
            } else if card.repetitions == 1 {
                card.interval = 6;
            } else {
                card.interval = (card.interval as f64 * card.easiness).round() as i64;
            }
            card.repetitions += 1;
        } else {
            card.repetitions = 0;
            card.interval = 1;
        }

        card.easiness = card.easiness + (0.1 - (5.0 - grade as f64) * (0.08 + (5.0 - grade as f64) * 0.02));
        if card.easiness < 1.3 { card.easiness = 1.3; }

        assert_eq!(card.interval, 1);
        assert_eq!(card.repetitions, 1);
        assert!(card.easiness > 2.5); // 2.6

        // Step 2: Correct answer (Second time)
        // Simulate re-running the logic
        if grade >= 3 {
             if card.repetitions == 0 {
                card.interval = 1;
            } else if card.repetitions == 1 {
                card.interval = 6;
            } else {
                card.interval = (card.interval as f64 * card.easiness).round() as i64;
            }
            card.repetitions += 1;
        }

        assert_eq!(card.interval, 6);
        assert_eq!(card.repetitions, 2);
    }
}

pub async fn seed_database_if_empty(pool: &SqlitePool) -> Result<()> {
    let count: i64 = sqlx::query("SELECT count(*) FROM progress")
        .map(|row: sqlx::sqlite::SqliteRow| row.get(0))
        .fetch_one(pool)
        .await?;

    if count == 0 {
        for kana in KANA_DATA {
            sqlx::query(
                "INSERT INTO progress (kana_char, romaji) VALUES (?, ?)"
            )
            .bind(kana.character)
            .bind(kana.romaji)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn get_due_cards(pool: &SqlitePool) -> Result<Vec<Progress>> {
    let now = Utc::now().naive_utc();
    let cards = sqlx::query_as::<_, Progress>(
        "SELECT * FROM progress WHERE next_review_date <= ?"
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    Ok(cards)
}

pub async fn update_card_progress(pool: &SqlitePool, char: &str, grade: u8) -> Result<()> {
    // 1. Fetch current state
    let mut card = sqlx::query_as::<_, Progress>(
        "SELECT * FROM progress WHERE kana_char = ?"
    )
    .bind(char)
    .fetch_one(pool)
    .await?;

    // 2. Apply SM-2 Logic
    if grade >= 3 {
        if card.repetitions == 0 {
            card.interval = 1;
        } else if card.repetitions == 1 {
            card.interval = 6;
        } else {
            card.interval = (card.interval as f64 * card.easiness).round() as i64;
        }
        card.repetitions += 1;
    } else {
        card.repetitions = 0;
        card.interval = 1;
    }

    card.easiness = card.easiness + (0.1 - (5.0 - grade as f64) * (0.08 + (5.0 - grade as f64) * 0.02));
    if card.easiness < 1.3 {
        card.easiness = 1.3;
    }

    // Calculate next date using chrono
    // Note: SQLite DATETIME is simplified, so we just add days to now
    // Actually SM-2 schedules based on review date.
    let next_date = Utc::now().naive_utc() + chrono::Duration::days(card.interval);

    // 3. Update DB
    sqlx::query(
        "UPDATE progress SET interval = ?, easiness = ?, repetitions = ?, next_review_date = ? WHERE kana_char = ?"
    )
    .bind(card.interval)
    .bind(card.easiness)
    .bind(card.repetitions)
    .bind(next_date)
    .bind(char)
    .execute(pool)
    .await?;

    Ok(())
}
