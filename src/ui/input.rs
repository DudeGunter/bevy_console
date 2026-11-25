use crate::{
    ConsoleSettings,
    commands::TryCommand,
    simple,
    ui::{
        ConsoleUI, MessageContainer,
        completion::{CommandQuery, Completion, suggest},
        history::CommandHistory,
    },
};
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input_focus::{FocusCause, FocusedInput, InputFocus},
    prelude::*,
    text::{EditableText, TextEdit},
};

#[derive(Component, Reflect, Debug, Clone, Default)]
pub struct TextInput;

pub fn handle_text_input(
    mut commands: Commands,
    input_focus: Res<InputFocus>,
    // Keys are hidden from ButtonInput by now.
    mut keys: MessageReader<KeyboardInput>,
    settings: Res<ConsoleSettings>,
    mut history: ResMut<CommandHistory>,
    text_input: Single<(Entity, &mut EditableText), With<TextInput>>,
    mut scroll_position: Single<&mut ScrollPosition, With<MessageContainer>>,
) {
    let (entity, mut text) = text_input.into_inner();
    let is_submitted = keys
        .read()
        .filter(|key| key.state.is_pressed() && !key.repeat)
        .any(|key| key.logical_key == Key::Enter);
    if is_submitted
        && let Some(focused_entity) = input_focus.get()
        && focused_entity == entity
    {
        let entry = text.value().to_string();
        text.clear();
        let entry = entry.trim();
        if entry.is_empty() {
            return;
        }
        history.push(entry, settings.max_history);
        // Show the output, even if something else was being read.
        scroll_position.y = f32::MAX;
        if settings.echo {
            simple!("> {entry}");
        }
        commands.trigger(TryCommand::from_entry(entry.into()));
    }
}

/// Handles the keys which mean something else in the console than in other text inputs.
/// This has to observe the text input, so that it runs after Bevy has handled the key.
pub fn on_key(
    mut key: On<FocusedInput<KeyboardInput>>,
    mut text_inputs: Query<&mut EditableText, With<TextInput>>,
    mut history: ResMut<CommandHistory>,
    mut completion: ResMut<Completion>,
    mut visibility: Single<&mut Visibility, With<ConsoleUI>>,
    keys: Res<ButtonInput<KeyCode>>,
    commands: CommandQuery,
) {
    let Ok(mut text) = text_inputs.get_mut(key.focused_entity) else {
        return;
    };
    if !key.input.state.is_pressed() || text.is_composing() {
        return;
    }

    let shown = text.value().to_string();
    let replacement = match key.input.logical_key {
        Key::ArrowUp => history.older(&shown),
        Key::ArrowDown => history.newer(&shown),
        // Without this, Tab would try to move the focus.
        Key::Tab => {
            key.propagate(false);
            let backwards = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            completion.next(&shown, backwards, || suggest(&shown, &commands))
        }
        Key::Escape => {
            // Bevy has already cleared the focus.
            **visibility = Visibility::Hidden;
            None
        }
        _ => None,
    };
    if let Some(replacement) = replacement {
        // Selecting everything instead would need the font to be loaded.
        text.clear();
        text.queue_edit(TextEdit::Insert(replacement.into()));
    }
}

pub fn open_close_console(
    input_focus: Res<InputFocus>,
    settings: Res<ConsoleSettings>,
    mut keys: MessageReader<KeyboardInput>,
    mut text_inputs: Query<(&mut EditableText, Has<TextInput>)>,
    mut visibility: Single<&mut Visibility, With<ConsoleUI>>,
) {
    for key in keys.read() {
        if key.key_code != settings.toggle_key || key.repeat || !key.state.is_pressed() {
            continue;
        }

        // Only typing matters. Something else usually has the focus, like the primary window.
        if let Some((mut text, is_console)) = input_focus
            .get()
            .and_then(|focused| text_inputs.get_mut(focused).ok())
        {
            if !is_console || is_typed(&key.logical_key) {
                continue;
            }
            // Keys like ` would be inserted as well.
            if let Some(typed) = &key.text
                && let Some(index) = text.pending_edits.iter().rposition(
                    |edit| matches!(edit, TextEdit::Insert(inserted) if inserted == typed),
                )
            {
                text.pending_edits.remove(index);
            }
        }

        **visibility = match **visibility {
            Visibility::Hidden => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}

/// Whether the key is needed to type commands.
fn is_typed(key: &Key) -> bool {
    match key {
        Key::Character(text) => text.chars().any(char::is_alphanumeric),
        Key::Space => true,
        _ => false,
    }
}

/// Moves the focus when the console is opened or closed, no matter by what.
pub fn focus_opened_console(
    mut was_open: Local<Option<bool>>,
    mut input_focus: ResMut<InputFocus>,
    settings: Res<ConsoleSettings>,
    visibility: Single<&Visibility, With<ConsoleUI>>,
    text_input: Single<Entity, With<TextInput>>,
) {
    let is_open = **visibility != Visibility::Hidden;
    match was_open.replace(is_open) {
        Some(false) if is_open && settings.focus_on_open => {
            input_focus.set(*text_input, FocusCause::Navigated);
        }
        Some(true) if !is_open && input_focus.get() == Some(*text_input) => input_focus.clear(),
        _ => {}
    }
}
