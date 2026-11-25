//! Hides input that is meant for the console from the app.
//!
//! Bevy needs the real state of every key as well, such as for shortcuts in text inputs. So the app
//! only sees a copy, from [`ConsoleSystems::BlockInput`] until input is read again in the next frame.

use crate::{
    ConsoleSettings,
    ui::{ConsoleUI, input::TextInput},
};
use bevy::{
    input::{InputSystems, keyboard::Key, mouse::AccumulatedMouseScroll},
    input_focus::{InputFocus, InputFocusSystems},
    picking::{PickingSystems, hover::HoverMap, pointer::PointerId},
    platform::collections::HashSet,
    prelude::*,
    ui::UiSystems,
};
use std::hash::Hash;

/// The systems of the console which others may have to be ordered with.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsoleSystems {
    /// Hides input from the app, in [`PreUpdate`]. Systems in [`PreUpdate`] that read input for the
    /// app should run after this. Those in later schedules don't have to care.
    BlockInput,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<WasTyping>()
        .init_resource::<Blocked<KeyCode>>()
        .init_resource::<Blocked<Key>>()
        .init_resource::<Blocked<MouseButton>>()
        .add_systems(
            PreUpdate,
            (
                restore::<KeyCode>,
                restore::<Key>,
                restore::<MouseButton>,
                remember_typing,
            )
                .before(InputSystems),
        )
        .add_systems(
            PreUpdate,
            (
                block_keyboard::<KeyCode>,
                block_keyboard::<Key>,
                block_mouse,
            )
                .in_set(ConsoleSystems::BlockInput),
        )
        .configure_sets(
            PreUpdate,
            // Everything of Bevy that reads input.
            ConsoleSystems::BlockInput
                .after(InputSystems)
                .after(InputFocusSystems::Dispatch)
                .after(PickingSystems::Last)
                .after(UiSystems::Focus),
        );
}

/// A run condition that is true while the text input of the console has focus.
///
/// Keys are hidden from [`ButtonInput`] by default, but not from messages like
/// [`KeyboardInput`](bevy::input::keyboard::KeyboardInput). Systems which read those can use this:
/// ```
/// # use bevy::prelude::*;
/// # use bevy_console::prelude::*;
/// # fn read_keyboard_messages() {}
/// App::new().add_systems(Update, read_keyboard_messages.run_if(not(typing_in_console)));
/// ```
pub fn typing_in_console(
    input_focus: Option<Res<InputFocus>>,
    text_inputs: Query<(), With<TextInput>>,
) -> bool {
    input_focus
        .and_then(|input_focus| input_focus.get())
        .is_some_and(|focused| text_inputs.contains(focused))
}

/// Whether the console had the focus before keys were handled. Escape takes it away.
#[derive(Resource, Default)]
struct WasTyping(bool);

#[derive(Resource)]
struct Blocked<T: Clone + Eq + Hash + Send + Sync + 'static> {
    /// What to restore, if anything is hidden.
    real: Option<ButtonInput<T>>,
    /// Pressed while blocked, and hidden until released.
    swallowed: HashSet<T>,
    /// Pressed before being blocked, and hidden until no longer blocked.
    held: HashSet<T>,
    was_blocked: bool,
}

impl<T: Clone + Eq + Hash + Send + Sync + 'static> Default for Blocked<T> {
    fn default() -> Self {
        Self {
            real: None,
            swallowed: HashSet::default(),
            held: HashSet::default(),
            was_blocked: false,
        }
    }
}

impl<T: Clone + Eq + Hash + Send + Sync + 'static> Blocked<T> {
    /// Hides everything that gets pressed from now on.
    fn swallow(&mut self, input: &ButtonInput<T>) {
        self.swallowed.extend(input.get_just_pressed().cloned());
    }

    /// Hides everything that was pressed already, after releasing it.
    /// Has to be called in every frame.
    fn hold(&mut self, input: &mut ButtonInput<T>, blocked: bool) -> Vec<T> {
        let mut released = Vec::new();
        if !blocked {
            self.held.clear();
        } else if !self.was_blocked {
            released.extend(
                input
                    .get_pressed()
                    .filter(|key| !self.swallowed.contains(*key))
                    .cloned(),
            );
            self.held.extend(released.iter().cloned());
        }
        self.was_blocked = blocked;
        released
    }

    fn hide(&mut self, input: &mut ButtonInput<T>, released: Vec<T>) {
        if self.swallowed.is_empty() && self.held.is_empty() {
            return;
        }
        let real = input.clone();
        for key in self.swallowed.iter().chain(&self.held) {
            input.reset(key.clone());
        }
        for key in released {
            input.press(key.clone());
            input.clear_just_pressed(key.clone());
            input.release(key);
        }
        self.swallowed.retain(|key| real.pressed(key.clone()));
        self.held.retain(|key| real.pressed(key.clone()));
        self.real = Some(real);
    }
}

fn restore<T: Clone + Eq + Hash + Send + Sync + 'static>(
    mut input: ResMut<ButtonInput<T>>,
    mut blocked: ResMut<Blocked<T>>,
) {
    if let Some(real) = blocked.real.take() {
        *input = real;
    }
}

fn remember_typing(
    mut was_typing: ResMut<WasTyping>,
    input_focus: Option<Res<InputFocus>>,
    text_inputs: Query<(), With<TextInput>>,
) {
    was_typing.0 = typing_in_console(input_focus, text_inputs);
}

