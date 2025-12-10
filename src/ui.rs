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

    match app.state {
        AppState::Dashboard => draw_dashboard(f, app, size),
        AppState::Quiz => draw_quiz(f, app, size),
        AppState::FakeLog => draw_fake_log(f, app, size),
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
    // Use 60% width, 50% height of the remaining space
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

        // Draw Kana
        // We want it vertically centered in its chunk too, so let's use a Paragraph with newline padding or alignment?
        // Ratatui Paragraph doesn't vertically align easily without constraints.
        // Let's just use a Block for the border, and render text inside.

        let kana_block = Block::default()
            .borders(Borders::ALL)
            .title(" Kana ");

        // To make text "LARGE", we can't really change font size in TUI, but we can make it BOLD and colored.
        // We can also center it.
        // Let's try to center the text vertically by adding empty lines.
        let empty_lines = card_chunks[0].height.saturating_sub(3) / 2; // rough estimate
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

            // Add detailed feedback if available (e.g. "Next review: 2 days")
            if !app.feedback_detail.is_empty() {
                // Split by newline to handle multi-line details
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

            // Vertically center the input box within the bottom half
            let input_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Spacer
                    Constraint::Length(3), // Box height
                    Constraint::Min(0),
                ].as_ref())
                .split(inner_interaction);

            f.render_widget(input_p, input_area[1]);
        }
    }
}

fn draw_fake_log(f: &mut Frame, app: &App, size: Rect) {
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
        .block(Block::default().borders(Borders::NONE))
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
