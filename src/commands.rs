use crate::{
    arguments::{self, Argument},
    into_command::CommandError,
    simple,
};
use bevy::{ecs::system::SystemId, prelude::*};
use std::sync::Arc;

/// Runs a command. This is triggered when something is submitted in the text input, but can be
/// triggered by anything else as well:
/// ```
/// # use bevy::prelude::*;
/// # use bevy_console::prelude::*;
/// fn on_click(mut commands: Commands) {
///     commands.trigger(TryCommand::from_entry("spawn 20".into()));
/// }
/// ```
/// The first element is the name of the command, the others are its arguments.
#[derive(Event)]
pub struct TryCommand(pub Vec<String>);

impl TryCommand {
    /// Splits the text by whitespace, except inside of double quotes.
    pub fn from_entry(string: String) -> Self {
        TryCommand(arguments::split(&string))
    }
}

pub fn try_command(
    trigger: On<TryCommand>,
    mut commands: Commands,
    query: Query<(
        &ConsoleCommand,
        &CommandExec,
        Option<&CommandMetadata>,
        Option<&CommandCondition>,
    )>,
) {
    let Some(input) = trigger.0.first() else {
        return;
    };

    let mut found = 0;

    for (command, exec, metadata, condition) in query {
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
                CommandExec::Run(run) => {
                    let run = run.clone();
                    let name = command.0.clone();
                    let usage = metadata.map(|metadata| metadata.usage.clone());
                    let condition = condition.map(|condition| condition.0);
                    let words = trigger.0[1..].to_vec();
                    commands.queue(move |world: &mut World| {
                        run_command(world, &run, &name, usage, condition, &words);
                    });
                }
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

fn run_command(
    world: &mut World,
    run: &RunCommand,
    name: &str,
    usage: Option<String>,
    condition: Option<SystemId<(), bool>>,
    words: &[String],
) {
    if let Some(condition) = condition {
        match world.run_system(condition) {
            Ok(true) => {}
            Ok(false) => {
                warn!("The command {name} is not available right now.");
                return;
            }
            Err(error) => {
                error!("The condition of the command {name} failed: {error}");
                return;
            }
        }
    }
    match (run.0)(world, words) {
        Ok(text) if text.is_empty() => {}
        Ok(text) => simple!("{text}"),
        Err(CommandError::Arguments(error)) => match usage {
            Some(usage) => error!("{name}: {error}. Usage: {usage}"),
            None => error!("{name}: {error}."),
        },
        Err(CommandError::Failed(error)) => error!("{name}: {error}"),
    }
}

/// What the `help` command shows for a [`ConsoleCommand`].
/// Usually set with [`about`](crate::prelude::IntoCommand::about).
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
    Run(RunCommand),
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

/// The arguments that `help` lists.
#[derive(Component)]
pub(crate) struct CommandArguments(pub Vec<Argument>);

/// The command is only executed while this returns true.
#[derive(Component)]
pub(crate) struct CommandCondition(pub SystemId<(), bool>);

type RunFn = dyn Fn(&mut World, &[String]) -> Result<String, CommandError> + Send + Sync;

/// Made by [`add_command`](crate::prelude::ConsoleAppExt::add_command).
#[derive(Clone)]
pub struct RunCommand(Arc<RunFn>);

impl RunCommand {
    pub(crate) fn new(
        run: impl Fn(&mut World, &[String]) -> Result<String, CommandError> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(run))
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
    use crate::{
        app_ext::ConsoleAppExt,
        into_command::{CommandBuilder, IntoCommand},
    };

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

    #[derive(Reflect)]
    struct Give {
        player: String,
        amount: Option<u32>,
    }

    fn give(In(give): In<Give>, mut calls: ResMut<Calls>) {
        calls.0.push(format!("{} {:?}", give.player, give.amount));
    }

    #[test]
    fn systems_are_named_after_themselves() {
        let mut app = app();
        app.add_command(record);
        run(&mut app, "record");
        run(&mut app, "record unexpected");
        assert_eq!(calls(&app), ["record"]);
    }

    #[test]
    fn systems_receive_any_arguments() {
        let mut app = app();
        app.add_command(record_args)
            .add_command(("alias", record_args))
            .add_command(give)
            .add_command((
                "add",
                |In((a, b)): In<(u32, u32)>, mut calls: ResMut<Calls>| {
                    calls.0.push((a + b).to_string());
                },
            ));
        run(&mut app, "record_args a b");
        run(&mut app, "alias");
        run(&mut app, "give \"two words\" 3");
        run(&mut app, "give Al");
        run(&mut app, "add 1 2");
        for wrong in ["give", "give Al x", "give Al 1 2", "add 1", "add 1 2 3"] {
            run(&mut app, wrong);
        }
        assert_eq!(
            calls(&app),
            ["a|b", "", "two words Some(3)", "Al None", "3"]
        );
    }

    #[test]
    fn systems_may_access_the_whole_world() {
        let mut app = app();
        app.add_command(("world", |world: &mut World| {
            world.resource_mut::<Calls>().0.push("world".into());
        }))
        .add_command(("set", |In(text): In<String>, world: &mut World| {
            world.resource_mut::<Calls>().0.push(text);
        }));
        run(&mut app, "world");
        run(&mut app, "set x");
        assert_eq!(calls(&app), ["world", "x"]);
    }

    #[test]
    fn systems_may_fail() {
        fn fails(In(number): In<String>, mut calls: ResMut<Calls>) -> Result<String> {
            let number: u32 = number.parse()?;
            calls.0.push(number.to_string());
            Ok("parsed".into())
        }
        let mut app = app();
        app.add_command(fails)
            .add_command(("text", || Err::<(), _>("a text error")))
            .add_command(("missing", |_: Res<Missing>| {}));
        for entry in ["fails x", "text", "missing", "fails 1"] {
            run(&mut app, entry);
        }
        assert_eq!(calls(&app), ["1"]);
    }

    #[derive(Resource)]
    struct Missing;

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
        app.add_command(("twin", record))
            .add_command(("twin", |mut calls: ResMut<Calls>| {
                calls.0.push("closure".into());
            }));
        run(&mut app, "twin");
        assert_eq!(calls(&app), ["record", "closure"]);
    }

    #[test]
    fn groups_add_every_kind() {
        let mut app = app();
        app.add_commands((
            record,
            ("named", record),
            (String::from("owned"), record.about("Is described.")),
            (Damage(1), Unit, (give, record.named("nested"))),
        ))
        .add_commands(record.named("single"))
        .add_commands(("also_single", record));
        for entry in [
            "record",
            "named",
            "owned",
            "nested",
            "single",
            "also_single",
        ] {
            run(&mut app, entry);
        }
        assert_eq!(calls(&app), ["record"; 6]);
        let world = app.world_mut();
        assert_eq!(world.query::<&ConsoleCommand>().iter(world).count(), 9);
    }

    #[test]
    fn commands_can_be_unavailable() {
        fn few(calls: Res<Calls>) -> bool {
            calls.0.len() < 2
        }
        let mut app = app();
        app.add_command(record.available_if(few))
            .add_command(("event", Damage(1).available_if(not(few))))
            .add_observer(|_: On<Damage>, mut calls: ResMut<Calls>| {
                calls.0.push("event".into());
            });
        for entry in ["event", "record", "record", "record", "event"] {
            run(&mut app, entry);
        }
        assert_eq!(calls(&app), ["record", "record", "event"]);
    }

    #[test]
    fn metadata_comes_from_the_command() {
        /// Is documented.
        #[derive(Reflect)]
        struct Documented {
            number: u32,
        }
        fn documented(In(_): In<Documented>) {}

        let mut app = app();
        app.add_commands((
            record,
            documented,
            documented.about("Overridden.").usage("custom").named("a"),
            ("b", documented.about("Renamed.")),
        ));
        let world = app.world_mut();
        let mut metadata: Vec<_> = world
            .query::<(&ConsoleCommand, &CommandMetadata)>()
            .iter(world)
            .map(|(name, metadata)| {
                (
                    name.0.as_str(),
                    metadata.description.as_str(),
                    metadata.usage.as_str(),
                )
            })
            .collect();
        metadata.sort();
        assert_eq!(
            metadata,
            [
                ("a", "Overridden.", "custom"),
                ("b", "Renamed.", "b <number: u32>"),
                ("documented", "Is documented.", "documented <number: u32>"),
                ("record", "", "record"),
            ]
        );
    }

    #[test]
    fn events_do_not_need_a_default() {
        let mut app = app();
        app.add_command(Damage(1))
            .add_command(("crit", Damage(9)))
            .add_observer(|damage: On<Damage>, mut calls: ResMut<Calls>| {
                calls.0.push(damage.0.to_string());
            });
        run(&mut app, "Damage");
        run(&mut app, "Damage 5");
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
        app.add_command(Score::default())
            .add_command(("win", score.clone()));
        run(&mut app, "Score");
        run(&mut app, "win");
        run(&mut app, "win");
        assert_eq!(
            written::<Score>(&app),
            [Score::default(), score.clone(), score]
        );
    }

    #[test]
    fn registering_a_message_again_keeps_what_was_written() {
        let mut app = app();
        app.add_command(Unit);
        run(&mut app, "Unit");
        app.add_command(("again", Unit));
        assert_eq!(written::<Unit>(&app).len(), 1);
    }

    #[test]
    fn types_that_are_both_can_be_told_apart() {
        #[derive(Event, Message, Clone)]
        struct Both;
        let mut app = app();
        app.add_command(CommandBuilder::message(Both))
            .add_command(CommandBuilder::event(Both).named("event"))
            .add_observer(|_: On<Both>, mut calls: ResMut<Calls>| {
                calls.0.push("event".into());
            });
        run(&mut app, "Both");
        assert!(calls(&app).is_empty());
        assert_eq!(written::<Both>(&app).len(), 1);
        run(&mut app, "event");
        assert_eq!(calls(&app), ["event"]);
    }

    #[test]
    fn commands_can_run_commands() {
        let mut app = app();
        app.add_command(record)
            .add_command(("outer", |mut commands: Commands| {
                commands.trigger(TryCommand::from_entry("record".into()));
            }));
        run(&mut app, "outer");
        assert_eq!(calls(&app), ["record"]);
    }

    #[test]
    fn unsupported_arguments_are_harmless() {
        let mut app = app();
        app.add_command((
            "wait",
            |In(_): In<std::time::Duration>, mut calls: ResMut<Calls>| {
                calls.0.push("waited".into());
            },
        ));
        run(&mut app, "wait 1");
        assert!(calls(&app).is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_methods_still_work() {
        let mut app = app();
        app.add_command_named("a", record)
            .add_command_piped(record_args)
            .add_command_piped_named("b", record_args)
            .add_command_event(Damage(1))
            .add_command_event_named("crit", Damage(9))
            .add_command_message::<Score>()
            .add_command_message_named("again", Score::default())
            .add_observer(|damage: On<Damage>, mut calls: ResMut<Calls>| {
                calls.0.push(damage.0.to_string());
            });
        for entry in [
            "a",
            "a ignored",
            "record_args x y",
            "b z",
            "Damage",
            "crit",
            "Score",
            "again",
        ] {
            run(&mut app, entry);
        }
        assert_eq!(calls(&app), ["record", "record", "x|y", "z", "1", "9"]);
        assert_eq!(written::<Score>(&app).len(), 2);
    }

    #[test]
    fn a_command_without_exec_is_harmless() {
        let mut app = app();
        app.world_mut().spawn(ConsoleCommand("empty".into()));
        run(&mut app, "empty");
        assert!(calls(&app).is_empty());
    }
}