fn block_keyboard<T: Clone + Eq + Hash + Send + Sync + 'static>(
    mut input: ResMut<ButtonInput<T>>,
    mut blocked: ResMut<Blocked<T>>,
    was_typing: Res<WasTyping>,
    settings: Res<ConsoleSettings>,
    input_focus: Option<Res<InputFocus>>,
    text_inputs: Query<(), With<TextInput>>,
) {
    let typing = was_typing.0 || typing_in_console(input_focus, text_inputs);
    let block = settings.block_input && typing;
    let input = input.bypass_change_detection();
    if block {
        blocked.swallow(input);
    }
    let released = blocked.hold(input, block);
    blocked.hide(input, released);
}

fn block_mouse(
    mut input: ResMut<ButtonInput<MouseButton>>,
    mut scroll: ResMut<AccumulatedMouseScroll>,
    mut blocked: ResMut<Blocked<MouseButton>>,
    settings: Res<ConsoleSettings>,
    hover_map: Option<Res<HoverMap>>,
    console: Query<(), With<ConsoleUI>>,
    parents: Query<&ChildOf>,
) {
    let is_hovered = hover_map
        .as_ref()
        .and_then(|map| map.get(&PointerId::Mouse))
        .is_some_and(|hovered| {
            hovered.keys().any(|entity| {
                console.contains(*entity)
                    || parents
                        .iter_ancestors(*entity)
                        .any(|parent| console.contains(parent))
            })
        });
    let input = input.bypass_change_detection();
    if settings.block_input && is_hovered {
        // Buttons that were pressed elsewhere still belong to the app.
        blocked.swallow(input);
        if scroll.delta != Vec2::ZERO {
            scroll.delta = Vec2::ZERO;
        }
    }
    blocked.hide(input, Vec::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the app sees.
    #[derive(Debug, PartialEq, Eq)]
    struct Seen {
        pressed: bool,
        just_pressed: bool,
        just_released: bool,
    }

    const NOTHING: Seen = Seen {
        pressed: false,
        just_pressed: false,
        just_released: false,
    };
    const PRESSED: Seen = Seen {
        pressed: true,
        just_pressed: true,
        just_released: false,
    };
    const HELD: Seen = Seen {
        pressed: true,
        just_pressed: false,
        just_released: false,
    };
    const RELEASED: Seen = Seen {
        pressed: false,
        just_pressed: false,
        just_released: true,
    };

    #[derive(Default)]
    struct Keyboard {
        input: ButtonInput<u8>,
        blocked: Blocked<u8>,
    }

    impl Keyboard {
        /// Runs a frame, in which a key may be pressed or released.
        fn frame(&mut self, change: Option<(u8, bool)>, block: bool) -> &mut Self {
            let Self { input, blocked } = self;
            if let Some(real) = blocked.real.take() {
                *input = real;
            }
            input.clear();
            match change {
                Some((key, true)) => input.press(key),
                Some((key, false)) => input.release(key),
                None => {}
            }
            if block {
                blocked.swallow(input);
            }
            let released = blocked.hold(input, block);
            blocked.hide(input, released);
            self
        }

        fn seen(&self, key: u8) -> Seen {
            Seen {
                pressed: self.input.pressed(key),
                just_pressed: self.input.just_pressed(key),
                just_released: self.input.just_released(key),
            }
        }
    }

    #[test]
    fn nothing_is_hidden_unless_blocked() {
        let mut keys = Keyboard::default();
        assert_eq!(keys.frame(Some((1, true)), false).seen(1), PRESSED);
        assert_eq!(keys.frame(None, false).seen(1), HELD);
        assert_eq!(keys.frame(Some((1, false)), false).seen(1), RELEASED);
        assert_eq!(keys.frame(None, false).seen(1), NOTHING);
    }

    #[test]
    fn presses_while_blocked_are_hidden_until_released() {
        let mut keys = Keyboard::default();
        assert_eq!(keys.frame(Some((1, true)), true).seen(1), NOTHING);
        assert_eq!(keys.frame(None, true).seen(1), NOTHING);
        // No longer blocked, but still the same press.
        assert_eq!(keys.frame(None, false).seen(1), NOTHING);
        assert_eq!(keys.frame(Some((1, false)), false).seen(1), NOTHING);
        assert_eq!(keys.frame(Some((1, true)), false).seen(1), PRESSED);
    }

    #[test]
    fn held_keys_are_released_once_and_come_back() {
        let mut keys = Keyboard::default();
        keys.frame(Some((1, true)), false);
        assert_eq!(keys.frame(None, true).seen(1), RELEASED);
        assert_eq!(keys.frame(None, true).seen(1), NOTHING);
        assert_eq!(keys.frame(None, false).seen(1), HELD);
        assert_eq!(keys.frame(Some((1, false)), false).seen(1), RELEASED);
    }

    #[test]
    fn held_keys_can_be_released_while_blocked() {
        let mut keys = Keyboard::default();
        keys.frame(Some((1, true)), false);
        keys.frame(None, true);
        assert_eq!(keys.frame(Some((1, false)), true).seen(1), NOTHING);
        assert_eq!(keys.frame(Some((1, true)), true).seen(1), NOTHING);
        assert_eq!(keys.frame(None, false).seen(1), NOTHING);
    }

    #[test]
    fn the_press_that_starts_blocking_is_hidden_too() {
        let mut keys = Keyboard::default();
        keys.frame(Some((1, true)), true);
        assert_eq!(keys.seen(1), NOTHING);
        assert!(keys.blocked.held.is_empty());
    }

    #[test]
    fn the_real_state_is_kept() {
        let mut keys = Keyboard::default();
        keys.frame(Some((1, true)), true);
        keys.frame(Some((2, true)), true);
        let real = keys.blocked.real.as_ref().unwrap();
        assert!(real.pressed(1) && real.just_pressed(2) && !real.just_pressed(1));
        keys.frame(Some((1, false)), true);
        keys.frame(Some((2, false)), true);
        keys.frame(None, true);
        // Nothing is left to hide.
        assert!(keys.blocked.real.is_none());
    }
}
