use std::time::Instant;
use crate::db::{Db, Card};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub enum AppState {
    Dashboard,
    Quiz,
}

#[allow(unused)]
pub struct App {
    pub db: Arc<Db>,
    pub state: AppState,
    pub due_cards: Vec<Card>,
    pub current_card_index: usize,
    pub user_input: String,
    pub current_feedback: Option<String>,
    pub feedback_detail: String,
    pub due_count: i64,
    pub session_start: Instant,

    // Cycle Fields
    pub cycle_seen_ids: Vec<String>,
    pub cycle_mistakes: std::collections::HashSet<String>,
    pub batch_counter: usize,
    pub total_cards_count: usize,

    // 新增：预估总轮数 (用于 UI 显示，例如 11)
    pub estimated_total_batches: usize,
    // 新增：明确的状态标记，不再只靠 batch_counter 猜
    pub is_review_phase: bool,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Db::new().await?);
        let due_count = db.get_count_due().await?;
        let total_cards_count = db.get_total_count().await?;

        // 计算预估轮次：例如 208 / 20 = 10.4 -> 11 轮
        let batch_size = 20.0;
        let estimated_total_batches = if total_cards_count > 0 {
            (total_cards_count as f64 / batch_size).ceil() as usize
        } else {
            1
        };

        Ok(Self {
            db,
            state: AppState::Dashboard,
            due_cards: Vec::new(),
            current_card_index: 0,
            user_input: String::new(),
            current_feedback: None,
            feedback_detail: String::new(),
            due_count,
            session_start: Instant::now(),
            cycle_seen_ids: Vec::new(),
            cycle_mistakes: std::collections::HashSet::new(),
            batch_counter: 0,
            total_cards_count,
            estimated_total_batches,
            is_review_phase: false,
        })
    }

    pub async fn start_quiz(&mut self) {
        // 重置循环状态
        self.cycle_seen_ids.clear();
        self.cycle_mistakes.clear();
        self.batch_counter = 1;
        self.is_review_phase = false;

        if let Ok(cards) = self.db.get_next_batch(&self.cycle_seen_ids).await {
            self.due_cards = cards;

            let ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
            self.cycle_seen_ids.extend(ids);

            self.current_card_index = 0;
            self.user_input.clear();
            self.current_feedback = None;
            self.feedback_detail.clear();

            if !self.due_cards.is_empty() {
                self.state = AppState::Quiz;
            } else {
                self.state = AppState::Dashboard;
            }
        }
    }

    #[allow(unused)]
    pub async fn submit_answer(&mut self) {
        // App::submit_answer 主要用于 TUI 或本地逻辑
        // API 模式下主要逻辑在 api.rs 中处理，但这里保留以防万一
        if self.current_card_index >= self.due_cards.len() {
            return;
        }

        let card = &self.due_cards[self.current_card_index];
        // 这里的逻辑主要被 API 端复用或替代
    }

    #[allow(unused)]
    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        // 检查当前批次是否做完
        if self.current_card_index >= self.due_cards.len() {

            // --- 核心逻辑修复：基于库存判断流转 ---

            // 1. 如果还在【新卡学习阶段】 (还没进入复习模式)
            if !self.is_review_phase {
                // 检查牌库里是否还有没见过的卡
                let total_seen = self.cycle_seen_ids.len();
                let has_unseen_cards = total_seen < self.total_cards_count;

                if has_unseen_cards {
                    // 【分支 A】还有生词：继续推下一组 (可能是第 10 轮，也可能是第 11 轮)
                    self.batch_counter += 1;
                    if let Ok(cards) = self.db.get_next_batch(&self.cycle_seen_ids).await {
                        // 只有当真的取到卡片时才更新
                        if !cards.is_empty() {
                            self.due_cards = cards;
                            let ids: Vec<String> = self.due_cards.iter().map(|c| c.id.clone()).collect();
                            self.cycle_seen_ids.extend(ids);
                            self.current_card_index = 0;
                            return;
                        }
                    }
                    // 如果代码走到这，说明数据库虽然理论上有卡但没取出来，防止死循环，进入复习
                }

                // 【分支 B】生词全看完了 (total_seen >= total_cards_count)
                // 此时准备进入复习模式
                if self.cycle_mistakes.is_empty() {
                    // 完美通关，没得复习 -> 直接开启下一轮大循环
                    self.start_quiz().await;
                } else {
                    // 进入复习模式
                    self.is_review_phase = true;
                    // 为了视觉上的区分，batch 计数器继续增加，表示“下一关”
                    self.batch_counter += 1;
                    self.load_review_batch().await;
                }
                self.current_card_index = 0;
                return;
            }

            // 2. 如果已经在【复习阶段】 (Review Phase)
            if self.is_review_phase {
                if self.cycle_mistakes.is_empty() {
                    // 债还完了 -> 重置大循环
                    self.start_quiz().await;
                } else {
                    // 还没还完 -> 无限惩罚轮
                    self.batch_counter += 1;
                    self.load_review_batch().await;
                }
                self.current_card_index = 0;
                return;
            }
        }
    }

    async fn load_review_batch(&mut self) {
        let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();
        if let Ok(cards) = self.db.get_specific_batch(&mistakes).await {
            self.due_cards = cards;
        }
    }

    #[allow(unused)]
    pub fn handle_input_char(&mut self, c: char) {
        self.user_input.push(c);
    }
}