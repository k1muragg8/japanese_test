use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, CurrentScreen};
use crate::data::KanaSet;

pub fn render(app: &App, frame: &mut Frame) {
    match app.current_screen {
        CurrentScreen::Menu => render_menu(app, frame),
        CurrentScreen::Dashboard => render_dashboard(app, frame),
        CurrentScreen::Quiz => render_quiz(app, frame),
    }
}

fn render_menu(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(frame.area());

    let title = Paragraph::new("Kana TUI Practice")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Welcome"));
    frame.render_widget(title, chunks[0]);

    let options = vec!["Dashboard", "Hiragana", "Katakana", "Mixed"];
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

fn render_dashboard(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Mastery Gauge
            Constraint::Min(5),    // Due List
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    let title = Paragraph::new("SRS Dashboard")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Mastery Gauge
    // Calculate mastery: percentage of items with interval > 21 days
    let all_kanas = KanaSet::Mixed.get_data();
    let total_items = all_kanas.len() as f64;
    let mastered_count = app.user_progress.values()
        .filter(|p| p.interval > 21)
        .count() as f64;

    let mastery_ratio = if total_items > 0.0 { mastered_count / total_items } else { 0.0 };
    let mastery_percent = (mastery_ratio * 100.0) as u16;

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Total Mastery (> 21 days interval)"))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::ITALIC))
        .percent(mastery_percent);
    frame.render_widget(gauge, chunks[1]);

    // Due List
    let due_items_display: Vec<ListItem> = app.due_items.iter().enumerate().map(|(i, s)| {
        let content = format!("{}. {}", i + 1, s);
        ListItem::new(Line::from(content))
    }).collect();

    let list = List::new(due_items_display)
        .block(Block::default().borders(Borders::ALL).title(format!("Due for Review ({})", app.due_items.len())))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_widget(list, chunks[2]);

    let footer = Paragraph::new("Esc to return to Menu")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[3]);
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
        let char_text = kana.character.as_str();

        // Show interval/easiness if in debug/SRS mode? Or just "Difficulty"?
        // Prompt says: Show "Streak" and "Difficulty Level".
        // Let's calculate difficulty from SRS data if available.
        let difficulty_info = if let Some(progress) = app.user_progress.get(&kana.character) {
             format!("Next review in: {} days (EF: {:.2})", progress.interval, progress.easiness_factor)
        } else {
             "New Item".to_string()
        };

        let block_title = format!("Kana ({})", difficulty_info);

        let character = Paragraph::new(char_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(block_title));
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
             let answer = app.current_kana.as_ref().map(|k| k.romaji.join(" / ")).unwrap_or("?".to_string());
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
