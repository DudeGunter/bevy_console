use crate::{
    arguments::{self, Argument},
    commands::RunCommand,
};
use bevy::{
    ecs::{
        message::MessageRegistry,
        system::{SystemId, SystemInput, SystemParamFunction},
    },
    prelude::*,
    reflect::Typed,
};
use std::fmt::Display;

/// Anything that can be given to [`add_command`](crate::prelude::ConsoleAppExt::add_command).
///
/// - Functions and closures that are systems. They may take arguments with [`In`], see
///   [`ConsoleInput`], and return something to show, see [`ConsoleOutput`].
/// - Events and messages, of which a clone is triggered or written.
/// - Any of those in a tuple with a name, like `("name", system)`.
/// - Any of those after [`named`](Self::named), [`about`](Self::about), [`usage`](Self::usage) or
///   [`available_if`](Self::available_if).
///
/// `M` only tells these apart, and never has to be written.
///
/// Combined systems, such as those made by `pipe`, have to be wrapped in a function.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a command",
    label = "not a valid command",
    note = "systems may take `In<T>`, if `T` implements `Reflect`",
    note = "systems may return `()`, text, a number, a `bool`, or a `Result` of those",
    note = "events and messages must implement `Clone`, and events must not need a target"
)]
pub trait IntoCommand<M>: Sized {
    fn into_command(self) -> CommandBuilder;

    /// Sets the name to type. By default this is the name of the function or type.
    fn named(self, name: impl Into<String>) -> CommandBuilder {
        let mut builder = self.into_command();
        builder.name = name.into();
        builder
    }

    /// Sets the description shown by `help`. By default this is the doc comment of the argument
    /// type, if the system takes one. Doc comments of functions can't be read.
    fn about(self, description: impl Into<String>) -> CommandBuilder {
        let mut builder = self.into_command();
        builder.description = description.into();
        builder
    }

    /// Replaces the generated usage shown by `help`, which lists all arguments.
    fn usage(self, usage: impl Into<String>) -> CommandBuilder {
        let mut builder = self.into_command();
        builder.usage = Some(usage.into());
        builder
    }

    /// Only runs the command while the condition is true. Takes the same conditions as
    /// [`run_if`](IntoScheduleConfigs::run_if). The last one set replaces any before it.
    fn available_if<N>(
        self,
        condition: impl IntoSystem<(), bool, N> + Send + Sync + 'static,
    ) -> CommandBuilder {
        let mut builder = self.into_command();
        builder.condition = Some(Box::new(move |world| world.register_system(condition)));
        builder
    }
}

type Setup<T> = Box<dyn FnOnce(&mut World) -> T + Send + Sync>;

/// A command that is not part of an [`App`] yet. Made by the methods of [`IntoCommand`].
#[must_use = "does nothing until it is given to `add_command`"]
pub struct CommandBuilder {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) usage: Option<String>,
    pub(crate) arguments: Vec<Argument>,
    pub(crate) unsupported: Option<&'static str>,
    pub(crate) condition: Option<Setup<SystemId<(), bool>>>,
    pub(crate) run: Setup<RunCommand>,
}

impl CommandBuilder {
    fn new(name: &str, run: impl FnOnce(&mut World) -> RunCommand + Send + Sync + 'static) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            usage: None,
            arguments: Vec::new(),
            unsupported: None,
            condition: None,
            run: Box::new(run),
        }
    }

    /// Triggers a clone of the event. Only needed for types which are also a [`Message`].
    pub fn event<E>(event: E) -> Self
    where
        E: Event + Clone,
        for<'a> E::Trigger<'a>: Default,
    {
        Self::new(short_type_name::<E>(), |_| {
            RunCommand::new(move |world, words| {
                no_arguments(words)?;
                world.trigger(event.clone());
                Ok(String::new())
            })
        })
    }

    /// Writes a clone of the message, and registers it if that has not been done yet.
    /// Only needed for types which are also an [`Event`].
    pub fn message<T: Message + Clone>(message: T) -> Self {
        Self::new(short_type_name::<T>(), |world| {
            if !world.contains_resource::<Messages<T>>() {
                MessageRegistry::register_message::<T>(world);
            }
            RunCommand::new(move |world, words| {
                no_arguments(words)?;
                world.write_message(message.clone());
                Ok(String::new())
            })
        })
    }

    fn system<F, M>(system: F) -> Self
    where
        M: 'static,
        F: SystemParamFunction<M, In: ConsoleInput, Out: ConsoleOutput>,
    {
        let mut builder = Self::new(short_type_name::<F>(), |world| {
            let id = world.register_system::<F::In, F::Out, _>(system);
            RunCommand::new(move |world, words| F::In::run(world, id, words))
        });
        F::In::describe(&mut builder);
        builder
    }
}

