use crossterm::event::KeyEvent;
use sqlx::SqlitePool;
use tokio::sync::mpsc::UnboundedSender;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::db::{get_due_cards, update_card_progress, Progress};

#[derive(PartialEq)]
pub enum CurrentScreen {
    Dashboard,
    Quiz,
}

#[derive(PartialEq, Clone)]
pub enum AiStatus {
    Idle,
    Thinking,
    Ready,
    Offline,
    Error(String),
}

pub enum Action {
    Tick,
    Quit,
    None,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub pool: SqlitePool,
    pub ai_sender: Option<UnboundedSender<(String, String, String)>>, // (correct_kana, correct_romaji, user_input)
    pub exit: bool,

    // Dashboard State
    pub due_count: usize,

    // Quiz State
    pub quiz_queue: Vec<Progress>,
    pub current_card: Option<Progress>,
    pub user_input: String,
    pub feedback: Option<bool>, // None = typing, Some(true/false)
    pub score: u32,
    pub ai_status: AiStatus,
    pub ai_explanation: String,
}

impl App {
    pub fn new(pool: SqlitePool, ai_sender: Option<UnboundedSender<(String, String, String)>>) -> App {
        let ai_status = if ai_sender.is_some() { AiStatus::Idle } else { AiStatus::Offline };
        App {
            current_screen: CurrentScreen::Dashboard,
            pool,
            ai_sender,
            exit: false,
            due_count: 0,
            quiz_queue: Vec::new(),
            current_card: None,
            user_input: String::new(),
            feedback: None,
            score: 0,
            ai_status,
            ai_explanation: String::new(),
        }
    }

    pub async fn refresh_dashboard(&mut self) {
        if let Ok(cards) = get_due_cards(&self.pool).await {
            self.due_count = cards.len();
        }
    }

    pub async fn start_quiz(&mut self) {
        if let Ok(mut cards) = get_due_cards(&self.pool).await {
            let mut rng = thread_rng();
            cards.shuffle(&mut rng);
            self.quiz_queue = cards;
            self.next_card();
            self.current_screen = CurrentScreen::Quiz;
        }
    }

    fn next_card(&mut self) {
        self.current_card = self.quiz_queue.pop();
        self.user_input.clear();
        self.feedback = None;
        if matches!(self.ai_status, AiStatus::Offline) {
            // Keep offline
        } else {
            self.ai_status = AiStatus::Idle;
        }
        self.ai_explanation.clear();

        if self.current_card.is_none() {
            // Quiz finished, return to dashboard
            self.current_screen = CurrentScreen::Dashboard;
        }
    }

    pub async fn submit_answer(&mut self) {
        if let Some(card) = &self.current_card {
            let correct = self.user_input.trim() == card.romaji;
            self.feedback = Some(correct);

            let grade = if correct { 5 } else { 0 }; // 5=perfect, 0=blackout
            if correct {
                self.score += 1;
            } else {
                // Trigger AI if incorrect
                if let Some(sender) = &self.ai_sender {
                    let _ = sender.send((
                        card.kana_char.clone(),
                        card.romaji.clone(),
                        self.user_input.clone(),
                    ));
                    self.ai_status = AiStatus::Thinking;
                }
            }

            // Update DB
            let _ = update_card_progress(&self.pool, &card.kana_char, grade).await;
        }
    }

    pub async fn handle_input(&mut self, key: KeyEvent) -> Action {
        match self.current_screen {
            CurrentScreen::Dashboard => match key.code {
                crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc => return Action::Quit,
                crossterm::event::KeyCode::Enter => {
                    self.start_quiz().await;
                }
                _ => {}
            },
            CurrentScreen::Quiz => match key.code {
                crossterm::event::KeyCode::Esc => {
                    self.current_screen = CurrentScreen::Dashboard;
                    self.refresh_dashboard().await;
                }
                crossterm::event::KeyCode::Enter => {
                    if self.feedback.is_none() {
                        self.submit_answer().await;
                    }
                }
                crossterm::event::KeyCode::Char(' ') => {
                    if self.feedback.is_some() {
                        self.next_card();
                        if self.current_screen == CurrentScreen::Dashboard {
                            self.refresh_dashboard().await;
                        }
                    }
                }
                crossterm::event::KeyCode::Backspace => {
                    if self.feedback.is_none() {
                        self.user_input.pop();
                    }
                }
                crossterm::event::KeyCode::Char(c) => {
                    if self.feedback.is_none() {
                        self.user_input.push(c);
                    }
                }
                _ => {}
            },
        }
        Action::Tick
    }
}
