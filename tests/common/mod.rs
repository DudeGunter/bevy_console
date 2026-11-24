//! Runs the whole console, without a window, GPU or event loop.
#![allow(dead_code)] // Every test file uses a different part of this.

use bevy::{
    feathers::controls::FeathersTextInput,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKeyCode},
    },
    input_focus::{FocusCause, InputFocus},
    log::LogPlugin,
    prelude::*,
    render::{
        RenderPlugin,
        settings::{RenderCreation, WgpuSettings},
    },
    text::EditableText,
    window::PrimaryWindow,
    winit::WinitPlugin,
};
use bevy_console::prelude::*;

/// The plugins of a normal app. `LogPlugin` is left out, as it can only be added once per process.
pub fn headless_plugins() -> bevy::app::PluginGroupBuilder {
    DefaultPlugins
        .set(RenderPlugin {
            render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                backends: None,
                ..default()
            })),
            ..default()
        })
        .disable::<WinitPlugin>()
        .disable::<LogPlugin>()
}

/// Does what `App::run` does before the first update, and then updates until the UI has settled.
pub fn start(app: &mut App) {
    app.finish();
    app.cleanup();
    for _ in 0..3 {
        app.update();
    }
}

pub fn press(app: &mut App, key_code: KeyCode, logical_key: Key, text: Option<&str>) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: logical_key.clone(),
            state,
            text: text
                .filter(|_| state == ButtonState::Pressed)
                .map(Into::into),
            repeat: false,
            window,
        });
        app.update();
    }
}

pub fn press_t(app: &mut App) {
    press(app, KeyCode::KeyT, Key::Character("t".into()), Some("t"));
}

pub fn press_enter(app: &mut App) {
    press(app, KeyCode::Enter, Key::Enter, None);
    // One update to run the command, and one to show what it logged.
    app.update();
    app.update();
}

/// Types the text key by key. The key codes don't matter to a text input.
pub fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        let text = character.to_string();
        let key = match character {
            ' ' => Key::Space,
            _ => Key::Character(text.as_str().into()),
        };
        let code = KeyCode::Unidentified(NativeKeyCode::Unidentified);
        press(app, code, key, Some(&text));
    }
}

pub fn text_input(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<FeathersTextInput>>()
        .single(app.world())
        .unwrap()
}

pub fn focus_text_input(app: &mut App) {
    let entity = text_input(app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.update();
}

pub fn text_input_value(app: &mut App) -> String {
    let entity = text_input(app);
    let text = app.world().get::<EditableText>(entity).unwrap();
    text.value().to_string()
}

/// Types the entry into the text input and submits it.
pub fn submit(app: &mut App, entry: &str) {
    focus_text_input(app);
    type_text(app, entry);
    press_enter(app);
}

/// The text of every message the console shows, from top to bottom.
pub fn shown_messages(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let container = world
        .query_filtered::<Entity, With<MessageContainer>>()
        .single(world)
        .unwrap();
    let rows: Vec<Entity> = world
        .get::<Children>(container)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    rows.into_iter()
        .filter(|row| world.get::<ConsoleMessage>(*row).is_some())
        .map(|row| {
            let mut text = String::new();
            let mut spans = vec![row];
            spans.extend(world.get::<Children>(row).into_iter().flatten().copied());
            for span in spans {
                if let Some(part) = world.get::<Text>(span) {
                    text.push_str(part);
                }
                if let Some(part) = world.get::<TextSpan>(span) {
                    text.push_str(part);
                }
            }
            text
        })
        .collect()
}

pub fn visibility(app: &mut App) -> Visibility {
    *app.world_mut()
        .query_filtered::<&Visibility, With<ConsoleUI>>()
        .single(app.world())
        .unwrap()
}