/// Why a command did not do anything.
pub enum CommandError {
    /// The arguments were wrong, so the usage should be shown.
    Arguments(String),
    /// The command itself failed.
    Failed(String),
}

fn no_arguments(words: &[String]) -> Result<(), CommandError> {
    match words.first() {
        Some(word) => Err(CommandError::Arguments(format!(
            "unexpected argument {word}"
        ))),
        None => Ok(()),
    }
}

mod sealed {
    use super::*;

    pub trait Input: SystemInput + Sized + 'static {
        fn describe(builder: &mut CommandBuilder);

        fn run<O: ConsoleOutput>(
            world: &mut World,
            system: SystemId<Self, O>,
            words: &[String],
        ) -> Result<String, CommandError>;
    }

    pub trait Output {
        fn into_text(self) -> Result<String, String>;
    }
}
use sealed::{Input, Output};

/// What a command may take: nothing, or `In<T>` if `T` implements [`Reflect`].
///
/// Arguments are separated by whitespace, unless they are inside of double quotes.
///
/// | `T`                     | Arguments                                    |
/// | ----------------------- | -------------------------------------------- |
/// | numbers, `bool`, `char` | one                                          |
/// | `String`                | one word, or anything in quotes              |
/// | enums                   | the name of a variant without fields         |
/// | structs, tuples, arrays | those of every field in order                |
/// | `Option`                | nothing if there are none left, so put it last |
/// | `Vec`                   | all that are left, so put it last            |
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be the input of a command",
    note = "use `In<T>`, where `T` implements `Reflect`"
)]
pub trait ConsoleInput: Input {}

impl<T: Input> ConsoleInput for T {}

impl Input for () {
    fn describe(_: &mut CommandBuilder) {}

    fn run<O: ConsoleOutput>(
        world: &mut World,
        system: SystemId<Self, O>,
        words: &[String],
    ) -> Result<String, CommandError> {
        no_arguments(words)?;
        finish(world.run_system(system))
    }
}

impl<A: FromReflect + Typed> Input for In<A> {
    fn describe(builder: &mut CommandBuilder) {
        builder.description = arguments::docs::<A>();
        match arguments::describe::<A>() {
            Ok(arguments) => builder.arguments = arguments,
            Err(path) => builder.unsupported = Some(path),
        }
    }

    fn run<O: ConsoleOutput>(
        world: &mut World,
        system: SystemId<Self, O>,
        words: &[String],
    ) -> Result<String, CommandError> {
        let arguments = arguments::parse(words).map_err(CommandError::Arguments)?;
        finish(world.run_system_with(system, arguments))
    }
}

fn finish<O: ConsoleOutput, E: Display>(result: Result<O, E>) -> Result<String, CommandError> {
    result
        .map_err(|error| error.to_string())
        .and_then(Output::into_text)
        .map_err(|error| CommandError::Failed(error.trim_end().into()))
}

/// What a command may return. The console shows it, unless it is `()` or empty.
///
/// Errors are logged, so `?` can be used in a system that returns [`Result`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be the output of a command",
    note = "return `()`, text, a number, a `bool`, or a `Result` of those"
)]
pub trait ConsoleOutput: Output + 'static {}

impl<T: Output + 'static> ConsoleOutput for T {}

