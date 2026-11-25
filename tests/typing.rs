//! What the text input does besides submitting commands.

mod common;

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputFocus,
    prelude::*,
    text::EditableText,
    window::PrimaryWindow,
};
use bevy_console::prelude::*;
use common::*;

#[derive(Reflect)]
enum Team {
    Red,
    Blue,
}

fn join(In((_, _)): In<(Team, u32)>) {}

fn app_with(settings: ConsoleSettings) -> App {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), ConsolePlugin))
        .insert_resource(settings)
        .add_commands((join, ("jump", || {})));
    start(&mut app);
    app
}

fn app() -> App {
    app_with(ConsoleSettings::default())
}

fn press_key(app: &mut App, code: KeyCode, key: Key) {
    press(app, code, key, None);
}

fn up(app: &mut App) {
    press_key(app, KeyCode::ArrowUp, Key::ArrowUp);
}

fn down(app: &mut App) {
    press_key(app, KeyCode::ArrowDown, Key::ArrowDown);
}

fn tab(app: &mut App) {
    press_key(app, KeyCode::Tab, Key::Tab);
}

fn shift_tab(app: &mut App) {
    set_key(app, KeyCode::ShiftLeft, Key::Shift, ButtonState::Pressed);
    tab(app);
    set_key(app, KeyCode::ShiftLeft, Key::Shift, ButtonState::Released);
}

fn escape(app: &mut App) {
    press_key(app, KeyCode::Escape, Key::Escape);
}

fn press_backquote(app: &mut App) {
    press(
        app,
        KeyCode::Backquote,
        Key::Character("`".into()),
        Some("`"),
    );
}

fn set_key(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: None,
        repeat: false,
        window,
    });
    app.update();
}

fn is_focused(app: &mut App) -> bool {
    let input = text_input(app);
    app.world().resource::<InputFocus>().get() == Some(input)
}

fn is_open(app: &mut App) -> bool {
    visibility(app) != Visibility::Hidden
}

/// The text below the text input, or nothing if it takes no space.
fn suggestions(app: &mut App) -> String {
    app.update();
    let world = app.world_mut();
    let (list, node) = world
        .query_filtered::<(Entity, &Node), With<SuggestionList>>()
        .single(world)
        .unwrap();
    let hidden = node.display == Display::None;
    let spans: Vec<Entity> = world
        .get::<Children>(list)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    let text: String = spans
        .into_iter()
        .map(|span| world.get::<TextSpan>(span).unwrap().0.clone())
        .collect();
    assert_eq!(hidden, text.is_empty());
    text
}

#[test]
fn history_is_recalled() {
    let mut app = app();
    for entry in ["jump 1", "  jump 2 ", "jump 2", "   "] {
        submit(&mut app, entry);
    }
    // Also removes the spaces that were submitted last.
    assert_eq!(text_input_value(&mut app), "");

    type_text(&mut app, "draft");
    up(&mut app);
    assert_eq!(text_input_value(&mut app), "jump 2");
    up(&mut app);
    assert_eq!(text_input_value(&mut app), "jump 1");
    up(&mut app);
    assert_eq!(text_input_value(&mut app), "jump 1");
    down(&mut app);
    assert_eq!(text_input_value(&mut app), "jump 2");

    // The cursor is at the end.
    type_text(&mut app, "!");
    assert_eq!(text_input_value(&mut app), "jump 2!");
    up(&mut app);
    down(&mut app);
    assert_eq!(text_input_value(&mut app), "jump 2!");
}

#[test]
fn the_draft_comes_back_even_if_empty() {
    let mut app = app();
    submit(&mut app, "jump");
    up(&mut app);
    assert_eq!(text_input_value(&mut app), "jump");
    down(&mut app);
    assert_eq!(text_input_value(&mut app), "");
}

#[test]
fn tab_completes() {
    let mut app = app();
    focus_text_input(&mut app);
    type_text(&mut app, "j");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "join");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "jump");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "join");
    shift_tab(&mut app);
    shift_tab(&mut app);
    assert_eq!(text_input_value(&mut app), "join");
    type_text(&mut app, " ");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "join Blue");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "join Red");
    assert!(is_focused(&mut app));

    // Nothing to complete, but the focus stays.
    type_text(&mut app, " 1");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "join Red 1");
    assert!(is_focused(&mut app));
}

