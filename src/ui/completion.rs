use crate::{
    ConsoleSettings,
    arguments::{Arity, split_open},
    commands::{CommandArguments, CommandMetadata, ConsoleCommand},
    ui::{input::TextInput, message::span},
};
use bevy::{
    color::palettes::{css::*, tailwind::*},
    feathers::{font_styles::InheritableFont, theme::ThemedText},
    prelude::*,
    text::EditableText,
};

pub type CommandQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ConsoleCommand,
        Option<&'static CommandArguments>,
        Option<&'static CommandMetadata>,
    ),
>;

/// What could be typed next.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Suggestions {
    /// Where the word that would be replaced starts.
    start: usize,
    /// Sorted, and without duplicates.
    candidates: Vec<String>,
    /// How to use the command that has been typed.
    usages: Vec<String>,
}

pub fn suggest(entry: &str, commands: &CommandQuery) -> Suggestions {
    let (words, open) = split_open(entry);
    let (start, word) = match words.last() {
        Some((start, word)) if open => (*start, word.as_str()),
        _ => (entry.len(), ""),
    };
    // The position of the word that is being typed.
    let position = if open { words.len() - 1 } else { words.len() };

    let names = || commands.iter().map(|(name, ..)| name.0.clone());
    let mut candidates: Vec<String> = Vec::new();
    let mut usages: Vec<String> = Vec::new();
    if position == 0 {
        candidates.extend(names());
    }
    if let Some((_, typed)) = words.first() {
        for (_, arguments, metadata) in commands.iter().filter(|(name, ..)| name.0 == *typed) {
            usages.extend(metadata.map(|metadata| metadata.usage.clone()));
            if let (Some(index), Some(arguments)) = (position.checked_sub(1), arguments) {
                candidates.extend(argument_values(&arguments.0, index));
            }
        }
        if position == 1 && typed == "help" {
            candidates.extend(names());
        }
    }

    let word = word.to_lowercase();
    candidates.retain(|candidate| candidate.to_lowercase().starts_with(&word));
    candidates.sort();
    candidates.dedup();
    usages.sort();
    usages.dedup();
    Suggestions {
        start,
        candidates,
        usages,
    }
}

fn argument_values(arguments: &[crate::arguments::Argument], index: usize) -> Vec<String> {
    // Whatever is in a list repeats.
    let single = arguments
        .iter()
        .take_while(|argument| argument.arity != Arity::Many)
        .count();
    let index = match arguments.len() - single {
        0 => index,
        _ if index < single => index,
        repeated => single + (index - single) % repeated,
    };
    arguments
        .get(index)
        .map(|argument| argument.values.clone())
        .unwrap_or_default()
}

/// The candidates that Tab goes through.
#[derive(Resource, Default)]
pub struct Completion {
    before: String,
    candidates: Vec<String>,
    index: usize,
    /// What the text is while the current candidate is still shown.
    text: String,
}

impl Completion {
    /// What to replace the text with, if there is anything.
    pub fn next(
        &mut self,
        entry: &str,
        backwards: bool,
        suggest: impl FnOnce() -> Suggestions,
    ) -> Option<&str> {
        if self.is_shown(entry) {
            let len = self.candidates.len();
            let step = if backwards { len - 1 } else { 1 };
            self.index = (self.index + step) % len;
        } else {
            let suggestions = suggest();
            self.before = entry[..suggestions.start].into();
            self.candidates = suggestions.candidates;
            self.index = match backwards {
                true => self.candidates.len().saturating_sub(1),
                false => 0,
            };
        }
        let candidate = self.candidates.get(self.index)?;
        self.text = self.before.clone() + &quote(candidate);
        // There is nothing to go through, so continue with the next word.
        if self.candidates.len() == 1 {
            self.text.push(' ');
            self.candidates.clear();
        }
        Some(&self.text)
    }

