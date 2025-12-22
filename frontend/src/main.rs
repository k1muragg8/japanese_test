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

    // Stealth Mode: Font Size Slider (Default 3.0rem)
    let (font_size, set_font_size) = create_signal(3.0);

    // Cycle Status Signals
    let (batch_current, set_batch_current) = create_signal(1);
    let (remaining_cards, set_remaining_cards) = create_signal(0);
    let (_mistakes_count, set_mistakes_count) = create_signal(0);

    let is_submitted = create_memo(move |_| feedback.get().is_some());
    let input_ref = create_node_ref::<Input>();

    // Fetch Function
    let fetch_next_batch = move || {
        set_loading.set(true);
        spawn_local(async move {
            let resp_res = Request::get("/api/next_batch")
                .send()
                .await;

            if let Ok(resp) = resp_res {
                 if let Ok(batch_data) = resp.json::<BatchResponse>().await {
                     set_cards.set(batch_data.cards);
                     set_batch_current.set(batch_data.batch_current);
                     set_remaining_cards.set(batch_data.remaining_in_deck);

                     // If batch_current == 1, it means reset happened, clear local mistakes
                     if batch_data.batch_current == 1 {
                         set_mistakes_count.set(0);
                     }

                     set_current_index.set(0);
                 }
            }
            set_loading.set(false);
        });
    };

    // Initial Fetch
    create_effect(move |_| {
        fetch_next_batch();
    });

    // Auto-Focus Effects
    create_effect(move |_| {
        let _ = loading.get();
        let _ = current_index.get();
        let _ = feedback.get();
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    create_effect(move |_| {
        if !is_submitted.get() {
             set_timeout(move || {
                 if let Some(input) = input_ref.get() {
                     let _ = input.focus();
                 }
             }, std::time::Duration::from_millis(10));
        }
    });

    let submit_answer = move || {
        let current_cards = cards.get();
        if current_index.get() >= current_cards.len() {
            return;
        }

        let card = &current_cards[current_index.get()];
        let input_val = user_input.get();
        let is_correct = input_val.trim().eq_ignore_ascii_case(&card.romaji);
        let card_id = card.id.clone();

        if !is_correct {
            set_mistakes_count.update(|c| *c += 1);
        }

        spawn_local(async move {
            let _ = Request::post("/api/submit")
                .json(&SubmitRequest { card_id, correct: is_correct })
                .unwrap()
                .send()
                .await;
        });

        if is_correct {
            set_feedback.set(Some((true, "Correct!".to_string())));
        } else {
            set_feedback.set(Some((false, format!("The answer is \"{}\"", card.romaji))));
        }
    };

    let next_card = move || {
        set_feedback.set(None);
        set_user_input.set(String::new());
        let next_idx = current_index.get() + 1;

        if next_idx >= cards.get().len() {
             fetch_next_batch();
        } else {
            set_current_index.set(next_idx);
        }
    };

    let handle_global_enter = window_event_listener(ev::keydown, move |ev| {
        if ev.key() == "Enter" {
            ev.prevent_default();
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
            // Header
            {move || {
                let current = batch_current.get();
                let is_review = current == 11;
                let review_mode_alert = if is_review {
                    view! { <div class="review-mode">"Reviewing Mistakes"</div> }.into_view()
                } else {
                    view! { }.into_view()
                };

                view! {
                    <div class="status-header">
                        <span class="cycle-badge">{format!("Batch {}/10", if is_review { 10 } else { current })}</span>
                        <span>{format!("{} Cards", remaining_cards.get())}</span>
                    </div>
                    {review_mode_alert}
                }
            }}

            // Main Card
            {move || {
                if loading.get() {
                    view! {
                        <div class="card">
                            <div class="kana-display" style="font-size: 2rem;">"Loading..."</div>
                        </div>
                    }.into_view()
                } else {
                    let current_cards = cards.get();
                    if let Some(card) = current_cards.get(current_index.get()) {
                        let feedback_state = feedback.get();
                        let is_sub = is_submitted.get();

                        view! {
                            <div class="card">
                                // Kana Display with Dynamic Font Size
                                <div
                                    class="kana-display"
                                    style=move || format!("font-size: {}rem", font_size.get())
                                >
                                    {card.kana_char.clone()}
                                </div>

                                <div style="width: 100%;">
                                    <input
                                        type="text"
                                        placeholder="Type Romaji..."
                                        prop:value=user_input
                                        prop:readonly=is_sub
                                        node_ref=input_ref
                                        on:input=move |ev| set_user_input.set(event_target_value(&ev))
                                    />

                                    // --- NEW: Stealth Slider ---
                                    <div style="margin-top: 20px; display: flex; align-items: center; justify-content: center; gap: 10px; opacity: 0.3; transition: opacity 0.3s;"
                                         on:mouseenter=|el| { let _ = el.target().expect("el").unchecked_into::<web_sys::HtmlElement>().style().set_property("opacity", "1"); }
                                         on:mouseleave=|el| { let _ = el.target().expect("el").unchecked_into::<web_sys::HtmlElement>().style().set_property("opacity", "0.3"); }
                                    >
                                        <span style="font-size: 10px;">"A"</span>
                                        <input
                                            type="range"
                                            min="0.5"
                                            max="6.0"
                                            step="0.1"
                                            prop:value=move || font_size.get()
                                            on:input=move |ev| {
                                                let val = event_target_value(&ev).parse::<f64>().unwrap_or(3.0);
                                                set_font_size.set(val);
                                            }
                                            style="width: 100px; cursor: pointer;"
                                        />
                                        <span style="font-size: 14px;">"A"</span>
                                    </div>
                                    // ---------------------------

                                    {move || match feedback_state.clone() {
                                        None => view! { }.into_view(),
                                        Some((is_correct, msg)) => {
                                            let feedback_class = if is_correct { "feedback success" } else { "feedback error" };
                                            view! {
                                                <div class={feedback_class}>
                                                    {msg}
                                                </div>
                                            }.into_view()
                                        }
                                    }}
                                </div>
                            </div>
                        }.into_view()
                    } else {
                         view! {
                            <div class="card">
                                <div class="kana-display" style="font-size: 2rem;">"Cycle Complete"</div>
                            </div>
                        }.into_view()
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
