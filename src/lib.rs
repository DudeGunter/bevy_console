use bevy::{
    feathers::{FeathersCorePlugin, dark_theme::create_dark_theme, theme::UiTheme},
    input_focus::tab_navigation::TabNavigationPlugin,
    prelude::*,
};

mod app_ext;
mod commands;
mod default_commands;
mod logging;
mod ui;

/// The plugin that spawns the developer console.
///
/// Log output is only captured when [`custom_log_layer`](prelude::custom_log_layer) is also given
/// to [`LogPlugin::custom_layer`](bevy::log::LogPlugin::custom_layer). Without it the console
/// still runs commands, but has no messages to show.
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_console::prelude::*;
///
/// App::new()
///     .add_plugins(DefaultPlugins.set(LogPlugin {
///         custom_layer: custom_log_layer, // The function
///         ..default()
///     }))
///     .add_plugins(ConsolePlugin) // The plugin
///     .run();
/// ```
///
/// # Apps that already use Feathers
///
/// The Feathers plugins are only added when they are missing, and the dark theme is only used when
/// there is no [`UiTheme`] yet. If your app adds `FeathersPlugins` itself, add it *before*
/// `ConsolePlugin`, since Bevy panics when a plugin is added twice.
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        // Must happen before FeathersCorePlugin, which inserts an empty theme.
        if !app.world().contains_resource::<UiTheme>() {
            app.insert_resource(UiTheme(create_dark_theme()));
        }
        // The contents of `FeathersPlugins`, minus whatever the app has already added.
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<FeathersCorePlugin>() {
            app.add_plugins(FeathersCorePlugin);
        }

        app.init_resource::<ConsoleSettings>();
        app.add_systems(Startup, ui::spawn_ui);
        app.add_systems(
            Update,
            (
                ui::input::handle_text_input,
                ui::message::receive_traced_message,
                ui::open_close_console,
            ),
        );
        app.add_observer(commands::try_command);

        use app_ext::*;
        use default_commands::*;
        app.add_command(clear).describe_command(
            "clear",
            "Removes every message from the console.",
            "clear",
        );
        app.add_command_piped(help).describe_command(
            "help",
            "Lists the commands, or describes one of them.",
            "help [command]",
        );
        app.add_command(quit)
            .describe_command("quit", "Exits the app.", "quit");
    }

    // Plugins may be added in any order, so this has to wait until all of them are built.
    fn finish(&self, app: &mut App) {
        if !app.world().contains_resource::<logging::TracingReceiver>() {
            warn!(
                "The console cannot show log messages. Set LogPlugin::custom_layer to bevy_console::prelude::custom_log_layer."
            );
        }
    }
}

/// Configures the console. Can be inserted before or after [`ConsolePlugin`] and changed at runtime.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct ConsoleSettings {
    /// The oldest messages are removed once there are more than this many.
    pub max_messages: usize,
}

impl Default for ConsoleSettings {
    fn default() -> Self {
        Self { max_messages: 500 }
    }
}

pub mod prelude {
    pub use crate::app_ext::*;
    pub use crate::commands::{
        CommandExec, CommandMetadata, ConsoleCommand, DeferredCommand, TryCommand,
    };
    pub use crate::logging::custom_log_layer;
    pub use crate::simple;
    pub use crate::ui::{ConsoleUI, MessageContainer, message::ConsoleMessage};
    pub use crate::{ConsolePlugin, ConsoleSettings};
    pub use bevy::log::LogPlugin;
    pub use tracing;

    use bevy::prelude::*;
    use std::fmt::Debug;
    /// A command function which
    /// simply prints out the resource
    pub fn debug_resource<R: Resource + Debug>(resource: Res<R>) {
        simple!("{:?}", resource);
    }
}
