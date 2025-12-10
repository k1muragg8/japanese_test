mod app;
mod data;
mod gemini;
mod models;
mod srs;
mod tui;
mod ui;

use app::{App, AiRequest, AiStatus};
use gemini::GeminiClient;
use color_eyre::eyre::Result;
use crossterm::event::{self, Event};
use std::time::Duration;
use tokio::sync::mpsc;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Load .env
    let _ = dotenvy::dotenv(); // Ignore error if file not found
    let api_key = env::var("GEMINI_API_KEY").ok();

    // Channel for UI to talk to AI
    let (tx, mut rx) = mpsc::unbounded_channel::<AiRequest>();

    // Channel for AI to talk to UI (Explanation received)
    let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<Result<String, String>>();

    // Spawn AI Task
    let ai_api_key = api_key.clone();
    tokio::spawn(async move {
        if let Some(key) = ai_api_key {
            let client = GeminiClient::new(key);
            while let Some(req) = rx.recv().await {
                match req {
                    AiRequest::ExplainMistake { correct, input } => {
                        let res = client.explain_mistake(&correct, &input).await;
                        match res {
                            Ok(explanation) => {
                                let _ = resp_tx.send(Ok(explanation));
                            }
                            Err(e) => {
                                let _ = resp_tx.send(Err(e.to_string()));
                            }
                        }
                    }
                }
            }
        } else {
            // No API Key, consume channel but do nothing or send error if requested?
            // The App knows it's offline, so it might not send requests.
            // But if it does, we just drop them or handle gracefully.
            while let Some(_) = rx.recv().await {
                // Do nothing
            }
        }
    });

    let mut terminal = tui::init()?;
    // If no API key, pass None to App so it knows it is offline
    let app_sender = if api_key.is_some() { Some(tx) } else { None };
    let mut app = App::new(app_sender);

    let res = run_app(&mut terminal, &mut app, &mut resp_rx).await;

    terminal.clear()?;
    tui::restore()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut tui::Tui,
    app: &mut App,
    resp_rx: &mut mpsc::UnboundedReceiver<Result<String, String>>
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(app, frame))?;

        if app.exit {
            return Ok(());
        }

        // Poll for input events
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }

        // Check for AI responses
        while let Ok(response) = resp_rx.try_recv() {
            match response {
                Ok(explanation) => {
                    app.ai_explanation = explanation;
                    app.ai_status = AiStatus::Ready;
                }
                Err(err) => {
                    app.ai_status = AiStatus::Error(err);
                }
            }
        }
    }
}
