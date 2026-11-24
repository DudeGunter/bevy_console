use crate::{
    ConsoleSettings,
    logging::{GetColor, TraceMessage, TracingReceiver},
    ui::MessageContainer,
};
use bevy::{
    color::palettes::{css::*, tailwind::*},
    feathers::{font_styles::InheritableFont, theme::ThemedText},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::ListItem,
};
use chrono::{DateTime, Local};
use std::collections::VecDeque;

/// A row in the console.
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
    container: Single<(Entity, Option<&Children>, &mut ScrollPosition), With<MessageContainer>>,
    rows: Query<(), With<ConsoleMessage>>,
    settings: Res<ConsoleSettings>,
    traced_messages: Option<Res<TracingReceiver>>,
) {
    let (entity, children, mut scroll_position) = container.into_inner();

    // Only what will be kept is worth spawning.
    let mut traces = VecDeque::new();
    for trace in traced_messages
        .iter()
        .flat_map(|receiver| receiver.try_iter())
    {
        traces.push_back(trace);
        if traces.len() > settings.max_messages {
            traces.pop_front();
        }
    }
    if traces.is_empty() && !settings.is_changed() {
        return;
    }

    let current: Vec<Entity> = children
        .into_iter()
        .flatten()
        .copied()
        .filter(|child| rows.contains(*child))
        .collect();
    let excess = (current.len() + traces.len()).saturating_sub(settings.max_messages);
    for row in current.into_iter().take(excess) {
        commands.entity(row).try_despawn();
    }

    if traces.is_empty() {
        return;
    }
    let new_messages: Vec<Entity> = traces
        .into_iter()
        .map(|trace| spawn_message(&mut commands, trace))
        .collect();
    commands.entity(entity).add_children(&new_messages);
    scroll_position.0.y = f32::MAX;
}

fn spawn_message(commands: &mut Commands, trace: TraceMessage) -> Entity {
    if trace.custom {
        return commands
            .spawn_scene(bsn! {
                @ConsoleMessage Children [ Text({trace.message}) ThemedText ]
            })
            .id();
    }
    let time = span(format_time(&trace.time) + " ", GRAY_300);
    let info = span(trace.level.to_string() + " ", trace.level.get_color());
    let path = span(trace.target + ": ", BLUE_600);
    let message = span(trace.message, WHITE_SMOKE);
    commands
        .spawn_scene(bsn! {
            @ConsoleMessage
            Children [time, info, path, message]
        })
        .id()
}

fn format_time(time: &DateTime<Local>) -> String {
    time.format("%H:%M:%S").to_string()
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

/// Logs a message which the console shows as is, without its time, level and target.
/// Takes the same arguments as [`format!`].
#[macro_export]
macro_rules! simple {
    ($($arg:tt)+) => {
        $crate::prelude::tracing::info!(custom = true, $($arg)+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn time_is_formatted_to_the_second() {
        let time = Local.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        // However many digits chrono would print for the fraction of a second.
        for nanos in [0, 120_000_000, 123_456_000, 123_456_789] {
            let time = time + chrono::Duration::nanoseconds(nanos);
            assert_eq!(format_time(&time), "03:04:05");
        }
    }
}
