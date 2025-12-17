use leptos::*;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use leptos::html::Button;

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
        <main class="container">
            <Quiz />
        </main>
    }
}

#[component]
fn Quiz() -> impl IntoView {
    let (cards, set_cards) = create_signal(Vec::<Card>::new());
    let (current_index, set_current_index) = create_signal(0);
    let (user_input, set_user_input) = create_signal(String::new());
    let (feedback, set_feedback) = create_signal(Option::<String>::None);
    let (loading, set_loading) = create_signal(true);

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

    // Focus effect when feedback appears
    create_effect(move |_| {
        if feedback.get().is_some() {
            // Slight delay might be needed for the DOM to update if conditional rendering is involved
            // but often create_effect runs after render.
            // In Leptos, effects run after rendering.
            if let Some(btn) = next_button_ref.get() {
                let _ = btn.focus();
            }
        }
    });

    let submit_answer = move |_| {
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
            set_feedback.set(Some("Correct!".to_string()));
        } else {
            set_feedback.set(Some(format!("Incorrect. Answer was: {}", card.romaji)));
        }
    };

    let next_card = move |_| {
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
        <div style="text-align: center; padding: 20px;">
            {move || {
                if loading.get() {
                    view! { <p>"Loading..."</p> }.into_view()
                } else {
                    let current_cards = cards.get();
                    if let Some(card) = current_cards.get(current_index.get()) {
                        view! {
                            <div>
                                <h1 style="font-size: 4em; margin-bottom: 20px;">{card.kana_char.clone()}</h1>

                                {move || match feedback.get() {
                                    None => view! {
                                        <div>
                                            <input
                                                type="text"
                                                on:input=move |ev| set_user_input.set(event_target_value(&ev))
                                                prop:value=user_input
                                                style="font-size: 1.5em; padding: 5px;"
                                                on:keydown=move |ev| {
                                                    if ev.key() == "Enter" {
                                                        submit_answer(());
                                                    }
                                                }
                                                autofocus
                                            />
                                            <button
                                                on:click=move |_| submit_answer(())
                                                style="font-size: 1.5em; padding: 5px 10px; margin-left: 10px;"
                                            >
                                                "Submit"
                                            </button>
                                        </div>
                                    }.into_view(),
                                    Some(msg) => view! {
                                        <div>
                                            <p style="font-size: 1.5em; color: blue;">{msg}</p>
                                            <button
                                                on:click=move |_| next_card(())
                                                style="font-size: 1.5em; padding: 5px 10px; margin-top: 10px;"
                                                node_ref=next_button_ref
                                                on:keydown=move |ev| {
                                                     if ev.key() == "Enter" {
                                                         next_card(());
                                                     }
                                                 }
                                            >
                                                "Next"
                                            </button>
                                        </div>
                                    }.into_view()
                                }}
                            </div>
                        }.into_view()
                    } else {
                        view! { <p>"No cards available."</p> }.into_view()
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
