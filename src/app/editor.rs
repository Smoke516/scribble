use super::*;

impl App {
    pub fn update_autocompletion(&mut self) {
        // Wiki-link completion takes priority over markdown snippets
        let note_titles: Vec<String> = self.notebook.notes.values()
            .map(|n| n.title.clone()).collect();
        if let Some(completions) = crate::autocomplete::MarkdownAutocomplete::check_for_wiki_completions(
            &self.editor_content,
            self.editor_cursor.0 as usize,
            self.editor_cursor.1 as usize,
            &note_titles,
        ) {
            self.autocomplete_state.activate(completions.0, completions.1);
        } else if let Some(completions) = self.markdown_autocomplete.check_for_completions(
            &self.editor_content,
            self.editor_cursor.0 as usize,
            self.editor_cursor.1 as usize,
        ) {
            self.autocomplete_state.activate(completions.0, completions.1);
        } else {
            self.autocomplete_state.deactivate();
        }
    }
    
    /// Apply the selected autocompletion
    pub fn apply_autocomplete(&mut self) -> bool {
        if !self.autocomplete_state.active {
            return false;
        }
        
        if let Some(suggestion) = self.autocomplete_state.get_selected_suggestion() {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            if self.editor_cursor.0 as usize >= lines.len() {
                return false;
            }
            
            let _current_line = lines[self.editor_cursor.0 as usize];
            let line_start = self.get_line_start_position(self.editor_cursor.0 as usize);
            
            // Calculate the absolute position in the content
            let trigger_abs_pos = line_start + self.autocomplete_state.trigger_start_pos;
            let cursor_abs_pos = line_start + self.editor_cursor.1 as usize;
            
            // Remove the trigger text and insert the completion
            let mut new_content = String::new();
            new_content.push_str(&self.editor_content[..trigger_abs_pos]);
            new_content.push_str(&suggestion.completion);
            new_content.push_str(&self.editor_content[cursor_abs_pos..]);
            
            self.editor_content = new_content;
            
            // Update cursor position
            let completion_end_pos = trigger_abs_pos + suggestion.completion.len();
            let new_cursor_pos = if suggestion.cursor_offset >= 0 {
                completion_end_pos + suggestion.cursor_offset as usize
            } else {
                completion_end_pos.saturating_sub((-suggestion.cursor_offset) as usize)
            };
            
            // Convert absolute position back to line/column
            self.update_cursor_from_absolute_position(new_cursor_pos);
            
            self.autocomplete_state.deactivate();
            self.mark_modified();
            return true;
        }
        
        false
    }
    
    /// Move to next autocomplete suggestion
    pub fn next_autocomplete_suggestion(&mut self) {
        self.autocomplete_state.next_suggestion();
    }
    
    /// Move to previous autocomplete suggestion
    pub fn previous_autocomplete_suggestion(&mut self) {
        self.autocomplete_state.previous_suggestion();
    }
    
    /// Cancel autocompletion
    pub fn cancel_autocomplete(&mut self) {
        self.autocomplete_state.deactivate();
    }
    
    /// Get the absolute character position of the start of a line
    pub(crate) fn get_line_start_position(&self, line_index: usize) -> usize {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut pos = 0;
        for line in lines.iter().take(line_index.min(lines.len())) {
            pos += line.len() + 1; // +1 for the newline character
        }
        pos
    }
    
    /// Update cursor position from absolute character position
    pub(crate) fn update_cursor_from_absolute_position(&mut self, abs_pos: usize) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut current_pos = 0;
        
        for (line_index, line) in lines.iter().enumerate() {
            if current_pos + line.len() >= abs_pos {
                self.editor_cursor.0 = line_index as u16;
                self.editor_cursor.1 = (abs_pos - current_pos) as u16;
                return;
            }
            current_pos += line.len() + 1; // +1 for newline
        }
        
        // If we get here, position is at the end
        self.editor_cursor.0 = lines.len().saturating_sub(1) as u16;
        self.editor_cursor.1 = lines.last().unwrap_or(&"").len() as u16;
    }

    pub fn open_in_external_editor(&mut self) -> Result<(), String> {
        if let Some(ref note) = self.current_note {
            if let Some(ref editor) = self.external_editor {
                // Create a temporary file with the note content
                let temp_path = create_temp_file(&note.title, &note.content)
                    .map_err(|e| format!("Failed to create temp file: {}", e))?;
                
                // Save the current terminal state and run the external editor
                let result = run_external_editor(editor, &temp_path);
                
                match result {
                    Ok(()) => {
                        // Read the content back from the temp file
                        match std::fs::read_to_string(&temp_path) {
                            Ok(new_content) => {
                                self.editor_content = new_content;
                                // Auto-save the changes
                                if let Err(e) = self.save_current_note() {
                                    self.set_message(format!("Failed to save: {}", e));
                                } else {
                                    self.set_message("✅ Note updated from external editor".to_string());
                                }
                                // Set flag to indicate we just returned from external editor
                                self.just_returned_from_editor = true;
                            }
                            Err(e) => {
                                self.set_message(format!("Failed to read edited file: {}", e));
                            }
                        }
                        
                        // Clean up temp file
                        let _ = std::fs::remove_file(&temp_path);
                    }
                    Err(e) => {
                        self.set_message(format!("External editor failed: {}", e));
                        let _ = std::fs::remove_file(&temp_path);
                    }
                }
                
                Ok(())
            } else {
                Err("No external editor configured".to_string())
            }
        } else {
            Err("No note selected".to_string())
        }
    }
}
