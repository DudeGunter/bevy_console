//! Every way to add a command.
//!
//! Run with `cargo run --example basic`, click the text input, and try these:
//! - `help`, `help greet`
//! - `count_entities`
//! - `greet Ferris`
//! - `twice`
//! - `Explosion`, `big_explosion`
//! - `Score`, `jackpot`
//! - `score`
//! - `overload`
//!
//! Press T to open and close the console. While the text input has focus T is just a letter, so
//! click outside the console first.

use bevy::prelude::*;
use bevy_console::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                // Lets the console see everything that is logged.
                custom_layer: custom_log_layer,
                ..default()
            }),
            ConsolePlugin,
        ))
        .init_resource::<TotalScore>()
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, add_score)
        .add_observer(on_explosion)
        // Systems, named after the function.
        .add_command(count_entities)
        .add_command(overload)
        // Systems that take arguments.
        .add_command_piped(greet)
        .describe_command("greet", "Greets someone.", "greet <name>")
        // Closures need a name. All commands with the same name are executed.
        .add_command_named("twice", || simple!("This is the first one."))
        .add_command_named("twice", || simple!("This is the second one."))
        // Events and messages, named after the type or explicitly.
        .add_command_event(Explosion { radius: 1.0 })
        .add_command_event_named("big_explosion", Explosion { radius: 50.0 })
        .add_command_message::<Score>()
        .add_command_message_named("jackpot", Score(1000))
        // Prints any resource that implements Debug.
        .add_command_named("score", debug_resource::<TotalScore>)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    // UI needs a camera to be rendered.
    commands.spawn(Camera2d);
    info!("Welcome! Type help for a list of commands.");
}

fn count_entities(query: Query<Entity>) {
    simple!("There are {} entities.", query.iter().count());
}

fn overload() {
    for i in 0..100 {
        info!("overload {i}");
    }
}

fn greet(In(args): In<Vec<String>>) {
    match args.first() {
        Some(name) => simple!("Hello, {name}!"),
        None => warn!("Usage: greet <name>"),
    }
}

#[derive(Event, Clone)]
struct Explosion {
    radius: f32,
}

fn on_explosion(explosion: On<Explosion>) {
    simple!("Something exploded in a radius of {}.", explosion.radius);
}

#[derive(Message, Clone)]
struct Score(u32);

impl Default for Score {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Resource, Debug, Default)]
struct TotalScore(u32);

fn add_score(mut reader: MessageReader<Score>, mut total: ResMut<TotalScore>) {
    for score in reader.read() {
        total.0 += score.0;
        simple!("Scored {}.", score.0);
    }
}
