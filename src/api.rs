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
use crate::db::Card;

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

#[derive(Serialize)]
pub struct BatchResponse {
    pub batch_current: usize, // 1-11
    pub batch_total: usize,   // 11
    pub remaining_in_deck: usize, // (Total DB Count - cycle_seen_ids.len())
    pub is_review: bool,
    pub cycle_mistakes_count: usize,
    pub cards: Vec<Card>,
}

async fn get_next_batch(State(state): State<ApiState>) -> impl IntoResponse {
    let mut app = state.app.lock().await;

    // Check if we need to fetch a new batch
    if app.due_cards.is_empty() || app.current_card_index >= app.due_cards.len() {
        app.start_quiz().await;
    }

    let remaining = app.total_cards_count.saturating_sub(app.cycle_seen_ids.len());

    // batch_counter is 1-indexed (1-11) as per updated logic

    let resp = BatchResponse {
        batch_current: app.batch_counter,
        batch_total: 11,
        remaining_in_deck: remaining,
        is_review: app.batch_counter == 11,
        cycle_mistakes_count: app.cycle_mistakes.len(),
        cards: app.due_cards.clone(),
    };

    Json(resp).into_response()
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

    if !payload.correct {
        app.cycle_mistakes.insert(payload.card_id.clone());
    }

    // If the card submitted is the current one:
    if let Some(card) = app.due_cards.get(app.current_card_index) {
        if card.id == payload.card_id {
             app.current_card_index += 1;

             // Check if batch finished immediately?
             if app.current_card_index >= app.due_cards.len() {
                 // Trigger next batch logic
                 app.next_card().await;
             }
        }
    }

    match app.db.update_card(&payload.card_id, payload.correct).await {
        Ok(interval) => Json(SubmitResponse { new_interval: interval }).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
