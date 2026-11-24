//! How the plugin behaves in apps which are set up in different ways.

mod common;

use bevy::{
    feathers::{
        FeathersPlugins,
        theme::{ThemeProps, UiTheme},
        tokens,
    },
    input_focus::tab_navigation::TabNavigationPlugin,
    prelude::*,
};
use bevy_console::prelude::*;
use common::*;

fn pink_theme() -> UiTheme {
    let mut props = ThemeProps::default();
    props
        .color
        .insert(tokens::SUBPANE_BODY_BG, Color::srgb(1.0, 0.0, 1.0));
    UiTheme(props)
}

fn theme_color(app: &App) -> Option<Color> {
    let theme = app.world().resource::<UiTheme>();
    theme.0.color.get(&tokens::SUBPANE_BODY_BG).copied()
}

#[test]
fn works_without_the_log_layer() {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), ConsolePlugin));
    start(&mut app);
    submit(&mut app, "help");
    assert!(shown_messages(&mut app).is_empty());
}

#[test]
fn uses_the_dark_theme_by_default() {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), ConsolePlugin));
    let dark = bevy::feathers::dark_theme::create_dark_theme();
    assert!(theme_color(&app).is_some());
    assert_eq!(
        theme_color(&app),
        dark.color.get(&tokens::SUBPANE_BODY_BG).copied()
    );
}

#[test]
fn keeps_the_plugins_and_theme_of_the_app() {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), FeathersPlugins))
        .insert_resource(pink_theme())
        .add_plugins(ConsolePlugin);
    start(&mut app);
    assert_eq!(theme_color(&app), Some(Color::srgb(1.0, 0.0, 1.0)));
}

#[test]
fn keeps_a_theme_inserted_before_any_plugin() {
    let mut app = App::new();
    app.insert_resource(pink_theme())
        .add_plugins((headless_plugins(), ConsolePlugin));
    start(&mut app);
    assert_eq!(theme_color(&app), Some(Color::srgb(1.0, 0.0, 1.0)));
}

#[test]
fn accepts_tab_navigation_without_the_rest_of_feathers() {
    let mut app = App::new();
    app.add_plugins((headless_plugins(), TabNavigationPlugin, ConsolePlugin));
    start(&mut app);
}

#[test]
fn keeps_settings_inserted_first() {
    let mut app = App::new();
    app.insert_resource(ConsoleSettings { max_messages: 3 })
        .add_plugins((headless_plugins(), ConsolePlugin));
    assert_eq!(app.world().resource::<ConsoleSettings>().max_messages, 3);
}
