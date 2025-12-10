mod app;
mod data;
mod models;
mod srs;
mod tui;
mod ui;

use app::App;
use color_eyre::eyre::Result;
use crossterm::event::{self, Event};
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = tui::init()?;
    let mut app = App::new();

    let res = run_app(&mut terminal, &mut app);

    tui::restore()?; // Ensure terminal is restored properly

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut tui::Tui, app: &mut App) -> Result<()> {
    while !app.exit {
        terminal.draw(|frame| ui::render(app, frame))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }
    }
    Ok(())
}
