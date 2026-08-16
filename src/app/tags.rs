use super::*;

impl App {
    pub fn initialize_tag_manager(&mut self) {
        self.tag_manager.build_tag_index(&self.notebook);
    }
    
    pub fn show_tag_browser(&mut self) {
        self.mode = AppMode::TagBrowser;
        self.tag_browser_selected = 0;
    }
    
    pub fn tag_browser_navigate_up(&mut self) {
        if self.tag_browser_selected > 0 {
            self.tag_browser_selected -= 1;
        }
    }
    
    pub fn tag_browser_navigate_down(&mut self) {
        let tag_count = if self.tag_browser_sort_by_frequency {
            self.tag_manager.get_tags_by_frequency().len()
        } else {
            self.tag_manager.get_tags_alphabetical().len()
        };
        
        if self.tag_browser_selected < tag_count.saturating_sub(1) {
            self.tag_browser_selected += 1;
        }
    }
    
    pub fn toggle_tag_browser_sort(&mut self) {
        self.tag_browser_sort_by_frequency = !self.tag_browser_sort_by_frequency;
        self.tag_browser_selected = 0;
    }
    
    pub fn get_tag_browser_items(&self) -> Vec<(&str, usize)> {
        let tags = if self.tag_browser_sort_by_frequency {
            self.tag_manager.get_tags_by_frequency()
        } else {
            self.tag_manager.get_tags_alphabetical()
        };
        
        tags.iter().map(|tag| (tag.name.as_str(), tag.count)).collect()
    }
    
    pub fn add_tag_filter(&mut self) {
        let tags = if self.tag_browser_sort_by_frequency {
            self.tag_manager.get_tags_by_frequency()
        } else {
            self.tag_manager.get_tags_alphabetical()
        };
        
        if let Some(selected_tag) = tags.get(self.tag_browser_selected) {
            let tag_name = selected_tag.name.clone();
            if !self.tag_filter_active.contains(&tag_name) {
                self.tag_filter_active.push(tag_name.clone());
                self.apply_tag_filter();
                self.set_operation_info(
                    format!("Added filter: #{}", tag_name),
                    Some("🏷️".to_string())
                );
            }
        }
    }
    
    pub fn remove_tag_filter(&mut self, tag_name: &str) {
        self.tag_filter_active.retain(|t| t != tag_name);
        self.apply_tag_filter();
        self.set_operation_info(
            format!("Removed filter: #{}", tag_name),
            Some("🗑️".to_string())
        );
    }
    
    pub fn clear_tag_filters(&mut self) {
        self.tag_filter_active.clear();
        self.apply_tag_filter();
        self.set_message("Cleared all tag filters".to_string());
    }
    
    pub(crate) fn apply_tag_filter(&mut self) {
        if self.tag_filter_active.is_empty() {
            self.tag_filter_note_ids.clear();
        } else {
            self.tag_filter_note_ids = self.tag_manager
                .get_notes_with_any_tags(&self.notebook, &self.tag_filter_active)
                .iter()
                .map(|n| n.id)
                .collect();
        }
        self.refresh_tree_view();
    }
    
    pub fn get_tag_suggestions(&self, partial: &str) -> Vec<String> {
        self.tag_manager.get_tag_suggestions(partial, 10)
    }

    // ── Tag input (edit the current note's tags) ──────────────────────────

    /// Enter tag-input mode for the open note (no-op when no note is open).
    /// Syncs inline `#tags` from the body first so the list is complete.
    pub fn start_tag_input(&mut self) {
        if self.current_note.is_some() {
            self.sync_current_note_tags();
            self.mode = AppMode::TagInput;
            self.input_buffer.clear();
        }
    }

    pub fn cancel_tag_input(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
    }

    /// Add the tag in the input buffer to the open note. Stays in tag-input mode
    /// so several tags can be added in a row; Esc finishes.
    pub fn submit_tag_input(&mut self) {
        let tag = self
            .input_buffer
            .trim()
            .trim_start_matches('#')
            .trim()
            .to_string();
        if !tag.is_empty() {
            self.add_tag_to_current_note(tag);
        }
        self.input_buffer.clear();
    }

