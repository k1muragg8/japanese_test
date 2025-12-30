use leptos::*;
use leptos::logging::error;
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
    pub current_card_index: usize,
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
        <main style="display: flex; justify-content: center; align-items: center; height: 100vh; background-color: transparent;">
            <Quiz />
        </main>
    }
}

fn normalize_input(input: &str) -> String {
    let mut s = input.trim().to_lowercase().replace(" ", "");
    let replacements = [
        ("tsu", "tsu"), ("tu", "tsu"),
        ("shi", "shi"), ("si", "shi"),
        ("chi", "chi"), ("ti", "chi"),
        ("fu", "fu"),   ("hu", "fu"),
        ("ji", "ji"),   ("zi", "ji"), ("di", "ji"),
        ("zu", "zu"),   ("du", "zu"),
        ("sha", "sha"), ("sya", "sha"),
        ("shu", "shu"), ("syu", "shu"),
        ("sho", "sho"), ("syo", "sho"),
        ("cha", "cha"), ("tya", "cha"),
        ("chu", "chu"), ("tyu", "chu"),
        ("cho", "cho"), ("tyo", "cho"),
        ("ja", "ja"),   ("zya", "ja"), ("jya", "ja"),
        ("ju", "ju"),   ("zyu", "ju"), ("jyu", "ju"),
        ("jo", "jo"),   ("zyo", "jo"), ("jyo", "jo"),
    ];
    for (target, replacement) in replacements.iter() {
        if target != replacement { s = s.replace(target, replacement); }
    }
    s
}

