use bevy::{
    feathers::{FeathersCorePlugin, dark_theme::create_dark_theme, theme::UiTheme},
    input_focus::tab_navigation::TabNavigationPlugin,
    prelude::*,
};

mod app_ext;
mod arguments;
mod blocking;
mod commands;
mod default_commands;
mod into_command;
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

        app.add_plugins(blocking::plugin);
        app.init_resource::<ConsoleSettings>()
            .init_resource::<ui::history::CommandHistory>()
            .init_resource::<ui::completion::Completion>();
        app.add_systems(Startup, ui::spawn_ui);
        app.add_systems(
            Update,
            (
                ui::input::open_close_console,
                ui::input::focus_opened_console,
                ui::input::handle_text_input,
                // After the commands that were entered.
                ui::message::receive_traced_message,
                ui::completion::update_suggestions,
            )
                .chain(),
        );
        app.add_observer(commands::try_command);

        use app_ext::*;
        use default_commands::*;
        use into_command::IntoCommand;
        app.add_commands((
            clear.about("Removes every message from the console."),
            help.about("Lists the commands, or describes one of them.")
                .usage("help [command]"),
            quit.about("Exits the app."),
        ));
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
/// ```
/// # use bevy::prelude::*;
/// # use bevy_console::prelude::*;
/// App::new().insert_resource(ConsoleSettings {
///     toggle_key: KeyCode::Backquote,
///     start_open: false,
///     ..default()
/// });
/// ```
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct ConsoleSettings {
    /// The oldest messages are removed once there are more than this many.
    pub max_messages: usize,
    /// Opens and closes the console, T by default. While its text input has focus, letters, digits
    /// and space are typed instead, but Escape always closes it.
    pub toggle_key: KeyCode,
    /// Whether the console is open at startup. True by default.
    pub start_open: bool,
    /// Whether the text input gets the focus when the console is opened. Does not apply at startup,
    /// so that the app keeps it. True by default.
    pub focus_on_open: bool,
    /// Whether input that is meant for the console is hidden from the app. True by default.
    ///
    /// Keys are hidden from [`ButtonInput`] while the text input has focus. Mouse buttons that are
    /// pressed over the console are hidden until they are released, and so is scrolling over it, from
    /// [`AccumulatedMouseScroll`](bevy::input::mouse::AccumulatedMouseScroll). Messages and gamepads
    /// are not affected, but see [`typing_in_console`](prelude::typing_in_console).
    pub block_input: bool,
    /// Whether entries are repeated above their output. True by default.
    pub echo: bool,
    /// How many entries can be recalled with the up and down keys. 100 by default.
    pub max_history: usize,
    /// How many suggestions are shown at once, or 0 to show neither those nor the usage.
    /// Tab completes either way. 5 by default.
    pub max_suggestions: usize,
}

impl Default for ConsoleSettings {
    fn default() -> Self {
        Self {
            max_messages: 500,
            toggle_key: KeyCode::KeyT,
            start_open: true,
            focus_on_open: true,
            block_input: true,
            echo: true,
            max_history: 100,
            max_suggestions: 5,
        }
    }
}

pub mod prelude {
    pub use crate::app_ext::*;
    pub use crate::blocking::{ConsoleSystems, typing_in_console};
    pub use crate::commands::{
        CommandExec, CommandMetadata, ConsoleCommand, DeferredCommand, RunCommand, TryCommand,
    };
    pub use crate::into_command::{
        CommandBuilder, ConsoleInput, ConsoleOutput, IntoCommand, IntoCommands,
    };
    pub use crate::logging::custom_log_layer;
    pub use crate::simple;
    pub use crate::ui::{
        ConsoleUI, MessageContainer, completion::SuggestionList, message::ConsoleMessage,
    };
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
