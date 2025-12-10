use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, CurrentScreen};

pub fn render(app: &App, frame: &mut Frame) {
    match app.current_screen {
        CurrentScreen::Menu => render_menu(app, frame),
        CurrentScreen::Quiz => render_quiz(app, frame),
    }
}

fn render_menu(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(frame.area());

    let title = Paragraph::new("Kana TUI Practice")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Welcome"));
    frame.render_widget(title, chunks[0]);

    let options = vec!["Hiragana", "Katakana", "Mixed"];
    let mut items = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let style = if i == app.menu_selection {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        items.push(Line::from(vec![Span::styled(*option, style)]));
    }

    let menu = Paragraph::new(items)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Select Mode"));
    frame.render_widget(menu, chunks[1]);

    let footer = Paragraph::new("Use Arrow Keys to select, Enter to start, 'q' to quit")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[2]);
}

fn render_quiz(app: &App, frame: &mut Frame) {
     let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3), // Stats
            Constraint::Percentage(50), // Character
            Constraint::Length(3), // Input
            Constraint::Min(3), // Feedback
        ])
        .split(frame.area());

    // Stats
    let stats_text = format!(
        "Score: {} | Attempts: {} | Streak: {}",
        app.score, app.total_attempts, app.streak
    );
    let stats = Paragraph::new(stats_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Stats"));
    frame.render_widget(stats, chunks[0]);

    // Character
    if let Some(kana) = &app.current_kana {
        let char_text = kana.character;
        let character = Paragraph::new(char_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)) // Placeholder for large text?
            .block(Block::default().borders(Borders::ALL).title("Kana"));
        // Note: Ratatui doesn't have "large text" font rendering built-in easily without ASCII art,
        // so we just display it. Terminals usually render CJK chars fine.
        frame.render_widget(character, chunks[1]);
    }

    // Input
    let input_style = if app.feedback.is_some() {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(app.user_input.as_str())
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title("Romaji Input"));
    frame.render_widget(input, chunks[2]);

    // Feedback
    if let Some(correct) = app.feedback {
        let (_text, color) = if correct {
            ("Correct! Press Enter to continue.", Color::Green)
        } else {
            ("Incorrect", Color::Red)
        };

        let feedback_text = if correct {
             "Correct! Press Enter to continue.".to_string()
        } else {
             let answer = app.current_kana.as_ref().map(|k| k.romaji).unwrap_or("?");
             format!("Incorrect. The answer was '{}'. Press Enter to continue.", answer)
        };

        let feedback = Paragraph::new(feedback_text)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(feedback, chunks[3]);
    } else {
        // Footer hint
         let footer = Paragraph::new("Type the Romaji reading and press Enter. Esc to menu.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(footer, chunks[3]);
    }
}
