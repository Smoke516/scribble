use super::*;

impl App {
    pub fn toggle_recent_files(&mut self) {
        if self.show_recent_files {
            // If already showing, turn off and return to normal
            self.show_recent_files = false;
            self.mode = AppMode::Normal;
        } else {
            // Show recent files and enter RecentFiles mode
            self.show_recent_files = true;
            self.mode = AppMode::RecentFiles;
            self.recent_files_selected = 0;
        }
    }

    pub fn get_recent_files_display(&self) -> Vec<(Uuid, String, String)> {
        self.notebook.get_recent_files()
            .iter()
            .filter_map(|rf| {
                self.notebook.notes.get(&rf.note_id)
                    .map(|note| (
                        rf.note_id,
                        note.title.clone(),
                        rf.last_accessed.format("%m/%d %H:%M").to_string()
                    ))
            })
            .collect()
    }

    pub fn select_recent_file(&mut self, index: usize) {
        let recent_files = self.get_recent_files_display();
        if let Some((note_id, _, _)) = recent_files.get(index) {
            self.open_note_by_id(*note_id);
            self.show_recent_files = false;
            self.mode = AppMode::Normal;
        }
    }

    // Quick Jump functionality
    pub fn start_quick_jump(&mut self) {
        self.mode = AppMode::QuickJump;
        self.quick_jump_query.clear();
        self.quick_jump_selected = 0;
        self.update_quick_jump_results();
    }

    pub fn update_quick_jump_results(&mut self) {
        if self.quick_jump_query.is_empty() {
            // Show recent files when no query
            self.quick_jump_results = self.notebook.get_recent_files()
                .iter()
                .map(|rf| rf.note_id)
                .collect();
        } else {
            // Use fuzzy search
            let results = self.notebook.fuzzy_search_notes(&self.quick_jump_query);
            self.quick_jump_results = results.iter()
                .map(|(note, _score)| note.id)
                .collect();
        }
        
        // Reset selection
        self.quick_jump_selected = 0;
    }

    pub fn quick_jump_navigate_up(&mut self) {
        if self.quick_jump_selected > 0 {
            self.quick_jump_selected -= 1;
        }
    }

    pub fn quick_jump_navigate_down(&mut self) {
        if self.quick_jump_selected < self.quick_jump_results.len().saturating_sub(1) {
            self.quick_jump_selected += 1;
        }
    }

    pub fn quick_jump_select(&mut self) {
        if let Some(&note_id) = self.quick_jump_results.get(self.quick_jump_selected) {
            self.open_note_by_id(note_id);
            self.mode = AppMode::Normal;
            self.quick_jump_query.clear();
        }
    }

    pub fn cancel_quick_jump(&mut self) {
        self.mode = AppMode::Normal;
        self.quick_jump_query.clear();
    }

    pub fn get_quick_jump_results_display(&self) -> Vec<(Uuid, String, String)> {
        self.quick_jump_results.iter()
            .filter_map(|&note_id| {
                self.notebook.notes.get(&note_id)
                    .map(|note| {
                        let folder_name = if let Some(folder_id) = note.folder_id {
                            self.notebook.folders.get(&folder_id)
                                .map(|f| f.name.clone())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Root".to_string()
                        };
                        (note_id, note.title.clone(), folder_name)
                    })
            })
            .collect()
    }

    pub fn preview_scroll_up(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_sub(1);
    }

    pub fn preview_scroll_down(&mut self) {
        if self.preview_scroll < self.preview_max_scroll() {
            self.preview_scroll = self.preview_scroll.saturating_add(1);
        }
    }

    /// Page the preview, by the same ten lines the editor moves.
    pub fn preview_scroll_page_up(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_sub(10);
    }

    pub fn preview_scroll_page_down(&mut self) {
        self.preview_scroll = (self.preview_scroll + 10).min(self.preview_max_scroll());
    }

    pub fn preview_scroll_to_bottom(&mut self) {
        self.preview_scroll = self.preview_max_scroll();
    }

    /// Upper bound for the preview scroll offset. Approximated from the editor's
    /// line count (the rendered preview is close in length); keeps scrolling from
    /// running off into blank space.
    pub(crate) fn preview_max_scroll(&self) -> u16 {
        self.editor_content.lines().count() as u16
    }

    // Welcome page functionality
}
