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
        .add_command(greet)
        .run();
}

fn greet(In(name): In<String>) -> String {
    format!("Hello, {name}!")
}
```

Click the text input and type `greet Ferris`, or `help` for a list of commands.

| Key             | Does                                                                        |
| --------------- | --------------------------------------------------------------------------- |
| T               | opens the console, and closes it unless you are typing                      |
| Escape          | closes the console while you are typing                                     |
| Tab, Shift+Tab  | goes through what could be typed: commands, enums and `bool`s               |
| Up, Down        | goes through earlier entries                                                |
| Enter           | runs the command                                                            |

## Commands

`add_command` takes whatever you have, and you only add detail when you need it:

```rust
app
    // Functions, with or without arguments.
    .add_command(quit)
    .add_command(add)
    // Events and messages, of which a clone is sent.
    .add_command(Explosion { radius: 5.0 })
    // Anything with a name of its own, which closures need.
    .add_command(("hi", || "Hello!"))
    // Anything with more details.
    .add_command(
        buy.named("shop.buy")
            .about("Buys something.")
            .available_if(in_state(Shopping)),
    )
    // Several at once.
    .add_commands((pause, resume.about("Continues.")));
```

Arguments are whatever type is inside of `In`:

```rust
fn add(In((a, b)): In<(f32, f32)>) -> f32 {
    a + b
}

/// Gives someone items.
#[derive(Reflect)]
struct Give {
    item: Item,
    /// Everyone by default.
    player: Option<String>,
}

fn give(In(give): In<Give>) -> Result<String> { /* ... */ }
```

- Numbers, `bool`, `char`, `String`, enums, structs, tuples, arrays, `Option` and `Vec` can be read.
  `Option` and `Vec` take what is left, so put them last. Quotes keep words together.
- What a command returns is shown, and errors are logged, so `?` works.
- `help give` shows the usage, and the doc comments of the struct and its fields. Those of functions
  can't be read, so use `about` instead.
- A command can also be run by triggering `TryCommand::from_entry("give sword".into())`.
- The older `add_command_*` methods still work, but are deprecated.

## Other features

- `simple!` logs a message without its time, level and target.
- `ConsoleSettings` changes the key, whether the console starts open and takes the focus, whether
  input is blocked, whether entries are repeated, and how many messages, earlier entries and suggestions there are:

  ```rust
  app.insert_resource(ConsoleSettings {
      toggle_key: KeyCode::Backquote,
      start_open: false,
      ..default()
  });
  ```
- What you type, click and scroll in the console is hidden from the `ButtonInput` and
  `AccumulatedMouseScroll` of your app. Systems that read messages like `KeyboardInput` can use
  `run_if(not(typing_in_console))`, and those that read input in `PreUpdate` should run after
  `ConsoleSystems::BlockInput`.
- New messages are followed, unless you have scrolled up to read.
- If your app uses Feathers as well, add `FeathersPlugins` before `ConsolePlugin`. A `UiTheme` you
  insert is left alone.

## Examples

- `cargo run --example basic`: every way to add a command.
- `cargo run --example sandbox`: controls a small scene with commands.
- `cargo run --example custom_theme`: an app with its own Feathers theme and settings.

## Versions

| Bevy | Branch |
| ---- | ------ |
| 0.20 | `0.20` |
| 0.19 | `0.19` |
