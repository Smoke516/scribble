//! Snippet completion for markdown.
//!
//! The rule this table lives by: a suggestion earns its place only if accepting
//! it leaves the note different from what you typed. Ten of the previous
//! seventeen replaced the trigger with itself — `- ` became `- ` — so the popup
//! appeared over every list item and every heading, swallowed the `Tab` that
//! would have indented it, and gave nothing back.

/// Where on a line a trigger is allowed to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// Any position — `[` opens a link mid-sentence as readily as at the margin.
    Anywhere,
    /// Only with nothing but whitespace before it. A table or a fence that began
    /// halfway through a sentence would not be one.
    LineStart,
}

/// A snippet, and the text that summons it.
#[derive(Debug, Clone, Copy)]
pub struct AutocompleteSuggestion {
    /// Typed by hand, and replaced when the suggestion is accepted.
    pub trigger: &'static str,
    /// What replaces the trigger. `$0` marks where the cursor lands.
    pub template: &'static str,
    pub description: &'static str,
    pub place: Where,
}

impl AutocompleteSuggestion {
    /// The text to insert, and the byte offset within it for the cursor.
    pub fn expand(&self) -> (String, usize) {
        match self.template.split_once("$0") {
            Some((before, after)) => (format!("{}{}", before, after), before.len()),
            None => (self.template.to_string(), self.template.len()),
        }
    }
}

const fn snip(
    trigger: &'static str,
    template: &'static str,
    description: &'static str,
    place: Where,
) -> AutocompleteSuggestion {
    AutocompleteSuggestion { trigger, template, description, place }
}

/// Every snippet, longest trigger first for readability; the match picks the
/// longest one regardless of order.
const SNIPPETS: &[AutocompleteSuggestion] = &[
    snip("```", "```\n$0\n```", "Code block", Where::LineStart),
    snip("![", "![$0](image.png)", "Image", Where::Anywhere),
    snip("**", "**$0**", "Bold", Where::Anywhere),
    snip("[", "[$0](url)", "Link", Where::Anywhere),
    snip("`", "`$0`", "Inline code", Where::Anywhere),
    snip("*", "*$0*", "Italic", Where::Anywhere),
    snip(
        "|",
        "| $0 | Header |\n|---|---|\n| Cell | Cell |",
        "Table",
        Where::LineStart,
    ),
];

#[derive(Debug, Clone)]
pub struct AutocompleteState {
    pub active: bool,
    pub suggestions: Vec<AutocompleteSuggestion>,
    pub selected_index: usize,
    /// Byte offset within the line where the trigger starts.
    pub trigger_start_pos: usize,
}

impl AutocompleteState {
    pub fn new() -> Self {
        Self {
            active: false,
            suggestions: Vec::new(),
            selected_index: 0,
            trigger_start_pos: 0,
        }
    }

    pub fn activate(&mut self, suggestions: Vec<AutocompleteSuggestion>, start_pos: usize) {
        self.active = true;
        self.suggestions = suggestions;
        self.selected_index = 0;
        self.trigger_start_pos = start_pos;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.suggestions.clear();
        self.selected_index = 0;
    }

    pub fn next_suggestion(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.suggestions.len();
        }
    }

    pub fn previous_suggestion(&mut self) {
        if !self.suggestions.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.suggestions.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn get_selected_suggestion(&self) -> Option<&AutocompleteSuggestion> {
        self.suggestions.get(self.selected_index)
    }
}

impl Default for AutocompleteState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MarkdownAutocomplete;

impl MarkdownAutocomplete {
    pub fn new() -> Self {
        Self
    }

