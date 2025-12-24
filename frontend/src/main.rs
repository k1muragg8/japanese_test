use leptos::*;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use leptos::html::Input;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub kana_char: String,
    pub romaji: String,
    pub stability: f64,
    pub difficulty: f64,
    pub last_review: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub batch_current: usize,
    pub batch_total: usize,
    pub remaining_in_deck: usize,
    pub is_review: bool,
    pub cycle_mistakes_count: usize,
    pub cards: Vec<Card>,
}

#[derive(Serialize)]
struct SubmitRequest {
    card_id: String,
    correct: bool,
}

#[derive(Deserialize)]
#[allow(unused)]
struct SubmitResponse {
    new_interval: i64,
}

#[component]
fn App() -> impl IntoView {
    view! {
        <main>
            <Quiz />
        </main>
    }
}

#[component]
fn Quiz() -> impl IntoView {
    let (cards, set_cards) = create_signal(Vec::<Card>::new());
    let (current_index, set_current_index) = create_signal(0);
    let (user_input, set_user_input) = create_signal(String::new());
    let (feedback, set_feedback) = create_signal(Option::<(bool, String)>::None);
    let (loading, set_loading) = create_signal(true);
    let (font_size, set_font_size) = create_signal(3.0);

    let (batch_current, set_batch_current) = create_signal(1);
    let (server_remaining_cards, set_server_remaining_cards) = create_signal(0);
    let (is_review_mode, set_is_review_mode) = create_signal(false);
    let (mistakes_count, set_mistakes_count) = create_signal(0);

    let current_batch_display = move || {
        format!("Batch {}", batch_current.get())
    };

    let is_submitted = create_memo(move |_| feedback.get().is_some());
    let input_ref = create_node_ref::<Input>();

    // --- 核心逻辑修改 1: fetch_next_batch 全权负责“新回合”的初始化 ---
    let fetch_next_batch = move || {
        set_loading.set(true);
        spawn_local(async move {
            let resp_res = Request::get("/api/next_batch").send().await;

            if let Ok(resp) = resp_res {
                if let Ok(batch_data) = resp.json::<BatchResponse>().await {
                    // 只有数据真正回来了，才重置所有状态
                    batch(move || {
                        set_cards.set(batch_data.cards);
                        set_batch_current.set(batch_data.batch_current);
                        set_server_remaining_cards.set(batch_data.remaining_in_deck);
                        set_is_review_mode.set(batch_data.is_review);
                        set_mistakes_count.set(batch_data.cycle_mistakes_count);

                        // 这里是关键：新卡来了，才把上一局的“结果”和“输入”清空
                        set_feedback.set(None);
                        set_user_input.set(String::new());
                        set_current_index.set(0);
                    });
                }
            }
            set_loading.set(false);
        });
    };

    create_effect(move |_| { fetch_next_batch(); });

    create_effect(move |_| {
        let _ = loading.get();
        let _ = current_index.get();
        let _ = feedback.get();
        if let Some(input) = input_ref.get() { let _ = input.focus(); }
    });

    create_effect(move |_| {
        if !is_submitted.get() {
            set_timeout(move || {
                if let Some(input) = input_ref.get() { let _ = input.focus(); }
            }, std::time::Duration::from_millis(10));
        }
    });

    let submit_answer = move || {
        let current_cards = cards.get();
        if current_index.get() >= current_cards.len() { return; }

        let card = &current_cards[current_index.get()];
        let input_val = user_input.get();
        let is_correct = input_val.trim().eq_ignore_ascii_case(&card.romaji);
        let card_id = card.id.clone();
        let romaji = card.romaji.clone();

        if is_correct && is_review_mode.get() {
            set_mistakes_count.update(|c| if *c > 0 { *c -= 1 });
        }

        // 提交时不全屏 loading，只锁输入框，体验更好
        // set_loading.set(true);

        spawn_local(async move {
            let _ = Request::post("/api/submit")
                .json(&SubmitRequest { card_id, correct: is_correct })
                .unwrap().send().await;

            // set_loading.set(false);

            if is_correct {
                set_feedback.set(Some((true, "Correct!".to_string())));
            } else {
                set_feedback.set(Some((false, format!("Ans: \"{}\"", romaji))));
            }
        });
    };

    // --- 核心逻辑修改 2: next_card 别急着清空 ---
    let next_card = move || {
        let next_idx = current_index.get() + 1;

        // 检查：是不是该去拿新卡了？
        if next_idx >= cards.get().len() {
            // 如果要去拿新卡，【什么状态都不要动】
            // 保持当前的 Feedback 显示，直接叫 Loading
            // 所有的重置工作交给 fetch_next_batch 的回调去处理
            fetch_next_batch();
        } else {
            // 只有确认是本地切换，才手动清空状态
            batch(move || {
                set_feedback.set(None);
                set_user_input.set(String::new());
                set_current_index.set(next_idx);
            });
        }
    };

    let handle_global_enter = window_event_listener(ev::keydown, move |ev| {
        if ev.key() == "Enter" {
            ev.prevent_default();
            if loading.get() { return; } // Loading 时禁止操作

            if is_submitted.get() {
                next_card();
            } else {
                submit_answer();
            }
        }
    });
    on_cleanup(move || handle_global_enter.remove());

    view! {
        <div class="app-container">
            {move || {
                let _current = batch_current.get();
                let is_review = is_review_mode.get();
                let local_remaining = cards.get().len().saturating_sub(current_index.get());

                let remaining_display = if is_review {
                    format!("{} Mistakes", local_remaining)
                } else {
                    let val = server_remaining_cards.get() + local_remaining;
                    format!("{} Cards", val)
                };

                let badge_style = if is_review { "background-color: #ed4956; color: white;" } else { "" };

                view! {
                    <div class="status-header">
                        <span class="cycle-badge" style=badge_style>{current_batch_display()}</span>
                        {move || match feedback.get() {
                            None => view! { <span></span> }.into_view(),
                            Some((is_correct, msg)) => {
                                let color = if is_correct { "#0095f6" } else { "#ed4956" };
                                view! {
                                    <span style=format!("color: {}; font-weight: bold; font-size: 14px;", color)>
                                        {msg}
                                    </span>
                                }.into_view()
                            }
                        }}
                        <span>{remaining_display}</span>
                    </div>
                }
            }}

            {move || {
                // 如果正在加载新 Batch (Loading且没卡，或者正在Fetch)
                // 这里做一个覆盖层，防止看到旧卡
                if loading.get() {
                    view! { <div class="card"><div class="kana-display" style="font-size: 2rem;">"Loading..."</div></div> }.into_view()
                } else {
                    let current_cards = cards.get();
                    if let Some(card) = current_cards.get(current_index.get()) {
                        let is_sub = is_submitted.get();
                        // 提交后或者 loading 时，输入框只读
                        let is_readonly = is_sub || loading.get();

                        view! {
                            <div class="card">
                                <div class="kana-display" style=move || format!("font-size: {}rem", font_size.get())>
                                    {card.kana_char.clone()}
                                </div>
                                <div style="width: 100%;">
                                    <input type="text" placeholder="Type Romaji..." prop:value=user_input prop:readonly=is_readonly node_ref=input_ref
                                        on:input=move |ev| set_user_input.set(event_target_value(&ev)) />

                                    <div style="margin-top: 20px; display: flex; align-items: center; justify-content: center; gap: 10px; opacity: 0.3; transition: opacity 0.3s;"
                                         on:mouseenter=|el| { let _ = el.target().expect("el").unchecked_into::<web_sys::HtmlElement>().style().set_property("opacity", "1"); }
                                         on:mouseleave=|el| { let _ = el.target().expect("el").unchecked_into::<web_sys::HtmlElement>().style().set_property("opacity", "0.3"); }>
                                        <span style="font-size: 10px;">"A"</span>
                                        <input type="range" min="0.5" max="6.0" step="0.1" prop:value=move || font_size.get()
                                            on:input=move |ev| { let val = event_target_value(&ev).parse::<f64>().unwrap_or(3.0); set_font_size.set(val); }
                                            style="width: 100px; cursor: pointer;" />
                                        <span style="font-size: 14px;">"A"</span>
                                    </div>
                                </div>
                            </div>
                        }.into_view()
                    } else {
                         view! { <div class="card"><div class="kana-display" style="font-size: 2rem;">"Syncing..."</div></div> }.into_view()
                    }
                }
            }}
        </div>
    }
}

pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}