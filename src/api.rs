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
    if app.due_cards.is_empty() {
        // Only start fresh if truly empty (e.g. init)
        app.start_quiz().await;
    } else if app.current_card_index >= app.due_cards.len() {
        // If current batch finished, force transition to next batch/state
        app.next_card().await;
    }

    let remaining = app.total_cards_count.saturating_sub(app.cycle_seen_ids.len());

    // batch_counter is 1-indexed (1-11) as per updated logic

    let resp = BatchResponse {
        batch_current: app.batch_counter,
        batch_total: 11,
        remaining_in_deck: remaining,
        is_review: app.batch_counter >= 11,
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

    // We trust App::submit_answer to handle mistakes insertion/removal
    // because it has the context of whether it's review mode or not.
    // However, App::submit_answer relies on `user_input` being set?
    // Wait, App::submit_answer sets user_input? No, it uses it.
    // The previous implementation of App::submit_answer used `self.user_input`.
    // But here in API we receive `correct` bool directly.
    // We should probably rely on the payload.
    // But `App::submit_answer` is designed for stateful interaction (TUI/Frontend matching).
    // Let's modify App logic or handle it here.

    // Actually, `App::submit_answer` logic regarding mistakes was:
    // If correct && batch >= 11 -> remove mistake.
    // If wrong -> insert mistake.

    // So we should replicate that logic here or call a method on App that does it.
    // Since we are modifying `app.rs`, let's make sure we update the API handler to match the logic.

    // Logic:
    if payload.correct {
        if app.batch_counter >= 11 {
            app.cycle_mistakes.remove(&payload.card_id);
        }
    } else {
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
