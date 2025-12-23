use sqlx::{SqlitePool, FromRow};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use crate::data::get_all_kana; // 引入数据源

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Card {
    pub id: String,
    pub kana_char: String,
    pub romaji: String,
    pub stability: f64,
    pub difficulty: f64,
    pub last_review: Option<String>,
}

pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn new() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:japanese_test.db".to_string());

        // 1. 配置连接选项：如果文件不存在，自动创建
        let options = SqliteConnectOptions::from_str(&database_url)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await?;

        // 2. 初始化表结构和数据
        Self::initialize_db(&pool).await?;

        Ok(Self { pool })
    }

    // 初始化数据库：建表 + 灌入数据
    async fn initialize_db(pool: &SqlitePool) -> Result<()> {
        // 创建表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cards (
                id TEXT PRIMARY KEY,
                kana_char TEXT NOT NULL,
                romaji TEXT NOT NULL,
                stability REAL DEFAULT 0.0,
                difficulty REAL DEFAULT 0.0,
                last_review TEXT
            );
            "#
        )
            .execute(pool)
            .await?;

        // 检查是否为空，如果为空则插入数据
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
            .fetch_one(pool)
            .await?;

        if count.0 == 0 {
            println!("Initializing database with seed data...");
            let all_kana = get_all_kana();
            let mut tx = pool.begin().await?;

            for (kana, romaji) in all_kana {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO cards (id, kana_char, romaji, stability, difficulty) VALUES (?, ?, ?, ?, ?)")
                    .bind(id)
                    .bind(kana)
                    .bind(romaji)
                    .bind(0.0) // 初始 stability
                    .bind(0.0) // 初始 difficulty
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            println!("Database initialized with {} cards.", count.0);
        }

        Ok(())
    }

    pub async fn get_count_due(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    pub async fn get_total_count(&self) -> Result<usize> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cards")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0 as usize)
    }

    pub async fn get_all_ids(&self) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM cards")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn get_batch_by_ids(&self, ids: &[String]) -> Result<Vec<Card>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let query = format!("SELECT * FROM cards WHERE id IN ({})", placeholders.join(","));

        let mut query_builder = sqlx::query_as::<_, Card>(&query);
        for id in ids {
            query_builder = query_builder.bind(id);
        }

        let cards = query_builder.fetch_all(&self.pool).await?;

        // 按输入 ID 的顺序排序
        let mut ordered_cards = Vec::new();
        for id in ids {
            if let Some(card) = cards.iter().find(|c| c.id == *id) {
                ordered_cards.push(card.clone());
            }
        }

        Ok(ordered_cards)
    }

    #[allow(unused)]
    pub async fn get_specific_batch(&self, ids: &[String]) -> Result<Vec<Card>> {
        self.get_batch_by_ids(ids).await
    }
    #[allow(unused)]
    pub async fn get_next_batch(&self, _seen_ids: &[String]) -> Result<Vec<Card>> {
        Ok(Vec::new())
    }

    pub async fn update_card(&self, id: &str, correct: bool) -> Result<i64> {
        let mut tx = self.pool.begin().await?;

        let card_res: Option<Card> = sqlx::query_as("SELECT * FROM cards WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(card) = card_res {
            let new_stability = if correct { card.stability * 1.5 } else { card.stability * 0.8 };
            let new_difficulty = if correct { card.difficulty - 0.1 } else { card.difficulty + 0.2 };

            sqlx::query("UPDATE cards SET stability = ?, difficulty = ?, last_review = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(new_stability)
                .bind(new_difficulty)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(0)
    }
}