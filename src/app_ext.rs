use crate::{
    arguments::Argument,
    commands::*,
    into_command::{CommandBuilder, IntoCommand, IntoCommands, short_type_name},
};
use bevy::prelude::*;

/// Adds console commands to an [`App`].
///
/// Commands are named after the function, event or message, without its module path. Closures have
/// no usable name, so give those one.
pub trait ConsoleAppExt {
    /// Adds a function, closure, event or message as a command. See [`IntoCommand`] for everything
    /// that is accepted.
    /// ```
    /// use bevy::prelude::*;
    /// use bevy_console::prelude::*;
    ///
    /// fn pause(mut time: ResMut<Time<Virtual>>) {
    ///     time.pause();
    /// }
    ///
    /// // Arguments are read from what comes after the name, and the text is shown.
    /// fn teleport(In((x, y)): In<(f32, f32)>, mut query: Query<&mut Transform>) -> String {
    ///     for mut transform in &mut query {
    ///         transform.translation = Vec3::new(x, y, 0.0);
    ///     }
    ///     format!("Teleported to {x}, {y}.")
    /// }
    ///
    /// // Structs give the arguments names.
    /// /// Gives someone items.
    /// #[derive(Reflect)]
    /// struct Give {
    ///     player: String,
    ///     /// 1 by default.
    ///     amount: Option<u32>,
    /// }
    ///
    /// fn give(In(give): In<Give>) -> String {
    ///     format!("Gave {} to {}.", give.amount.unwrap_or(1), give.player)
    /// }
    ///
    /// #[derive(Event, Clone)]
    /// struct Explosion {
    ///     radius: f32,
    /// }
    ///
    /// App::new()
    ///     .add_command(pause)
    ///     .add_command(teleport.about("Moves everything."))
    ///     .add_command(give)
    ///     .add_command(Explosion { radius: 5.0 })
    ///     .add_command(("hi", || "Hello!"))
    ///     .add_command(
    ///         pause
    ///             .named("stop")
    ///             .about("Same as pause, but only with a camera.")
    ///             .available_if(any_with_component::<Camera>),
    ///     );
    /// ```
    fn add_command<M>(&mut self, command: impl IntoCommand<M>) -> &mut Self;

    /// Adds a tuple of commands.
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_console::prelude::*;
    /// # fn pause() {}
    /// # fn speed(In(speed): In<f32>) {}
    /// App::new().add_commands((pause, speed.about("Sets the speed."), ("hi", || "Hello!")));
    /// ```
    fn add_commands<M>(&mut self, commands: impl IntoCommands<M>) -> &mut Self;

    /// Runs the given system when the command is called.
    #[deprecated = "use `add_command((name, system))`"]
    fn add_command_named<S: Into<String>, M: 'static>(
        &mut self,
        name: S,
        system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    ) -> &mut Self;

    /// Runs the given system when the command is called, passing it the arguments which came after
    /// the name of the command.
    #[deprecated = "use `add_command(system)`, which accepts systems with any arguments"]
    fn add_command_piped<M: 'static, F>(&mut self, system: F) -> &mut Self
    where
        F: IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static;

    /// Like `add_command_piped`, with a custom name.
    #[deprecated = "use `add_command((name, system))`"]
    fn add_command_piped_named<S: Into<String>, M: 'static>(
        &mut self,
        name: S,
        system: impl IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static,
    ) -> &mut Self;

    /// Triggers a clone of the given event when the command is called.
    #[deprecated = "use `add_command(event)`"]
    fn add_command_event<E>(&mut self, event: E) -> &mut Self
    where
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default;

    /// Like `add_command_event`, with a custom name.
    #[deprecated = "use `add_command((name, event))`"]
    fn add_command_event_named<S, E>(&mut self, name: S, event: E) -> &mut Self
    where
        S: Into<String>,
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default;

    /// Writes the default value of the message when the command is called.
    /// Also registers the message, if that has not been done yet.
    #[deprecated = "use `add_command(message)`"]
    fn add_command_message<M: Message + Default + Clone>(&mut self) -> &mut Self;

    /// Writes a clone of the given message when the command is called.
    /// Also registers the message, if that has not been done yet.
    #[deprecated = "use `add_command((name, message))`"]
    fn add_command_message_named<S: Into<String>, M: Message + Clone>(
        &mut self,
        name: S,
        message: M,
    ) -> &mut Self;

    /// Replaces what `help` shows for every command with the given name.
    /// The commands must have been added already. For your own commands, prefer
    /// [`about`](IntoCommand::about).
    fn describe_command(
        &mut self,
        name: &str,
        description: impl Into<String>,
        usage: impl Into<String>,
    ) -> &mut Self;
}

