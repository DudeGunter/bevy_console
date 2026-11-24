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

#[test]
fn dragging_moves_the_console_as_far_as_the_pointer() {
    use bevy::{
        camera::NormalizedRenderTarget,
        picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerId},
        },
        window::{PrimaryWindow, WindowRef},
    };

    let mut app = app();
    start(&mut app);
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<ConsoleUI>>()
        .single(world)
        .unwrap();
    let window = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .unwrap();
    let target =
        NormalizedRenderTarget::Window(WindowRef::Primary.normalize(Some(window)).unwrap());
    let pointer = |x, y| {
        let position = Vec2::new(x, y);
        let target = target.clone();
        Pointer::new(PointerId::Mouse, Location { target, position })
    };

    // Two drags, each pressed at (50, 10) and then moved twice.
    for drag in [0.0, 1.0] {
        // Bevy sends this once the pointer has already moved, with its new position.
        world.trigger(PointerDragStart {
            entity,
            pointer: pointer(150.0, 30.0),
            button: PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
        });
        for step in [1.0, 2.0] {
            let delta = Vec2::new(100.0, 20.0);
            world.trigger(PointerDrag {
                entity,
                pointer: pointer(50.0 + 100.0 * step, 10.0 + 20.0 * step),
                button: PointerButton::Primary,
                distance: delta * step,
                delta,
            });
            let moved = delta * step + Vec2::new(200.0, 40.0) * drag;
            let translation = world.get::<UiTransform>(entity).unwrap().translation;
            assert_eq!(translation, Val2::px(moved.x, moved.y));
        }
    }
}
