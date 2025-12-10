use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge, List, ListItem},
    Frame,
};

use crate::app::{App, CurrentScreen};

pub fn render(app: &App, frame: &mut Frame) {
    match app.current_screen {
        CurrentScreen::Menu => render_menu(app, frame),
        CurrentScreen::Quiz => render_quiz(app, frame),
        CurrentScreen::Dashboard => render_dashboard(app, frame),
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

    // 0: Dashboard, 1: Hiragana, 2: Katakana, 3: Mixed
    let options = vec!["Dashboard", "Hiragana Quiz", "Katakana Quiz", "Mixed Quiz"];
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

    let title = Paragraph::new("User Progress Dashboard")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Mastery
    let mastery = app.get_mastery_percentage();
    let gauge = Gauge::default()
        .block(Block::default().title("Total Mastery (>21 days interval)").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(mastery);
    frame.render_widget(gauge, chunks[1]);

    // Due Items
    let due_count = app.get_due_count();
    let list_items = vec![
        ListItem::new(format!("Items Due for Review: {}", due_count)),
        ListItem::new(format!("Total Items Tracked: {}", app.user_progress.len())),
    ];
    let list = List::new(list_items)
        .block(Block::default().title("Statistics").borders(Borders::ALL));
    frame.render_widget(list, chunks[2]);

    let footer = Paragraph::new("Press Esc/Enter to return to Menu")
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
        let char_text = &kana.character;
        // Check difficulty level
        let difficulty = if let Some(prog) = app.user_progress.get(char_text) {
             format!("Interval: {}d", prog.interval)
        } else {
             "New".to_string()
        };

        let character = Paragraph::new(char_text.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(format!("Kana ({})", difficulty)));
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
             let answer = app.current_kana.as_ref().map(|k| k.romaji.join(", ")).unwrap_or("?".to_string());
             format!("Incorrect. Answer: '{}'. Press Enter.", answer)
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