    /// Snippets triggered by the text immediately before the cursor.
    ///
    /// `col` counts characters, as the cursor does; the returned position is a
    /// byte offset within the line, as the buffer is indexed.
    pub fn check_for_completions(
        &self,
        content: &str,
        line: usize,
        col: usize,
    ) -> Option<(Vec<AutocompleteSuggestion>, usize)> {
        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(line)?;

        // `col` counts characters; the slice below is in bytes.
        let byte_col = match current_line.char_indices().nth(col) {
            Some((byte, _)) => byte,
            None if col == current_line.chars().count() => current_line.len(),
            None => return None,
        };
        let before_cursor = &current_line[..byte_col];

        let best = SNIPPETS
            .iter()
            .filter(|s| before_cursor.ends_with(s.trigger))
            .filter(|s| match s.place {
                Where::Anywhere => true,
                Where::LineStart => before_cursor[..before_cursor.len() - s.trigger.len()]
                    .chars()
                    .all(char::is_whitespace),
            })
            .max_by_key(|s| s.trigger.len())?;

        Some((vec![*best], before_cursor.len() - best.trigger.len()))
    }
}

impl Default for MarkdownAutocomplete {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_end(line: &str) -> Option<(Vec<AutocompleteSuggestion>, usize)> {
        MarkdownAutocomplete::new().check_for_completions(line, 0, line.chars().count())
    }

    fn label(line: &str) -> Option<&'static str> {
        at_end(line).map(|(s, _)| s[0].description)
    }

    /// The seven that could never fire before: `should_trigger` demanded the text
    /// before the cursor end in a space, and none of these do.
    #[test]
    fn every_snippet_can_actually_be_reached() {
        assert_eq!(label("["), Some("Link"));
        assert_eq!(label("!["), Some("Image"));
        assert_eq!(label("**"), Some("Bold"));
        assert_eq!(label("*"), Some("Italic"));
        assert_eq!(label("`"), Some("Inline code"));
        assert_eq!(label("```"), Some("Code block"));
        assert_eq!(label("|"), Some("Table"));
    }

    /// Nothing fires for the markers that used to offer to replace themselves.
    #[test]
    fn the_no_op_suggestions_are_gone() {
        for line in ["- ", "# ", "## ", "### ", "> ", "1. ", "- [ ] ", "---"] {
            assert_eq!(label(line), None, "{:?} still opens the popup", line);
        }
    }

    #[test]
    fn the_longest_trigger_wins() {
        assert_eq!(label("**"), Some("Bold"));
        assert_eq!(label("```"), Some("Code block"));
        assert_eq!(label("!["), Some("Image"));
    }

    #[test]
    fn a_link_fires_mid_sentence_but_a_table_does_not() {
        assert_eq!(label("see the ["), Some("Link"));
        assert_eq!(label("a | b |"), None, "a table began mid-sentence");
        assert_eq!(label("  |"), Some("Table"), "indented table did not fire");
    }

    #[test]
    fn a_space_after_the_trigger_dismisses_it() {
        assert_eq!(label("* "), None);
        assert_eq!(label("[ "), None);
    }

    #[test]
    fn every_template_places_the_cursor() {
        for snippet in SNIPPETS {
            let (text, cursor) = snippet.expand();
            assert!(
                snippet.template.contains("$0"),
                "{} has nowhere to put the cursor",
                snippet.description
            );
            assert!(cursor <= text.len());
            assert!(!text.contains("$0"), "the marker was left in {}", snippet.description);
            assert_ne!(
                text, snippet.trigger,
                "{} replaces the trigger with itself",
                snippet.description
            );
        }
    }

    #[test]
    fn the_reported_position_is_where_the_trigger_starts() {
        let (_, pos) = at_end("see the [").unwrap();
        assert_eq!(pos, "see the ".len());
    }

    /// The scan runs on every keystroke, including in text that is not ASCII.
    #[test]
    fn a_multi_byte_line_is_scanned_by_character() {
        assert_eq!(label("café ["), Some("Link"));
        assert_eq!(label("日本語 **"), Some("Bold"));
        assert!(MarkdownAutocomplete::new()
            .check_for_completions("café", 0, 4)
            .is_none());
    }
}
