use bevy::prelude::*;

mod app_ext;
mod commands;
mod default_commands;
mod logging;
mod ui;

/// The Plugin that implements the log reading functionality for the
/// developer console via [`LogPlugin::custom_layer`](bevy::log::LogPlugin::custom_layer).
/// ```rust
/// # use bevy::prelude::*;
/// // The Default plugin - I assume - isn't actually needed
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.set(LogPlugin {
///             custom_layer: custom_log_layer, // The function
///             ..default()
///         }))
///         .add_plugin(ConsolePlugin) // The plugin
///         .run();
/// }
/// ```
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, ui::create_ui);
        app.add_systems(
            Update,
            (
                ui::input::handle_selected_boxes,
                ui::message::receive_traced_message,
                ui::open_close_console,
            ),
        );
        app.add_observer(commands::try_command);
        app.add_observer(ui::message::handle_custom_messages);

        use app_ext::*;
        use default_commands::*;
        app.add_command(clear);
        app.add_command(help);
        app.add_command(quit);
    }
}

pub mod prelude {
    pub use crate::ConsolePlugin;
    pub use crate::app_ext::*;
    pub use crate::logging::custom_log_layer;
    pub use crate::simple;
    pub use bevy::log::LogPlugin;
    pub use tracing;

    use bevy::prelude::*;
    use std::fmt::Debug;
    /// A command function which
    /// simple prints out a the resource
    pub fn debug_resource<R: Resource + Debug>(resource: Res<R>) {
        simple!("{:?}", resource);
    }
}
