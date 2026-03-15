use crate::{commands::Command, simple, ui::message::ConsoleMessage};
use bevy::prelude::*;

pub fn clear(mut commands: Commands, query: Query<Entity, With<ConsoleMessage>>) {
    for entity in query {
        commands.entity(entity).despawn();
    }
}

pub fn help(query: Query<&Command>) {
    // todo! add arguments functionality
    for command in query {
        simple!("{}", command.0);
    }
}

pub fn quit(mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
