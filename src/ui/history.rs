use bevy::prelude::*;
use std::collections::VecDeque;

/// What has been submitted, from oldest to newest.
#[derive(Resource, Default)]
pub struct CommandHistory {
    entries: VecDeque<String>,
    /// The entry that was recalled last.
    recalled: Option<usize>,
    /// What was typed before anything was recalled.
    draft: String,
}

impl CommandHistory {
    /// Adds an entry, unless it just repeats the last one.
    pub fn push(&mut self, entry: &str, max: usize) {
        self.recalled = None;
        self.draft.clear();
        if self.entries.back().is_none_or(|last| last != entry) {
            self.entries.push_back(entry.into());
        }
        while self.entries.len() > max {
            self.entries.pop_front();
        }
    }

    /// The entry before the one that is shown.
    pub fn older(&mut self, shown: &str) -> Option<&str> {
        let index = match self.recalled_as(shown) {
            Some(index) => index.checked_sub(1)?,
            None => {
                let last = self.entries.len().checked_sub(1)?;
                self.draft = shown.into();
                last
            }
        };
        self.recalled = Some(index);
        self.entries.get(index).map(String::as_str)
    }

    /// The entry after the one that is shown, and finally what was typed before.
    pub fn newer(&mut self, shown: &str) -> Option<&str> {
        let index = self.recalled_as(shown)? + 1;
        if index < self.entries.len() {
            self.recalled = Some(index);
            self.entries.get(index).map(String::as_str)
        } else {
            self.recalled = None;
            Some(&self.draft)
        }
    }

    /// The recalled entry, as long as it has not been edited.
    fn recalled_as(&self, shown: &str) -> Option<usize> {
        self.recalled
            .filter(|index| self.entries.get(*index).is_some_and(|entry| entry == shown))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(entries: &[&str]) -> CommandHistory {
        let mut history = CommandHistory::default();
        for entry in entries {
            history.push(entry, 3);
        }
        history
    }

    #[test]
    fn empty_history_recalls_nothing() {
        let mut history = history(&[]);
        assert_eq!(history.older("typed"), None);
        assert_eq!(history.newer("typed"), None);
    }

    #[test]
    fn recalls_back_and_forth_to_the_draft() {
        let mut history = history(&["a", "b"]);
        assert_eq!(history.newer("typed"), None);
        assert_eq!(history.older("typed"), Some("b"));
        assert_eq!(history.older("b"), Some("a"));
        assert_eq!(history.older("a"), None);
        assert_eq!(history.newer("a"), Some("b"));
        assert_eq!(history.newer("b"), Some("typed"));
        assert_eq!(history.newer("typed"), None);
    }

    #[test]
    fn an_edited_entry_becomes_the_draft() {
        let mut history = history(&["a", "b"]);
        assert_eq!(history.older(""), Some("b"));
        assert_eq!(history.older("b edited"), Some("b"));
        assert_eq!(history.newer("b"), Some("b edited"));
    }

    #[test]
    fn is_limited_and_skips_repeats() {
        let mut history = history(&["a", "b", "b", "c", "d"]);
        assert_eq!(history.entries, ["b", "c", "d"]);
        history.push("e", 0);
        assert!(history.entries.is_empty());
    }
}
