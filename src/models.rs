use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Kana {
    pub character: String,
    pub romaji: Vec<String>,
    pub category: KanaCategory,
    pub kana_type: KanaType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProgress {
    pub kana_char: String,
    pub easiness_factor: f64,
    pub interval: u64, // days
    pub repetitions: u32,
    pub next_review_date: DateTime<Utc>,
}

impl UserProgress {
    pub fn new(kana_char: String) -> Self {
        Self {
            kana_char,
            easiness_factor: 2.5,
            interval: 0,
            repetitions: 0,
            next_review_date: Utc::now(),
        }
    }
}
