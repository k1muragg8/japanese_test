use crate::db::{Db, QuizCard};
use crate::feedback::FeedbackGenerator;

#[derive(PartialEq, Clone, Copy)]
pub enum AppState {
    Dashboard,
    Quiz,
    FakeLog, // Boss Key Mode
}

pub struct App {
    pub db: Db,
    pub state: AppState,
    pub previous_state: AppState, // To return from FakeLog (optional, or just go back to Quiz/Dashboard)
    pub due_cards: Vec<QuizCard>,
    pub current_card_index: usize,
    pub user_input: String,
    pub current_feedback: Option<String>,
    pub feedback_detail: String,
    pub due_count: i64,
    pub fake_logs: Vec<String>,
    pub fake_log_scroll_state: u16,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Db::new().await?;
        let due_count = db.get_count_due().await?;

        let fake_logs = vec![
            "[INFO] Compiling crate 'syn' v1.0.109...".to_string(),
            "[WARN] Deprecated usage in module 'core'...".to_string(),
            "[INFO] Finished release [optimized] target(s) in 12.4s".to_string(),
            "[INFO] Downloading crates ...".to_string(),
            "[INFO] Compiling tokio v1.0.0".to_string(),
            "[ERROR] Connection reset by peer (os error 104)".to_string(),
            "[INFO] Waiting for file lock on package cache".to_string(),
            "[INFO] Running `target/debug/kana-tutor`".to_string(),
            "[DEBUG] Loaded configuration from .env".to_string(),
            "[INFO] Optimization level: 3".to_string(),
        ];

        Ok(Self {
            db,
            state: AppState::Dashboard,
            previous_state: AppState::Dashboard,
            due_cards: Vec::new(),
            current_card_index: 0,
            user_input: String::new(),
            current_feedback: None,
            feedback_detail: String::new(),
            due_count,
            fake_logs,
            fake_log_scroll_state: 0,
        })
    }

    pub fn toggle_boss_mode(&mut self) {
        if self.state == AppState::FakeLog {
            self.state = if self.previous_state == AppState::FakeLog { AppState::Dashboard } else { match self.previous_state {
                AppState::FakeLog => AppState::Dashboard, // prevent loop
                other => other,
            } };
        } else {
            self.previous_state = match self.state {
                 AppState::FakeLog => AppState::Dashboard,
                 other => other,
            };
            self.state = AppState::FakeLog;
        }
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

        if self.current_feedback.is_some() {
            return;
        }

        let card = &self.due_cards[self.current_card_index];
        let correct = self.user_input.trim().eq_ignore_ascii_case(&card.answer);

        // Update DB
        let interval_res = self.db.update_card(card, correct).await;

        if correct {
            self.current_feedback = Some("Correct!".to_string());
            if let Ok(days) = interval_res {
                self.feedback_detail = format!("回答正确！\n下次复习: {}天后", days);
            } else {
                self.feedback_detail = "回答正确！".to_string();
            }
        } else {
            self.current_feedback = Some(format!("Incorrect. Correct: {}", card.answer));

            let question_display = card.question.replace("\n", " ");

            self.feedback_detail = FeedbackGenerator::generate_explanation(
                &question_display, // Use question as "correct kana" for display purpose, imperfect but works for feedback text
                &card.answer,
                &self.user_input
            );
        }
    }

    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        if self.current_card_index >= self.due_cards.len() {
            self.state = AppState::Dashboard;
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
