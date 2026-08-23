use super::*;

impl App {
    pub fn update_autocompletion(&mut self) {
        if let Some(completions) = self.markdown_autocomplete.check_for_completions(
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
    ///
    /// The template's `$0` decides where the cursor lands, rather than an offset
    /// counted back from the end — every one of those offsets was wrong, and
    /// nothing noticed because the suggestions carrying them could not fire.
    pub fn apply_autocomplete(&mut self) -> bool {
        if !self.autocomplete_state.active {
            return false;
        }

        let Some(suggestion) = self.autocomplete_state.get_selected_suggestion().copied() else {
            return false;
        };
        if self.editor_cursor.0 as usize >= self.editor_content.lines().count() {
            return false;
        }

        let line_start = self.get_line_start_position(self.editor_cursor.0 as usize);
        let trigger_abs_pos = line_start + self.autocomplete_state.trigger_start_pos;
        let cursor_abs_pos = self.cursor_byte_index();
        if trigger_abs_pos > cursor_abs_pos || cursor_abs_pos > self.editor_content.len() {
            return false;
        }

        let (insert, cursor_in_insert) = suggestion.expand();
        let mut new_content = String::with_capacity(self.editor_content.len() + insert.len());
        new_content.push_str(&self.editor_content[..trigger_abs_pos]);
        new_content.push_str(&insert);
        new_content.push_str(&self.editor_content[cursor_abs_pos..]);
        self.editor_content = new_content;

        self.update_cursor_from_absolute_position(trigger_abs_pos + cursor_in_insert);
        self.autocomplete_state.deactivate();
        self.mark_modified();
        true
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
    
    /// Byte offset of the cursor in the buffer.
    ///
    /// The column counts characters; the buffer is indexed in bytes. Anything
    /// that slices `editor_content` at the cursor must go through here, or it
    /// lands inside a multi-byte character and panics.
    pub(crate) fn cursor_byte_index(&self) -> usize {
        crate::vim::offset_of(
            &self.editor_content,
            (self.editor_cursor.0 as usize, self.editor_cursor.1 as usize),
        )
    }

    /// Byte offset of the start of a line.
    pub(crate) fn get_line_start_position(&self, line_index: usize) -> usize {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut pos = 0;
        for line in lines.iter().take(line_index.min(lines.len())) {
            pos += line.len() + 1; // +1 for the newline character
        }
        pos
    }
    
    /// Set the cursor from a byte offset in the buffer.
    ///
    /// The offset is in bytes and the column is in characters, so the conversion
    /// has to count characters rather than subtract offsets.
    pub(crate) fn update_cursor_from_absolute_position(&mut self, abs_pos: usize) {
        let mut consumed = 0;
        let lines: Vec<&str> = self.editor_content.lines().collect();
        for (row, line) in lines.iter().enumerate() {
            if consumed + line.len() >= abs_pos {
                let byte_col = abs_pos - consumed;
                let col = line.char_indices().take_while(|(i, _)| *i < byte_col).count();
                self.editor_cursor = (row as u16, col as u16);
                return;
            }
            consumed += line.len() + 1; // +1 for the newline
        }
        self.editor_cursor = (
            lines.len().saturating_sub(1) as u16,
            lines.last().map(|l| l.chars().count()).unwrap_or(0) as u16,
        );
    }

    /// Hand the open note to the external editor.
    ///
    /// Requested rather than performed here: the buffer does not reach the disk
    /// until the main loop's save runs, and an editor opened before that would
    /// show the last saved version and then have its work overwritten on return.
    pub fn request_external_edit(&mut self) -> Result<(), String> {
        if self.current_note.is_none() {
            return Err("No note selected".to_string());
        }
        if self.external_editor.is_none() {
            return Err("No external editor configured".to_string());
        }
        // Fold the buffer into the note and queue the write the editor will read.
        self.save_current_note()?;
        self.disk.pending_external_edit = true;
        Ok(())
    }
}

#[cfg(test)]
mod external_editor_tests {
    use super::*;
    use crate::models::Note;

    fn app_editing(content: &str) -> App {
        let mut app = App::default();
        app.notebook.notes.clear();
        let mut note = Note::new("N".to_string(), None);
        note.content = content.to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = content.to_string();
        app.external_editor = Some("true".to_string()); // a command that exits 0
        app
    }

    /// The editor must read what is on screen. The buffer only reaches the disk
    /// via the main loop's save, so the request has to fold it into the note and
    /// queue that write before the handoff happens.
    #[test]
    fn requesting_an_edit_queues_the_buffer_for_writing_first() {
        let mut app = app_editing("original");
        app.editor_content = "typed but not saved".to_string();

        app.request_external_edit().unwrap();

        let id = app.current_note.as_ref().unwrap().id;
        assert_eq!(
            app.notebook.notes.get(&id).unwrap().content,
            "typed but not saved",
            "the buffer was not folded into the note"
        );
        assert!(app.disk.dirty_note_ids.contains(&id), "no write was queued");
        assert!(app.disk.pending_disk_save, "the write was not requested");
        assert!(app.disk.pending_external_edit, "the handoff was not requested");
    }

    /// Requested, not performed — the main loop owns the flush and the terminal.
    #[test]
    fn requesting_an_edit_does_not_launch_anything_itself() {
        let mut app = app_editing("x");
        app.request_external_edit().unwrap();
        assert!(!app.just_returned_from_editor, "the handoff ran inside the request");
    }

    #[test]
    fn an_edit_needs_a_note_and_an_editor() {
        let mut app = app_editing("x");
        app.current_note = None;
        assert!(app.request_external_edit().is_err(), "edited with no note open");

        let mut app = app_editing("x");
        app.external_editor = None;
        assert!(app.request_external_edit().is_err(), "edited with no editor configured");
        assert!(!app.disk.pending_external_edit, "a handoff was queued anyway");
    }
}
