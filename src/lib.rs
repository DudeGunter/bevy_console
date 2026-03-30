use bevy::prelude::*;

mod app_ext;
#[cfg(feature = "cli")]
mod cli;
mod commands;
#[cfg(feature = "default_commands")]
mod default_commands;
#[cfg(feature = "logging")]
mod logging;
#[cfg(feature = "ui")]
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
        #[cfg(feature = "ui")]
        {
            app.add_systems(Startup, ui::create_ui);
            app.add_systems(
                Update,
                (
                    ui::input::handle_selected_boxes,
                    ui::message::receive_traced_message,
                    ui::open_close_console,
                ),
            );
            app.add_observer(ui::message::handle_custom_messages);
        }

        #[cfg(feature = "default_commands")]
        {
            use app_ext::*;
            use default_commands::*;
            app.add_command(clear);
            app.add_command(help);
            app.add_command(quit);
        }

        #[cfg(feature = "cli")]
        {
            app.add_systems(Update, cli::receive_cli_inputs);
        }
        app.add_observer(commands::try_command);
    }
}

#[macro_export]
macro_rules! simple {
    ($msg:expr) => {
        $crate::prelude::tracing::info!(custom = true, $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::prelude::tracing::info!(custom = true, $fmt, $($arg)*);
    };
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
