use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen, AiStatus};

pub fn render(app: &App, frame: &mut Frame) {
    match app.current_screen {
        CurrentScreen::Dashboard => render_dashboard(app, frame),
        CurrentScreen::Quiz => render_quiz(app, frame),
    }
}

fn render_dashboard(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .split(frame.area());

    let title = Paragraph::new("Kana Tutor (AI Powered)")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let msg = format!("欢迎回来！今天有 {} 个假名需要复习。", app.due_count);
    let info = Paragraph::new(msg)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    frame.render_widget(info, chunks[1]);

    let footer = Paragraph::new("按 Enter 开始复习 | 按 Esc 退出")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(footer, chunks[2]);
}

fn render_quiz(app: &App, frame: &mut Frame) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(frame.area());

    // Left: Quiz
    let quiz_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Stats
            Constraint::Min(5),    // Card
            Constraint::Length(3), // Input
            Constraint::Length(3), // Feedback
        ])
        .split(main_layout[0]);

    // Stats
    let stats = Paragraph::new(format!("得分: {}", app.score))
        .block(Block::default().borders(Borders::ALL).title("统计"));
    frame.render_widget(stats, quiz_chunks[0]);

    // Card
    if let Some(card) = &app.current_card {
        let char_display = Paragraph::new(card.kana_char.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("假名"));
        frame.render_widget(char_display, quiz_chunks[1]);
    }

    // Input
    let input = Paragraph::new(app.user_input.as_str())
        .block(Block::default().borders(Borders::ALL).title("输入罗马音"));
    frame.render_widget(input, quiz_chunks[2]);

    // Feedback
    if let Some(correct) = app.feedback {
        let (msg, color) = if correct {
            ("正确！按空格键继续。", Color::Green)
        } else {
            ("错误！", Color::Red)
        };
        let feedback = Paragraph::new(msg)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(feedback, quiz_chunks[3]);
    } else {
        let hint = Paragraph::new("输入答案并按 Enter 提交")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(hint, quiz_chunks[3]);
    }

    // Right: AI Blackboard
    let ai_title = match app.ai_status {
        AiStatus::Idle => "AI 老师板书",
        AiStatus::Thinking => "AI 正在思考...",
        AiStatus::Ready => "AI 老师板书",
        AiStatus::Offline => "AI 未连接",
        AiStatus::Error(_) => "AI 错误",
    };

    let ai_content = if !app.ai_explanation.is_empty() {
        app.ai_explanation.clone()
    } else if let AiStatus::Thinking = app.ai_status {
        "正在分析你的错误...".to_string()
    } else if let AiStatus::Offline = app.ai_status {
        "离线模式".to_string()
    } else if let AiStatus::Error(ref e) = app.ai_status {
        format!("错误: {}", e)
    } else {
        "这里会显示解释。".to_string()
    };

    let blackboard = Paragraph::new(ai_content)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(ai_title));
    frame.render_widget(blackboard, main_layout[1]);
}
