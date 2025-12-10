mod app;
mod data;
mod db;
mod gemini;
mod ui;

use app::{Action, AiStatus, App};
use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use db::{init_db, seed_database_if_empty};
use dotenvy::dotenv;
use gemini::GeminiClient;
use ratatui::{backend::CrosstermBackend, Terminal};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::env;
use std::io::stdout;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let _ = dotenv();

    // Database Setup
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename("kana.db")
                .create_if_missing(true),
        )
        .await?;

    init_db(&pool).await?;
    seed_database_if_empty(&pool).await?;

    // AI Setup
    let api_key = env::var("GEMINI_API_KEY").ok();
    let (tx, mut rx) = mpsc::unbounded_channel::<(String, String, String)>();
    let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<Result<String, String>>();

    let ai_api_key = api_key.clone();
    tokio::spawn(async move {
        if let Some(key) = ai_api_key {
            let client = GeminiClient::new(key);
            while let Some((kana, romaji, input)) = rx.recv().await {
                match client.fetch_explanation(&kana, &romaji, &input).await {
                    Ok(text) => {
                        let _ = resp_tx.send(Ok(text));
                    }
                    Err(e) => {
                        let _ = resp_tx.send(Err(e.to_string()));
                    }
                }
            }
        }
    });

    // Terminal Setup
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App Setup
    let app_sender = if api_key.is_some() { Some(tx) } else { None };
    let mut app = App::new(pool.clone(), app_sender);

    // Initial fetch
    app.refresh_dashboard().await;

    // Main Loop
    let res = run_loop(&mut terminal, &mut app, &mut resp_rx).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {:?}", e);
    }

    Ok(())
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    resp_rx: &mut mpsc::UnboundedReceiver<Result<String, String>>,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(app, f))?;

        // 1. Check for AI responses
        while let Ok(res) = resp_rx.try_recv() {
            match res {
                Ok(text) => {
                    app.ai_explanation = text;
                    app.ai_status = AiStatus::Ready;
                }
                Err(e) => {
                    app.ai_status = AiStatus::Error(e);
                }
            }
        }

        // 2. Handle Input
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match app.handle_input(key).await {
                    Action::Quit => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
