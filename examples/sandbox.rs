//! Controls a small scene from the console.
//!
//! Run with `cargo run --example sandbox`, click the text input, and try these:
//! - `spawn 20`
//! - `speed 3`
//! - `color 1 0 0.5`
//! - `pause`
//! - `despawn_all`
//! - `settings`

use bevy::prelude::*;
use bevy_console::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                custom_layer: custom_log_layer,
                ..default()
            }),
            ConsolePlugin,
        ))
        .insert_resource(Settings {
            speed: 1.0,
            paused: false,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, orbit)
        .add_commands((
            spawn.about("Spawns squares.").usage("spawn [amount]"),
            speed
                .about("Sets the speed of all squares.")
                .usage("speed <number>"),
            color,
            pause.about("Stops or resumes all squares."),
            despawn_all
                .about("Removes all squares.")
                .available_if(any_with_component::<Orbit>),
            (
                "settings",
                debug_resource::<Settings>.about("Prints the current settings."),
            ),
        ))
        .run();
}

#[derive(Resource, Debug)]
struct Settings {
    speed: f32,
    paused: bool,
}

/// Where a square is on its orbit.
#[derive(Component)]
struct Orbit {
    radius: f32,
    angle: f32,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    info!("Type help for a list of commands, or spawn 20 to get started.");
}

fn orbit(
    time: Res<Time>,
    settings: Res<Settings>,
    mut squares: Query<(&mut Orbit, &mut Transform)>,
) {
    if settings.paused {
        return;
    }
    for (mut orbit, mut transform) in &mut squares {
        orbit.angle += settings.speed * time.delta_secs();
        transform.translation = (Vec2::from_angle(orbit.angle) * orbit.radius).extend(0.0);
        transform.rotation = Quat::from_rotation_z(orbit.angle);
    }
}

fn spawn(
    In(amount): In<Option<usize>>,
    mut commands: Commands,
    squares: Query<(), With<Orbit>>,
) -> Result<String, &'static str> {
    let amount = amount.unwrap_or(1);
    if amount > 1000 {
        return Err("the amount must be a number from 0 to 1000");
    }
    let existing = squares.iter().count();
    for i in existing..existing + amount {
        let i = i as f32;
        commands.spawn((
            Sprite::from_color(Color::hsl((i * 20.0) % 360.0, 0.8, 0.6), Vec2::splat(20.0)),
            Orbit {
                radius: 120.0 + (i * 37.0) % 200.0,
                angle: i * 0.7,
            },
        ));
    }
    Ok(format!(
        "Spawned {amount}, there are now {}.",
        existing + amount
    ))
}

fn speed(In(speed): In<f32>, mut settings: ResMut<Settings>) -> String {
    settings.speed = speed;
    format!("The speed is now {speed}.")
}

/// Colors all squares. Each channel goes from 0 to 1.
#[derive(Reflect)]
struct Channels {
    red: f32,
    green: f32,
    blue: f32,
}

fn color(In(channels): In<Channels>, mut squares: Query<&mut Sprite, With<Orbit>>) {
    for mut sprite in &mut squares {
        sprite.color = Color::srgb(channels.red, channels.green, channels.blue);
    }
}

fn pause(mut settings: ResMut<Settings>) -> &'static str {
    settings.paused = !settings.paused;
    if settings.paused {
        "Paused."
    } else {
        "Resumed."
    }
}

fn despawn_all(mut commands: Commands, squares: Query<Entity, With<Orbit>>) {
    for square in squares {
        commands.entity(square).despawn();
    }
}
