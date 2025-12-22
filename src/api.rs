use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use crate::app::App;

#[derive(Clone)]
pub struct ApiState {
    pub app: Arc<Mutex<App>>,
}

pub fn app_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/next_batch", get(get_next_batch))
        .route("/api/submit", post(submit_answer))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn get_next_batch(State(state): State<ApiState>) -> impl IntoResponse {
    let mut app = state.app.lock().await;

    // If we have due cards, return them. But wait, `App` logic usually fetches on start_quiz or next_card.
    // If the frontend asks for a batch, it expects cards.
    // If `app.due_cards` is empty or exhausted, we should fetch new ones.

    // Check if we need to fetch a new batch
    if app.due_cards.is_empty() || app.current_card_index >= app.due_cards.len() {
        // Trigger start_quiz or next_card logic to fetch
        // Since `start_quiz` resets everything, it's safer if we treat this as a "Get me cards" request.
        app.start_quiz().await;
    }

    // If still empty, return empty
    if app.due_cards.is_empty() {
        return Json(Vec::<crate::db::Card>::new()).into_response();
    }

    // Return the cards
    // Note: The frontend might be stateless. If so, it might request the same batch again?
    // If the frontend is stateful (Leptos), it will ask for a batch once and iterate.
    // So we return the `due_cards`.
    Json(app.due_cards.clone()).into_response()
}

#[derive(Deserialize)]
struct SubmitRequest {
    card_id: String,
    correct: bool,
}

#[derive(Serialize)]
struct SubmitResponse {
    new_interval: i64,
}

async fn submit_answer(
    State(state): State<ApiState>,
    Json(payload): Json<SubmitRequest>,
) -> impl IntoResponse {
    let mut app = state.app.lock().await;

    // Remove the card from due_cards to ensure progress
    if let Some(pos) = app.due_cards.iter().position(|c| c.id == payload.card_id) {
        app.due_cards.remove(pos);
    }

    match app.db.update_card(&payload.card_id, payload.correct).await {
        Ok(interval) => Json(SubmitResponse { new_interval: interval }).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
