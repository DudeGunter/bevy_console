use bevy::{
    color::palettes::css::*,
    log::{BoxedLayer, tracing_subscriber},
    prelude::*,
};
use chrono::{DateTime, Local};
use crossbeam::channel::{Receiver, Sender};
use tracing::Level;

/// A function that implements the log reading functionality for the
/// developer console via [`LogPlugin::custom_layer`](bevy::log::LogPlugin::custom_layer).
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_console::prelude::*;
///
/// App::new()
///     .add_plugins(DefaultPlugins.set(LogPlugin {
///         custom_layer: custom_log_layer, // The function
///         ..default()
///     }))
///     .add_plugins(ConsolePlugin)
///     .run();
/// ```
pub fn custom_log_layer(app: &mut App) -> Option<BoxedLayer> {
    Some(Box::new(create_custom_log_layer(app)))
}

fn create_custom_log_layer(app: &mut App) -> BoxedLayer {
    let (sender, receiver) = crossbeam::channel::unbounded();
    app.insert_resource(TracingReceiver(receiver));

    Box::new(ConsoleLayer(sender))
}

#[derive(Debug)]
pub struct TraceMessage {
    pub time: DateTime<Local>,
    pub level: Level,
    pub target: String,
    pub message: String,
    pub custom: bool,
}

#[derive(Resource, Deref, DerefMut)]
pub struct TracingReceiver(pub Receiver<TraceMessage>);

struct ConsoleLayer(Sender<TraceMessage>);

impl<S> tracing_subscriber::Layer<S> for ConsoleLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = ConsoleVisitor::default();
        event.record(&mut visitor);
        let time = Local::now(); // need to compare
        let level = *event.metadata().level();
        let target = event.metadata().target().to_string();

        let _ = self.0.send(TraceMessage {
            time,
            level,
            target,
            message: visitor.message,
            custom: visitor.custom,
        });
    }
}

#[derive(Default)]
struct ConsoleVisitor {
    message: String,
    custom: bool,
}

impl tracing::field::Visit for ConsoleVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    // Without this, `record_debug` would surround the message with quotes and escape it.
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "custom" {
            self.custom = value;
        }
    }
}

pub trait GetColor {
    fn get_color(&self) -> Color;
}

impl GetColor for Level {
    fn get_color(&self) -> Color {
        Color::from(match *self {
            Level::ERROR => RED,
            Level::WARN => YELLOW,
            Level::INFO => GREEN,
            Level::DEBUG => GRAY,
            Level::TRACE => DARK_GRAY,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::prelude::*;

    /// Everything `log` logs, as the console would receive it.
    fn capture(log: impl FnOnce()) -> Vec<TraceMessage> {
        let mut app = App::new();
        let layer = custom_log_layer(&mut app).unwrap();
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, log);
        app.world()
            .resource::<TracingReceiver>()
            .try_iter()
            .collect()
    }

    #[test]
    fn captures_level_target_and_message() {
        let messages = capture(|| warn!("{} left", 3));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].level, Level::WARN);
        assert_eq!(messages[0].target, module_path!());
        assert_eq!(messages[0].message, "3 left");
        assert!(!messages[0].custom);
    }

    #[test]
    fn keeps_quotes_and_order() {
        let messages = capture(|| {
            info!("\"quoted\"");
            error!(message = "a \"b\"");
        });
        let messages: Vec<_> = messages.iter().map(|m| m.message.as_str()).collect();
        assert_eq!(messages, ["\"quoted\"", "a \"b\""]);
    }

    #[test]
    fn simple_is_custom_and_works_as_an_expression() {
        let value = 2;
        let messages = capture(|| {
            let log = || crate::simple!("one");
            log();
            crate::simple!("{value}");
            crate::simple!("{} {}", 3, "four",);
        });
        assert!(messages.iter().all(|m| m.custom));
        let messages: Vec<_> = messages.iter().map(|m| m.message.as_str()).collect();
        assert_eq!(messages, ["one", "2", "3 four"]);
    }
}
