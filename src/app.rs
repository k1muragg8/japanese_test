use std::time::Instant;
use crate::db::{Db, Card};
use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Clone, Copy, PartialEq)]
pub enum AppState {
    Dashboard,
    Quiz,
}

#[allow(unused)]
pub struct App {
    pub db: Arc<Db>,
    pub state: AppState,
    pub due_cards: Vec<Card>,
    pub current_card_index: usize,
    pub user_input: String,
    pub current_feedback: Option<String>,
    pub feedback_detail: String,
    pub due_count: i64,
    pub session_start: Instant,

    // Cycle Fields
    pub deck_queue: Vec<String>,
    pub cycle_mistakes: std::collections::HashSet<String>,
    pub batch_counter: usize,
    pub total_cards_count: usize,
    pub estimated_total_batches: usize,
    pub is_review_phase: bool,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Db::new().await?);
        let due_count = db.get_count_due().await?;
        let total_cards_count = db.get_total_count().await?;

        let batch_size = 20.0;
        let estimated_total_batches = if total_cards_count > 0 {
            (total_cards_count as f64 / batch_size).ceil() as usize
        } else {
            1
        };

        Ok(Self {
            db,
            state: AppState::Dashboard,
            due_cards: Vec::new(),
            current_card_index: 0,
            user_input: String::new(),
            current_feedback: None,
            feedback_detail: String::new(),
            due_count,
            session_start: Instant::now(),
            deck_queue: Vec::new(),
            cycle_mistakes: std::collections::HashSet::new(),
            batch_counter: 0,
            total_cards_count,
            estimated_total_batches,
            is_review_phase: false,
        })
    }

    pub async fn start_quiz(&mut self) {
        self.cycle_mistakes.clear();
        self.batch_counter = 1;
        self.is_review_phase = false;

        if let Ok(mut all_ids) = self.db.get_all_ids().await {
            {
                let mut rng = thread_rng();
                all_ids.shuffle(&mut rng);
            }

            self.deck_queue = all_ids;
            self.load_next_queue_batch().await;
        }
    }

    async fn load_next_queue_batch(&mut self) {
        // === 配置区域 ===
        const COMBO_SIZE: usize = 3; // 3连击
        const BATCH_SIZE: usize = 20;
        // ================

        let ids_needed = BATCH_SIZE * COMBO_SIZE;
        let drain_count = std::cmp::min(ids_needed, self.deck_queue.len());

        let batch_ids: Vec<String> = self.deck_queue.drain(0..drain_count).collect();

        if !batch_ids.is_empty() {
            if let Ok(raw_cards) = self.db.get_batch_by_ids(&batch_ids).await {

                let mut combo_cards = Vec::new();

                for chunk in raw_cards.chunks(COMBO_SIZE) {
                    if chunk.is_empty() { continue; }

                    let mut merged_kana = String::new();
                    let mut merged_romaji = String::new();

                    let primary_id = chunk[0].id.clone();
                    let primary_stability = chunk[0].stability;
                    let primary_difficulty = chunk[0].difficulty;

                    for card in chunk {
                        merged_kana.push_str(&card.kana_char);
                        // 【关键修复】加上 .trim() 去除数据库可能存在的隐形空格
                        // 否则 "ka " + "no" 会变成 "ka no"，导致你输入 "kano" 报错
                        merged_romaji.push_str(card.romaji.trim());
                    }

                    let virtual_card = Card {
                        id: primary_id,
                        kana_char: merged_kana,
                        romaji: merged_romaji,
                        stability: primary_stability,
                        difficulty: primary_difficulty,
                        last_review: None,
                    };

                    combo_cards.push(virtual_card);
                }

                self.due_cards = combo_cards;
                self.current_card_index = 0;
                self.state = AppState::Quiz;
            }
        }
    }

    #[allow(unused)]
    async fn load_review_batch(&mut self) {
        // 占位符，现在不使用复习模式
        self.due_cards = Vec::new();
        self.current_card_index = 0;
    }

    #[allow(unused)]
    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        if self.current_card_index >= self.due_cards.len() {
            // 简单的无限循环模式：有牌发牌，没牌洗牌
            if !self.deck_queue.is_empty() {
                self.batch_counter += 1;
                self.load_next_queue_batch().await;
            } else {
                self.start_quiz().await;
            }
        }
    }
}