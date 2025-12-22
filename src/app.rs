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
        self.fetch_batch_logic().await;
    }

    async fn fetch_batch_logic(&mut self) {
        let mut new_cards: Option<Vec<Card>> = None;

        // Using a loop to handle state transitions (Random -> Review -> Random) if needed
        loop {
            // Logic: Batches 1-10 (Indices 0-9) are Random
            if self.batch_counter < 10 {
                if let Ok(cards) = self.db.get_next_batch(&self.cycle_seen_ids).await {
                    if !cards.is_empty() {
                        new_cards = Some(cards.clone());
                        // Add to seen
                        let current_ids: Vec<String> = cards.iter().map(|c| c.id.clone()).collect();
                        self.cycle_seen_ids.extend(current_ids);
                        break;
                    } else {
                        // Deck exhausted early
                        if !self.cycle_mistakes.is_empty() {
                             self.batch_counter = 10; // Jump to review
                             continue; // Loop again to handle batch_counter == 10
                        } else {
                             // Full Reset
                             self.cycle_seen_ids.clear();
                             self.batch_counter = 0;
                             // Loop again to fetch random with fresh deck
                             continue;
                        }
                    }
                }
            }

            // Logic: Batch 11 (Index 10) is Review
            if self.batch_counter == 10 {
                 let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();
                 if !mistakes.is_empty() {
                     if let Ok(cards) = self.db.get_specific_batch(&mistakes).await {
                         new_cards = Some(cards);
                     }
                 } else {
                     // No mistakes? Bonus random batch or immediate reset?
                     // Prompt: "If no mistakes, give a random bonus batch."
                     // Fetch simple random batch
                     if let Ok(cards) = self.db.get_next_batch(&[]).await {
                         new_cards = Some(cards);
                     }
                 }
                 break;
            }

            // Safety break
            break;
        }

        if let Some(cards) = new_cards {
            self.due_cards = cards;
            self.current_card_index = 0;
            self.user_input.clear();
            self.current_feedback = None;
            self.feedback_detail.clear();
            if !self.due_cards.is_empty() {
                self.state = AppState::Quiz;
            }
        } else {
             self.state = AppState::Dashboard;
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

        if self.current_card_index >= self.due_cards.len() {
            // Batch Finished

            // Increment Counter
            self.batch_counter += 1;

            if self.batch_counter > 10 {
                // We just finished Batch 11 (Index 10)
                // Reset Cycle
                self.batch_counter = 0;
                self.cycle_seen_ids.clear();
                self.cycle_mistakes.clear();
            }

            self.fetch_batch_logic().await;
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
