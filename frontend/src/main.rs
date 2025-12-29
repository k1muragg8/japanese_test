use leptos::*;
use leptos::logging::error;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use leptos::html::Input;
use wasm_bindgen::JsCast;

// ... (结构体定义保持不变)
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
    // 新增一个错误状态，用于显示重试按钮
    let (error_msg, set_error_msg) = create_signal(Option::<String>::None);

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
        set_error_msg.set(None); // 清除错误

        spawn_local(async move {
            // 【关键修复 1】添加随机时间戳，防止浏览器缓存 GET 请求
            let url = format!("/api/next_batch?t={}", js_sys::Date::now());
            let resp_res = Request::get(&url).send().await;

            match resp_res {
                Ok(resp) => {
                    if let Ok(batch_data) = resp.json::<BatchResponse>().await {
                        // 成功获取数据
                        batch(move || {
                            set_cards.set(batch_data.cards);
                            set_batch_current.set(batch_data.batch_current);
                            set_server_remaining_cards.set(batch_data.remaining_in_deck);
                            set_is_review_mode.set(batch_data.is_review);
                            set_mistakes_count.set(batch_data.cycle_mistakes_count);

                            set_feedback.set(None);
                            set_user_input.set(String::new());
                            set_current_index.set(0);

                            set_loading.set(false);
                        });
                    } else {
                        error!("Failed to parse batch response");
                        // 【关键修复 2】解析失败时，不要简单的关闭 loading，而是显示错误状态
                        // 这样用户就看不到底下的旧卡片了
                        set_error_msg.set(Some("数据解析失败，请点击重试".to_string()));
                        set_loading.set(false);
                    }
                },
                Err(e) => {
                    error!("Network error: {:?}", e);
                    set_error_msg.set(Some("网络连接错误，请点击重试".to_string()));
                    set_loading.set(false);
                }
            }
        });
    };

    create_effect(move |_| { fetch_next_batch(); });

    create_effect(move |_| {
        let _ = loading.get();
        let _ = current_index.get();
        let _ = feedback.get();
        if let Some(input) = input_ref.get() {
            if !loading.get() && error_msg.get().is_none() {
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
            // 如果有错误，回车键触发重试
            if error_msg.get().is_some() {
                fetch_next_batch();
                return;
            }
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

            <div class="card" style="position: relative; min-height: 200px;">

                // 1. Loading 遮罩层 (最高优先级)
                {move || if loading.get() {
                    view! {
                        <div style="
                            position: absolute;
                            top: 0; left: 0; right: 0; bottom: 0;
                            background: rgba(255, 255, 255, 0.95);
                            display: flex;
                            justify-content: center;
                            align-items: center;
                            z-index: 20;
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

                // 2. Error 遮罩层 (次高优先级，覆盖卡片)
                {move || if let Some(msg) = error_msg.get() {
                    view! {
                        <div style="
                            position: absolute;
                            top: 0; left: 0; right: 0; bottom: 0;
                            background: rgba(255, 200, 200, 0.95);
                            display: flex;
                            flex-direction: column;
                            justify-content: center;
                            align-items: center;
                            z-index: 15;
                            border-radius: 8px;
                            color: #d32f2f;
                            gap: 10px;
                        ">
                            <span style="font-weight: bold;">{msg}</span>
                            <button
                                on:click=move |_| fetch_next_batch()
                                style="padding: 5px 15px; cursor: pointer;">
                                "Retry"
                            </button>
                        </div>
                    }.into_view()
                } else {
                    view! { <span style="display: none"></span> }.into_view()
                }}

                // 3. 卡片内容层
                {move || {
                    let current_cards = cards.get();
                    if current_cards.is_empty() {
                         view! { <div style="height: 150px;"></div> }.into_view()
                    } else {
                        let card = current_cards.get(current_index.get()).cloned().unwrap_or_else(|| current_cards[0].clone());
                        let is_sub = is_submitted.get();
                        // 如果有 loading 或者 error，禁用输入
                        let is_readonly = is_sub || loading.get() || error_msg.get().is_some();

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