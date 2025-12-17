use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::sync::Arc;
use tower_http::services::ServeDir;

mod app;
mod db;
mod feedback;
mod ui;
mod data;
mod api;

use app::{App, AppState};
use db::Db;
use api::ApiState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--web") {
        let db = Db::new().await?;
        let state = ApiState { db: Arc::new(db) };

        let app_router = api::app_router(state)
            .fallback_service(ServeDir::new("frontend/dist")); // Serve static files

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        println!("Server running on http://0.0.0.0:3000");
        axum::serve(listener, app_router).await?;

        return Ok(());
    }

    // Initialize App
    let mut app = App::new().await?;

    // TUI Setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main Loop
    let res = run_app(&mut terminal, &mut app).await;

    // Graceful Shutdown
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100); // Faster tick for smooth logs
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Global Keybinds
                if key.code == KeyCode::F(10) {
                    app.toggle_fake_log();
                    continue;
                }

                // If in FakeLog, ignore most inputs except quit or toggle
                if let AppState::FakeLog = app.state {
                    if key.code == KeyCode::Char('q') {
                         return Ok(());
                    }
                    continue;
                }

                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    return Ok(());
                }

                match app.state {
                    AppState::Dashboard => {
                         match key.code {
                             KeyCode::Enter => {
                                 app.start_quiz().await;
                             }
                             // Removed 'm' keybind for mode toggle
                             _ => {}
                         }
                    }
                    AppState::Quiz => {
                         match key.code {
                             KeyCode::Enter => {
                                 if app.current_feedback.is_some() {
                                     app.next_card().await;
                                 } else {
                                     app.submit_answer().await;
                                 }
                             }
                             KeyCode::Char(' ') => {
                                 if app.current_feedback.is_some() {
                                     app.next_card().await;
                                 } else {
                                     app.handle_input_char(' ');
                                 }
                             }
                             KeyCode::Backspace => {
                                 app.handle_backspace();
                             }
                             KeyCode::Char(c) => {
                                 if app.current_feedback.is_none() {
                                     app.handle_input_char(c);
                                 }
                             }
                             _ => {}
                         }
                    }
                    AppState::FakeLog => {
                        // Handled above
                    }
                }
            }
        }

        // Ticks
        if last_tick.elapsed() >= tick_rate {
            app.tick_fake_log();
            last_tick = std::time::Instant::now();
        }
    }
}