impl Output for () {
    fn into_text(self) -> Result<String, String> {
        Ok(String::new())
    }
}

impl<T: Output, E: Display> Output for Result<T, E> {
    fn into_text(self) -> Result<String, String> {
        self.map_err(|error| error.to_string())?.into_text()
    }
}

macro_rules! display_output {
    ($($type:ty),* $(,)?) => {
        $(impl Output for $type {
            fn into_text(self) -> Result<String, String> {
                Ok(self.to_string())
            }
        })*
    };
}

display_output!(
    String,
    &'static str,
    bool,
    char,
    f32,
    f64,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
);

#[doc(hidden)]
pub struct IsBuilder;
#[doc(hidden)]
pub struct IsSystem;
#[doc(hidden)]
pub struct IsEvent;
#[doc(hidden)]
pub struct IsMessage;
#[doc(hidden)]
pub struct IsNamed;
#[doc(hidden)]
pub struct IsSingle;
#[doc(hidden)]
pub struct IsGroup;

impl IntoCommand<IsBuilder> for CommandBuilder {
    fn into_command(self) -> CommandBuilder {
        self
    }
}

impl<F, M> IntoCommand<(IsSystem, M)> for F
where
    M: 'static,
    F: SystemParamFunction<M, In: ConsoleInput, Out: ConsoleOutput>,
{
    fn into_command(self) -> CommandBuilder {
        CommandBuilder::system(self)
    }
}

impl<E> IntoCommand<IsEvent> for E
where
    E: Event + Clone,
    for<'a> E::Trigger<'a>: Default,
{
    fn into_command(self) -> CommandBuilder {
        CommandBuilder::event(self)
    }
}

impl<T: Message + Clone> IntoCommand<IsMessage> for T {
    fn into_command(self) -> CommandBuilder {
        CommandBuilder::message(self)
    }
}

impl<M, C: IntoCommand<M>> IntoCommand<(IsNamed, M)> for (&str, C) {
    fn into_command(self) -> CommandBuilder {
        self.1.named(self.0)
    }
}

impl<M, C: IntoCommand<M>> IntoCommand<(IsNamed, M)> for (String, C) {
    fn into_command(self) -> CommandBuilder {
        self.1.named(self.0)
    }
}

/// A command, or a tuple of up to 12 of them. Tuples can be nested.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as commands",
    label = "not a valid command, or tuple of commands",
    note = "give the one at fault to `add_command` for details"
)]
pub trait IntoCommands<M> {
    fn push_to(self, builders: &mut Vec<CommandBuilder>);
}

impl<M, C: IntoCommand<M>> IntoCommands<(IsSingle, M)> for C {
    fn push_to(self, builders: &mut Vec<CommandBuilder>) {
        builders.push(self.into_command());
    }
}

macro_rules! groups {
    ($(($command:ident, $marker:ident)),*) => {
        impl<$($command, $marker),*> IntoCommands<(IsGroup, $($marker),*)> for ($($command,)*)
        where
            $($command: IntoCommands<$marker>),*
        {
            #[allow(non_snake_case)]
            fn push_to(self, builders: &mut Vec<CommandBuilder>) {
                let ($($command,)*) = self;
                $($command.push_to(builders);)*
            }
        }
    };
}

#[rustfmt::skip]
mod groups {
    use super::*;

    groups!((A, MA));
    groups!((A, MA), (B, MB));
    groups!((A, MA), (B, MB), (C, MC));
    groups!((A, MA), (B, MB), (C, MC), (D, MD));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF), (G, MG));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF), (G, MG), (H, MH));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF), (G, MG), (H, MH), (I, MI));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF), (G, MG), (H, MH), (I, MI), (J, MJ));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF), (G, MG), (H, MH), (I, MI), (J, MJ), (K, MK));
    groups!((A, MA), (B, MB), (C, MC), (D, MD), (E, ME), (F, MF), (G, MG), (H, MH), (I, MI), (J, MJ), (K, MK), (L, ML));
}

/// The name of a type without its module path.
pub(crate) fn short_type_name<T>() -> &'static str {
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
}
