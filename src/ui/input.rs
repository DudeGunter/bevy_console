use bevy::{input_focus::InputFocus, prelude::*, text::EditableText};

use crate::commands::TryCommand;

#[derive(Component, Reflect, Debug, Clone, Default)]
pub struct TextInput;

pub fn handle_text_input(
    mut commands: Commands,
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    text_input: Single<(Entity, &mut EditableText), With<TextInput>>,
) {
    let (entity, mut text) = text_input.into_inner();
    if keyboard_input.just_pressed(KeyCode::Enter)
        && let Some(focused_entity) = input_focus.get()
        && focused_entity == entity
    {
        commands.trigger(TryCommand::from_entry(text.value().to_string()));
        text.clear();
    }
}