#[allow(deprecated)]
impl ConsoleAppExt for App {
    fn add_command<M>(&mut self, command: impl IntoCommand<M>) -> &mut Self {
        spawn_command(self.world_mut(), command.into_command());
        self
    }

    fn add_commands<M>(&mut self, commands: impl IntoCommands<M>) -> &mut Self {
        let mut builders = Vec::new();
        commands.push_to(&mut builders);
        for builder in builders {
            spawn_command(self.world_mut(), builder);
        }
        self
    }

    fn add_command_named<S: Into<String>, M: 'static>(
        &mut self,
        name: S,
        system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    ) -> &mut Self {
        let world = self.world_mut();
        let system = world.register_system(system);
        world.spawn((ConsoleCommand(name.into()), CommandExec::system(system)));
        self
    }

    fn add_command_piped<M: 'static, F>(&mut self, system: F) -> &mut Self
    where
        F: IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static,
    {
        self.add_command_piped_named(short_type_name::<F>(), system)
    }

    fn add_command_piped_named<S: Into<String>, M: 'static>(
        &mut self,
        name: S,
        system: impl IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static,
    ) -> &mut Self {
        let world = self.world_mut();
        let system = world.register_system(system);
        world.spawn((
            ConsoleCommand(name.into()),
            CommandExec::system_piped(system),
        ));
        self
    }

    fn add_command_event<E>(&mut self, event: E) -> &mut Self
    where
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default,
    {
        self.add_command_event_named(short_type_name::<E>(), event)
    }

    fn add_command_event_named<S, E>(&mut self, name: S, event: E) -> &mut Self
    where
        S: Into<String>,
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default,
    {
        self.world_mut()
            .spawn((ConsoleCommand(name.into()), CommandExec::event(event)));
        self
    }

    fn add_command_message<M: Message + Default + Clone>(&mut self) -> &mut Self {
        self.add_command_message_named(short_type_name::<M>(), M::default())
    }

    fn add_command_message_named<S: Into<String>, M: Message + Clone>(
        &mut self,
        name: S,
        message: M,
    ) -> &mut Self {
        self.add_message::<M>();
        self.world_mut()
            .spawn((ConsoleCommand(name.into()), CommandExec::message(message)));
        self
    }

    fn describe_command(
        &mut self,
        name: &str,
        description: impl Into<String>,
        usage: impl Into<String>,
    ) -> &mut Self {
        let metadata = CommandMetadata {
            description: description.into(),
            usage: usage.into(),
        };
        let world = self.world_mut();
        let entities: Vec<Entity> = world
            .query::<(Entity, &ConsoleCommand)>()
            .iter(world)
            .filter(|(_, command)| command.0 == name)
            .map(|(entity, _)| entity)
            .collect();
        if entities.is_empty() {
            warn!(
                "The command {} cannot be described, as it was not found.",
                name
            );
        }
        for entity in entities {
            world.entity_mut(entity).insert(metadata.clone());
        }
        self
    }
}

fn spawn_command(world: &mut World, builder: CommandBuilder) {
    let CommandBuilder {
        name,
        description,
        usage,
        arguments,
        unsupported,
        condition,
        run,
    } = builder;

    if name.contains("{{closure}}") {
        warn!("The closure {name} was added as a command, but has no name that can be typed.");
    }
    if let Some(path) = unsupported {
        error!("The command {name} cannot be used, as {path} cannot be read from text.");
    }
    let usage = usage.unwrap_or_else(|| {
        let mut words = vec![name.clone()];
        words.extend(arguments.iter().map(Argument::usage));
        words.join(" ")
    });

    let run = run(world);
    let condition = condition.map(|condition| CommandCondition(condition(world)));
    let mut entity = world.spawn((
        ConsoleCommand(name),
        CommandExec::Run(run),
        CommandMetadata { description, usage },
        CommandArguments(arguments),
    ));
    if let Some(condition) = condition {
        entity.insert(condition);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() {}

    #[test]
    fn describe_command_applies_to_every_command_with_the_name() {
        let mut app = App::new();
        app.add_command(("twin", system))
            .add_command(("twin", system))
            .add_command(("other", system))
            .describe_command("twin", "One of two.", "twin");

        let world = app.world_mut();
        let described: Vec<_> = world
            .query::<(&ConsoleCommand, &CommandMetadata)>()
            .iter(world)
            .map(|(command, metadata)| (command.0.as_str(), metadata.description.as_str()))
            .filter(|(_, description)| !description.is_empty())
            .collect();
        assert_eq!(described, [("twin", "One of two."); 2]);
    }

    #[test]
    fn describing_an_unknown_command_does_nothing() {
        let mut app = App::new();
        app.describe_command("missing", "", "");
        let world = app.world_mut();
        assert_eq!(world.query::<&CommandMetadata>().iter(world).count(), 0);
    }
}
