use crate::{
    logging::{GetColor, TracingReceiver},
    ui::MessageContainer,
};
use bevy::{
    color::palettes::{css::*, tailwind::*},
    feathers::{controls::FeathersListRow, theme::ThemedText},
    prelude::*,
};

#[derive(SceneComponent, Clone, Copy, Default)]
pub struct ConsoleMessage;

impl ConsoleMessage {
    fn scene() -> impl Scene {
        bsn! {
            @FeathersListRow
            ThemedText
            Text
        }
    }
}

pub fn receive_traced_message(
    mut commands: Commands,
    container: Query<Entity, With<MessageContainer>>,
    traced_messages: ResMut<TracingReceiver>,
) {
    if let Ok(entity) = container.single_inner() {
        let mut new_messages: Vec<Entity> = Vec::new();
        while let Ok(trace) = traced_messages.try_recv() {
            if !trace.custom {
                let time = trace.time.time().to_string(); // todo! (not really) this formatting could be done better via chronos
                let formatted_time = &time[..time.len() - 7];
                let time = span(formatted_time.to_owned() + " ", GRAY_300);
                let info = span(trace.level.to_string() + " ", trace.level.get_color());
                let path = span(trace.target + ": ", BLUE_600);
                let message = span(trace.message, WHITE_SMOKE);
                let message = commands
                    .spawn_scene(bsn! {
                        @ConsoleMessage
                        Children [time, info, path, message]
                    })
                    .id();
                new_messages.push(message);
            } else {
                let message = commands
                    .spawn_scene(bsn! {
                        @ConsoleMessage Text({trace.message})
                    })
                    .id();
                new_messages.push(message);
            }
        }
        commands.entity(entity).add_children(&new_messages);
    }
}

pub fn span<S: Into<String> + Send + Sync + 'static, C: Into<Color>>(
    string: S,
    color: C,
) -> impl Scene {
    bsn! {
        TextSpan::new(string)
        TextColor(color)
        ThemedText
    }
}

#[macro_export]
macro_rules! simple {
    ($msg:expr) => {
        $crate::prelude::tracing::info!(custom = true, $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::prelude::tracing::info!(custom = true, $fmt, $($arg)*);
    };
}
