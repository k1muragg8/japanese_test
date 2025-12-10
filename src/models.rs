use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KanaCategory {
    Hiragana,
    Katakana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KanaType {
    Seion,
    Dakuon,
    Handakuon,
    Yoon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kana {
    pub character: String, // Changed to String to own the data
    pub romaji: Vec<String>,
    pub category: KanaCategory,
    pub kana_type: KanaType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProgress {
    pub kana_char: String,
    pub easiness_factor: f64,
    pub interval: u64, // Days
    pub repetitions: u32,
    pub next_review_date: DateTime<Utc>,
}

impl Default for UserProgress {
    fn default() -> Self {
        Self {
            kana_char: String::new(),
            easiness_factor: 2.5,
            interval: 0,
            repetitions: 0,
            next_review_date: Utc::now(),
        }
    }
}

impl UserProgress {
    pub fn new(char: &str) -> Self {
        Self {
            kana_char: char.to_string(),
            ..Default::default()
        }
    }
}
