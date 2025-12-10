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

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;

#[derive(PartialEq)]
pub enum CurrentScreen {
    Menu,
    Dashboard,
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
    pub menu_selection: usize, // 0: Dashboard, 1: Hiragana, 2: Katakana, 3: Mixed
    pub user_progress: HashMap<String, UserProgress>,
    pub due_items: Vec<String>,
}

impl App {
    pub fn new() -> App {
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
        };
        app.load_progress();
        app.calculate_due_items();
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
        // Collect all available kanas
        let all_kanas = KanaSet::Mixed.get_data();
        let now = Utc::now();

        self.due_items.clear();
        for kana in all_kanas {
            if let Some(progress) = self.user_progress.get(&kana.character) {
                if progress.next_review_date <= now {
                    self.due_items.push(kana.character.clone());
                }
            } else {
                // If no progress, it's new, so it's "due" (or part of the new pool)
                // For this app, let's say unknown items are available for review if we are in learning mode,
                // but for "Due Items" list in Dashboard, we might only list those that were already seen?
                // Or maybe list everything. Let's list everything "New" or "Due".
                self.due_items.push(kana.character.clone());
            }
        }
    }

    pub fn start_quiz(&mut self) {
        self.kana_set = match self.menu_selection {
            1 => KanaSet::Hiragana,
            2 => KanaSet::Katakana,
            3 => KanaSet::Mixed,
            _ => KanaSet::Mixed, // Default
        };

        // Filter pool based on SRS if in dashboard mode or just generally?
        // The prompt says: "Update start_quiz to filter questions based on next_review_date"
        // But also we have specific modes.
        // If the user selects "Hiragana", should we show ALL Hiragana or only DUE Hiragana?
        // Let's assume standard quiz modes (Hiragana/Katakana/Mixed) include BOTH due items and new items,
        // prioritizing due items.

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

        // Strategy: Mix Due + New.
        // If we have due items, prioritize them.
        // If we don't have enough due items, add new items.
        // If we have neither, maybe review items that are coming up soon? Or just random practice?
        // Let's make the pool = Due + New.
        self.pool = [due, new_items].concat();

        // If pool is empty (completed everything for today), maybe allow reviewing "future" items?
        // Or just tell the user "Good job".
        // For a "Quiz App", preventing play might be annoying.
        // Let's fallback to "review_later" if pool is empty, but maybe mark them as "cramming"?
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

                // Update SRS
                let grade = 5; // Perfect
                let current_progress = self.user_progress.entry(kana.character.clone())
                    .or_insert_with(|| UserProgress::new(kana.character.clone()));

                let new_progress = calculate_new_progress(current_progress, grade);
                self.user_progress.insert(kana.character.clone(), new_progress);

                self.save_progress();

            } else {
                self.feedback = Some(false);
                self.streak = 0;

                // Update SRS
                let grade = 0; // Incorrect
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
                        self.menu_selection = 3; // wrap
                    }
                }
                KeyCode::Down => {
                    if self.menu_selection < 3 {
                        self.menu_selection += 1;
                    } else {
                        self.menu_selection = 0; // wrap
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
                    self.pool.clear(); // cleanup
                    self.calculate_due_items(); // Update due items count
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
