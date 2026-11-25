use crate::{
    commands::{CommandArguments, CommandMetadata, ConsoleCommand},
    logging::TracingReceiver,
    simple,
    ui::message::ConsoleMessage,
};
use bevy::prelude::*;

pub fn clear(
    mut commands: Commands,
    query: Query<Entity, With<ConsoleMessage>>,
    receiver: Option<Res<TracingReceiver>>,
) {
    // Messages that are not shown yet, such as the echo of this command.
    if let Some(receiver) = receiver {
        receiver.try_iter().for_each(drop);
    }
    for entity in query {
        commands.entity(entity).try_despawn();
    }
}

pub fn help(
    In(name): In<Option<String>>,
    query: Query<(
        &ConsoleCommand,
        Option<&CommandMetadata>,
        Option<&CommandArguments>,
    )>,
) {
    let mut entries: Vec<_> = query.iter().collect();
    entries.sort_by(|(a, ..), (b, ..)| a.0.cmp(&b.0));

    let Some(name) = name else {
        for (command, metadata, _) in entries {
            match metadata {
                Some(metadata) if !metadata.description.is_empty() => {
                    simple!("{} - {}", command.0, metadata.description);
                }
                _ => simple!("{}", command.0),
            }
        }
        return;
    };

    entries.retain(|(command, ..)| command.0 == name);
    if entries.is_empty() {
        warn!("The command {} was not found.", name);
    }
    for (command, metadata, arguments) in entries {
        match metadata {
            Some(metadata) if !metadata.description.is_empty() => {
                simple!("{} - {}", command.0, metadata.description);
            }
            _ => simple!("{} has no description.", command.0),
        }
        if let Some(metadata) = metadata {
            simple!("Usage: {}", metadata.usage);
        }
        for argument in arguments.into_iter().flat_map(|arguments| &arguments.0) {
            if !argument.docs.is_empty() {
                simple!("{} - {}", argument.name, argument.docs);
            }
        }
    }
}

pub fn quit(mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
