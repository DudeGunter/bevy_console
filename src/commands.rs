use bevy::{ecs::system::SystemId, prelude::*};

/// Text submitted from the text input, split by whitespace.
/// The first element is the name of the command, the others are its arguments.
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
    query: Query<(&ConsoleCommand, &CommandExec)>,
) {
    let Some(input) = trigger.0.first() else {
        return;
    };

    let mut found = 0;

    for (command, exec) in query {
        if input == command.0.as_str() {
            found += 1;
            match exec {
                CommandExec::System(id) => commands.run_system(*id),
                CommandExec::SystemPiped(id) => {
                    let mut args = trigger.0.clone();
                    args.remove(0);
                    commands.run_system_with(*id, args);
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

/// What the `help` command shows for a [`ConsoleCommand`].
/// Usually set with [`describe_command`](crate::prelude::ConsoleAppExt::describe_command).
#[derive(Component, Reflect, Clone)]
pub struct CommandMetadata {
    pub description: String,
    pub usage: String,
}

/// The name of a command. Every entity with a matching name is executed.
#[derive(Component, Reflect, Clone, PartialEq, Eq)]
#[require(CommandExec)]
pub struct ConsoleCommand(pub String);

/// What a [`ConsoleCommand`] does when it is called.
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

type DeferredFn = dyn Fn(&mut Commands) + Send + Sync;

/// Queues something on [`Commands`] when fired.
pub struct DeferredCommand {
    trigger: Box<DeferredFn>,
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
                commands.write_message(message.clone());
            }),
        }
    }

    pub fn fire(&self, commands: &mut Commands) {
        (self.trigger)(commands);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_ext::ConsoleAppExt;

    #[derive(Resource, Default)]
    struct Calls(Vec<String>);

    #[derive(Event, Clone)]
    struct Damage(u32);

    #[derive(Message, Clone, Default, Debug, PartialEq)]
    struct Score {
        points: u32,
        player: String,
    }

    #[derive(Message, Clone, Default)]
    struct Unit;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Calls>().add_observer(try_command);
        app
    }

    fn run(app: &mut App, entry: &str) {
        app.world_mut()
            .trigger(TryCommand::from_entry(entry.to_string()));
        app.world_mut().flush();
    }

    fn calls(app: &App) -> &[String] {
        &app.world().resource::<Calls>().0
    }

    fn written<M: Message + Clone>(app: &App) -> Vec<M> {
        let messages = app.world().resource::<Messages<M>>();
        messages.get_cursor().read(messages).cloned().collect()
    }

    fn record(mut calls: ResMut<Calls>) {
        calls.0.push("record".into());
    }

    fn record_args(In(args): In<Vec<String>>, mut calls: ResMut<Calls>) {
        calls.0.push(args.join("|"));
    }

    #[test]
    fn entry_is_split_by_whitespace() {
        let command = TryCommand::from_entry("  give  apple\t3 ".into());
        assert_eq!(command.0, ["give", "apple", "3"]);
        assert!(TryCommand::from_entry("   ".into()).0.is_empty());
    }

    #[test]
    fn systems_are_named_after_themselves() {
        let mut app = app();
        app.add_command(record);
        run(&mut app, "record");
        run(&mut app, "record ignored arguments");
        assert_eq!(calls(&app), ["record", "record"]);
    }

    #[test]
    fn piped_systems_receive_arguments() {
        let mut app = app();
        app.add_command_piped(record_args)
            .add_command_piped_named("alias", record_args);
        run(&mut app, "record_args a b");
        run(&mut app, "alias");
        assert_eq!(calls(&app), ["a|b", ""]);
    }

    #[test]
    fn names_must_match_exactly() {
        let mut app = app();
        app.add_command(record);
        for entry in ["", "  ", "Record", "rec", "records", "missing"] {
            run(&mut app, entry);
        }
        assert!(calls(&app).is_empty());
    }

    #[test]
    fn duplicate_names_all_run() {
        let mut app = app();
        app.add_command_named("twin", record).add_command_named(
            "twin",
            |mut calls: ResMut<Calls>| {
                calls.0.push("closure".into());
            },
        );
        run(&mut app, "twin");
        assert_eq!(calls(&app), ["record", "closure"]);
    }

    #[test]
    fn events_do_not_need_a_default() {
        let mut app = app();
        app.add_command_event(Damage(1))
            .add_command_event_named("crit", Damage(9))
            .add_observer(|damage: On<Damage>, mut calls: ResMut<Calls>| {
                calls.0.push(damage.0.to_string());
            });
        run(&mut app, "Damage");
        run(&mut app, "crit");
        assert_eq!(calls(&app), ["1", "9"]);
    }

    #[test]
    fn messages_can_have_fields() {
        let score = Score {
            points: 7,
            player: "ferris".into(),
        };
        let mut app = app();
        app.add_command_message::<Score>()
            .add_command_message_named("win", score.clone());
        run(&mut app, "Score");
        run(&mut app, "win");
        run(&mut app, "win");
        assert_eq!(
            written::<Score>(&app),
            [Score::default(), score.clone(), score]
        );
    }

    #[test]
    fn messages_can_be_unit_structs() {
        let mut app = app();
        app.add_command_message::<Unit>();
        run(&mut app, "Unit");
        assert_eq!(written::<Unit>(&app).len(), 1);
    }

    #[test]
    fn registering_a_message_again_keeps_what_was_written() {
        let mut app = app();
        app.add_command_message::<Unit>();
        run(&mut app, "Unit");
        app.add_command_message_named::<_, Unit>("again", Unit);
        assert_eq!(written::<Unit>(&app).len(), 1);
    }

    #[test]
    fn a_command_without_exec_is_harmless() {
        let mut app = app();
        app.world_mut().spawn(ConsoleCommand("empty".into()));
        run(&mut app, "empty");
        assert!(calls(&app).is_empty());
    }
}
