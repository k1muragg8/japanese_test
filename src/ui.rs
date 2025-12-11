use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Paragraph, List, ListItem, Wrap},
    Frame,
};
use crate::app::{App, AppState};

pub fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Responsive Threshold
    // "Nano Mode" if height < 10 OR width < 40
    let is_mini = size.height < 10 || size.width < 40;

    match app.state {
        AppState::Dashboard => {
            if is_mini {
                draw_dashboard_mini(f, app, size);
            } else {
                draw_dashboard(f, app, size);
            }
        }
        AppState::Quiz => {
            if is_mini {
                draw_quiz_mini(f, app, size);
            } else {
                draw_quiz(f, app, size);
            }
        }
        AppState::FakeLog => {
            // Boss Mode should look like logs regardless of size, but maybe remove borders in mini
            draw_fake_log(f, app, size, is_mini);
        }
    }
}

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
}

// --- STANDARD MODE (Large Window) ---

fn draw_dashboard(f: &mut Frame, app: &App, size: Rect) {
    let area = centered_rect(60, 50, size);

    let title = " Kana Tutor ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let welcome_msg = if app.due_count > 0 {
        format!("Welcome! Reviews Due: {}", app.due_count)
    } else {
        "All Reviews Done! (Infinite Mode Available)".to_string()
    };

    let text = vec![
        Line::from(Span::styled(
            welcome_msg,
            Style::default().fg(if app.due_count > 0 { Color::Green } else { Color::Cyan }).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Press [Enter] to Start"),
        Line::from("Press [F10] for Boss Mode"),
        Line::from("Press [q] to Quit"),
    ];

    let p = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(p, area);
}

fn draw_quiz(f: &mut Frame, app: &App, size: Rect) {
    // 1. Top Block: Title + Session Timer
    let infinite_str = if app.due_count <= 0 { " (Infinite Mode)" } else { "" };
    let title_text = format!(
        " Kana Tutor | Time: {}{} ",
        format_duration(app.session_start.elapsed()),
        infinite_str
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Main Content
        ].as_ref())
        .split(size);

    let title_p = Paragraph::new(title_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title_p, chunks[0]);

    // 2. Middle (The Card) - Centered Area
    let center_area = centered_rect(60, 50, chunks[1]);

    // Within the center area, we split into Card (Top) and Interaction (Bottom)
    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // The Kana
            Constraint::Percentage(40), // Input / Feedback
        ].as_ref())
        .split(center_area);

    if app.current_card_index < app.due_cards.len() {
        let card = &app.due_cards[app.current_card_index];

        let kana_block = Block::default()
            .borders(Borders::ALL)
            .title(" Kana ");

        // Center vertically
        let empty_lines = card_chunks[0].height.saturating_sub(3) / 2;
        let mut kana_text = vec![];
        for _ in 0..empty_lines {
            kana_text.push(Line::from(""));
        }
        kana_text.push(Line::from(Span::styled(
            &card.kana_char,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        )));

        let kana_p = Paragraph::new(kana_text)
            .block(kana_block)
            .alignment(Alignment::Center);

        f.render_widget(kana_p, card_chunks[0]);

        // Draw Interaction
        let interaction_block = Block::default().borders(Borders::NONE);
        let inner_interaction = interaction_block.inner(card_chunks[1]);

        if let Some(feedback) = &app.current_feedback {
            // RESULT STATE
            let color = if feedback == "Correct!" { Color::Green } else { Color::Red };

            let mut feedback_lines = vec![
                Line::from(Span::styled(feedback, Style::default().fg(color).add_modifier(Modifier::BOLD))),
                Line::from(""),
            ];

            if !app.feedback_detail.is_empty() {
                for line in app.feedback_detail.lines() {
                    feedback_lines.push(Line::from(Span::styled(line, Style::default().fg(Color::Gray))));
                }
                feedback_lines.push(Line::from(""));
            }

            feedback_lines.push(Line::from(Span::styled("(Press Enter to continue)", Style::default().fg(Color::DarkGray))));

            let feedback_p = Paragraph::new(feedback_lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            f.render_widget(feedback_p, inner_interaction);

        } else {
            // QUIZ STATE
            let input_block = Block::default()
                .title(" Enter Romaji ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White));

            let input_p = Paragraph::new(app.user_input.as_str())
                .block(input_block)
                .alignment(Alignment::Center);

            let input_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ].as_ref())
                .split(inner_interaction);

            f.render_widget(input_p, input_area[1]);
        }
    }
}

// --- NANO MODE (Tiny Window) ---

fn draw_dashboard_mini(f: &mut Frame, app: &App, size: Rect) {
    // No borders, just essential info
    let text = if app.due_count > 0 {
        vec![
            Line::from(Span::styled(
                format!("Due: {}", app.due_count),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            )),
            Line::from("Hit [Enter]"),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "Done!",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            )),
            Line::from("[Enter] Inf."),
        ]
    };

    let p = Paragraph::new(text)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(p, size);
}

fn draw_quiz_mini(f: &mut Frame, app: &App, size: Rect) {
    // Nano Layout:
    // Line 1: Timer (Right aligned or just small)
    // Line 2: The Kana (Left or Center)
    // Line 3: Input Prompt "> ..."

    // Constraints: 1 line top, rest middle
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Timer / Info
            Constraint::Length(1), // Kana (Main)
            Constraint::Min(1),    // Input
        ].as_ref())
        .split(size);

    // 1. Timer
    let infinite_str = if app.due_count <= 0 { " (Inf)" } else { "" };
    let timer_text = format!("{}{}", format_duration(app.session_start.elapsed()), infinite_str);
    let timer_p = Paragraph::new(timer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Right);
    f.render_widget(timer_p, chunks[0]);

    if app.current_card_index < app.due_cards.len() {
        let card = &app.due_cards[app.current_card_index];

        // 2. Kana
        // Display as "Question: [Kana]" or just "[Kana]"
        let kana_text = Span::styled(
            format!("Card: {}", card.kana_char),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        );
        let kana_p = Paragraph::new(Line::from(kana_text)).alignment(Alignment::Left);
        f.render_widget(kana_p, chunks[1]);

        // 3. Input or Feedback
        if let Some(feedback) = &app.current_feedback {
            // Feedback State
            let color = if feedback == "Correct!" { Color::Green } else { Color::Red };
            // In Nano mode, we might just show "Correct" or "Wrong: [Ans]"
            let short_feedback = if feedback == "Correct!" {
                "Correct!".to_string()
            } else {
                 format!("X {}", card.romaji)
            };

            let feedback_p = Paragraph::new(Span::styled(short_feedback, Style::default().fg(color)))
                .alignment(Alignment::Left);
            f.render_widget(feedback_p, chunks[2]);

        } else {
            // Input State
            // Prompt: "> [user_input]"
            let input_text = format!("> {}_", app.user_input);
            let input_p = Paragraph::new(input_text).alignment(Alignment::Left);
            f.render_widget(input_p, chunks[2]);
        }
    }
}

// --- SHARED ---

fn draw_fake_log(f: &mut Frame, app: &App, size: Rect, is_mini: bool) {
    let items: Vec<ListItem> = app.fake_logs
        .iter()
        .map(|line| {
            let style = if line.contains("WARN") {
                Style::default().fg(Color::Yellow)
            } else if line.contains("ERROR") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };
            ListItem::new(Line::from(Span::styled(line, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(if is_mini { Borders::NONE } else { Borders::NONE })) // Always none for logs
        .style(Style::default().bg(Color::Black));

    f.render_widget(list, size);
}

fn format_duration(d: std::time::Duration) -> String {
    let total_seconds = d.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}
