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
        AppState::Quiz => draw_focus_mode(f, app, size),
        AppState::FakeLog => draw_fake_log(f, app, size),
    }
}

fn draw_dashboard(f: &mut Frame, app: &App, size: Rect) {
    // Simple centered dashboard
    // Use flexible constraints but ensure enough height for content
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Top spacer
            Constraint::Length(7), // Content height (5 lines text + spacing)
            Constraint::Min(1),    // Bottom spacer
        ].as_ref())
        .split(size);

    // If screen is extremely small, use the full area, otherwise center
    let area = if size.height < 7 { size } else { layout[1] };

    let welcome_msg = if app.due_count > 0 {
        format!("Reviews Due: {}", app.due_count)
    } else {
        "All Done!".to_string()
    };

    let text = vec![
        Line::from(Span::styled(
            "Kana Tutor",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            welcome_msg,
            Style::default().fg(if app.due_count > 0 { Color::Green } else { Color::Gray }),
        )),
        Line::from(""),
        Line::from(Span::styled("Press [Enter] to Start", Style::default().fg(Color::DarkGray))),
    ];

    let p = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(p, area);
}

fn draw_focus_mode(f: &mut Frame, app: &App, size: Rect) {
    // Focus Mode Layout:
    // 1. Top (20%): Timer & Status
    // 2. Middle (40%): The Question Card (Big Kana)
    // 3. Bottom (40%): Input & Feedback

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ].as_ref())
        .split(size);

    // --- 1. Top: Info ---
    let infinite_str = if app.due_count <= 0 { " (Inf)" } else { "" };
    let timer_text = format!("Time: {}{}", format_duration(app.session_start.elapsed()), infinite_str);

    // Use a dim style for the header so it doesn't distract
    let header = Paragraph::new(timer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(header, chunks[0]);


    if app.current_card_index < app.due_cards.len() {
        let card = &app.due_cards[app.current_card_index];

        // --- 2. Middle: The Question (Big Kana) ---
        // Style: Yellow, Bold, Underlined, Centered.
        // Vertically centered in the 40% chunk.

        let mid_chunk = chunks[1];
        let vertical_center_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25), // Spacer
                Constraint::Percentage(50), // Content
                Constraint::Percentage(25), // Spacer
            ].as_ref())
            .split(mid_chunk);

        let kana_content = Span::styled(
            &card.kana_char,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        );

        let kana_p = Paragraph::new(Line::from(kana_content))
            .alignment(Alignment::Center);

        f.render_widget(kana_p, vertical_center_layout[1]);


        // --- 3. Bottom: Input & Feedback ---
        let bot_chunk = chunks[2];
        let bot_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10), // Top padding
                Constraint::Percentage(80), // Content
                Constraint::Percentage(10), // Bot padding
            ].as_ref())
            .split(bot_chunk);

        if let Some(feedback) = &app.current_feedback {
            // FEEDBACK STATE
            let is_correct = feedback == "Correct!";
            let color = if is_correct { Color::Green } else { Color::Red };

            let mut lines = vec![];

            if is_correct {
                lines.push(Line::from(Span::styled("Good!", Style::default().fg(color).add_modifier(Modifier::BOLD))));
            } else {
                // Show correction
                lines.push(Line::from(Span::styled(feedback, Style::default().fg(color).add_modifier(Modifier::BOLD))));
            }

            // Detail
            if !app.feedback_detail.is_empty() {
                lines.push(Line::from(""));
                for subline in app.feedback_detail.lines() {
                     lines.push(Line::from(Span::styled(subline, Style::default().fg(Color::Gray))));
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("[Enter] to Continue", Style::default().fg(Color::DarkGray))));

            let feedback_p = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            f.render_widget(feedback_p, bot_layout[1]);

        } else {
            // INPUT STATE
            // Prompt: > [ ______ ]
            // We want to visualize the text being typed.

            // "When typing, show text in Color::White"
            let input_content = vec![
                Line::from(vec![
                    Span::styled("> [ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&app.user_input, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(" ]", Style::default().fg(Color::DarkGray)),
                ]),
            ];

            let input_p = Paragraph::new(input_content)
                .alignment(Alignment::Center);

            f.render_widget(input_p, bot_layout[1]);
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
