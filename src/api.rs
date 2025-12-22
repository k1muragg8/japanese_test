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
    pub cards: Vec<Card>,
}

async fn get_next_batch(State(state): State<ApiState>) -> impl IntoResponse {
    let mut app = state.app.lock().await;

    // Check if we need to fetch a new batch
    if app.due_cards.is_empty() || app.current_card_index >= app.due_cards.len() {
        app.start_quiz().await;
    }

    let remaining = if app.total_cards_count > app.cycle_seen_ids.len() {
        app.total_cards_count - app.cycle_seen_ids.len()
    } else {
        0
    };

    // batch_counter is 0-indexed internally (0-10). UI wants 1-11.
    // However, if we just finished a batch and are waiting for the next, the counter might have incremented?
    // start_quiz calls fetch_batch_logic which sets due_cards.
    // If due_cards are set, we are "In" that batch.
    // batch_counter 0 means Batch 1.

    let resp = BatchResponse {
        batch_current: app.batch_counter + 1,
        batch_total: 11,
        remaining_in_deck: remaining,
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

    // Remove the card from due_cards to ensure progress (if using index based, this might be redundant but safe)
    // Actually app uses index, removing might mess up index if not careful.
    // But api is stateless-ish.
    // App Logic: `submit_answer` uses `current_card_index`.
    // If the frontend calls submit, we should ideally call `app.submit_answer()`.
    // But `app.submit_answer` uses `app.user_input`. The API passes `correct` boolean directly.
    // So we need to update state manually or use DB directly.
    // The previous implementation used DB directly and ignored App state for submission,
    // BUT `App` accumulates mistakes now! So we MUST update App state.

    // 1. Find card
    // Since we are strictly following "10+1" cycle which is stateful in App, we need to ensure App knows about the mistake.

    if !payload.correct {
        app.cycle_mistakes.insert(payload.card_id.clone());
    }

    // Also advance the index in App if it matches?
    // The API might be called out of sync if multiple clients (unlikely).
    // Let's assume one client.
    // We should advance `app.current_card_index` so `get_next_batch` knows when to fetch new one.

    // If the card submitted is the current one:
    if let Some(card) = app.due_cards.get(app.current_card_index) {
        if card.id == payload.card_id {
             app.current_card_index += 1;

             // Check if batch finished immediately?
             if app.current_card_index >= app.due_cards.len() {
                 // Trigger next batch logic?
                 // Ideally next call to `get_next_batch` triggers it.
             }
        }
    }

    match app.db.update_card(&payload.card_id, payload.correct).await {
        Ok(interval) => Json(SubmitResponse { new_interval: interval }).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
