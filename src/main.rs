use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use dotenvy::dotenv;

mod api;
mod app;
mod data;
mod db;
mod feedback;
// mod ui; // Removed

use crate::app::App;
use crate::api::{app_router, ApiState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();

    // Initialize App (which inits DB)
    let app = App::new().await?;
    let app_state = Arc::new(Mutex::new(app));

    let api_state = ApiState {
        app: app_state,
    };

    let app = app_router(api_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
