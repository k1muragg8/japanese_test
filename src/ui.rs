use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem},
    Frame,
};
use crate::app::{App, AppState};

pub fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    match app.state {
        AppState::FakeLog => {
            // Render full screen fake log
            draw_fake_log(f, app, size);
        }
        AppState::Dashboard => {
            let block = Block::default()
                .title(" Kana Tutor ")
                .borders(Borders::ALL);

            let text = vec![
                Line::from(Span::styled(
                    format!("欢迎回来！今天有 {} 个卡片 (Kana/Vocab) 需要复习。", app.due_count),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("按 Enter 开始复习 (Press Enter to start)"),
                Line::from("按 F10 切换工作模式 (Boss Key)"),
                Line::from("按 q 退出 (Press q to quit)"),
            ];

            let p = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(20), Constraint::Percentage(40)].as_ref())
                .split(size);

            f.render_widget(p, chunks[1]);
        }
        AppState::Quiz => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(size);

            // Left: Quiz
            draw_quiz(f, app, chunks[0]);

            // Right: Feedback / Assistant
            draw_assistant(f, app, chunks[1]);
        }
    }
}

fn draw_fake_log(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app.fake_logs
        .iter()
        .cycle() // Infinite scroll effect
        .take(50) // Just take enough to fill screen
        .map(|log| {
             let style = if log.contains("ERROR") {
                 Style::default().fg(Color::Red)
             } else if log.contains("WARN") {
                 Style::default().fg(Color::Yellow)
             } else {
                 Style::default().fg(Color::Green) // "Matrix" look or standard terminal green
             };
             ListItem::new(Span::styled(log, style))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE).style(Style::default().bg(Color::Black)));

    f.render_widget(list, area);
}

fn draw_quiz(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().title(" Quiz ").borders(Borders::ALL);
    f.render_widget(block.clone(), area);

    let inner_area = block.inner(area);

    if app.current_card_index < app.due_cards.len() {
        let card = &app.due_cards[app.current_card_index];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Question (larger for vocab)
                Constraint::Length(3),      // Input
                Constraint::Length(3),      // Feedback
                Constraint::Min(0),
            ].as_ref())
            .split(inner_area);

        // Question
        // Handle multiline question (e.g. Kanji \n Meaning)
        let question_text = card.question.clone();

        let q_p = Paragraph::new(question_text)
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default());

        f.render_widget(q_p, chunks[0]);

        // Input
        let input_text = format!("Answer: {}", app.user_input);
        let input_p = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::ALL).title(" Romaji "));
        f.render_widget(input_p, chunks[1]);

        // Feedback
        if let Some(feedback) = &app.current_feedback {
            let color = if feedback == "Correct!" { Color::Green } else { Color::Red };
            let fb_p = Paragraph::new(Span::styled(feedback, Style::default().fg(color)))
                .alignment(Alignment::Center);
            f.render_widget(fb_p, chunks[2]);

            let hint = Paragraph::new("Press Space to continue")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            f.render_widget(hint, chunks[3]);
        }
    }
}

fn draw_assistant(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default().title(" 学习助手 (Study Assistant) ").borders(Borders::ALL);

    let text = if !app.feedback_detail.is_empty() {
        app.feedback_detail.clone()
    } else {
        "在此显示反馈和提示...".to_string()
    };

    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(p, area);
}
