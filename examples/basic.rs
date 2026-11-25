//! Every way to add a command.
//!
//! Run with `cargo run --example basic`, click the text input, and try these:
//! - `help`, `help greet`
//! - `count_entities`
//! - `greet Ferris`, `greet "Ferris the crab"`
//! - `add 1 2`
//! - `give sword 3 Ferris`, `give sword x`
//! - `fails`
//! - `win`, which only works once something was scored
//! - `twice`
//! - `Explosion`, `big_explosion`
//! - `Score`, `jackpot`
//! - `score`
//! - `overload`
//!
//! Tab completes, and the up and down keys recall earlier entries. Press T to open the console, and
//! Escape to close it. T closes it too, unless the text input has focus, where it is just a letter.

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
        // Functions, which become commands of the same name.
        .add_command(count_entities)
        .add_command(overload)
        // Functions that take arguments, which are read from the text.
        .add_command(add)
        .add_command(give)
        // Anything can be described, and several can be added at once.
        .add_commands((
            greet.about("Greets someone."),
            fails.about("Shows what happens to errors."),
        ))
        // Closures need a name. All commands with the same name are executed.
        .add_command(("twice", || "This is the first one."))
        .add_command(("twice", || "This is the second one."))
        // Events and messages, named after the type or explicitly.
        .add_command(Explosion { radius: 1.0 })
        .add_command(("big_explosion", Explosion { radius: 50.0 }))
        .add_command(Score(1))
        .add_command(("jackpot", Score(1000)))
        // Prints any resource that implements Debug.
        .add_command(("score", debug_resource::<TotalScore>))
        // Only runs while a condition is true.
        .add_command(win.available_if(has_scored))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    // UI needs a camera to be rendered.
    commands.spawn(Camera2d);
    info!("Welcome! Type help for a list of commands.");
}

// Text that is returned gets shown.
fn count_entities(query: Query<Entity>) -> String {
    format!("There are {} entities.", query.iter().count())
}

fn overload() {
    for i in 0..100 {
        info!("overload {i}");
    }
}

fn greet(In(name): In<String>) -> String {
    format!("Hello, {name}!")
}

fn add(In((a, b)): In<(f32, f32)>) -> f32 {
    a + b
}

// A struct gives names to the arguments. The comments are shown by `help give`.
/// Gives someone items.
#[derive(Reflect)]
struct Give {
    item: Item,
    amount: u32,
    /// Everyone by default.
    player: Option<String>,
}

#[derive(Reflect, Debug)]
enum Item {
    Sword,
    Apple,
}

fn give(In(give): In<Give>) -> String {
    let player = give.player.as_deref().unwrap_or("everyone");
    format!("Gave {:?} x{} to {player}.", give.item, give.amount)
}

// Errors are logged, so `?` works.
fn fails() -> Result {
    "not a number".parse::<i32>()?;
    Ok(())
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

#[derive(Resource, Debug, Default)]
struct TotalScore(u32);

fn add_score(mut reader: MessageReader<Score>, mut total: ResMut<TotalScore>) {
    for score in reader.read() {
        total.0 += score.0;
        simple!("Scored {}.", score.0);
    }
}

fn has_scored(total: Res<TotalScore>) -> bool {
    total.0 > 0
}

fn win() -> &'static str {
    "You win!"
}
