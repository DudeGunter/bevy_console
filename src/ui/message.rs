use crate::{
    logging::{GetColor, TracingReceiver},
    ui::MessageContainer,
};
use bevy::{
    color::palettes::{css::*, tailwind::*},
    feathers::{font_styles::InheritableFont, theme::ThemedText},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::ListItem,
};

#[derive(SceneComponent, Clone, Copy, Default)]
pub struct ConsoleMessage;

impl ConsoleMessage {
    fn scene() -> impl Scene {
        bsn! {
            Hovered
            ListItem
            ThemedText
            Text
            InheritableFont {
                font_size: FontSize::Px(14.0),
                weight: FontWeight::NORMAL,
            }
            TextLayout::new(Justify::Left, LineBreak::AnyCharacter)
        }
    }
}

pub fn receive_traced_message(
    mut commands: Commands,
    container: Query<(Entity, &mut ScrollPosition), With<MessageContainer>>,
    traced_messages: ResMut<TracingReceiver>,
) {
    if let Ok((entity, mut scroll_position)) = container.single_inner() {
        let mut new_messages: Vec<Entity> = Vec::new();
        let mut message_received = false;
        while let Ok(trace) = traced_messages.try_recv() {
            message_received = true;
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
                        @ConsoleMessage Children [ Text({trace.message}) ThemedText ]
                    })
                    .id();
                new_messages.push(message);
            }
        }
        commands.entity(entity).add_children(&new_messages);
        if message_received {
            scroll_position.0.y = f32::MAX;
        }
    }
}

pub fn span<S: Into<String> + Send + Sync + 'static, C: Into<Color>>(
    string: S,
    color: C,
) -> impl Scene {
    bsn! {
        TextSpan::new(string)
        ThemedText
        TextColor(color)
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