#[component]
fn Quiz() -> impl IntoView {
    let (cards, set_cards) = create_signal(Vec::<Card>::new());
    let (current_index, set_current_index) = create_signal(0);
    let (user_input, set_user_input) = create_signal(String::new());
    let (feedback, set_feedback) = create_signal(Option::<(bool, String)>::None);
    let (loading, set_loading) = create_signal(true);
    let (error_msg, set_error_msg) = create_signal(Option::<String>::None);

    // 【新增】控制大小的信号
    let (font_size, set_font_size) = create_signal(2.0); // 默认假名大小
    let (card_width, set_card_width) = create_signal(160); // 默认卡片宽度

    let is_submitted = create_memo(move |_| feedback.get().is_some());
    let input_ref = create_node_ref::<Input>();

    let fetch_next_batch = move || {
        set_loading.set(true);
        set_error_msg.set(None);

        spawn_local(async move {
            let url = format!("/api/next_batch?t={}", js_sys::Date::now());
            let resp_res = Request::get(&url).send().await;

            match resp_res {
                Ok(resp) => {
                    if let Ok(batch_data) = resp.json::<BatchResponse>().await {
                        batch(move || {
                            set_cards.set(batch_data.cards);
                            set_feedback.set(None);
                            set_user_input.set(String::new());
                            set_current_index.set(0);
                            set_loading.set(false);
                        });
                    } else {
                        error!("Failed to parse batch response");
                        set_error_msg.set(Some("Error".to_string()));
                        set_loading.set(false);
                    }
                },
                Err(e) => {
                    error!("Network error: {:?}", e);
                    set_error_msg.set(Some("NetErr".to_string()));
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
        if current_cards.is_empty() { return; }

        let card = &current_cards[0];
        let input_val = user_input.get();

        let normalized_user_input = normalize_input(&input_val);
        let normalized_correct_romaji = normalize_input(&card.romaji);

        let is_correct = normalized_user_input == normalized_correct_romaji;
        let card_id = card.id.clone();
        let romaji = card.romaji.clone();

        spawn_local(async move {
            let _ = Request::post("/api/submit")
                .json(&SubmitRequest { card_id, correct: is_correct })
                .unwrap().send().await;

            if is_correct {
                set_feedback.set(Some((true, "".to_string())));
            } else {
                set_feedback.set(Some((false, format!("{}", romaji))));
            }
        });
    };

    let next_card = move || { fetch_next_batch(); };

    let handle_global_enter = window_event_listener(ev::keydown, move |ev| {
        if ev.key() == "Enter" {
            ev.prevent_default();
            if error_msg.get().is_some() { fetch_next_batch(); return; }
            if loading.get() { return; }
            if is_submitted.get() { next_card(); } else { submit_answer(); }
        }
    });
    on_cleanup(move || handle_global_enter.remove());

    view! {
        // 卡片容器，宽度由 card_width 信号控制
        <div class="card" style=move || format!("
            width: {}px;
            padding: 15px;
            background: #ffffff;
            border-radius: 12px;
            box-shadow: 0 4px 15px rgba(0,0,0,0.05);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            font-family: 'Segoe UI', sans-serif;
            border: 1px solid #f0f0f0;
            transition: width 0.2s;
        ", card_width.get())>

            // Loading
            {move || if loading.get() {
                view! { <div style="height: 40px; font-size: 14px; color: #999; display: flex; align-items: center;">"..."</div> }.into_view()
            } else {
                view! { <span style="display: none"></span> }.into_view()
            }}

            // Error
            {move || if let Some(msg) = error_msg.get() {
                view! { <div style="color: red; font-size: 12px; cursor: pointer;" on:click=move |_| fetch_next_batch()>{msg} " ↻"</div> }.into_view()
            } else {
                view! { <span style="display: none"></span> }.into_view()
            }}

            // Content
            {move || {
                let current_cards = cards.get();
                if current_cards.is_empty() {
                     view! { <div style="height: 40px;"></div> }.into_view()
                } else {
                    let card = current_cards.get(0).cloned().unwrap_or_else(|| current_cards[0].clone());
                    let is_sub = is_submitted.get();
                    let is_readonly = is_sub || loading.get() || error_msg.get().is_some();

                    let (kana_color, input_border) = match feedback.get() {
                        None => ("#333", "1px solid #eee"),
                        Some((true, _)) => ("#4caf50", "1px solid #4caf50"),
                        Some((false, _)) => ("#333", "1px solid #e57373"),
                    };

                    view! {
                        <div style="width: 100%; display: flex; flex-direction: column; align-items: center;">
                            // 假名显示区，大小由 font_size 控制
                            <div style=move || format!("font-size: {}rem; color: {}; font-weight: bold; margin-bottom: 5px; transition: color 0.2s;", font_size.get(), kana_color)>
                                {card.kana_char}
                            </div>

                            <input type="text"
                                prop:value=user_input
                                prop:readonly=is_readonly
                                node_ref=input_ref
                                on:input=move |ev| set_user_input.set(event_target_value(&ev))
                                style=format!("
                                    width: 100%;
                                    text-align: center;
                                    border: none;
                                    border-bottom: {};
                                    outline: none;
                                    font-size: 14px;
                                    padding: 4px;
                                    color: #555;
                                    background: transparent;
                                ", input_border)
                            />

                            <div style="height: 16px; margin-top: 5px; font-size: 12px; font-weight: bold;">
                                {move || match feedback.get() {
                                    Some((false, ans)) => view! {
                                        <span style="color: #e57373;">{"❌ "}{ans}</span>
                                    }.into_view(),
                                    Some((true, _)) => view! {
                                        <span style="color: #4caf50;">"✓"</span>
                                    }.into_view(),
                                    _ => view! { <span></span> }.into_view()
                                }}
                            </div>
                        </div>
                    }.into_view()
                }
            }}

            // 【新增】隐形控制栏 (鼠标悬停时显示)
            <div style="
                margin-top: 10px;
                width: 100%;
                opacity: 0.1;
                transition: opacity 0.3s;
                display: flex;
                flex-direction: column;
                gap: 5px;
                border-top: 1px dashed #f0f0f0;
                padding-top: 5px;
            "
            on:mouseenter=|el| { let _ = el.target().expect("el").unchecked_into::<web_sys::HtmlElement>().style().set_property("opacity", "1"); }
            on:mouseleave=|el| { let _ = el.target().expect("el").unchecked_into::<web_sys::HtmlElement>().style().set_property("opacity", "0.1"); }
            >
                <div style="display: flex; align-items: center; justify-content: space-between; font-size: 10px; color: #ccc;">
                    <span>"字"</span>
                    <input type="range" min="1.0" max="4.0" step="0.1"
                        prop:value=move || font_size.get()
                        on:input=move |ev| { let val = event_target_value(&ev).parse::<f64>().unwrap_or(2.0); set_font_size.set(val); }
                        style="width: 70%; cursor: pointer;"
                    />
                </div>
                <div style="display: flex; align-items: center; justify-content: space-between; font-size: 10px; color: #ccc;">
                    <span>"宽"</span>
                    <input type="range" min="120" max="400" step="10"
                        prop:value=move || card_width.get()
                        on:input=move |ev| { let val = event_target_value(&ev).parse::<i32>().unwrap_or(160); set_card_width.set(val); }
                        style="width: 70%; cursor: pointer;"
                    />
                </div>
            </div>
        </div>
    }
}

pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}