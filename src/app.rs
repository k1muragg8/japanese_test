use std::time::Instant;
use crate::db::{Db, Card};
use crate::feedback::FeedbackGenerator;
use std::sync::Arc;

#[derive(Clone, Copy)]
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
    pub recent_batch_ids: Vec<String>, // 200-Card Buffer
    pub batch_counter: usize,
    pub cycle_mistakes: std::collections::HashSet<String>,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Db::new().await?);
        let due_count = db.get_count_due().await?;

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
            recent_batch_ids: Vec::new(),
            batch_counter: 0,
            cycle_mistakes: std::collections::HashSet::new(),
        })
    }

    pub async fn start_quiz(&mut self) {
        if let Ok(cards) = self.db.get_next_batch(&self.recent_batch_ids).await {
            self.due_cards = cards;

            // Add new batch IDs to buffer
            let current_ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
            self.recent_batch_ids.extend(current_ids);

            // Truncate to keep only last 200 IDs (approx. 10 batches)
            if self.recent_batch_ids.len() > 200 {
                let remove_count = self.recent_batch_ids.len() - 200;
                self.recent_batch_ids.drain(0..remove_count);
            }

            self.current_card_index = 0;
            self.user_input.clear();
            self.current_feedback = None;
            self.feedback_detail.clear();

            if !self.due_cards.is_empty() {
                self.state = AppState::Quiz;
            }
        }
    }

    #[allow(unused)]
    pub async fn submit_answer(&mut self) {
        if self.current_card_index >= self.due_cards.len() {
            return;
        }

        if self.current_feedback.is_some() {
            return;
        }

        let card = &self.due_cards[self.current_card_index];
        let correct = self.user_input.trim().eq_ignore_ascii_case(&card.romaji);

        let interval_res = self.db.update_card(&card.id, correct).await;

        if correct {
            self.current_feedback = Some("Correct!".to_string());
            if let Ok(seconds) = interval_res {
                // Convert seconds to human readable
                let days = seconds as f64 / 86400.0;
                if days < 1.0 {
                     // Less than a day
                     let hours = seconds / 3600;
                     if hours < 1 {
                         let mins = seconds / 60;
                         self.feedback_detail = format!("回答正确！\n下次复习: {}分钟后", mins);
                     } else {
                         self.feedback_detail = format!("回答正确！\n下次复习: {}小时后", hours);
                     }
                } else {
                     self.feedback_detail = format!("回答正确！\n下次复习: {:.1}天后", days);
                }
            } else {
                self.feedback_detail = "回答正确！".to_string();
            }
        } else {
            self.current_feedback = Some(format!("Wrong! Correct was: {}", card.romaji));

            // Track mistakes for cycle review
            self.cycle_mistakes.insert(card.id.clone());

            // Generate Feedback using local logic
            let front_text = card.kana_char.clone();

            self.feedback_detail = FeedbackGenerator::generate_explanation(
                &front_text,
                &card.romaji,
                &self.user_input
            );
        }
    }

    #[allow(unused)]
    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        if self.current_card_index >= self.due_cards.len() {
            // Batch Finished
            self.batch_counter += 1;

            let mut new_cards: Option<Vec<Card>> = None;
            let mut is_review = false;

            if self.batch_counter >= 10 {
                // Trigger Review
                let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();

                // Reset
                self.batch_counter = 0;
                self.cycle_mistakes.clear();

                if !mistakes.is_empty() {
                     if let Ok(cards) = self.db.get_specific_batch(&mistakes).await {
                         new_cards = Some(cards);
                         is_review = true;
                     }
                }
            }

            // If not review batch (or review batch failed/was empty/no mistakes), fetch normal
            if new_cards.is_none() {
                 if let Ok(cards) = self.db.get_next_batch(&self.recent_batch_ids).await {
                     new_cards = Some(cards);
                     is_review = false;
                 }
            }

            if let Some(cards) = new_cards {
                if !cards.is_empty() {
                    self.due_cards = cards;

                    if !is_review {
                        // Add new batch IDs to buffer ONLY for normal batches
                        let current_ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
                        self.recent_batch_ids.extend(current_ids);

                        // Truncate to keep only last 200 IDs
                        if self.recent_batch_ids.len() > 200 {
                            let remove_count = self.recent_batch_ids.len() - 200;
                            self.recent_batch_ids.drain(0..remove_count);
                        }
                    }

                    self.current_card_index = 0;
                } else {
                     // Truly empty or error
                     self.state = AppState::Dashboard;
                     if let Ok(c) = self.db.get_count_due().await {
                        self.due_count = c;
                     }
                }
            } else {
                 self.state = AppState::Dashboard;
            }
        }
    }

    #[allow(unused)]
    pub fn handle_input_char(&mut self, c: char) {
        self.user_input.push(c);
    }

    #[allow(unused)]
    pub fn handle_backspace(&mut self) {
        self.user_input.pop();
    }
}
