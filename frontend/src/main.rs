use leptos::*;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use leptos::html::{Button, Input};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub kana_char: String,
    pub romaji: String,
    pub interval: i64,
    pub easiness: f64,
    pub repetitions: i64,
}

#[derive(Serialize)]
struct SubmitRequest {
    card_id: String,
    correct: bool,
}

#[derive(Deserialize)]
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

    let input_ref = create_node_ref::<Input>();
    let next_button_ref = create_node_ref::<Button>();

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

    // Focus Management Effect
    create_effect(move |_| {
        if loading.get() {
            return;
        }

        if feedback.get().is_some() {
            // Feedback Mode: Focus Next Button
            // Using request_animation_frame or minimal delay can sometimes help if DOM isn't ready,
            // but Leptos effects usually run after render.
            if let Some(btn) = next_button_ref.get() {
                let _ = btn.focus();
            }
        } else {
            // Input Mode: Focus Input
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        }
    });

    let submit_answer = move || {
        let current_cards = cards.get();
        if current_index.get() >= current_cards.len() {
            return;
        }

        let card = &current_cards[current_index.get()];
        let is_correct = user_input.get().trim().eq_ignore_ascii_case(&card.romaji);

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
            set_feedback.set(Some((false, format!("Wrong. It was '{}'", card.romaji))));
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

                        view! {
                            <div class="bg-white rounded-3xl shadow-sm border border-gray-100 p-8 flex flex-col items-center text-center transition-all duration-300">
                                // Kana Display
                                <h1 class="text-6xl font-bold text-gray-800 mb-8 select-none">
                                    {card.kana_char.clone()}
                                </h1>

                                {move || match feedback_state.clone() {
                                    None => view! {
                                        <div class="w-full space-y-4">
                                            <input
                                                type="text"
                                                class="w-full bg-gray-100 text-gray-800 text-center text-xl rounded-full py-3 px-4 focus:outline-none focus:ring-2 focus:ring-blue-100 transition-all placeholder-gray-400"
                                                placeholder="Type Romaji..."
                                                prop:value=user_input
                                                node_ref=input_ref
                                                on:input=move |ev| set_user_input.set(event_target_value(&ev))
                                                on:keydown=move |ev| {
                                                    if ev.key() == "Enter" {
                                                        submit_answer();
                                                    }
                                                }
                                            />
                                            // Visual Submit Button (Enter also works)
                                            <button
                                                on:click=move |_| submit_answer()
                                                class="w-full bg-blue-500 hover:bg-blue-600 text-white font-semibold rounded-lg py-3 px-4 transition-colors shadow-md active:scale-95 transform duration-100"
                                            >
                                                "Check"
                                            </button>
                                        </div>
                                    }.into_view(),
                                    Some((is_correct, msg)) => {
                                        let text_color = if is_correct { "text-green-500" } else { "text-red-500" };
                                        view! {
                                            <div class="w-full space-y-6 animate-in fade-in zoom-in duration-200">
                                                <div class={format!("text-2xl font-bold {}", text_color)}>
                                                    {msg}
                                                </div>
                                                <button
                                                    node_ref=next_button_ref
                                                    on:click=move |_| next_card()
                                                    class="w-full bg-blue-500 hover:bg-blue-600 text-white font-semibold rounded-lg py-3 px-4 transition-colors shadow-md outline-none ring-2 ring-offset-2 ring-blue-500 active:scale-95 transform duration-100"
                                                >
                                                    "Next"
                                                </button>
                                            </div>
                                        }.into_view()
                                    }
                                }}
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