    fn is_shown(&self, entry: &str) -> bool {
        !self.candidates.is_empty() && self.text == entry
    }
}

fn quote(word: &str) -> String {
    if !word.is_empty() && !word.contains(|c: char| c.is_whitespace() || c == '"') {
        return word.into();
    }
    format!("\"{}\"", word.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Shows the suggestions below the text input.
#[derive(SceneComponent, Clone, Copy, Default)]
pub struct SuggestionList;

impl SuggestionList {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::None,
            }
            ThemedText
            Text
            InheritableFont {
                font_size: FontSize::Px(14.0),
                weight: FontWeight::NORMAL,
            }
            TextLayout::new(Justify::Left, LineBreak::WordBoundary)
        }
    }
}

pub fn update_suggestions(
    mut commands: Commands,
    mut shown: Local<Option<(String, usize)>>,
    settings: Res<ConsoleSettings>,
    completion: Res<Completion>,
    text_input: Single<&EditableText, With<TextInput>>,
    list: Single<(Entity, &mut Node), With<SuggestionList>>,
    console_commands: CommandQuery,
) {
    // The text is about to change, and may be empty until then.
    if !text_input.pending_edits.is_empty() {
        return;
    }
    let entry = text_input.value().to_string();
    let now = (entry, completion.index);
    if shown.as_ref() == Some(&now) && !settings.is_changed() {
        return;
    }
    let (entry, _) = shown.insert(now);

    let (list, mut node) = list.into_inner();
    let spans = if entry.trim().is_empty() || settings.max_suggestions == 0 {
        Vec::new()
    } else if completion.is_shown(entry) {
        candidate_spans(
            &completion.candidates,
            Some(completion.index),
            settings.max_suggestions,
        )
    } else {
        let mut suggestions = suggest(entry, &console_commands);
        // Nothing to add to what has been typed.
        if let [only] = suggestions.candidates.as_slice()
            && entry[suggestions.start..] == *only
        {
            suggestions.candidates.clear();
        }
        let mut spans = candidate_spans(&suggestions.candidates, None, settings.max_suggestions);
        for usage in suggestions.usages {
            let line = if spans.is_empty() { "" } else { "\n" };
            spans.push((format!("{line}{usage}"), GRAY_400));
        }
        spans
    };

    node.display = match spans.is_empty() {
        true => Display::None,
        false => Display::Flex,
    };
    commands.entity(list).despawn_children();
    for (text, color) in spans {
        let span = commands.spawn_scene(span(text, color)).id();
        commands.entity(list).add_child(span);
    }
}

