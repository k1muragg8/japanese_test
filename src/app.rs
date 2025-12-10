use crate::data::KanaSet;
use crate::models::{Kana, UserProgress};
use crate::srs::calculate_new_progress;
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use tokio::sync::mpsc::UnboundedSender;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;

#[derive(PartialEq)]
pub enum CurrentScreen {
    Menu,
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

pub enum AiRequest {
    ExplainMistake { correct: String, input: String },
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub kana_set: KanaSet,
    pub current_kana: Option<Kana>,
    pub user_input: String,
    pub feedback: Option<bool>, // None = typing, Some(true) = correct, Some(false) = incorrect
    pub score: u32,
    pub total_attempts: u32,
    pub streak: u32,
    pub exit: bool,
    pub pool: Vec<Kana>,
    pub menu_selection: usize, // 0: Dashboard, 1: Hiragana, 2: Katakana, 3: Mixed
    pub user_progress: HashMap<String, UserProgress>,
    pub due_items: Vec<String>,

    // AI Integration
    pub ai_sender: Option<UnboundedSender<AiRequest>>,
    pub ai_status: AiStatus,
    pub ai_explanation: String,
}

impl App {
    pub fn new(ai_sender: Option<UnboundedSender<AiRequest>>) -> App {
        let mut app = App {
            current_screen: CurrentScreen::Menu,
            kana_set: KanaSet::Hiragana,
            current_kana: None,
            user_input: String::new(),
            feedback: None,
            score: 0,
            total_attempts: 0,
            streak: 0,
            exit: false,
            pool: Vec::new(),
            menu_selection: 0,
            user_progress: HashMap::new(),
            due_items: Vec::new(),
            ai_sender,
            ai_status: AiStatus::Idle,
            ai_explanation: String::new(),
        };
        app.load_progress();
        app.calculate_due_items();

        // If no sender (offline mode), set status to Offline
        if app.ai_sender.is_none() {
            app.ai_status = AiStatus::Offline;
        }

        app
    }

    pub fn load_progress(&mut self) {
        if let Ok(file) = File::open("progress.json") {
            let reader = BufReader::new(file);
            if let Ok(progress) = serde_json::from_reader(reader) {
                self.user_progress = progress;
            }
        }
    }

    pub fn save_progress(&self) {
        if let Ok(file) = File::create("progress.json") {
            let writer = BufWriter::new(file);
            let _ = serde_json::to_writer(writer, &self.user_progress);
        }
    }

    pub fn calculate_due_items(&mut self) {
        let all_kanas = KanaSet::Mixed.get_data();
        let now = Utc::now();

        self.due_items.clear();
        for kana in all_kanas {
            if let Some(progress) = self.user_progress.get(&kana.character) {
                if progress.next_review_date <= now {
                    self.due_items.push(kana.character.clone());
                }
            } else {
                self.due_items.push(kana.character.clone());
            }
        }
    }

    pub fn start_quiz(&mut self) {
        self.kana_set = match self.menu_selection {
            1 => KanaSet::Hiragana,
            2 => KanaSet::Katakana,
            3 => KanaSet::Mixed,
            _ => KanaSet::Mixed,
        };

        let all_kanas = self.kana_set.get_data();
        let now = Utc::now();

        let mut due: Vec<Kana> = Vec::new();
        let mut new_items: Vec<Kana> = Vec::new();
        let mut review_later: Vec<Kana> = Vec::new();

        for kana in all_kanas {
            if let Some(progress) = self.user_progress.get(&kana.character) {
                if progress.next_review_date <= now {
                    due.push(kana);
                } else {
                    review_later.push(kana);
                }
            } else {
                new_items.push(kana);
            }
        }

        self.pool = [due, new_items].concat();

        if self.pool.is_empty() && !review_later.is_empty() {
            self.pool = review_later;
        }

        self.score = 0;
        self.total_attempts = 0;
        self.streak = 0;
        self.current_screen = CurrentScreen::Quiz;
        self.next_question();
    }

    pub fn next_question(&mut self) {
        let mut rng = thread_rng();
        if let Some(kana) = self.pool.choose(&mut rng) {
            self.current_kana = Some(kana.clone());
        }
        self.user_input.clear();
        self.feedback = None;
        self.reset_ai_state(); // Clear previous explanation
    }

    fn reset_ai_state(&mut self) {
        if let AiStatus::Offline = self.ai_status {
            // Keep Offline status
        } else {
            self.ai_status = AiStatus::Idle;
        }
        self.ai_explanation.clear();
    }

    pub fn check_answer(&mut self) {
        if let Some(kana) = &self.current_kana {
            self.total_attempts += 1;
            let input = self.user_input.trim().to_lowercase();
            let is_correct = kana.romaji.contains(&input);

            if is_correct {
                self.feedback = Some(true);
                self.score += 1;
                self.streak += 1;

                let grade = 5;
                let current_progress = self.user_progress.entry(kana.character.clone())
                    .or_insert_with(|| UserProgress::new(kana.character.clone()));

                let new_progress = calculate_new_progress(current_progress, grade);
                self.user_progress.insert(kana.character.clone(), new_progress);

                self.save_progress();

            } else {
                self.feedback = Some(false);
                self.streak = 0;

                // Request AI Explanation
                if let Some(sender) = &self.ai_sender {
                     let _ = sender.send(AiRequest::ExplainMistake {
                         correct: kana.character.clone(),
                         input: self.user_input.clone(),
                     });
                     self.ai_status = AiStatus::Thinking;
                }

                let grade = 0;
                let current_progress = self.user_progress.entry(kana.character.clone())
                    .or_insert_with(|| UserProgress::new(kana.character.clone()));

                let new_progress = calculate_new_progress(current_progress, grade);
                self.user_progress.insert(kana.character.clone(), new_progress);

                self.save_progress();
            }
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.current_screen {
            CurrentScreen::Menu => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
                KeyCode::Up => {
                    if self.menu_selection > 0 {
                        self.menu_selection -= 1;
                    } else {
                        self.menu_selection = 3;
                    }
                }
                KeyCode::Down => {
                    if self.menu_selection < 3 {
                        self.menu_selection += 1;
                    } else {
                        self.menu_selection = 0;
                    }
                }
                KeyCode::Enter => {
                    if self.menu_selection == 0 {
                        self.calculate_due_items();
                        self.current_screen = CurrentScreen::Dashboard;
                    } else {
                        self.start_quiz();
                    }
                }
                _ => {}
            },
            CurrentScreen::Dashboard => match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => self.current_screen = CurrentScreen::Menu,
                _ => {}
            }
            CurrentScreen::Quiz => match key_event.code {
                KeyCode::Esc => {
                    self.current_screen = CurrentScreen::Menu;
                    self.pool.clear();
                    self.calculate_due_items();
                }
                KeyCode::Enter => {
                    if self.feedback.is_some() {
                        self.next_question();
                    } else {
                        self.check_answer();
                    }
                }
                KeyCode::Backspace => {
                    if self.feedback.is_none() {
                        self.user_input.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if self.feedback.is_none() {
                         self.user_input.push(c);
                    }
                }
                _ => {}
            },
        }
    }
}
