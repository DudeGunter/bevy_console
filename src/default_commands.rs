use crate::{
    commands::{CommandMetadata, ConsoleCommand},
    simple,
    ui::message::ConsoleMessage,
};
use bevy::prelude::*;

pub fn clear(mut commands: Commands, query: Query<Entity, With<ConsoleMessage>>) {
    for entity in query {
        commands.entity(entity).try_despawn();
    }
}

pub fn help(In(args): In<Vec<String>>, query: Query<(&ConsoleCommand, Option<&CommandMetadata>)>) {
    let mut entries: Vec<_> = query.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.0.cmp(&b.0));

    let Some(name) = args.first() else {
        for (command, metadata) in entries {
            match metadata {
                Some(metadata) => simple!("{} - {}", command.0, metadata.description),
                None => simple!("{}", command.0),
            }
        }
        return;
    };

    entries.retain(|(command, _)| command.0 == *name);
    if entries.is_empty() {
        warn!("The command {} was not found.", name);
    }
    for (command, metadata) in entries {
        match metadata {
            Some(metadata) => {
                simple!("{} - {}", command.0, metadata.description);
                simple!("Usage: {}", metadata.usage);
            }
            None => simple!("{} has no description.", command.0),
        }
    }
}

pub fn quit(mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
