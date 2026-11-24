# bevy_console

A light weight developer console for Bevy. Shows the log output of your app and runs commands,
using Bevy's own UI and Feathers widgets.

Made just for me, so expect quirks.

## Usage

```rust
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
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_command_piped(greet)
        .describe_command("greet", "Greets someone.", "greet <name>")
        .run();
}

fn greet(In(args): In<Vec<String>>) {
    match args.first() {
        Some(name) => simple!("Hello, {name}!"),
        None => warn!("Usage: greet <name>"),
    }
}
```

Click the text input and type `greet Ferris`, or `help` for a list of commands. Press T to open and
close the console, as long as the text input does not have focus.

| To run             | Use                                                 |
| ------------------ | --------------------------------------------------- |
| a system           | `add_command`, `add_command_named`                  |
| one with arguments | `add_command_piped`, `add_command_piped_named`      |
| an event           | `add_command_event`, `add_command_event_named`      |
| a message          | `add_command_message`, `add_command_message_named`  |

- `simple!` logs a message without its time, level and target.
- `ConsoleSettings` limits how many messages are kept, 500 by default.
- If your app uses Feathers as well, add `FeathersPlugins` before `ConsolePlugin`. A `UiTheme` you
  insert is left alone.

## Examples

- `cargo run --example basic`: every way to add a command.
- `cargo run --example sandbox`: controls a small scene with commands.
- `cargo run --example custom_theme`: an app with its own Feathers theme and settings.

## Versions

| Bevy | Branch |
| ---- | ------ |
| 0.19 | `0.19` |
