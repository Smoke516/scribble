#![allow(dead_code)] // palette/API surface; some helpers kept for completeness
use regex::Regex;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use crate::models::{Note, NotebookData};

/// The parts of a line that are not inside a backtick span.
///
/// `` `#!/bin/sh` `` in prose is a code sample, not a tag, and splitting on
/// backticks is enough to keep those out without parsing markdown properly.
fn strip_inline_code(line: &str) -> Vec<String> {
    line.split('`')
        .step_by(2) // even segments are outside the spans
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
    pub count: usize,
    pub notes: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct TagManager {
    pub tag_index: HashMap<String, TagInfo>,
    pub inline_tag_regex: Regex,
}

impl TagManager {
    pub fn new() -> Self {
        Self {
            tag_index: HashMap::new(),
            // A tag has to START a word, or `###BASICS` and every `x#y` in a URL
            // becomes one. The first character must not be a digit, so `#1` in
            // prose stays prose.
            inline_tag_regex: Regex::new(r"(?:^|\s)#([a-zA-Z][a-zA-Z0-9_/-]*)").unwrap(),
        }
    }

    /// Extract all tags from a note (both YAML frontmatter and inline hashtags)
    pub fn extract_tags_from_note(&self, note: &Note) -> HashSet<String> {
        let mut tags = HashSet::new();
        
        // Add tags from the note's tags field (usually from YAML frontmatter)
        for tag in &note.tags {
            tags.insert(tag.clone());
        }
        
        // Extract inline hashtags from content
        let inline_tags = self.extract_inline_tags(&note.content);
        tags.extend(inline_tags);
        
        tags
    }

    /// Extract inline hashtags from markdown content.
    ///
    /// Code and headings are skipped, which is most of the work. `#` starts a
    /// comment in Python, Bash, Ruby and others, so a cheat sheet full of
    /// `#result` and `#my_age is now 16` was reporting a dozen tags that were
    /// never tags — and a heading written `### #BASICS` was reporting another.
    /// Counting those made the vault look tagged when nothing in it was.
    pub fn extract_inline_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let mut in_fence = false;

        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            // A heading is structure, not a tag, even when it contains a `#`.
            // The test is the same one the outline uses — `#` repeated then a
            // space — because `#inbox` at the start of a line is a tag, not a
            // heading, and treating it as one loses the most natural way to write
            // a tag.
            let level = trimmed.bytes().take_while(|&b| b == b'#').count();
            let is_heading = (1..=6).contains(&level) && trimmed[level..].starts_with(' ');
            if in_fence || is_heading {
                continue;
            }

            for segment in strip_inline_code(line) {
                for cap in self.inline_tag_regex.captures_iter(&segment) {
                    tags.push(cap[1].to_string());
                }
            }
        }
        tags
    }

    /// Extract tags from YAML frontmatter
    pub fn extract_yaml_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();
        
        if let Some(frontmatter) = self.extract_yaml_frontmatter(content) {
            // Try to parse as YAML and extract tags
            if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(&frontmatter) {
                if let Some(yaml_tags) = yaml_value.get("tags") {
                    match yaml_tags {
                        serde_yaml::Value::Sequence(seq) => {
                            for item in seq {
                                if let Some(tag_str) = item.as_str() {
                                    tags.push(tag_str.to_string());
                                }
                            }
                        },
                        serde_yaml::Value::String(tag_str) => {
                            // Single tag as string
                            tags.push(tag_str.clone());
                        },
                        _ => {}
                    }
                }
            }
        }
        
        tags
    }

    /// Extract YAML frontmatter from content
    fn extract_yaml_frontmatter(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.is_empty() || lines[0] != "---" {
            return None;
        }
        
        let mut frontmatter_lines = Vec::new();
        let mut in_frontmatter = false;
        let mut end_found = false;
        
        for (i, line) in lines.iter().enumerate() {
            if i == 0 && *line == "---" {
                in_frontmatter = true;
                continue;
            }
            
            if in_frontmatter {
                if *line == "---" {
                    end_found = true;
                    break;
                }
                frontmatter_lines.push(*line);
            }
        }
        
        if end_found {
            Some(frontmatter_lines.join("\n"))
        } else {
            None
        }
    }

    /// Build tag index from all notes in the notebook
    pub fn build_tag_index(&mut self, notebook: &NotebookData) {
        self.tag_index.clear();
        
        for note in notebook.notes.values() {
            let tags = self.extract_tags_from_note(note);
            
            for tag in tags {
                let tag_info = self.tag_index.entry(tag.clone()).or_insert_with(|| TagInfo {
                    name: tag.clone(),
                    count: 0,
                    notes: Vec::new(),
                });
                
                tag_info.count += 1;
                if !tag_info.notes.contains(&note.id) {
                    tag_info.notes.push(note.id);
                }
            }
        }
    }

    /// Get all tags sorted by frequency (most used first)
    pub fn get_tags_by_frequency(&self) -> Vec<&TagInfo> {
        let mut tags: Vec<&TagInfo> = self.tag_index.values().collect();
        tags.sort_by(|a, b| b.count.cmp(&a.count));
        tags
    }

    /// Get all tags sorted alphabetically
    pub fn get_tags_alphabetical(&self) -> Vec<&TagInfo> {
        let mut tags: Vec<&TagInfo> = self.tag_index.values().collect();
        tags.sort_by(|a, b| a.name.cmp(&b.name));
        tags
    }

    /// Search for tags matching a pattern
    pub fn search_tags(&self, pattern: &str) -> Vec<&TagInfo> {
        let pattern_lower = pattern.to_lowercase();
        self.tag_index
            .values()
            .filter(|tag| tag.name.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    /// Get notes that have specific tags
    pub fn get_notes_with_tags<'a>(&self, notebook: &'a NotebookData, tag_names: &[String]) -> Vec<&'a Note> {
        let mut matching_notes = Vec::new();
        
        for note in notebook.notes.values() {
            let note_tags = self.extract_tags_from_note(note);
            let has_all_tags = tag_names.iter().all(|tag| note_tags.contains(tag));
            
            if has_all_tags {
                matching_notes.push(note);
            }
        }
        
        matching_notes
    }

    /// Get notes that have any of the specified tags
    pub fn get_notes_with_any_tags<'a>(&self, notebook: &'a NotebookData, tag_names: &[String]) -> Vec<&'a Note> {
        let mut matching_notes = Vec::new();
        
        for note in notebook.notes.values() {
            let note_tags = self.extract_tags_from_note(note);
            let has_any_tag = tag_names.iter().any(|tag| note_tags.contains(tag));
            
            if has_any_tag {
                matching_notes.push(note);
            }
        }
        
        matching_notes
    }

    /// Get tag suggestions based on partial input
    pub fn get_tag_suggestions(&self, partial: &str, limit: usize) -> Vec<String> {
        let partial_lower = partial.to_lowercase();
        let mut suggestions: Vec<_> = self.tag_index
            .keys()
            .filter(|tag| tag.to_lowercase().starts_with(&partial_lower))
            .cloned()
            .collect();
        
        // Sort by tag frequency (most used first)
        suggestions.sort_by(|a, b| {
            let count_a = self.tag_index.get(a).map(|info| info.count).unwrap_or(0);
            let count_b = self.tag_index.get(b).map(|info| info.count).unwrap_or(0);
            count_b.cmp(&count_a)
        });
        
        suggestions.truncate(limit);
        suggestions
    }

    /// Add tags to a note (both to the note struct and update the index)
    pub fn add_tags_to_note(&mut self, note: &mut Note, tags: Vec<String>) {
        for tag in tags {
            if !note.tags.contains(&tag) {
                note.add_tag(tag.clone());
                
                // Update index
                let tag_info = self.tag_index.entry(tag.clone()).or_insert_with(|| TagInfo {
                    name: tag.clone(),
                    count: 0,
                    notes: Vec::new(),
                });
                
                if !tag_info.notes.contains(&note.id) {
                    tag_info.count += 1;
                    tag_info.notes.push(note.id);
                }
            }
        }
    }

    /// Remove tags from a note (both from note struct and update the index)
    pub fn remove_tags_from_note(&mut self, note: &mut Note, tags: Vec<String>) {
        for tag in tags {
            if note.tags.contains(&tag) {
                note.remove_tag(&tag);
                
                // Update index
                if let Some(tag_info) = self.tag_index.get_mut(&tag) {
                    tag_info.notes.retain(|&id| id != note.id);
                    tag_info.count = tag_info.count.saturating_sub(1);
                    
                    // Remove tag from index if no notes have it
                    if tag_info.count == 0 {
                        self.tag_index.remove(&tag);
                    }
                }
            }
        }
    }

    /// Get total number of unique tags
    pub fn get_tag_count(&self) -> usize {
        self.tag_index.len()
    }

    /// Get total number of tagged notes
    pub fn get_tagged_note_count(&self) -> usize {
        let mut unique_notes: HashSet<Uuid> = HashSet::new();
        for tag_info in self.tag_index.values() {
            unique_notes.extend(&tag_info.notes);
        }
        unique_notes.len()
    }

    /// Update a note's tags based on its content and YAML frontmatter
    /// Fold the tags written in a note's body into its tag list.
    ///
    /// Merges rather than replaces. This used to assign the extracted tags over
    /// `note.tags`, and storage strips frontmatter out of `content` before storing
    /// it — so `extract_yaml_tags` found nothing and every frontmatter tag was
    /// dropped. Opening the tag dialog wiped the note's tags, and the next save
    /// wrote that loss to disk: a tag added this way survived exactly until the
    /// next time you looked at it.
    ///
    /// Sorted, because the set had no order of its own and the frontmatter `tags:`
    /// line was being rewritten in a different order on every save — which rewrites
    /// the file, which makes the sync client re-upload a note nothing changed in.
    pub fn sync_note_tags(&mut self, note: &mut Note) {
        let mut all: HashSet<String> = note.tags.iter().cloned().collect();
        all.extend(self.extract_inline_tags(&note.content));
        // A no-op for vault storage, which has already parsed the frontmatter out,
        // but the single-file backend can still have it sitting in `content`.
        all.extend(self.extract_yaml_tags(&note.content));

        let mut tags: Vec<String> = all.into_iter().collect();
        tags.sort();
        note.tags = tags;
    }

    /// Format tags for display in UI
    pub fn format_tag_list(&self, tags: &[String]) -> String {
        if tags.is_empty() {
            "No tags".to_string()
        } else {
            tags.iter()
                .map(|tag| format!("#{}", tag))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    /// Check if a tag name is valid
    pub fn is_valid_tag_name(name: &str) -> bool {
        !name.is_empty() && 
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
        !name.starts_with(|c: char| c.is_numeric())
    }
}

impl Default for TagManager {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn tags_in(content: &str) -> Vec<String> {
        let mut t = TagManager::new().extract_inline_tags(content);
        t.sort();
        t.dedup();
        t
    }

    #[test]
    fn a_hashtag_in_prose_is_a_tag() {
        assert_eq!(tags_in("Talked to Sam #work about #follow-up"), vec!["follow-up", "work"]);
    }

    #[test]
    fn a_tag_can_start_the_line() {
        assert_eq!(tags_in("#inbox and some text"), vec!["inbox"]);
    }

    /// `#` opens a comment in Python, Bash and Ruby. A cheat sheet full of them was
    /// reporting a dozen tags that were never tags.
    #[test]
    fn comments_inside_a_fenced_block_are_not_tags() {
        let content = "\
notes about python

```python
# This is a comment
x = 1  #result 354.0
```

back to #realtag
";
        assert_eq!(tags_in(content), vec!["realtag"]);
    }

    #[test]
    fn inline_code_spans_are_not_tags() {
        assert_eq!(tags_in("run `#!/bin/sh` first, then tag it #shell"), vec!["shell"]);
    }

    /// A heading is structure. `### #BASICS` was being counted as a tag.
    #[test]
    fn headings_are_not_tags() {
        assert_eq!(tags_in("### #BASICS\nsome text"), Vec::<String>::new());
        assert_eq!(tags_in("# Title\n#realtag here"), vec!["realtag"]);
    }

    /// A tag has to start a word, or every `#` in a URL fragment becomes one.
    #[test]
    fn a_hash_in_the_middle_of_a_word_is_not_a_tag() {
        assert_eq!(tags_in("see https://example.com/page#section"), Vec::<String>::new());
        assert_eq!(tags_in("C#Sharp"), Vec::<String>::new());
    }

    #[test]
    fn a_numeric_hash_is_not_a_tag() {
        assert_eq!(tags_in("issue #123 is open"), Vec::<String>::new());
    }

    #[test]
    fn nested_tags_keep_their_slashes() {
        assert_eq!(tags_in("filed under #work/admin"), vec!["work/admin"]);
    }

    /// The bug that made tagging pointless: `sync_note_tags` assigned the tags
    /// extracted from `content` over `note.tags`, and storage strips frontmatter
    /// out of `content` before storing it — so the extraction found nothing and
    /// every frontmatter tag was dropped. Opening the tag dialog wiped the note's
    /// tags, and the next save wrote that loss to disk.
    #[test]
    fn syncing_keeps_the_tags_a_note_already_had() {
        let mut tm = TagManager::new();
        let mut note = Note::new("N".to_string(), None);
        // As storage loads it: frontmatter parsed into `tags`, body without it.
        note.tags = vec!["work".to_string(), "urgent".to_string()];
        note.content = "body text with #inline\n".to_string();

        tm.sync_note_tags(&mut note);

        assert!(note.tags.contains(&"work".to_string()), "a frontmatter tag was destroyed");
        assert!(note.tags.contains(&"urgent".to_string()), "a frontmatter tag was destroyed");
        assert!(note.tags.contains(&"inline".to_string()), "the inline tag was not folded in");
    }

    /// Syncing twice must not keep changing the note, or every save rewrites the
    /// file and the sync client re-uploads a note nothing changed in.
    #[test]
    fn syncing_is_stable_and_ordered() {
        let mut tm = TagManager::new();
        let mut note = Note::new("N".to_string(), None);
        note.tags = vec!["zebra".to_string(), "apple".to_string()];
        note.content = "text #mango\n".to_string();

        tm.sync_note_tags(&mut note);
        let once = note.tags.clone();
        tm.sync_note_tags(&mut note);

        assert_eq!(once, note.tags, "a second sync changed the note again");
        assert_eq!(once, vec!["apple", "mango", "zebra"], "tags are not in a stable order");
    }

    #[test]
    fn syncing_does_not_duplicate_a_tag_written_in_both_places() {
        let mut tm = TagManager::new();
        let mut note = Note::new("N".to_string(), None);
        note.tags = vec!["work".to_string()];
        note.content = "text #work\n".to_string();

        tm.sync_note_tags(&mut note);
        assert_eq!(note.tags, vec!["work"]);
    }

    /// Frontmatter tags are a separate source and must still come through.
    #[test]
    fn frontmatter_tags_are_still_collected() {
        let mut note = Note::new("T".to_string(), None);
        note.tags = vec!["explicit".to_string()];
        note.content = "body with #inline".to_string();
        let found = TagManager::new().extract_tags_from_note(&note);
        assert!(found.contains("explicit"));
        assert!(found.contains("inline"));
    }
}