#[test]
fn tab_finishes_the_only_candidate() {
    let mut app = app();
    focus_text_input(&mut app);
    type_text(&mut app, "JU");
    tab(&mut app);
    assert_eq!(text_input_value(&mut app), "jump ");
}

#[test]
fn tab_goes_through_all_commands_while_nothing_is_typed() {
    let mut app = app();
    focus_text_input(&mut app);
    for name in ["clear", "help", "join", "jump", "quit", "clear"] {
        tab(&mut app);
        assert_eq!(text_input_value(&mut app), name);
    }
    assert!(is_focused(&mut app));
}

#[test]
fn suggestions_are_shown() {
    let mut app = app();
    assert_eq!(suggestions(&mut app), "");
    focus_text_input(&mut app);
    type_text(&mut app, "j");
    assert_eq!(suggestions(&mut app), "join  jump  ");
    type_text(&mut app, "oin");
    assert_eq!(suggestions(&mut app), "join <Red|Blue> <u32>");
    type_text(&mut app, " ");
    assert_eq!(suggestions(&mut app), "Blue  Red  \njoin <Red|Blue> <u32>");
    press_enter(&mut app);
    assert_eq!(suggestions(&mut app), "");

    app.world_mut()
        .resource_mut::<ConsoleSettings>()
        .max_suggestions = 1;
    type_text(&mut app, "j");
    assert_eq!(suggestions(&mut app), "join  +1");
    app.world_mut()
        .resource_mut::<ConsoleSettings>()
        .max_suggestions = 0;
    assert_eq!(suggestions(&mut app), "");
}

#[test]
fn opening_focuses_and_closing_restores() {
    let mut app = app();
    // The app keeps the focus at startup.
    assert!(is_open(&mut app) && !is_focused(&mut app));
    press_t(&mut app);
    assert!(!is_open(&mut app));
    press_t(&mut app);
    assert!(is_open(&mut app) && is_focused(&mut app));
    // The key that opened it is not typed, but it is from now on.
    assert_eq!(text_input_value(&mut app), "");
    press_t(&mut app);
    assert_eq!(text_input_value(&mut app), "t");

    escape(&mut app);
    assert!(!is_open(&mut app) && !is_focused(&mut app));
    press_t(&mut app);
    assert!(is_open(&mut app) && is_focused(&mut app));
    assert_eq!(text_input_value(&mut app), "t");
}

#[test]
fn escape_elsewhere_does_not_close() {
    let mut app = app();
    escape(&mut app);
    assert!(is_open(&mut app));
}

#[test]
fn visibility_can_be_set_by_the_app() {
    let mut app = app_with(ConsoleSettings {
        start_open: false,
        ..default()
    });
    assert!(!is_open(&mut app));
    let mut set = |value| {
        let world = app.world_mut();
        *world
            .query_filtered::<&mut Visibility, With<ConsoleUI>>()
            .single_mut(world)
            .unwrap() = value;
        app.update();
        is_focused(&mut app)
    };
    assert!(set(Visibility::Visible));
    assert!(!set(Visibility::Hidden));
}

#[test]
fn focusing_can_be_turned_off() {
    let mut app = app_with(ConsoleSettings {
        start_open: false,
        focus_on_open: false,
        ..default()
    });
    press_t(&mut app);
    assert!(is_open(&mut app) && !is_focused(&mut app));
    press_t(&mut app);
    assert!(!is_open(&mut app));
}

#[test]
fn toggle_key_can_be_changed() {
    let mut app = app_with(ConsoleSettings {
        toggle_key: KeyCode::Backquote,
        start_open: false,
        ..default()
    });
    press_t(&mut app);
    assert!(!is_open(&mut app));
    press_backquote(&mut app);
    assert!(is_open(&mut app) && is_focused(&mut app));
    type_text(&mut app, "jump");

    // Closes even while typing, and is not typed itself.
    press_backquote(&mut app);
    assert!(!is_open(&mut app) && !is_focused(&mut app));
    assert_eq!(text_input_value(&mut app), "jump");
}

#[test]
fn other_text_inputs_are_left_alone() {
    let mut app = app_with(ConsoleSettings {
        toggle_key: KeyCode::Backquote,
        ..default()
    });
    let other = app
        .world_mut()
        .spawn((Node::default(), EditableText::default()))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(other, bevy::input_focus::FocusCause::Navigated);
    press_backquote(&mut app);
    escape(&mut app);
    assert!(is_open(&mut app));
}