    /// Tags on the currently open note, from both places a tag can be written.
    ///
    /// Reading only the frontmatter field meant the tag dialog could tell you a
    /// note had no tags while the browser and the palette both listed the
    /// `#hashtags` in its body — the same disagreement, in the one place you would
    /// go to check.
    pub fn current_note_tags(&self) -> Vec<String> {
        let Some(note) = self.current_note.as_ref() else {
            return Vec::new();
        };
        let mut tags: Vec<String> = self
            .tag_manager
            .extract_tags_from_note(note)
            .into_iter()
            .collect();
        tags.sort();
        tags
    }

    /// Remove the most recently added tag (Backspace on an empty input).
    pub fn remove_last_tag_from_current_note(&mut self) {
        if let Some(tag) = self.current_note.as_ref().and_then(|n| n.tags.last().cloned()) {
            self.remove_tag_from_current_note(tag);
        }
    }

    pub fn add_tag_to_current_note(&mut self, tag: String) {
        if let Some(current_note_id) = self.current_note.as_ref().map(|n| n.id) {
            if let Some(note) = self.notebook.notes.get_mut(&current_note_id) {
                self.tag_manager.add_tags_to_note(note, vec![tag.clone()]);
                
                // Update current note reference
                if let Some(ref mut current) = self.current_note {
                    *current = note.clone();
                }
                
                self.set_operation_info(
                    format!("Added tag: #{}", tag),
                    Some("➕".to_string())
                );
                self.mark_note_dirty(current_note_id);
            }
        }
    }
    
    pub fn remove_tag_from_current_note(&mut self, tag: String) {
        if let Some(current_note_id) = self.current_note.as_ref().map(|n| n.id) {
            if let Some(note) = self.notebook.notes.get_mut(&current_note_id) {
                self.tag_manager.remove_tags_from_note(note, vec![tag.clone()]);
                
                // Update current note reference
                if let Some(ref mut current) = self.current_note {
                    *current = note.clone();
                }
                
                self.set_operation_info(
                    format!("Removed tag: #{}", tag),
                    Some("🗑️".to_string())
                );
                self.mark_note_dirty(current_note_id);
            }
        }
    }
    
    pub fn sync_current_note_tags(&mut self) {
        if let Some(current_note_id) = self.current_note.as_ref().map(|n| n.id) {
            if let Some(note) = self.notebook.notes.get_mut(&current_note_id) {
                self.tag_manager.sync_note_tags(note);
                
                // Update current note reference
                if let Some(ref mut current) = self.current_note {
                    *current = note.clone();
                }
                
                // Rebuild tag index after sync
                self.tag_manager.build_tag_index(&self.notebook);
            }
        }
    }
    
    pub fn cancel_tag_browser(&mut self) {
        self.mode = AppMode::Normal;
    }
    
    pub fn get_tag_stats(&self) -> (usize, usize) {
        (
            self.tag_manager.get_tag_count(),
            self.tag_manager.get_tagged_note_count()
        )
    }
    
    // Theme management methods
}

#[cfg(test)]
mod tag_visibility_tests {
    use super::*;
    use crate::models::Note;

    fn app_with_note(tags: &[&str], content: &str) -> App {
        let mut app = App::default();
        app.notebook.notes.clear();
        app.notebook.folders.clear();
        let mut note = Note::new("N".to_string(), None);
        note.tags = tags.iter().map(|t| t.to_string()).collect();
        note.content = content.to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app
    }

    /// The dialog used to read only the frontmatter field, so it could report no
    /// tags on a note the browser and palette both listed as tagged.
    #[test]
    fn the_open_notes_tags_include_inline_hashtags() {
        let app = app_with_note(&["work"], "some body with #idea in it\n");
        assert_eq!(app.current_note_tags(), vec!["idea", "work"]);
    }

    #[test]
    fn a_tag_written_both_ways_is_listed_once() {
        let app = app_with_note(&["work"], "text #work\n");
        assert_eq!(app.current_note_tags(), vec!["work"]);
    }

    #[test]
    fn an_untagged_note_has_no_tags() {
        let app = app_with_note(&[], "plain text\n");
        assert!(app.current_note_tags().is_empty());
    }

    /// Code comments are not tags, here as everywhere else.
    #[test]
    fn a_hashtag_inside_a_code_fence_is_not_listed() {
        let app = app_with_note(&[], "```\n# not a tag\nx = 1  #result\n```\n");
        assert!(app.current_note_tags().is_empty());
    }

    #[test]
    fn with_no_note_open_there_are_no_tags() {
        let mut app = App::default();
        app.current_note = None;
        assert!(app.current_note_tags().is_empty());
    }
}
