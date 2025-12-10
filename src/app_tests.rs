use super::*;

#[test]
fn test_new_app_state() {
    let app = App::new(None);
    assert!(matches!(app.current_screen, CurrentScreen::Menu));
    assert_eq!(app.score, 0);
    assert_eq!(app.menu_selection, 0);
}

#[test]
fn test_start_quiz() {
    let mut app = App::new(None);
    app.menu_selection = 1; // Select Hiragana (0 is Dashboard)
    app.start_quiz();
    assert!(matches!(app.current_screen, CurrentScreen::Quiz));
    assert!(app.current_kana.is_some());
    assert_eq!(app.score, 0);
}

#[test]
fn test_correct_answer() {
    let mut app = App::new(None);
    app.menu_selection = 1; // Select Hiragana
    app.start_quiz();
    let kana = app.current_kana.as_ref().unwrap().clone();
    app.user_input = kana.romaji[0].clone();
    app.check_answer();

    assert_eq!(app.feedback, Some(true));
    assert_eq!(app.score, 1);
    assert_eq!(app.streak, 1);
}

#[test]
fn test_incorrect_answer() {
    let mut app = App::new(None);
    app.menu_selection = 1; // Select Hiragana
    app.start_quiz();
    app.user_input = "wrong".to_string();
    app.check_answer();

    assert_eq!(app.feedback, Some(false));
    assert_eq!(app.score, 0);
    assert_eq!(app.streak, 0);
}

#[test]
fn test_menu_navigation() {
    let mut app = App::new(None);

    // Down from 0 -> 1
    app.handle_key_event(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Down));
    assert_eq!(app.menu_selection, 1);

    // Down from 1 -> 2
    app.handle_key_event(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Down));
    assert_eq!(app.menu_selection, 2);

    // Down from 2 -> 3
    app.handle_key_event(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Down));
    assert_eq!(app.menu_selection, 3);

    // Down from 3 -> 0 (wrap)
    app.handle_key_event(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Down));
    assert_eq!(app.menu_selection, 0);

    // Up from 0 -> 3 (wrap)
    app.handle_key_event(crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Up));
    assert_eq!(app.menu_selection, 3);
}
