use leptos::*;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use leptos::html::Input;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub kana_char: String,
    pub romaji: String,
    // FSRS Fields
    pub stability: f64,
    pub difficulty: f64,
    // Option<String> or specialized Date handling, but simple String or Option is safest for JSON if backend sends timestamp
    pub last_review: Option<String>,
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
        <main class="min-h-screen bg-gray-50 flex flex-col items-center justify-center p-4 font-sans">
            <Quiz />
        </main>
    }
}

#[component]
fn Quiz() -> impl IntoView {
    let (cards, set_cards) = create_signal(Vec::<Card>::new());
    let (current_index, set_current_index) = create_signal(0);
    let (user_input, set_user_input) = create_signal(String::new());
    let (feedback, set_feedback) = create_signal(Option::<(bool, String)>::None); // (is_correct, message)
    let (loading, set_loading) = create_signal(true);
    let is_submitted = create_memo(move |_| feedback.get().is_some());

    let input_ref = create_node_ref::<Input>();

    // Fetch cards on mount
    create_effect(move |_| {
        spawn_local(async move {
            let fetched_cards: Vec<Card> = Request::get("/api/next_batch")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            set_cards.set(fetched_cards);
            set_loading.set(false);
        });
    });

    // Aggressive Auto-Focus Effect (for user convenience only)
    create_effect(move |_| {
        let _ = loading.get();
        let _ = current_index.get();
        // Track feedback changes so we can potentially focus if needed
        let _ = feedback.get();

        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    // Aggressive Auto-Focus on State Change: When going back to typing mode
    create_effect(move |_| {
        // Track the submission state
        let submitted = is_submitted.get();

        // If we are back to typing mode (not submitted)
        if !submitted {
             // Small delay to ensure DOM update (feedback removed, input potentially re-rendered or just needs focus)
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

        spawn_local(async move {
            let _resp: SubmitResponse = Request::post("/api/submit")
                .json(&SubmitRequest { card_id, correct: is_correct })
                .unwrap()
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
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
             set_loading.set(true);
             set_current_index.set(0);
             spawn_local(async move {
                let fetched_cards: Vec<Card> = Request::get("/api/next_batch")
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                set_cards.set(fetched_cards);
                set_loading.set(false);
            });
        } else {
            set_current_index.set(next_idx);
        }
    };

    // Global Window Event Listener for "Infinite Enter" logic
    let handle_global_enter = window_event_listener(ev::keydown, move |ev| {
        if ev.key() == "Enter" {
            ev.prevent_default();

            if is_submitted.get() {
                // State B: Feedback shown -> Next Card
                next_card();
            } else {
                // State A: Typing -> Submit Answer
                submit_answer();
            }
        }
    });

    on_cleanup(move || handle_global_enter.remove());

    view! {
        <div class="w-full max-w-md">
            {move || {
                if loading.get() {
                    view! {
                        <div class="flex justify-center items-center h-64">
                            <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
                        </div>
                    }.into_view()
                } else {
                    let current_cards = cards.get();
                    if let Some(card) = current_cards.get(current_index.get()) {
                        let feedback_state = feedback.get();
                        let is_sub = is_submitted.get();

                        view! {
                            <div class="bg-white rounded-3xl shadow-sm border border-gray-100 p-8 flex flex-col items-center text-center transition-all duration-300">
                                // Kana Display
                                <h1 class="text-6xl font-bold text-gray-800 mb-8 select-none">
                                    {card.kana_char.clone()}
                                </h1>

                                <div class="w-full space-y-4">
                                    <input
                                        type="text"
                                        class="w-full bg-gray-100 text-gray-800 text-center text-xl rounded-full py-3 px-4 focus:outline-none focus:ring-0 transition-all placeholder-gray-400"
                                        placeholder="Type Romaji..."
                                        prop:value=user_input
                                        prop:readonly=is_sub
                                        node_ref=input_ref
                                        on:input=move |ev| set_user_input.set(event_target_value(&ev))
                                        // Removed on:keydown here; using global listener
                                    />

                                    // Visual Feedback & Hints (No Buttons)
                                    {move || match feedback_state.clone() {
                                        None => view! {
                                            <div class="text-sm text-gray-400 mt-4 h-8 animate-in fade-in duration-300">
                                                "Press Enter to Check"
                                            </div>
                                        }.into_view(),
                                        Some((is_correct, msg)) => {
                                            let text_color = if is_correct { "text-green-500" } else { "text-red-500" };
                                            view! {
                                                <div class="w-full space-y-4 animate-in fade-in zoom-in duration-200">
                                                    <div class={format!("text-xl font-bold {}", text_color)}>
                                                        {msg}
                                                    </div>
                                                    <div class="text-sm text-gray-400">
                                                        "Press Enter to Continue ↵"
                                                    </div>
                                                </div>
                                            }.into_view()
                                        }
                                    }}
                                </div>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="bg-white rounded-3xl shadow-sm border border-gray-100 p-8 text-center">
                                <p class="text-gray-500 text-xl">"All caught up!"</p>
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
