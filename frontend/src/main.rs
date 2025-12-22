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
    let (feedback, set_feedback) = create_signal(Option::<(bool, String)>::None); // (is_correct, message)
    let (loading, set_loading) = create_signal(true);
    let is_submitted = create_memo(move |_| feedback.get().is_some());

    let input_ref = create_node_ref::<Input>();

    // Fetch cards on mount
    create_effect(move |_| {
        spawn_local(async move {
            // Simulate initialization delay for effect
            // set_timeout(|| {}, std::time::Duration::from_millis(500));

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

    // Aggressive Auto-Focus
    create_effect(move |_| {
        let _ = loading.get();
        let _ = current_index.get();
        let _ = feedback.get();

        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    create_effect(move |_| {
        let submitted = is_submitted.get();
        if !submitted {
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
            set_feedback.set(Some((true, format!("[SUCCESS] Hash matched. Uploading to database..."))));
        } else {
            set_feedback.set(Some((false, format!("[ERROR] Checksum mismatch. Expected: '{}'. Retrying...", card.romaji))));
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

    // Global Window Event Listener for "Infinite Enter"
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
        <div>
            // Header / Init Sequence
            <div class="log-line system-msg">"root@kana-server:~# ./init_sequence.sh"</div>
            <div class="log-line system-msg">"[SYSTEM] Initializing memory buffers..."</div>

            {move || {
                if loading.get() {
                    view! {
                         <div class="log-line system-msg">"[SYSTEM] Connecting to remote endpoint... <span class='cursor'></span>"</div>
                    }.into_view()
                } else {
                     let current_cards = cards.get();
                     // Count total cards loaded vs index
                     let total = current_cards.len();
                     let idx = current_index.get();

                     if let Some(card) = current_cards.get(idx) {
                        let feedback_state = feedback.get();
                        let is_sub = is_submitted.get();

                        view! {
                            <div class="log-line system-msg">
                                {format!("[SYSTEM] Loaded batch... {} entities found.", total)}
                            </div>
                            <div class="log-line system-msg">
                                {format!("[SYSTEM] Processing entity [{}/{}]...", idx + 1, total)}
                            </div>

                            // The "Card" is now a log entry
                            <div class="log-line">
                                <span class="highlight">"> INCOMING_PACKET: "</span>
                                <span class="highlight text-white" style="font-size: 1.2em;">" [ " {card.kana_char.clone()} " ] "</span>
                                <span class="system-msg">" type: kana size: 8b"</span>
                            </div>

                            // Input Area
                            <div class="log-line" style="margin-top: 10px;">
                                <span class="prompt">"root@worker:~/answer$"</span>
                                <input
                                    type="text"
                                    prop:value=user_input
                                    prop:readonly=is_sub
                                    node_ref=input_ref
                                    on:input=move |ev| set_user_input.set(event_target_value(&ev))
                                    autocomplete="off"
                                    spellcheck="false"
                                />
                                // Blinking cursor if focusing input isn't enough visual cue,
                                // but the input caret is usually visible.
                                // Let's add a fake block cursor if input is empty?
                                // Actually standard input caret is fine for 'terminal'.
                            </div>

                            // Feedback Section
                            {move || match feedback_state.clone() {
                                None => view! {
                                    <div class="log-line system-msg" style="margin-top: 5px; opacity: 0.5;">
                                        "[WAITING FOR INPUT...]"
                                    </div>
                                }.into_view(),
                                Some((is_correct, msg)) => {
                                    let msg_class = if is_correct { "log-line success-msg" } else { "log-line error-msg" };
                                    view! {
                                        <div class={msg_class} style="margin-top: 5px;">
                                            {msg}
                                        </div>
                                        <div class="log-line system-msg">
                                            "[SYSTEM] Press Enter to continue..."
                                        </div>
                                    }.into_view()
                                }
                            }}
                        }.into_view()
                     } else {
                        view! {
                            <div class="log-line system-msg">"[SYSTEM] Batch processing complete."</div>
                            <div class="log-line highlight">"root@kana-server:~# shutdown -h now"</div>
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
