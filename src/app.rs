use crate::db::{Db, Card};
use crate::feedback::FeedbackGenerator;

pub enum AppState {
    Dashboard,
    Quiz,
}

pub struct App {
    pub db: Db,
    pub state: AppState,
    pub due_cards: Vec<Card>,
    pub current_card_index: usize,
    pub user_input: String,
    pub current_feedback: Option<String>,
    pub feedback_detail: String, // Detail text for the right pane
    pub due_count: i64,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Db::new().await?;
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
        })
    }

    pub async fn start_quiz(&mut self) {
        if let Ok(cards) = self.db.get_due_cards().await {
            self.due_cards = cards;
            self.current_card_index = 0;
            self.user_input.clear();
            self.current_feedback = None;
            self.feedback_detail.clear();

            if !self.due_cards.is_empty() {
                self.state = AppState::Quiz;
            }
        }
    }

    pub async fn submit_answer(&mut self) {
        if self.current_card_index >= self.due_cards.len() {
            return;
        }

        // If already showing result, ignore submit (user should press Space to continue)
        if self.current_feedback.is_some() {
            return;
        }

        let card = &self.due_cards[self.current_card_index];
        let correct = self.user_input.trim().eq_ignore_ascii_case(&card.romaji);

        // Update DB
        let interval_res = self.db.update_card(&card.kana_char, correct).await;

        if correct {
            self.current_feedback = Some("Correct!".to_string());
            // Show stats
            if let Ok(days) = interval_res {
                self.feedback_detail = format!("回答正确！\n下次复习: {}天后", days);
            } else {
                self.feedback_detail = "回答正确！".to_string();
            }
        } else {
            self.current_feedback = Some(format!("Incorrect. Correct: {}", card.romaji));

            // Generate Feedback using local logic
            self.feedback_detail = FeedbackGenerator::generate_explanation(
                &card.kana_char,
                &card.romaji,
                &self.user_input
            );
        }
    }

    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        // If finished
        if self.current_card_index >= self.due_cards.len() {
            self.state = AppState::Dashboard;
            // Update count
            if let Ok(c) = self.db.get_count_due().await {
                self.due_count = c;
            }
        }
    }

    pub fn handle_input_char(&mut self, c: char) {
        self.user_input.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.user_input.pop();
    }
}
