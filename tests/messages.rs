//! Log messages shown by the console.
//!
//! `LogPlugin` can only be added once per process, so this file is a single test.

mod common;

use bevy::{log::LogPlugin, prelude::*};
use bevy_console::prelude::*;
use common::*;

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
    .add_command_named("greet", || simple!("hello"))
    .describe_command("greet", "Says hello.", "greet");
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
            "greet - Says hello.",
            "help - Lists the commands, or describes one of them.",
            "quit - Exits the app.",
        ]
    );
    submit(&mut app, "clear");
    submit(&mut app, "help greet");
    assert_eq!(
        shown_messages(&mut app),
        ["greet - Says hello.", "Usage: greet"]
    );

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

    // Only the newest messages are kept, in order, however they arrive.
    submit(&mut app, "clear");
    app.insert_resource(ConsoleSettings { max_messages: 5 });
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

    // Lowering the limit applies right away.
    app.insert_resource(ConsoleSettings { max_messages: 2 });
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
