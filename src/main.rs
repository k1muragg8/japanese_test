use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod db;
mod feedback;
mod ui;
mod data;

use app::{App, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging (optional)
    // env_logger::init();

    // Initialize App (DB connection, Migration, Seeding)
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
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Global quit
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    return Ok(());
                }

                match app.state {
                    AppState::Dashboard => {
                         if key.code == KeyCode::Enter {
                             app.start_quiz().await;
                         }
                    }
                    AppState::Quiz => {
                         // Check input handling
                         match key.code {
                             KeyCode::Enter => {
                                 // Submit
                                 app.submit_answer().await;
                             }
                             KeyCode::Char(' ') => {
                                 // Continue if result shown
                                 if app.current_feedback.is_some() {
                                     app.next_card().await;
                                 } else {
                                     // Treat space as input char
                                     app.handle_input_char(' ');
                                 }
                             }
                             KeyCode::Backspace => {
                                 app.handle_backspace();
                             }
                             KeyCode::Char(c) => {
                                 // Only handle chars if feedback not shown (locked)
                                 if app.current_feedback.is_none() {
                                     app.handle_input_char(c);
                                 }
                             }
                             _ => {}
                         }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }
}
