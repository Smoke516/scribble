use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AutocompleteSuggestion {
    pub trigger: String,       // What the user types to trigger this
    pub completion: String,    // What gets inserted
    pub description: String,   // User-friendly description
    pub cursor_offset: i16,    // Where to position cursor after insertion (negative = back from end)
}

#[derive(Debug, Clone)]
pub struct AutocompleteState {
    pub active: bool,
    pub suggestions: Vec<AutocompleteSuggestion>,
    pub selected_index: usize,
    pub trigger_start_pos: usize, // Position where the trigger started
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

pub struct MarkdownAutocomplete {
    suggestions: HashMap<String, Vec<AutocompleteSuggestion>>,
}

impl MarkdownAutocomplete {
    pub fn new() -> Self {
        let mut autocomplete = Self {
            suggestions: HashMap::new(),
        };
        autocomplete.initialize_suggestions();
        autocomplete
    }

    fn initialize_suggestions(&mut self) {
        // Headers
        self.add_suggestion("# ", AutocompleteSuggestion {
            trigger: "#".to_string(),
            completion: "# ".to_string(),
            description: "Heading 1".to_string(),
            cursor_offset: 0,
        });

        self.add_suggestion("## ", AutocompleteSuggestion {
            trigger: "##".to_string(),
            completion: "## ".to_string(),
            description: "Heading 2".to_string(),
            cursor_offset: 0,
        });

        self.add_suggestion("### ", AutocompleteSuggestion {
            trigger: "###".to_string(),
            completion: "### ".to_string(),
            description: "Heading 3".to_string(),
            cursor_offset: 0,
        });

        // Lists
        self.add_suggestion("- ", AutocompleteSuggestion {
            trigger: "-".to_string(),
            completion: "- ".to_string(),
            description: "Bullet list item".to_string(),
            cursor_offset: 0,
        });

        self.add_suggestion("* ", AutocompleteSuggestion {
            trigger: "*".to_string(),
            completion: "* ".to_string(),
            description: "Bullet list item (alt)".to_string(),
            cursor_offset: 0,
        });

        self.add_suggestion("1. ", AutocompleteSuggestion {
            trigger: "1.".to_string(),
            completion: "1. ".to_string(),
            description: "Numbered list item".to_string(),
            cursor_offset: 0,
        });

        // Checkboxes
        self.add_suggestion("- [ ] ", AutocompleteSuggestion {
            trigger: "- [".to_string(),
            completion: "- [ ] ".to_string(),
            description: "Todo checkbox (unchecked)".to_string(),
            cursor_offset: 0,
        });

        self.add_suggestion("- [x] ", AutocompleteSuggestion {
            trigger: "- [x".to_string(),
            completion: "- [x] ".to_string(),
            description: "Todo checkbox (checked)".to_string(),
            cursor_offset: 0,
        });

        // Code blocks
        self.add_suggestion("```", AutocompleteSuggestion {
            trigger: "```".to_string(),
            completion: "```\n\n```".to_string(),
            description: "Code block".to_string(),
            cursor_offset: -4, // Position cursor inside the code block
        });

        self.add_suggestion("`", AutocompleteSuggestion {
            trigger: "`".to_string(),
            completion: "``".to_string(),
            description: "Inline code".to_string(),
            cursor_offset: -1, // Position cursor between the backticks
        });

        // Emphasis
        self.add_suggestion("**", AutocompleteSuggestion {
            trigger: "**".to_string(),
            completion: "****".to_string(),
            description: "Bold text".to_string(),
            cursor_offset: -2,
        });

        self.add_suggestion("*", AutocompleteSuggestion {
            trigger: "*".to_string(),
            completion: "**".to_string(),
            description: "Italic text".to_string(),
            cursor_offset: -1,
        });

        // Links and images
        self.add_suggestion("[", AutocompleteSuggestion {
            trigger: "[".to_string(),
            completion: "[](url)".to_string(),
            description: "Link".to_string(),
            cursor_offset: -5, // Position cursor at the beginning of link text
        });

        self.add_suggestion("![", AutocompleteSuggestion {
            trigger: "![".to_string(),
            completion: "![alt text](image.png)".to_string(),
            description: "Image".to_string(),
            cursor_offset: -17, // Position cursor at alt text
        });

        // Blockquotes
        self.add_suggestion("> ", AutocompleteSuggestion {
            trigger: ">".to_string(),
            completion: "> ".to_string(),
            description: "Blockquote".to_string(),
            cursor_offset: 0,
        });

        // Tables
        self.add_suggestion("| ", AutocompleteSuggestion {
            trigger: "|".to_string(),
            completion: "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |".to_string(),
            description: "Table".to_string(),
            cursor_offset: -49, // Position cursor at first header
        });

        // Horizontal rule
        self.add_suggestion("---", AutocompleteSuggestion {
            trigger: "---".to_string(),
            completion: "---".to_string(),
            description: "Horizontal rule".to_string(),
            cursor_offset: 0,
        });
    }

    fn add_suggestion(&mut self, trigger: &str, suggestion: AutocompleteSuggestion) {
        self.suggestions
            .entry(trigger.to_string())
            .or_insert_with(Vec::new)
            .push(suggestion);
    }

    /// Check if the current cursor position should trigger autocompletion
    pub fn check_for_completions(
        &self,
        content: &str,
        line: usize,
        col: usize,
    ) -> Option<(Vec<AutocompleteSuggestion>, usize)> {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return None;
        }

        let current_line = lines[line];
        if col > current_line.len() {
            return None;
        }

        // Extract text from start of line up to cursor
        let line_up_to_cursor = &current_line[..col];
        
        // Only trigger at the beginning of a line or after whitespace
        let should_trigger = line_up_to_cursor.is_empty() 
            || line_up_to_cursor.chars().all(|c| c.is_whitespace())
            || line_up_to_cursor.ends_with(' ');

        if !should_trigger {
            return None;
        }

        // Find the longest matching trigger
        let mut best_match: Option<(String, usize)> = None;
        
        for trigger in self.suggestions.keys() {
            if line_up_to_cursor.ends_with(trigger) {
                let trigger_start = line_up_to_cursor.len() - trigger.len();
                if let Some((_, current_start)) = &best_match {
                    if trigger_start < *current_start {
                        best_match = Some((trigger.clone(), trigger_start));
                    }
                } else {
                    best_match = Some((trigger.clone(), trigger_start));
                }
            }
        }

        if let Some((trigger, start_pos)) = best_match {
            if let Some(suggestions) = self.suggestions.get(&trigger) {
                return Some((suggestions.clone(), start_pos));
            }
        }

        None
    }
}
