use crate::data::{Kana, KanaSet};
use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use rand::thread_rng;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;

pub enum CurrentScreen {
    Menu,
    Quiz,
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
    pub menu_selection: usize, // 0: Hiragana, 1: Katakana, 2: Mixed
}

impl App {
    pub fn new() -> App {
        App {
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
        }
    }

    pub fn start_quiz(&mut self) {
        self.kana_set = match self.menu_selection {
            0 => KanaSet::Hiragana,
            1 => KanaSet::Katakana,
            _ => KanaSet::Mixed,
        };
        self.pool = self.kana_set.get_data();
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
    }

    pub fn check_answer(&mut self) {
        if let Some(kana) = &self.current_kana {
            self.total_attempts += 1;
            if self.user_input.trim().to_lowercase() == kana.romaji {
                self.feedback = Some(true);
                self.score += 1;
                self.streak += 1;
            } else {
                self.feedback = Some(false);
                self.streak = 0;
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
                        self.menu_selection = 2; // wrap
                    }
                }
                KeyCode::Down => {
                    if self.menu_selection < 2 {
                        self.menu_selection += 1;
                    } else {
                        self.menu_selection = 0; // wrap
                    }
                }
                KeyCode::Enter => self.start_quiz(),
                _ => {}
            },
            CurrentScreen::Quiz => match key_event.code {
                KeyCode::Esc => {
                    self.current_screen = CurrentScreen::Menu;
                    self.pool.clear(); // cleanup
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
