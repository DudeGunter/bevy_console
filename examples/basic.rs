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
        .add_systems(Startup, spawn_camera)
        .add_command_named("strange", special)
        .add_command_named("strange", || simple!("This is the second strange command."))
        .add_command_event(Special)
        .add_observer(on_special)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    info!("Basic example, spawning scene");
    commands.spawn((
        Camera3d::default(),
        Transform::default().looking_at(Vec3::new(10.0, 0.0, 0.0), Vec3::Y),
    ));
}

fn special(query: Query<Entity>) {
    simple!("67");
    simple!("There are {} entities", query.iter().count());
}

#[derive(Event, Default, Clone)]
pub struct Special;

pub fn on_special(_trigger: On<Special>) {
    simple!("Triggered special")
}
