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
        .add_command_piped(spawn)
        .describe_command("spawn", "Spawns squares.", "spawn [amount]")
        .add_command_piped(speed)
        .describe_command("speed", "Sets the speed of all squares.", "speed <number>")
        .add_command_piped(color)
        .describe_command("color", "Colors all squares.", "color <red> <green> <blue>")
        .add_command(pause)
        .describe_command("pause", "Stops or resumes all squares.", "pause")
        .add_command(despawn_all)
        .describe_command("despawn_all", "Removes all squares.", "despawn_all")
        .add_command_named("settings", debug_resource::<Settings>)
        .describe_command("settings", "Prints the current settings.", "settings")
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

fn spawn(In(args): In<Vec<String>>, mut commands: Commands, squares: Query<(), With<Orbit>>) {
    let amount = match args.first().map(|arg| arg.parse::<usize>()) {
        None => 1,
        Some(Ok(amount)) if amount <= 1000 => amount,
        Some(_) => {
            error!("The amount must be a number from 0 to 1000.");
            return;
        }
    };
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
    simple!("Spawned {amount}, there are now {}.", existing + amount);
}

fn speed(In(args): In<Vec<String>>, mut settings: ResMut<Settings>) {
    let Some(Ok(speed)) = args.first().map(|arg| arg.parse()) else {
        error!("Usage: speed <number>");
        return;
    };
    settings.speed = speed;
    simple!("The speed is now {speed}.");
}

fn color(In(args): In<Vec<String>>, mut squares: Query<&mut Sprite, With<Orbit>>) {
    let channels: Vec<f32> = args.iter().filter_map(|arg| arg.parse().ok()).collect();
    let [red, green, blue] = channels[..] else {
        error!("Usage: color <red> <green> <blue>, each from 0 to 1");
        return;
    };
    for mut sprite in &mut squares {
        sprite.color = Color::srgb(red, green, blue);
    }
}

fn pause(mut settings: ResMut<Settings>) {
    settings.paused = !settings.paused;
    simple!(
        "{}",
        if settings.paused {
            "Paused."
        } else {
            "Resumed."
        }
    );
}

fn despawn_all(mut commands: Commands, squares: Query<Entity, With<Orbit>>) {
    for square in squares {
        commands.entity(square).despawn();
    }
}
