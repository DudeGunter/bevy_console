//! Commands typed into the console. These don't need to see any log messages.

mod common;

use bevy::prelude::*;
use bevy_console::prelude::*;
use common::*;

#[derive(Resource, Default)]
struct Calls(Vec<String>);

#[derive(Message, Clone, Default)]
struct Score(u32);

#[derive(Event, Clone)]
struct Special;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), ConsolePlugin))
        .init_resource::<Calls>();
    app
}

fn calls(app: &App) -> &[String] {
    &app.world().resource::<Calls>().0
}

fn echo(In(args): In<Vec<String>>, mut calls: ResMut<Calls>) {
    calls.0.push(args.join(" "));
}

#[test]
fn entries_run_commands_and_clear_the_input() {
    let mut app = app();
    app.add_command_piped(echo);
    start(&mut app);

    focus_text_input(&mut app);
    type_text(&mut app, "echo hello world");
    assert_eq!(text_input_value(&mut app), "echo hello world");
    assert!(calls(&app).is_empty());

    press_enter(&mut app);
    assert_eq!(calls(&app), ["hello world"]);
    assert_eq!(text_input_value(&mut app), "");

    submit(&mut app, "echo again");
    assert_eq!(calls(&app), ["hello world", "again"]);
}

#[test]
fn enter_is_ignored_without_focus() {
    let mut app = app();
    app.add_command_piped(echo);
    start(&mut app);

    focus_text_input(&mut app);
    type_text(&mut app, "echo");
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .clear();
    press_enter(&mut app);
    assert!(calls(&app).is_empty());
}

#[test]
fn events_and_messages_reach_the_app() {
    let mut app = app();
    app.add_command_event(Special)
        .add_command_message_named("win", Score(10))
        .add_observer(|_: On<Special>, mut calls: ResMut<Calls>| {
            calls.0.push("special".into());
        })
        .add_systems(
            Update,
            |mut reader: MessageReader<Score>, mut calls: ResMut<Calls>| {
                for score in reader.read() {
                    calls.0.push(score.0.to_string());
                }
            },
        );
    start(&mut app);

    submit(&mut app, "Special");
    submit(&mut app, "win");
    assert_eq!(calls(&app), ["special", "10"]);
}

#[test]
fn quit_exits_the_app() {
    let mut app = app();
    start(&mut app);
    assert!(app.should_exit().is_none());
    focus_text_input(&mut app);
    type_text(&mut app, "quit");
    press(
        &mut app,
        KeyCode::Enter,
        bevy::input::keyboard::Key::Enter,
        None,
    );
    assert_eq!(app.should_exit(), Some(AppExit::Success));
}

#[test]
fn t_opens_and_closes_the_console() {
    let mut app = app();
    start(&mut app);
    assert_ne!(visibility(&mut app), Visibility::Hidden);

    press_t(&mut app);
    assert_eq!(visibility(&mut app), Visibility::Hidden);
    press_t(&mut app);
    assert_eq!(visibility(&mut app), Visibility::Visible);

    // While typing, it is just a letter.
    focus_text_input(&mut app);
    press_t(&mut app);
    assert_eq!(visibility(&mut app), Visibility::Visible);
    assert_eq!(text_input_value(&mut app), "t");
}
