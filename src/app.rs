use std::time::Instant;
use crate::db::{Db, Card};
use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::thread_rng;

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
    pub deck_queue: Vec<String>,
    pub cycle_mistakes: std::collections::HashSet<String>,
    pub batch_counter: usize,
    pub total_cards_count: usize,
    pub estimated_total_batches: usize,
    pub is_review_phase: bool,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Db::new().await?);
        let due_count = db.get_count_due().await?;
        let total_cards_count = db.get_total_count().await?;

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
            deck_queue: Vec::new(),
            cycle_mistakes: std::collections::HashSet::new(),
            batch_counter: 0,
            total_cards_count,
            estimated_total_batches,
            is_review_phase: false,
        })
    }

    pub async fn start_quiz(&mut self) {
        self.cycle_mistakes.clear();
        self.batch_counter = 1;
        self.is_review_phase = false;

        if let Ok(mut all_ids) = self.db.get_all_ids().await {
            // 【关键修复】使用独立代码块，确保 rng 在 await 前被销毁
            {
                let mut rng = thread_rng();
                all_ids.shuffle(&mut rng);
            }
            // 出了这个花括号，rng 已经死透了，下面的 await 就是安全的

            self.deck_queue = all_ids;
            self.load_next_queue_batch().await;
        }
    }

    async fn load_next_queue_batch(&mut self) {
        // === 配置区域 ===
        // 这里设置难度：1 = 单字, 2 = 双字组合, 3 = 三字组合...
        const COMBO_SIZE: usize = 3;
        const BATCH_SIZE: usize = 20; // 每轮做多少道题
        // ================

        // 1. 计算我们需要从队列里取多少个原始 ID
        let ids_needed = BATCH_SIZE * COMBO_SIZE;
        let drain_count = std::cmp::min(ids_needed, self.deck_queue.len());

        // 2. 取出 ID 并去数据库查详情
        let batch_ids: Vec<String> = self.deck_queue.drain(0..drain_count).collect();

        if !batch_ids.is_empty() {
            if let Ok(raw_cards) = self.db.get_batch_by_ids(&batch_ids).await {

                // 3. 开始“缝合”逻辑
                let mut combo_cards = Vec::new();

                // chunks: 将原始卡片按 COMBO_SIZE 分组
                for chunk in raw_cards.chunks(COMBO_SIZE) {
                    if chunk.is_empty() { continue; }

                    // 拼接假名和罗马音
                    let mut merged_kana = String::new();
                    let mut merged_romaji = String::new();

                    // 使用第一张卡的 ID 作为这个组合的“代表 ID”
                    // (副作用：答错了只会降低第一张卡的评分，但这对于随机练习来说可以接受)
                    let primary_id = chunk[0].id.clone();
                    let primary_stability = chunk[0].stability;
                    let primary_difficulty = chunk[0].difficulty;

                    for card in chunk {
                        merged_kana.push_str(&card.kana_char);
                        merged_romaji.push_str(&card.romaji);
                    }

                    // 创建一个“虚拟卡片”
                    let virtual_card = Card {
                        id: primary_id,
                        kana_char: merged_kana,
                        romaji: merged_romaji,
                        stability: primary_stability,
                        difficulty: primary_difficulty,
                        last_review: None,
                    };

                    combo_cards.push(virtual_card);
                }

                // 4. 将缝合好的卡片放入待办列表
                self.due_cards = combo_cards;
                self.current_card_index = 0;
                self.state = AppState::Quiz;
            }
        }
    }

    #[allow(unused)]
    async fn load_review_batch(&mut self) {
        let mistakes: Vec<String> = self.cycle_mistakes.iter().cloned().collect();
        if let Ok(cards) = self.db.get_batch_by_ids(&mistakes).await {
            self.due_cards = cards;
            self.current_card_index = 0;
        }
    }

    #[allow(unused)]
    pub async fn next_card(&mut self) {
        self.current_card_index += 1;
        self.user_input.clear();
        self.current_feedback = None;
        self.feedback_detail.clear();

        // 检查当前批次是否做完
        if self.current_card_index >= self.due_cards.len() {

            // 逻辑简化：不再进入复习模式
            // 1. 如果队列里还有牌，继续发下一批“缝合怪”
            if !self.deck_queue.is_empty() {
                self.batch_counter += 1;
                self.load_next_queue_batch().await;
            } else {
                // 2. 队列空了，直接重新洗牌，开始新的一轮
                // (错题本 cycle_mistakes 虽然在 api.rs 里还在记录，但我们这里直接无视它)
                self.start_quiz().await;
            }
        }
    }
}