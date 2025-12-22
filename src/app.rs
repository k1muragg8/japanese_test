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

            // Track mistakes
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

        // Loop to handle immediate transitions (e.g. Deck Exhausted -> Force Review)
        while self.current_card_index >= self.due_cards.len() {
            // Batch Finished
            self.batch_counter += 1;

            if self.batch_counter <= 10 {
                // Normal Cycle Batch
                if let Ok(cards) = self.db.get_next_batch(&self.cycle_seen_ids).await {
                    if !cards.is_empty() {
                        self.due_cards = cards;
                        // CRITICAL FIX: Track new IDs
                        let ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
                        self.cycle_seen_ids.extend(ids);
                        // Batch found, exit loop
                        break;
                    } else {
                        // Deck exhausted early? Trigger review early if mistakes exist, or reset
                        if !self.cycle_mistakes.is_empty() {
                            self.batch_counter = 10; // Will increment to 11 in next loop iteration
                            continue;
                        } else {
                            // Full reset
                            self.start_quiz().await;
                            return;
                        }
                    }
                }
            } else if self.batch_counter == 11 {
                // Review Round
                let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();
                if !mistakes.is_empty() {
                    if let Ok(cards) = self.db.get_specific_batch(&mistakes).await {
                        self.due_cards = cards;
                        break;
                    }
                } else {
                    // No mistakes? Reset and start new cycle immediately
                    self.start_quiz().await;
                    return;
                }
            } else {
                // Cycle Complete (Post-Review) -> Reset
                self.start_quiz().await;
                return;
            }
        }

        // Ensure index is reset if we loaded new cards (which happens if loop breaks)
        // If we returned early (start_quiz called), it resets index there.
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
