//! The console in an app that already uses Feathers, with its own theme and settings.
//!
//! Run with `cargo run --example custom_theme`. Only the newest 10 messages are kept.

use bevy::{
    feathers::{FeathersPlugins, dark_theme::create_dark_theme, theme::UiTheme, tokens},
    prelude::*,
    time::common_conditions::on_timer,
};
use bevy_console::prelude::*;
use std::time::Duration;

fn main() {
    let mut theme = create_dark_theme();
    theme
        .color
        .insert(tokens::SUBPANE_HEADER_BG, Color::srgb(0.45, 0.1, 0.35));
    theme
        .color
        .insert(tokens::SUBPANE_BODY_BG, Color::srgb(0.16, 0.06, 0.14));

    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                custom_layer: custom_log_layer,
                ..default()
            }),
            // Has to be added before the console.
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(theme))
        .insert_resource(ConsoleSettings { max_messages: 10 })
        .add_plugins(ConsolePlugin)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, tick.run_if(on_timer(Duration::from_millis(500))))
        .run();
}

fn tick(mut count: Local<u32>) {
    *count += 1;
    info!("Message {}", *count);
}
