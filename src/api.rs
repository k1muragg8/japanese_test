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
    pub batch_current: usize,
    pub batch_total: usize,     // 动态的总轮数
    pub remaining_in_deck: usize,
    pub is_review: bool,
    pub cycle_mistakes_count: usize,
    pub cards: Vec<Card>,
}

async fn get_next_batch(State(state): State<ApiState>) -> impl IntoResponse {
    let mut app = state.app.lock().await;

    // 状态流转检查：如果当前批次空了或完了，尝试推到下一状态
    let is_batch_empty = app.due_cards.is_empty();
    let is_batch_finished = app.current_card_index >= app.due_cards.len();

    if is_batch_empty {
        app.start_quiz().await;
    } else if is_batch_finished {
        app.next_card().await;
    }

    let remaining = app.total_cards_count.saturating_sub(app.cycle_seen_ids.len());

    let resp = BatchResponse {
        batch_current: app.batch_counter,
        // 如果在复习模式，就不显示总轮数了（或者显示当前轮数），否则显示预估总数
        batch_total: if app.is_review_phase { app.batch_counter } else { app.estimated_total_batches },
        remaining_in_deck: remaining,
        is_review: app.is_review_phase,
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

    // 1. 处理错题记录逻辑
    if payload.correct {
        // 如果在复习模式，答对了就从错题本移除
        if app.is_review_phase {
            app.cycle_mistakes.remove(&payload.card_id);
        }
    } else {
        // 答错永远进错题本
        app.cycle_mistakes.insert(payload.card_id.clone());
    }

    // 2. 强制推进进度
    // 只要接收到提交，就无条件 +1，防止死锁
    app.current_card_index += 1;

    // 3. 预判：如果这组做完了，立刻触发下一组准备工作
    // 这样当前端请求 next_batch 时，数据已经准备好了
    if app.current_card_index >= app.due_cards.len() {
        app.next_card().await;
    }

    // 4. 更新数据库 FSRS 状态
    match app.db.update_card(&payload.card_id, payload.correct).await {
        Ok(interval) => Json(SubmitResponse { new_interval: interval }).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}