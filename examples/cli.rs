use bevy::prelude::*;
use bevy_console::prelude::*;

fn main() {
    App::new().add_plugins((MinimalPlugins, ConsolePlugin));
}
