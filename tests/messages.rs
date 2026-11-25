//! Log messages shown by the console.
//!
//! `LogPlugin` can only be added once per process, so this file is a single test.

mod common;

use bevy::{log::LogPlugin, prelude::*};
use bevy_console::prelude::*;
use common::*;

#[derive(Reflect, Debug)]
enum Paint {
    Red,
    Blue,
}

#[derive(Reflect)]
struct PaintArgs {
    color: Paint,
    /// 1 by default.
    layers: Option<u32>,
}

fn paint(In(args): In<PaintArgs>) -> String {
    format!(
        "Painted {:?} {} times.",
        args.color,
        args.layers.unwrap_or(1)
    )
}

#[test]
fn messages_are_shown_limited_and_cleared() {
    let mut app = App::new();
    app.add_plugins((
        headless_plugins()
            .set(LogPlugin {
                custom_layer: custom_log_layer,
                ..default()
            })
            .enable::<LogPlugin>(),
        ConsolePlugin,
    ))
    .insert_resource(ConsoleSettings {
        echo: false,
        ..default()
    })
    .add_commands((
        ("greet", (|| "hello").about("Says hello.")),
        paint.about("Paints."),
        ("fails", || Err::<(), _>("something broke")),
        ("locked", (|| "unlocked").available_if(|| false)),
    ));
    app.world_mut().spawn(Camera2d);
    start(&mut app);

    // Typed commands show their output, sorted and described.
    submit(&mut app, "clear");
    assert!(shown_messages(&mut app).is_empty());
    submit(&mut app, "greet");
    assert_eq!(shown_messages(&mut app), ["hello"]);
    submit(&mut app, "clear");
    submit(&mut app, "help");
    assert_eq!(
        shown_messages(&mut app),
        [
            "clear - Removes every message from the console.",
            "fails",
            "greet - Says hello.",
            "help - Lists the commands, or describes one of them.",
            "locked",
            "paint - Paints.",
            "quit - Exits the app.",
        ]
    );
    submit(&mut app, "clear");
    submit(&mut app, "help greet");
    assert_eq!(
        shown_messages(&mut app),
        ["greet - Says hello.", "Usage: greet"]
    );
    submit(&mut app, "clear");
    submit(&mut app, "help help");
    assert_eq!(
        shown_messages(&mut app),
        [
            "help - Lists the commands, or describes one of them.",
            "Usage: help [command]",
        ]
    );
    submit(&mut app, "clear");
    submit(&mut app, "help fails");
    assert_eq!(
        shown_messages(&mut app),
        ["fails has no description.", "Usage: fails"]
    );

    // Arguments are parsed, described and checked.
    submit(&mut app, "clear");
    submit(&mut app, "help paint");
    assert_eq!(
        shown_messages(&mut app),
        [
            "paint - Paints.",
            "Usage: paint <color: Red|Blue> [layers: u32]",
            "layers - 1 by default.",
        ]
    );
    submit(&mut app, "clear");
    submit(&mut app, "paint blue 2");
    submit(&mut app, "paint red");
    assert_eq!(
        shown_messages(&mut app),
        ["Painted Blue 2 times.", "Painted Red 1 times."]
    );
    for (entry, level, expected) in [
        (
            "paint green",
            "ERROR",
            "paint: color: expected Red|Blue, got green. Usage: paint <color: Red|Blue> [layers: u32]",
        ),
        (
            "greet you",
            "ERROR",
            "greet: unexpected argument you. Usage: greet",
        ),
        ("fails", "ERROR", "fails: something broke"),
        (
            "locked",
            "WARN",
            "The command locked is not available right now.",
        ),
    ] {
        submit(&mut app, "clear");
        submit(&mut app, entry);
        let shown = shown_messages(&mut app);
        assert_eq!(shown.len(), 1, "{shown:?}");
        assert!(shown[0].contains(level), "{shown:?}");
        assert!(shown[0].ends_with(expected), "{shown:?}");
    }

    // Regular logs show their time, level and target.
    submit(&mut app, "clear");
    error!("broken \"thing\"");
    app.update();
    app.update();
    let shown = shown_messages(&mut app);
    assert_eq!(shown.len(), 1);
    let (time, rest) = shown[0].split_once(' ').unwrap();
    assert_eq!(rest, "ERROR messages: broken \"thing\"");
    assert_eq!(time.len(), "00:00:00".len());

    submit(&mut app, "clear");
    submit(&mut app, "nonsense");
    let shown = shown_messages(&mut app);
    assert_eq!(shown.len(), 1);
    assert!(shown[0].ends_with("The command nonsense was not found."));

    // Entries are shown above their output, unless that is turned off. Clearing includes them.
    app.world_mut().resource_mut::<ConsoleSettings>().echo = true;
    submit(&mut app, "clear");
    assert!(shown_messages(&mut app).is_empty());
    submit(&mut app, "  greet ");
    submit(&mut app, "   ");
    assert_eq!(shown_messages(&mut app), ["> greet", "hello"]);

    // Only the newest messages are kept, in order, however they arrive.
    submit(&mut app, "clear");
    app.insert_resource(ConsoleSettings {
        max_messages: 5,
        echo: false,
        ..default()
    });
    for i in 0..1000 {
        simple!("{i}");
    }
    app.update();
    app.update();
    assert_eq!(
        shown_messages(&mut app),
        ["995", "996", "997", "998", "999"]
    );
    for i in 1000..1003 {
        simple!("{i}");
        app.update();
    }
    app.update();
    assert_eq!(
        shown_messages(&mut app),
        ["998", "999", "1000", "1001", "1002"]
    );

    scrolling(&mut app);
    app.insert_resource(ConsoleSettings {
        max_messages: 5,
        echo: false,
        ..default()
    });
    submit(&mut app, "clear");
    for i in 998..1003 {
        simple!("{i}");
    }
    app.update();
    app.update();

    // Lowering the limit applies right away.
    app.insert_resource(ConsoleSettings {
        max_messages: 2,
        echo: false,
        ..default()
    });
    app.update();
    app.update();
    assert_eq!(shown_messages(&mut app), ["1001", "1002"]);

    // Other entities in the container are left alone.
    let container = app
        .world_mut()
        .query_filtered::<Entity, With<MessageContainer>>()
        .single(app.world())
        .unwrap();
    let custom = app
        .world_mut()
        .spawn((Node::default(), ChildOf(container)))
        .id();
    simple!("1003");
    app.update();
    app.update();
    submit(&mut app, "clear");
    assert!(shown_messages(&mut app).is_empty());
    assert!(app.world().get_entity(custom).is_ok());
}

