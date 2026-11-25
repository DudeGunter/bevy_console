//! What the app gets to see of input that is meant for the console.

mod common;

use bevy::{
    input::{
        ButtonState, InputSystems,
        keyboard::{Key, KeyboardInput},
        mouse::{AccumulatedMouseScroll, MouseButtonInput, MouseScrollUnit, MouseWheel},
    },
    input_focus::InputFocus,
    picking::{
        backend::{HitData, PointerHits},
        pointer::PointerId,
    },
    prelude::*,
    window::PrimaryWindow,
};
use bevy_console::prelude::*;
use common::*;

/// What a system of the app has seen, in the last frame in which it saw anything.
#[derive(Resource, Default, Debug, PartialEq)]
struct Seen {
    pressed: Vec<String>,
    just_pressed: Vec<String>,
    just_released: Vec<String>,
    scroll: f32,
    typing: bool,
}

/// Everything that was pressed or released since this was cleared.
#[derive(Resource, Default)]
struct Log(Vec<String>);

/// Where the mouse is, as far as picking is concerned.
#[derive(Resource)]
struct Over(Option<Entity>);

fn look(
    keys: Res<ButtonInput<KeyCode>>,
    logical: Res<ButtonInput<Key>>,
    mouse: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    mut seen: ResMut<Seen>,
    mut log: ResMut<Log>,
) {
    fn names<T: std::fmt::Debug>(values: impl Iterator<Item = T>) -> Vec<String> {
        let mut names: Vec<_> = values.map(|value| format!("{value:?}")).collect();
        names.sort();
        names
    }
    seen.pressed = names(keys.get_pressed());
    seen.pressed.extend(names(mouse.get_pressed()));
    seen.just_pressed = names(keys.get_just_pressed());
    seen.just_pressed.extend(names(mouse.get_just_pressed()));
    seen.just_released = names(keys.get_just_released());
    seen.just_released.extend(names(mouse.get_just_released()));
    seen.scroll = scroll.delta.y;
    // Both kinds of keys have to agree.
    assert_eq!(logical.get_pressed().len(), keys.get_pressed().len());

    let Seen {
        just_pressed,
        just_released,
        ..
    } = &*seen;
    log.0
        .extend(just_pressed.iter().map(|name| format!("+{name}")));
    log.0
        .extend(just_released.iter().map(|name| format!("-{name}")));
}

fn point(over: Res<Over>, mut hits: MessageWriter<PointerHits>) {
    if let Some(entity) = over.0 {
        hits.write(PointerHits::new(
            PointerId::Mouse,
            vec![(entity, HitData::new(entity, 0.0, None, None))],
            1000.0,
        ));
    }
}

fn app_with(settings: ConsoleSettings) -> App {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), ConsolePlugin))
        .insert_resource(settings)
        .insert_resource(Over(None))
        .init_resource::<Seen>()
        .init_resource::<Log>()
        .add_systems(PreUpdate, point.before(InputSystems))
        .add_systems(Update, look)
        .add_systems(
            Update,
            (|mut seen: ResMut<Seen>| seen.typing = true)
                .run_if(typing_in_console)
                .after(look),
        )
        .add_systems(First, |mut seen: ResMut<Seen>| seen.typing = false);
    start(&mut app);
    app
}

fn app() -> App {
    app_with(ConsoleSettings::default())
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap()
}

fn set_key(app: &mut App, key_code: KeyCode, key: &str, is_pressed: bool) {
    let window = window(app);
    let logical_key = match key_code {
        KeyCode::Escape => Key::Escape,
        _ => Key::Character(key.into()),
    };
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state: state(is_pressed),
        text: (is_pressed && !key.is_empty()).then(|| key.into()),
        repeat: false,
        window,
    });
    app.update();
}

fn set_mouse(app: &mut App, is_pressed: bool) {
    let window = window(app);
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Left,
        state: state(is_pressed),
        window,
    });
    app.update();
}

fn state(is_pressed: bool) -> ButtonState {
    match is_pressed {
        true => ButtonState::Pressed,
        false => ButtonState::Released,
    }
}

fn scroll(app: &mut App) -> f32 {
    let window = window(app);
    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: 1.0,
        window,
        phase: bevy::input::touch::TouchPhase::Moved,
    });
    app.update();
    seen(app).scroll
}

fn seen(app: &App) -> &Seen {
    app.world().resource::<Seen>()
}

/// Takes everything that was seen since the last time.
fn log(app: &mut App) -> Vec<String> {
    std::mem::take(&mut app.world_mut().resource_mut::<Log>().0)
}

fn console(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<ConsoleUI>>()
        .single(app.world())
        .unwrap()
}

