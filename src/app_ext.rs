use crate::commands::*;
use bevy::prelude::*;

pub trait ConsoleAppExt {
    fn add_system_command<S: Into<String>, M: 'static>(
        &mut self,
        string: S,
        system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    ) -> &mut Self;

    fn add_system_piped_command<S: Into<String>, M: 'static>(
        &mut self,
        string: S,
        system: impl IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static,
    ) -> &mut Self;

    fn add_event<E>(&mut self) -> &mut Self
    where
        E: Event + Default + Clone,
        for<'a> E::Trigger<'a>: Default;

    fn add_event_command<S, E>(&mut self, string: S, event: E) -> &mut Self
    where
        S: Into<String>,
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default;

    fn add_message<M: Message + Default + Clone>(&mut self) -> &mut Self;

    fn add_message_command<S: Into<String>, M: Message + Clone>(
        &mut self,
        string: S,
        message: M,
    ) -> &mut Self;
}

impl ConsoleAppExt for App {
    /// Runs the given system when the command is called.
    fn add_system_command<S: Into<String>, M: 'static>(
        &mut self,
        string: S,
        system: impl IntoSystem<(), (), M> + Send + Sync + 'static,
    ) -> &mut Self {
        let world = self.world_mut();
        let system = world.register_system(system);
        world.spawn((Command(string.into()), CommandExec::system(system)));
        self
    }

    /// Runs the given system when the command is called.
    /// The system requires that it accept ```In<Vec<String>>``` in addition to its other arguements
    /// ```
    /// fn my_epic_system(In(arguements): In<Vec<String>>, mut commands: Commands, query: Query<&mut EpicComponenet>) {}
    fn add_system_piped_command<S: Into<String>, M: 'static>(
        &mut self,
        string: S,
        system: impl IntoSystem<In<Vec<String>>, (), M> + Send + Sync + 'static,
    ) -> &mut Self {
        let world = self.world_mut();
        let system = world.register_system(system);
        world.spawn((Command(string.into()), CommandExec::system_piped(system)));
        self
    }

    /// Add a command which is called by the event type name and
    /// calls the events default implementation
    fn add_event<E>(&mut self) -> &mut Self
    where
        E: Event + Default + Clone,
        for<'a> E::Trigger<'a>: Default,
    {
        let name = short_type_name::<E>();
        self.add_event_command(name, E::default());
        self
    }

    fn add_event_command<S, E>(&mut self, string: S, event: E) -> &mut Self
    where
        S: Into<String>,
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default,
    {
        let world = self.world_mut();
        world.spawn((Command(string.into()), CommandExec::event(event)));
        self
    }

    fn add_message<M: Message + Default + Clone>(&mut self) -> &mut Self {
        let name = short_type_name::<M>();
        self.add_message_command(name, M::default());
        self
    }

    fn add_message_command<S: Into<String>, M: Message + Clone>(
        &mut self,
        string: S,
        message: M,
    ) -> &mut Self {
        let world = self.world_mut();
        world.spawn((Command(string.into()), CommandExec::message(message)));
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
