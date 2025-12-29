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
        .route(
            "/api/next_batch",
            get(get_next_batch).with_state(state.clone())
        )
        .route(
            "/api/submit",
            post(submit_answer).with_state(state)
        )
        .layer(CorsLayer::permissive())
}

#[derive(Serialize)]
pub struct BatchResponse {
    pub batch_current: usize,
    pub batch_total: usize,
    pub remaining_in_deck: usize,
    pub is_review: bool,
    pub cycle_mistakes_count: usize,
    pub cards: Vec<Card>,
    // 【新增】告诉前端当前的真实进度索引
    pub current_card_index: usize,
}

async fn get_next_batch(State(state): State<ApiState>) -> impl IntoResponse {
    let mut app = state.app.lock().await;

    let is_batch_empty = app.due_cards.is_empty();
    let is_batch_finished = app.current_card_index >= app.due_cards.len();

    // 如果当前批次做完了，或者为空，才去生成新的
    if is_batch_empty {
        app.start_quiz().await;
    } else if is_batch_finished {
        app.next_card().await;
    }

    let remaining = app.deck_queue.len();
    let is_review_effective = app.is_review_phase || (remaining == 0 && !app.cycle_mistakes.is_empty());

    let resp = BatchResponse {
        batch_current: app.batch_counter,
        batch_total: if is_review_effective { app.batch_counter } else { app.estimated_total_batches },
        remaining_in_deck: remaining,
        is_review: is_review_effective,
        cycle_mistakes_count: app.cycle_mistakes.len(),
        cards: app.due_cards.clone(),
        // 【关键】返回服务端记录的当前索引
        current_card_index: app.current_card_index,
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

    if payload.correct {
        if app.is_review_phase {
            app.cycle_mistakes.remove(&payload.card_id);
        }
    } else {
        app.cycle_mistakes.insert(payload.card_id.clone());
    }

    // 后端推移进度
    app.current_card_index += 1;

    // 如果做完了，准备下一批
    if app.current_card_index >= app.due_cards.len() {
        app.next_card().await;
    }

    match app.db.update_card(&payload.card_id, payload.correct).await {
        Ok(interval) => Json(SubmitResponse { new_interval: interval }).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}