fn candidate_spans(
    candidates: &[String],
    selected: Option<usize>,
    max: usize,
) -> Vec<(String, Srgba)> {
    // Keeps the selected one in view.
    let first = selected.map_or(0, |selected| (selected + 1).saturating_sub(max));
    let mut spans: Vec<_> = candidates
        .iter()
        .enumerate()
        .skip(first)
        .take(max)
        .map(|(index, candidate)| {
            let color = match selected == Some(index) {
                true => YELLOW,
                false => WHITE_SMOKE,
            };
            (format!("{candidate}  "), color)
        })
        .collect();
    let hidden = candidates.len().saturating_sub(max);
    if hidden > 0 {
        spans.push((format!("+{hidden}"), GRAY_400));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app_ext::ConsoleAppExt, into_command::IntoCommand};
    use bevy::ecs::system::RunSystemOnce;

    #[derive(Reflect)]
    enum Team {
        Red,
        Blue,
    }

    fn join(In((_, _, _)): In<(Team, u32, Option<bool>)>) {}

    fn teams(In((_, _)): In<(u32, Vec<(Team, bool)>)>) {}

    fn suggestions(entry: &str) -> Suggestions {
        let mut app = App::new();
        app.add_commands((
            join,
            teams,
            ("jump", || {}),
            ("jump", || {}),
            ("help", (|| {}).usage("help [command]")),
        ));
        let entry = entry.to_string();
        app.world_mut()
            .run_system_once(move |commands: CommandQuery| suggest(&entry, &commands))
            .unwrap()
    }

    fn candidates(entry: &str) -> Vec<String> {
        suggestions(entry).candidates
    }

    #[test]
    fn names_are_completed() {
        assert_eq!(candidates("j"), ["join", "jump"]);
        assert_eq!(candidates("JU"), ["jump"]);
        assert_eq!(candidates(""), ["help", "join", "jump", "teams"]);
        assert!(candidates("x").is_empty());
        assert_eq!(suggestions("  j").start, 2);
    }

    #[test]
    fn arguments_are_completed() {
        assert_eq!(candidates("join "), ["Blue", "Red"]);
        assert_eq!(candidates("join r"), ["Red"]);
        assert_eq!(suggestions("join r").start, 5);
        assert!(candidates("join red ").is_empty());
        assert_eq!(candidates("join red 1 "), ["false", "true"]);
        assert!(candidates("join red 1 true ").is_empty());
        assert!(candidates("unknown ").is_empty());
        assert_eq!(candidates("help j"), ["join", "jump"]);
        assert!(candidates("help join ").is_empty());
    }

    #[test]
    fn lists_repeat() {
        assert!(candidates("teams ").is_empty());
        for entry in [
            "teams 1 ",
            "teams 1 red true ",
            "teams 1 red true red true ",
        ] {
            assert_eq!(candidates(entry), ["Blue", "Red"], "{entry}");
        }
        assert_eq!(candidates("teams 1 red true red "), ["false", "true"]);
    }

    #[test]
    fn usage_is_shown_for_typed_commands() {
        assert!(suggestions("joi").usages.is_empty());
        assert_eq!(
            suggestions("join r").usages,
            ["join <Red|Blue> <u32> [bool]"]
        );
        assert_eq!(suggestions("jump").usages, ["jump"]);
    }

    fn fixed(start: usize, candidates: &[&str]) -> Suggestions {
        Suggestions {
            start,
            candidates: candidates.iter().map(|name| name.to_string()).collect(),
            usages: Vec::new(),
        }
    }

    #[test]
    fn several_candidates_are_cycled() {
        let mut completion = Completion::default();
        let mut next = |entry: &str, backwards| {
            completion
                .next(entry, backwards, || fixed(1, &["aa", "ab", "ac"]))
                .map(str::to_string)
        };
        assert_eq!(next(" a", false).as_deref(), Some(" aa"));
        assert_eq!(next(" aa", false).as_deref(), Some(" ab"));
        assert_eq!(next(" ab", true).as_deref(), Some(" aa"));
        assert_eq!(next(" aa", true).as_deref(), Some(" ac"));
        assert_eq!(next(" ac", false).as_deref(), Some(" aa"));
        // Starts over once something else was typed.
        assert_eq!(next(" a", true).as_deref(), Some(" ac"));
    }

    #[test]
    fn a_single_candidate_is_finished() {
        let mut completion = Completion::default();
        assert_eq!(
            completion.next("x a", false, || fixed(2, &["two words"])),
            Some("x \"two words\" ")
        );
        let entry = completion.text.clone();
        assert_eq!(completion.next(&entry, false, || fixed(14, &[])), None);
    }

    #[test]
    fn the_selected_candidate_stays_in_view() {
        let names: Vec<String> = ["a", "b", "c", "d"].map(String::from).into();
        let texts = |selected, max| {
            candidate_spans(&names, selected, max)
                .into_iter()
                .map(|(text, _)| text)
                .collect::<String>()
        };
        assert_eq!(texts(None, 4), "a  b  c  d  ");
        assert_eq!(texts(None, 2), "a  b  +2");
        assert_eq!(texts(Some(0), 2), "a  b  +2");
        assert_eq!(texts(Some(3), 2), "c  d  +2");
    }
}
