use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem},
    Frame,
};
use crate::app::{App, AppState, QuizMode};

pub fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    match app.state {
        AppState::Dashboard => {
            let title = format!(" Kana Tutor - Time: {} ", format_duration(app.session_start.elapsed()));
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL);

            let mode_str = match app.quiz_mode {
                QuizMode::Kana => "Kana Only",
                QuizMode::Vocab => "Vocabulary Only",
                QuizMode::Mixed => "Mixed Mode",
            };

            let welcome_msg = if app.due_count > 0 {
                format!("欢迎回来！今天有 {} 个项目需要复习。", app.due_count)
            } else {
                "今日任务已完成！(Mission Complete! Entering Infinite Mode...)".to_string()
            };

            let text = vec![
                Line::from(Span::styled(
                    welcome_msg,
                    Style::default().fg(if app.due_count > 0 { Color::Green } else { Color::Cyan }).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!("当前模式 (Current Mode): {} (Press 'm' to switch)", mode_str)),
                Line::from(""),
                Line::from("按 Enter 开始复习 (Press Enter to start)"),
                Line::from("按 F10 开启隐蔽模式 (Stealth Mode)"),
                Line::from("按 q 退出 (Press q to quit)"),
            ];

            let p = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);

            // Center vert/horiz
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(20), Constraint::Percentage(40)].as_ref())
                .split(size);

            f.render_widget(p, chunks[1]);
        }
        AppState::Quiz => {
            let title = format!(" Quiz - Time: {} ", format_duration(app.session_start.elapsed()));
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
                .split(size);

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(chunks[0]);

            // Timer at the top right of the whole screen? Or just in the block title?
            // The previous block covered the whole screen if I'm not careful.
            // Let's pass the title to draw_quiz

            // Left: Quiz
            draw_quiz(f, app, main_chunks[0], &title);

            // Right: Feedback / Assistant
            draw_assistant(f, app, main_chunks[1]);
        }
        AppState::FakeLog => {
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
    }
}

fn draw_quiz(f: &mut Frame, app: &App, area: ratatui::layout::Rect, title: &str) {
    let block = Block::default().title(title).borders(Borders::ALL);
    f.render_widget(block.clone(), area);

    let inner_area = block.inner(area);

    if app.current_card_index < app.due_cards.len() {
        let card = &app.due_cards[app.current_card_index];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Question (Kanji/Kana)
                Constraint::Length(3),      // Input
                Constraint::Length(3),      // Feedback
                Constraint::Min(0),
            ].as_ref())
            .split(inner_area);

        // Render Question
        // If it's vocab, we show Kanji (if available) and maybe Meaning?
        // Prompt says: "Display: If testing Vocabulary: Show word_kanji (if available) and word_kana."
        // Wait, if we show word_kana, the user just types it?
        // Usually vocab cards show Kanji and ask for reading (Kana) or Reading and ask for Meaning?
        // The prompt says: "User input expects romaji."
        // So we show Kanji/Kana and expect Romaji.
        // If Kanji exists, show it. Maybe show Kana as a hint or hidden?
        // "Show word_kanji (if available) and word_kana."
        // This suggests showing both. If I show "猫 (ねこ)", the user types "neko".

        let question_text = if card.is_vocab {
            let mut text = vec![
                Line::from(Span::styled(
                    &card.kana_char, // This holds kanji if available, or kana
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
            ];

            // If we have sub_text (word_kana) and it's different from main text
            if let Some(sub) = &card.sub_text {
                if sub != &card.kana_char {
                    text.push(Line::from(Span::styled(
                        format!("({})", sub),
                        Style::default().fg(Color::Gray),
                    )));
                }
            }

            // Also show meaning? Prompt says "Show word_kanji ... and word_kana". Doesn't explicitly say meaning.
            // But usually meaning is helpful. I'll add it if it's there.
            if let Some(meaning) = &card.meaning {
                 text.push(Line::from(Span::styled(
                    format!("Meaning: {}", meaning),
                    Style::default().fg(Color::Magenta),
                )));
            }
            text
        } else {
             // Kana mode
             vec![Line::from(Span::styled(
                &card.kana_char,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))]
        };

        let question_p = Paragraph::new(question_text)
            .alignment(Alignment::Center)
            .block(Block::default());

        f.render_widget(question_p, chunks[0]);

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

            let hint = Paragraph::new("Press Enter to continue")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            f.render_widget(hint, chunks[3]);
        }
    }
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