#[test]
fn keys_reach_the_app_while_not_typing() {
    let mut app = app();
    set_key(&mut app, KeyCode::KeyW, "w", true);
    assert_eq!(seen(&app).pressed, ["KeyW"]);
    assert!(!seen(&app).typing);
    set_key(&mut app, KeyCode::KeyW, "w", false);
    assert_eq!(log(&mut app), ["+KeyW", "-KeyW"]);
}

#[test]
fn keys_are_hidden_while_typing() {
    let mut app = app();
    focus_text_input(&mut app);
    type_text(&mut app, "hello");
    set_key(&mut app, KeyCode::KeyW, "w", true);
    assert!(seen(&app).pressed.is_empty());
    assert!(seen(&app).typing);
    set_key(&mut app, KeyCode::KeyW, "w", false);
    press_enter(&mut app);
    assert!(log(&mut app).is_empty());
    assert_eq!(text_input_value(&mut app), "");
}

#[test]
fn held_keys_are_released_when_typing_starts() {
    let mut app = app();
    set_key(&mut app, KeyCode::KeyW, "w", true);
    focus_text_input(&mut app);
    app.update();
    assert!(seen(&app).pressed.is_empty());
    assert_eq!(log(&mut app), ["+KeyW", "-KeyW"]);

    // Still held once the console is closed.
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(seen(&app).pressed, ["KeyW"]);
}

#[test]
fn the_keys_that_open_and_close_do_not_leak() {
    let mut app = app_with(ConsoleSettings {
        start_open: false,
        toggle_key: KeyCode::Backquote,
        ..default()
    });
    set_key(&mut app, KeyCode::Backquote, "`", true);
    app.update();
    assert!(seen(&app).typing && seen(&app).pressed.is_empty());
    set_key(&mut app, KeyCode::Backquote, "`", false);
    log(&mut app);

    // Escape closes the console in the same frame, but must not open a menu of the app.
    set_key(&mut app, KeyCode::Escape, "", true);
    assert_eq!(visibility(&mut app), Visibility::Hidden);
    app.update();
    set_key(&mut app, KeyCode::Escape, "", false);
    assert!(log(&mut app).is_empty());
    set_key(&mut app, KeyCode::Escape, "", true);
    assert_eq!(log(&mut app), ["+Escape"]);
    set_key(&mut app, KeyCode::Escape, "", false);
    log(&mut app);

    set_key(&mut app, KeyCode::Backquote, "`", true);
    set_key(&mut app, KeyCode::Backquote, "`", false);
    log(&mut app);
    set_key(&mut app, KeyCode::Backquote, "`", true);
    assert_eq!(visibility(&mut app), Visibility::Hidden);
    app.update();
    set_key(&mut app, KeyCode::Backquote, "`", false);
    assert!(log(&mut app).is_empty());
}

#[test]
fn the_mouse_is_hidden_over_the_console() {
    let mut app = app();
    let console = console(&mut app);
    let inner = text_input(&mut app);
    for over in [console, inner] {
        app.insert_resource(Over(Some(over)));
        app.update();
        set_mouse(&mut app, true);
        // Dragged out of the console.
        app.insert_resource(Over(None));
        app.update();
        app.update();
        assert!(seen(&app).pressed.is_empty());
        set_mouse(&mut app, false);
        assert!(log(&mut app).is_empty());
    }

    app.insert_resource(Over(Some(console)));
    app.update();
    assert_eq!(scroll(&mut app), 0.0);
    // Keys are only hidden while typing.
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    set_key(&mut app, KeyCode::KeyW, "w", true);
    assert_eq!(seen(&app).pressed, ["KeyW"]);
}

#[test]
fn the_mouse_reaches_the_app_elsewhere() {
    let mut app = app();
    let console = console(&mut app);
    set_mouse(&mut app, true);
    assert_eq!(scroll(&mut app), 1.0);

    // Dragged into the console.
    app.insert_resource(Over(Some(console)));
    app.update();
    app.update();
    assert_eq!(seen(&app).pressed, ["Left"]);
    set_mouse(&mut app, false);
    assert_eq!(log(&mut app), ["+Left", "-Left"]);
}

#[test]
fn blocking_can_be_turned_off() {
    let mut app = app_with(ConsoleSettings {
        block_input: false,
        ..default()
    });
    focus_text_input(&mut app);
    set_key(&mut app, KeyCode::KeyW, "w", true);
    assert_eq!(seen(&app).pressed, ["KeyW"]);
    assert!(seen(&app).typing);

    let console = console(&mut app);
    app.insert_resource(Over(Some(console)));
    app.update();
    set_mouse(&mut app, true);
    assert_eq!(seen(&app).pressed, ["KeyW", "Left"]);
    assert_eq!(scroll(&mut app), 1.0);
}
