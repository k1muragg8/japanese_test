use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen, AiStatus};
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

    let title = Paragraph::new("日语假名练习 (Kana TUI Practice)")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("欢迎"));
    frame.render_widget(title, chunks[0]);

    let options = vec!["仪表板 (Dashboard)", "平假名 (Hiragana)", "片假名 (Katakana)", "混合 (Mixed)"];
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
        .block(Block::default().borders(Borders::ALL).title("选择模式"));
    frame.render_widget(menu, chunks[1]);

    let footer = Paragraph::new("使用方向键选择，回车键开始，'q' 退出")
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

    let title = Paragraph::new("SRS 仪表板")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Mastery Gauge
    let all_kanas = KanaSet::Mixed.get_data();
    let total_items = all_kanas.len() as f64;
    let mastered_count = app.user_progress.values()
        .filter(|p| p.interval > 21)
        .count() as f64;

    let mastery_ratio = if total_items > 0.0 { mastered_count / total_items } else { 0.0 };
    let mastery_percent = (mastery_ratio * 100.0) as u16;

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("总掌握度 (> 21 天间隔)"))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::ITALIC))
        .percent(mastery_percent);
    frame.render_widget(gauge, chunks[1]);

    // Due List
    let due_items_display: Vec<ListItem> = app.due_items.iter().enumerate().map(|(i, s)| {
        let content = format!("{}. {}", i + 1, s);
        ListItem::new(Line::from(content))
    }).collect();

    let list = List::new(due_items_display)
        .block(Block::default().borders(Borders::ALL).title(format!("待复习 ({})", app.due_items.len())))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_widget(list, chunks[2]);

    let footer = Paragraph::new("按 Esc 返回菜单")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[3]);
}

fn render_quiz(app: &App, frame: &mut Frame) {
    // Split screen: Left for Quiz, Right for AI Blackboard
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(frame.area());

    // Left Pane: Quiz
    let quiz_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3), // Stats
            Constraint::Percentage(50), // Character
            Constraint::Length(3), // Input
            Constraint::Min(3), // Feedback
        ])
        .split(main_layout[0]);

    // Stats
    let stats_text = format!(
        "得分: {} | 次数: {} | 连胜: {}",
        app.score, app.total_attempts, app.streak
    );
    let stats = Paragraph::new(stats_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("统计"));
    frame.render_widget(stats, quiz_chunks[0]);

    // Character
    if let Some(kana) = &app.current_kana {
        let char_text = kana.character.as_str();

        let difficulty_info = if let Some(progress) = app.user_progress.get(&kana.character) {
             format!("下次复习: {} 天后 (EF: {:.2})", progress.interval, progress.easiness_factor)
        } else {
             "新项目".to_string()
        };

        let block_title = format!("假名 ({})", difficulty_info);

        let character = Paragraph::new(char_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(block_title));
        frame.render_widget(character, quiz_chunks[1]);
    }

    // Input
    let input_style = if app.feedback.is_some() {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(app.user_input.as_str())
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title("输入罗马音"));
    frame.render_widget(input, quiz_chunks[2]);

    // Feedback
    if let Some(correct) = app.feedback {
        let (_text, color) = if correct {
            ("正确！按回车继续。", Color::Green)
        } else {
            ("错误", Color::Red)
        };

        let feedback_text = if correct {
             "正确！按回车继续。".to_string()
        } else {
             let answer = app.current_kana.as_ref().map(|k| k.romaji.join(" / ")).unwrap_or("?".to_string());
             format!("错误。答案是 '{}'。按回车继续。", answer)
        };

        let feedback = Paragraph::new(feedback_text)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(feedback, quiz_chunks[3]);
    } else {
        // Footer hint
         let footer = Paragraph::new("输入罗马音并回车。按 Esc 返回菜单。")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(footer, quiz_chunks[3]);
    }

    // Right Pane: AI Blackboard
    let ai_block_title = match app.ai_status {
        AiStatus::Idle => "AI 老师板书",
        AiStatus::Thinking => "AI 正在思考...",
        AiStatus::Ready => "AI 老师板书",
        AiStatus::Offline => "AI 离线",
        AiStatus::Error(_) => "AI 错误",
    };

    let ai_text = if !app.ai_explanation.is_empty() {
        app.ai_explanation.clone()
    } else if let AiStatus::Thinking = app.ai_status {
        "正在分析你的错误...".to_string()
    } else if let AiStatus::Offline = app.ai_status {
        "未检测到 API Key，AI 功能不可用。".to_string()
    } else if let AiStatus::Error(ref e) = app.ai_status {
        format!("发生错误: {}", e)
    } else {
        "这里会显示 AI 对错误的解释。".to_string()
    };

    let ai_widget = Paragraph::new(ai_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(ai_block_title));

    frame.render_widget(ai_widget, main_layout[1]);
}