/// New messages are followed, unless earlier ones are being read.
fn scrolling(app: &mut App) {
    /// How far the first message is above the top, and how far there is left to scroll.
    fn position(app: &mut App) -> (f32, f32) {
        let world = app.world_mut();
        let node = world
            .query_filtered::<&ComputedNode, With<MessageContainer>>()
            .single(world)
            .unwrap();
        let scale = node.inverse_scale_factor();
        let max = (node.content_size().y - node.size().y) * scale;
        (
            node.scroll_position.y * scale,
            max - node.scroll_position.y * scale,
        )
    }
    fn scroll_to(app: &mut App, y: f32) {
        let world = app.world_mut();
        world
            .query_filtered::<&mut ScrollPosition, With<MessageContainer>>()
            .single_mut(world)
            .unwrap()
            .y = y;
        app.update();
    }
    fn log(app: &mut App, amount: usize) {
        for i in 0..amount {
            simple!("scroll {i}");
        }
        app.update();
        app.update();
    }

    app.insert_resource(ConsoleSettings {
        max_messages: 100,
        echo: false,
        ..default()
    });
    submit(app, "clear");
    log(app, 50);
    let (top, left) = position(app);
    assert!(top > 100.0 && left < 1.5, "{top} {left}");
    log(app, 10);
    let (top, left) = position(app);
    assert!(left < 1.5, "{top} {left}");

    // Reading, and then back at the bottom.
    scroll_to(app, 80.0);
    log(app, 10);
    assert_eq!(position(app).0, 80.0);
    scroll_to(app, f32::MAX);
    log(app, 10);
    let (top, left) = position(app);
    assert!(left < 1.5, "{top} {left}");

    // The oldest are removed, but what is being read stays in place.
    log(app, 20);
    assert_eq!(shown_messages(app).len(), 100);
    let bottom = position(app).0;
    let row = (bottom + 300.0) / 100.0;
    scroll_to(app, 200.0);
    log(app, 5);
    let expected = 200.0 - 5.0 * row;
    let (top, _) = position(app);
    assert!((top - expected).abs() < 2.0, "{top} {expected}");

    // Entering a command shows its output.
    submit(app, "greet");
    let (top, left) = position(app);
    assert!(left < 1.5, "{top} {left}");
}
