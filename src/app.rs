use crate::data::{KanaSet};
use crate::models::{Kana, UserProgress};
use crate::srs::calculate_next_review;
use crossterm::event::{KeyCode, KeyEvent};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use chrono::Utc;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;

pub enum CurrentScreen {
    Menu,
    Quiz,
    Dashboard,
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
    pub menu_selection: usize, // 0: Dashboard, 1: Hiragana, 2: Katakana, 3: Mixed (Shifted by 1)
    pub user_progress: HashMap<String, UserProgress>,
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
        };
        app.load_progress();
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
            let _ = serde_json::to_writer(file, &self.user_progress);
        }
    }

    pub fn start_quiz(&mut self) {
        self.kana_set = match self.menu_selection {
            1 => KanaSet::Hiragana,
            2 => KanaSet::Katakana,
            _ => KanaSet::Mixed,
        };

        let all_kana = self.kana_set.get_data();
        let now = Utc::now();

        // Filter: Prefer items due for review
        let due_items: Vec<Kana> = all_kana.iter()
            .filter(|k| {
                if let Some(prog) = self.user_progress.get(&k.character) {
                    prog.next_review_date <= now
                } else {
                    true // New items are always due
                }
            })
            .cloned()
            .collect();

        // If no due items, use all items (cram mode) or just some random ones
        // For now, if due_items is empty, we just practice everything.
        self.pool = if due_items.is_empty() {
            all_kana
        } else {
            due_items
        };

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

            // Check against all valid answers
            if kana.romaji.contains(&input) {
                self.feedback = Some(true);
                self.score += 1;
                self.streak += 1;

                // Update Progress (Grade 5)
                let mut progress = self.user_progress.get(&kana.character)
                    .cloned()
                    .unwrap_or_else(|| UserProgress::new(&kana.character));

                progress = calculate_next_review(&progress, 5);
                self.user_progress.insert(kana.character.clone(), progress);

            } else {
                self.feedback = Some(false);
                self.streak = 0;

                // Update Progress (Grade 0)
                let mut progress = self.user_progress.get(&kana.character)
                    .cloned()
                    .unwrap_or_else(|| UserProgress::new(&kana.character));

                progress = calculate_next_review(&progress, 0);
                self.user_progress.insert(kana.character.clone(), progress);
            }

            self.save_progress();
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.current_screen {
            CurrentScreen::Menu => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.save_progress();
                    self.exit = true;
                },
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
                        self.current_screen = CurrentScreen::Dashboard;
                    } else {
                        self.start_quiz();
                    }
                },
                _ => {}
            },
            CurrentScreen::Dashboard => match key_event.code {
                 KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace => {
                     self.current_screen = CurrentScreen::Menu;
                 }
                 _ => {}
            }
            CurrentScreen::Quiz => match key_event.code {
                KeyCode::Esc => {
                    self.save_progress();
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

    pub fn get_mastery_percentage(&self) -> f64 {
        if self.user_progress.is_empty() {
            return 0.0;
        }
        let mastered = self.user_progress.values().filter(|p| p.interval > 21).count();
        // Total possible items (approximate or just count tracked items)
        // If we want total possible kana, we'd need to count them all.
        // For now, let's just use the count of items the user has ever seen (in map),
        // or hardcode the total (~200).
        // Better: count tracked items.
        (mastered as f64 / self.user_progress.len() as f64).min(1.0)
    }

    pub fn get_due_count(&self) -> usize {
        let now = Utc::now();
        self.user_progress.values().filter(|p| p.next_review_date <= now).count()
    }
}
