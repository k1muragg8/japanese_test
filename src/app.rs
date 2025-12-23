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
            // 【关键修复】使用独立代码块，确保 rng 在 await 前被销毁
            {
                let mut rng = thread_rng();
                all_ids.shuffle(&mut rng);
            }
            // 出了这个花括号，rng 已经死透了，下面的 await 就是安全的

            self.deck_queue = all_ids;
            self.load_next_queue_batch().await;
        }
    }

    async fn load_next_queue_batch(&mut self) {
        let batch_size = 20;
        let drain_count = std::cmp::min(batch_size, self.deck_queue.len());

        let batch_ids: Vec<String> = self.deck_queue.drain(0..drain_count).collect();

        if !batch_ids.is_empty() {
            if let Ok(cards) = self.db.get_batch_by_ids(&batch_ids).await {
                self.due_cards = cards;
                self.current_card_index = 0;
                self.state = AppState::Quiz;
            }
        }
    }

    async fn load_review_batch(&mut self) {
        let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();
        if let Ok(cards) = self.db.get_batch_by_ids(&mistakes).await {
            self.due_cards = cards;
            self.current_card_index = 0;
        }
    }

    #[allow(unused)]
    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        if self.current_card_index >= self.due_cards.len() {

            // 1. 新卡阶段
            if !self.is_review_phase {
                if !self.deck_queue.is_empty() {
                    self.batch_counter += 1;
                    self.load_next_queue_batch().await;
                    return;
                }

                if self.cycle_mistakes.is_empty() {
                    self.start_quiz().await;
                } else {
                    self.is_review_phase = true;
                    self.batch_counter += 1;
                    self.load_review_batch().await;
                }
                return;
            }

            // 2. 复习阶段
            if self.is_review_phase {
                if self.cycle_mistakes.is_empty() {
                    self.start_quiz().await;
                } else {
                    self.batch_counter += 1;
                    self.load_review_batch().await;
                }
                return;
            }
        }
    }
}