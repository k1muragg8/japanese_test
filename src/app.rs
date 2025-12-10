use std::time::Instant;
use crate::db::{Db, Card};
use crate::feedback::FeedbackGenerator;

#[derive(Clone, Copy)]
pub enum AppState {
    Dashboard,
    Quiz,
    FakeLog,
}

pub enum QuizMode {
    Kana,
    Vocab,
    Mixed,
}

pub struct App {
    pub db: Db,
    pub state: AppState,
    pub previous_state: Option<AppState>, // To restore state after FakeLog
    pub due_cards: Vec<Card>,
    pub current_card_index: usize,
    pub user_input: String,
    pub current_feedback: Option<String>,
    pub feedback_detail: String,
    pub due_count: i64,
    pub quiz_mode: QuizMode,
    pub fake_logs: Vec<String>,
    pub fake_log_index: usize,
    pub session_start: Instant,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Db::new().await?;
        let due_count = db.get_count_due().await?;

        // Initialize with some fake logs so it doesn't look empty immediately
        let initial_logs = vec![
            "[INFO] Compiling libc v0.2.147".to_string(),
            "[INFO] Compiling proc-macro2 v1.0.63".to_string(),
            "[INFO] Compiling quote v1.0.28".to_string(),
            "[INFO] Compiling unicode-ident v1.0.9".to_string(),
            "[INFO] Compiling syn v2.0.22".to_string(),
        ];

        Ok(Self {
            db,
            state: AppState::Dashboard,
            previous_state: None,
            due_cards: Vec::new(),
            current_card_index: 0,
            user_input: String::new(),
            current_feedback: None,
            feedback_detail: String::new(),
            due_count,
            quiz_mode: QuizMode::Mixed, // Default to Mixed or make selectable
            fake_logs: initial_logs,
            fake_log_index: 0,
            session_start: Instant::now(),
        })
    }

    pub async fn start_quiz(&mut self) {
        let mode_str = match self.quiz_mode {
            QuizMode::Kana => "kana",
            QuizMode::Vocab => "vocab",
            QuizMode::Mixed => "mixed",
        };

        if let Ok(cards) = self.db.get_next_batch(mode_str).await {
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
        let correct = self.user_input.trim().eq_ignore_ascii_case(&card.romaji);

        let interval_res = self.db.update_card(&card.id, card.is_vocab, correct).await;

        if correct {
            self.current_feedback = Some("Correct!".to_string());
            if let Ok(days) = interval_res {
                self.feedback_detail = format!("回答正确！\n下次复习: {}天后", days);
            } else {
                self.feedback_detail = "回答正确！".to_string();
            }
        } else {
            self.current_feedback = Some(format!("Incorrect. Correct: {}", card.romaji));

            // Generate Feedback using local logic
            let front_text = if let Some(meaning) = &card.meaning {
                format!("{} ({})", card.kana_char, meaning)
            } else {
                card.kana_char.clone()
            };

            self.feedback_detail = FeedbackGenerator::generate_explanation(
                &front_text,
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

        if self.current_card_index >= self.due_cards.len() {
            // Fetch next batch (Infinite Mode)
            let mode_str = match self.quiz_mode {
                QuizMode::Kana => "kana",
                QuizMode::Vocab => "vocab",
                QuizMode::Mixed => "mixed",
            };

            if let Ok(cards) = self.db.get_next_batch(mode_str).await {
                if !cards.is_empty() {
                    self.due_cards = cards;
                    self.current_card_index = 0;
                } else {
                    // Truly empty (shouldn't happen with infinite logic unless DB empty)
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

    pub fn handle_input_char(&mut self, c: char) {
        self.user_input.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.user_input.pop();
    }

    pub fn toggle_fake_log(&mut self) {
        match self.state {
            AppState::FakeLog => {
                // Restore previous state or default to Dashboard
                if let Some(prev) = self.previous_state {
                    self.state = prev;
                } else {
                    self.state = AppState::Dashboard;
                }
                self.previous_state = None;
            }
            _ => {
                // Save current state and switch to FakeLog
                self.previous_state = Some(self.state);
                self.state = AppState::FakeLog;
            }
        }
    }

    pub fn tick_fake_log(&mut self) {
        if let AppState::FakeLog = self.state {
            // Generate a fake log line occasionally
            self.fake_log_index += 1;
            if self.fake_log_index % 5 == 0 { // Adjust speed
                 let crates = ["tokio", "syn", "quote", "serde", "rand", "http", "hyper", "sqlx", "ratatui", "crossterm"];
                 let statuses = ["Compiling", "Checking", "Downloaded"];
                 let versions = ["v1.0.0", "v0.4.2", "v2.1.0", "v0.12.33"];

                 use rand::Rng;
                 let mut rng = rand::thread_rng();
                 let c = crates[rng.gen_range(0..crates.len())];
                 let s = statuses[rng.gen_range(0..statuses.len())];
                 let v = versions[rng.gen_range(0..versions.len())];

                 let line = if s == "Downloaded" {
                     format!("[INFO] {} {} {}", s, c, v)
                 } else {
                     format!("[INFO] {} {} {}", s, c, v)
                 };

                 self.fake_logs.push(line);
                 if self.fake_logs.len() > 100 {
                     self.fake_logs.remove(0);
                 }
            }
        }
    }

    pub fn toggle_mode(&mut self) {
        self.quiz_mode = match self.quiz_mode {
            QuizMode::Kana => QuizMode::Vocab,
            QuizMode::Vocab => QuizMode::Mixed,
            QuizMode::Mixed => QuizMode::Kana,
        };
    }
}
