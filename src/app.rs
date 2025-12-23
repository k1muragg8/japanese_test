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
    // Cycle Fields
    pub cycle_seen_ids: Vec<String>,
    pub cycle_mistakes: std::collections::HashSet<String>,
    pub batch_counter: usize,
    pub total_cards_count: usize,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Db::new().await?);
        let due_count = db.get_count_due().await?;
        let total_cards_count = db.get_total_count().await?;

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
            cycle_seen_ids: Vec::new(),
            cycle_mistakes: std::collections::HashSet::new(),
            batch_counter: 0,
            total_cards_count,
        })
    }

    pub async fn start_quiz(&mut self) {
        // Initialize Cycle
        self.cycle_seen_ids.clear();
        self.cycle_mistakes.clear();
        self.batch_counter = 1;

        if let Ok(cards) = self.db.get_next_batch(&self.cycle_seen_ids).await {
            self.due_cards = cards;

            // CRITICAL FIX: Track seen IDs immediately
            let ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
            self.cycle_seen_ids.extend(ids);

            self.current_card_index = 0;
            self.user_input.clear();
            self.current_feedback = None;
            self.feedback_detail.clear();

            if !self.due_cards.is_empty() {
                self.state = AppState::Quiz;
            } else {
                self.state = AppState::Dashboard;
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
            // In Review Mode (Batch 11), correct answer removes mistake
            if self.batch_counter == 11 {
                self.cycle_mistakes.remove(&card.id);
            }

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

            // Track mistakes (Always add/ensure in set)
            self.cycle_mistakes.insert(card.id.clone());

            // Generate Feedback
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

        // Check if current batch is finished
        while self.current_card_index >= self.due_cards.len() {
            // Determine next step based on current batch
            if self.batch_counter < 10 {
                // Batches 1-9 -> Go to next batch
                self.batch_counter += 1;

                // Try to fetch next batch
                if let Ok(cards) = self.db.get_next_batch(&self.cycle_seen_ids).await {
                    if !cards.is_empty() {
                        self.due_cards = cards;
                        // Track seen IDs
                        let ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
                        self.cycle_seen_ids.extend(ids);
                        break; // Loaded new batch
                    } else {
                        // Deck exhausted early -> Force Review Mode
                        self.batch_counter = 11;
                        // Fall through to Batch 11 logic
                    }
                } else {
                    // Error? Force Review Mode
                    self.batch_counter = 11;
                }
            } else if self.batch_counter == 10 {
                // Batch 10 finished -> Enter Review Mode
                self.batch_counter = 11;
            }

            // Handle Batch 11 (Review Mode) Logic
            if self.batch_counter == 11 {
                if self.cycle_mistakes.is_empty() {
                    // Clean sweep! Reset cycle.
                    self.start_quiz().await;
                    return;
                } else {
                    // Load mistakes for review
                    let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();
                    if let Ok(cards) = self.db.get_specific_batch(&mistakes).await {
                        self.due_cards = cards;
                        break; // Loaded review batch
                    } else {
                        // DB Error? Reset to avoid infinite loop
                        self.start_quiz().await;
                        return;
                    }
                }
            }
        }

        // Reset index for the new batch
        if self.current_card_index >= self.due_cards.len() {
             self.current_card_index = 0;
        }

        if self.due_cards.is_empty() {
             self.state = AppState::Dashboard;
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
