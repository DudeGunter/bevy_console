use bevy::{ecs::system::SystemId, prelude::*};

/// Text submitted from TextInputBox
#[derive(Event)]
pub struct TryCommand(pub Vec<String>);

impl TryCommand {
    pub fn from_entry(string: String) -> Self {
        TryCommand(string.split_whitespace().map(String::from).collect())
    }
}

pub fn try_command(
    trigger: On<TryCommand>,
    mut commands: Commands,
    query: Query<(&Command, &CommandExec)>,
) {
    let Some(input) = trigger.0.first() else {
        return;
    };

    let mut found = 0;

    for (command, exec) in query {
        if input == command.0.as_str() {
            found += 1;
            match exec {
                CommandExec::System(id) => commands.run_system(id.clone()),
                CommandExec::SystemPiped(id) => {
                    let mut args = trigger.0.clone();
                    args.remove(0);
                    commands.run_system_with(id.clone(), args);
                }
                CommandExec::Deferred(deferred) => deferred.fire(&mut commands),
                CommandExec::None => warn!(
                    "The command {} does not execute anything. Insert the component CommandExec to add functionality.",
                    command.0
                ),
            }
        }
    }
    match found {
        0 => warn!("The command {} was not found.", input),
        1 => {}
        _ => warn!("{} commands have been triggered", found),
    }
}

#[derive(Component, Reflect, Clone)]
pub struct CommandMetadata {
    pub description: String,
    pub usage: String,
}

#[derive(Component, Reflect, Clone, PartialEq, Eq)]
#[require(CommandExec)]
pub struct Command(pub String);

#[derive(Component, Default)]
pub enum CommandExec {
    System(SystemId),
    SystemPiped(SystemId<In<Vec<String>>>),
    Deferred(DeferredCommand),
    #[default]
    None,
}

impl CommandExec {
    pub fn system(id: SystemId) -> Self {
        Self::System(id)
    }

    pub fn system_piped(id: SystemId<In<Vec<String>>>) -> Self {
        Self::SystemPiped(id)
    }

    pub fn event<E: Event + Clone>(event: E) -> Self
    where
        for<'a> E::Trigger<'a>: Default,
    {
        Self::Deferred(DeferredCommand::event(event))
    }

    pub fn message<M: Message + Clone>(message: M) -> Self {
        Self::Deferred(DeferredCommand::message(message))
    }
}

pub struct DeferredCommand {
    trigger: Box<dyn Fn(&mut Commands) + Send + Sync>,
}

impl DeferredCommand {
    pub fn event<E: Event + Clone>(event: E) -> Self
    where
        for<'a> E::Trigger<'a>: Default,
    {
        Self {
            trigger: Box::new(move |commands: &mut Commands| {
                commands.trigger(event.clone());
            }),
        }
    }

    pub fn message<M: Message + Clone>(message: M) -> Self {
        Self {
            trigger: Box::new(move |commands: &mut Commands| {
                let message = message.clone();
                commands.run_system_cached(move |mut writer: MessageWriter<M>| {
                    writer.write(message.clone());
                });
            }),
        }
    }

    pub fn fire(&self, commands: &mut Commands) {
        (self.trigger)(commands);
    }
}
