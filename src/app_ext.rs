use crate::commands::*;
use bevy::prelude::*;

/// Adds console commands to an [`App`].
///
/// Methods that do not end in `_named` name the command after the type of the system, event or
/// message, without its module path. Closures have no usable type name, so give those a name.
pub trait ConsoleAppExt {
    /// Runs the given system when the command is called.
    fn add_command<M: 'static, F>(&mut self, system: F) -> &mut Self
    where
        F: IntoSystem<(), (), M> + Send + Sync + 'static;

    /// Like [`add_command`](Self::add_command), with a custom name.
    fn add_command_named<S: Into<String>, M: 'static>(
        &mut self,
        name: S,
        system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    ) -> &mut Self;

    /// Runs the given system when the command is called, passing it the whitespace separated
    /// arguments which came after the name of the command.
    ///
    /// The system must accept `In<Vec<String>>` in addition to its other parameters.
    /// ```
    /// use bevy::prelude::*;
    /// use bevy_console::prelude::*;
    ///
    /// fn teleport(In(args): In<Vec<String>>, mut query: Query<&mut Transform>) {
    ///     let Some(x) = args.first().and_then(|x| x.parse().ok()) else {
    ///         warn!("Usage: teleport <x>");
    ///         return;
    ///     };
    ///     for mut transform in &mut query {
    ///         transform.translation.x = x;
    ///     }
    /// }
    ///
    /// App::new().add_command_piped(teleport);
    /// ```
    fn add_command_piped<M: 'static, F>(&mut self, system: F) -> &mut Self
    where
        F: IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static;

    /// Like [`add_command_piped`](Self::add_command_piped), with a custom name.
    fn add_command_piped_named<S: Into<String>, M: 'static>(
        &mut self,
        name: S,
        system: impl IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static,
    ) -> &mut Self;

    /// Triggers a clone of the given event when the command is called.
    fn add_command_event<E>(&mut self, event: E) -> &mut Self
    where
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default;

    /// Like [`add_command_event`](Self::add_command_event), with a custom name.
    fn add_command_event_named<S, E>(&mut self, name: S, event: E) -> &mut Self
    where
        S: Into<String>,
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default;

    /// Writes the default value of the message when the command is called.
    /// Also registers the message, if that has not been done yet.
    fn add_command_message<M: Message + Default + Clone>(&mut self) -> &mut Self;

    /// Writes a clone of the given message when the command is called.
    /// Also registers the message, if that has not been done yet.
    fn add_command_message_named<S: Into<String>, M: Message + Clone>(
        &mut self,
        name: S,
        message: M,
    ) -> &mut Self;

    /// Sets what `help` shows for every command with the given name.
    /// The commands must have been added already.
    fn describe_command(
        &mut self,
        name: &str,
        description: impl Into<String>,
        usage: impl Into<String>,
    ) -> &mut Self;
}

impl ConsoleAppExt for App {
    fn add_command<M: 'static, F>(&mut self, system: F) -> &mut Self
    where
        F: IntoSystem<(), (), M> + Send + Sync + 'static,
    {
        self.add_command_named(short_type_name::<F>(), system)
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

fn short_type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    // Find the last '::' that isn't inside angle brackets
    let mut depth = 0usize;
    let mut last_colon = 0;
    let bytes = name.as_bytes();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' => depth = depth.saturating_sub(1),
            b':' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b':' => {
                last_colon = i + 2;
            }
            _ => {}
        }
    }
    &name[last_colon..]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Plain;
    struct Generic<T>(std::marker::PhantomData<T>);

    fn system() {}

    fn name_of<T>(_: T) -> &'static str {
        short_type_name::<T>()
    }

    #[test]
    fn short_type_name_removes_the_module_path() {
        assert_eq!(short_type_name::<Plain>(), "Plain");
        assert_eq!(short_type_name::<u32>(), "u32");
        assert_eq!(name_of(system), "system");
    }

    #[test]
    fn short_type_name_keeps_the_path_of_generics() {
        assert_eq!(
            short_type_name::<Generic<Plain>>(),
            format!("Generic<{}>", std::any::type_name::<Plain>())
        );
    }

    #[test]
    fn describe_command_applies_to_every_command_with_the_name() {
        let mut app = App::new();
        app.add_command_named("twin", system)
            .add_command_named("twin", system)
            .add_command_named("other", system)
            .describe_command("twin", "One of two.", "twin");

        let world = app.world_mut();
        let described: Vec<_> = world
            .query::<(&ConsoleCommand, &CommandMetadata)>()
            .iter(world)
            .map(|(command, metadata)| (command.0.as_str(), metadata.description.as_str()))
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
