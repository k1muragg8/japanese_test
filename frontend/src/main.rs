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
    #[allow(unused)]
    let (mistakes_count, set_mistakes_count) = create_signal(0);

    let current_batch_display = move || {
        format!("Batch {}", batch_current.get())
    };

    let is_submitted = create_memo(move |_| feedback.get().is_some());
    let input_ref = create_node_ref::<Input>();

    // 获取下一批数据
    let fetch_next_batch = move || {
        set_loading.set(true);
        spawn_local(async move {
            let resp_res = Request::get("/api/next_batch").send().await;

            if let Ok(resp) = resp_res {
                if let Ok(batch_data) = resp.json::<BatchResponse>().await {
                    // 使用 batch 确保原子更新，避免闪烁
                    batch(move || {
                        set_cards.set(batch_data.cards);
                        set_batch_current.set(batch_data.batch_current);
                        set_server_remaining_cards.set(batch_data.remaining_in_deck);
                        set_is_review_mode.set(batch_data.is_review);
                        set_mistakes_count.set(batch_data.cycle_mistakes_count);

                        // 重置状态
                        set_feedback.set(None);
                        set_user_input.set(String::new());
                        set_current_index.set(0);

                        // 【关键】数据更新完毕后，直接在这里关闭 loading
                        // 这样“新卡显示”和“遮罩消失”是同时发生的，视觉最丝滑
                        set_loading.set(false);
                    });
                    return;
                }
            }
            // 如果出错，也要关闭 loading
            set_loading.set(false);
        });
    };

    create_effect(move |_| { fetch_next_batch(); });

    // 自动聚焦逻辑
    create_effect(move |_| {
        let _ = loading.get();
        let _ = current_index.get();
        let _ = feedback.get();
        if let Some(input) = input_ref.get() {
            // 只有当不处于 loading 状态时才强制聚焦，避免键盘跳出跳回
            if !loading.get() {
                let _ = input.focus();
            }
        }
    });

    create_effect(move |_| {
        if !is_submitted.get() && !loading.get() {
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

        spawn_local(async move {
            let _ = Request::post("/api/submit")
                .json(&SubmitRequest { card_id, correct: is_correct })
                .unwrap().send().await;

            if is_correct {
                set_feedback.set(Some((true, "Correct!".to_string())));
            } else {
                set_feedback.set(Some((false, format!("Ans: \"{}\"", romaji))));
            }
        });
    };

    let next_card = move || {
        let next_idx = current_index.get() + 1;

        if next_idx >= cards.get().len() {
            // 到底了，去取新数据。保持当前界面不动，只显示 Loading 遮罩
            fetch_next_batch();
        } else {
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
            if loading.get() { return; }

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

                let badge_style = "";

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

            // 【UI 结构优化】
            // 不再使用 if loading 替换整个 div，而是保持 div.card 结构稳定
            <div class="card" style="position: relative; min-height: 200px;">

                // 1. Loading 遮罩层 (绝对定位，盖在卡片上面)
                {move || if loading.get() {
                    view! {
                        <div style="
                            position: absolute;
                            top: 0; left: 0; right: 0; bottom: 0;
                            background: rgba(255, 255, 255, 0.9);
                            display: flex;
                            justify-content: center;
                            align-items: center;
                            z-index: 10;
                            border-radius: 8px;
                            font-size: 1.5rem;
                            color: #666;
                        ">
                            "Loading..."
                        </div>
                    }.into_view()
                } else {
                    view! { <span style="display: none"></span> }.into_view()
                }}

                // 2. 卡片内容层 (始终渲染，但在 loading 时会被上面的遮罩挡住)
                {move || {
                    let current_cards = cards.get();
                    // 安全检查：如果数据还没回来（空数组），显示空占位
                    if current_cards.is_empty() {
                         view! { <div style="height: 150px;"></div> }.into_view()
                    } else {
                        // 如果有数据，显示当前卡片（即使是旧的，也被遮罩挡住了，用户看不见）
                        // 使用 unwrap_or 保证安全
                        let card = current_cards.get(current_index.get()).cloned().unwrap_or_else(|| current_cards[0].clone());
                        let is_sub = is_submitted.get();
                        let is_readonly = is_sub || loading.get();

                        view! {
                            <div>
                                <div class="kana-display" style=move || format!("font-size: {}rem", font_size.get())>
                                    {card.kana_char}
                                </div>
                                <div style="width: 100%;">
                                    <input type="text" placeholder="Type Romaji..."
                                        prop:value=user_input
                                        prop:readonly=is_readonly
                                        node_ref=input_ref
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
                    }
                }}
            </div>
        </div>
    }
}

pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}