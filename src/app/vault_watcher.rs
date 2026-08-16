use super::*;

impl App {
    pub fn initialize_available_vaults(&mut self, config: &crate::config::Config) {
        self.available_vaults.clear();
        
        // Add recent vaults from config
        for vault in &config.vaults.recent {
            if vault.exists() && vault.join(".obsidian").exists() {
                self.available_vaults.push(vault.clone());
            }
        }
        
        // Add default vault if not already in recent
        if let Some(default_vault) = &config.vaults.default {
            if default_vault.exists() && default_vault.join(".obsidian").exists()
                && !self.available_vaults.contains(default_vault) {
                    self.available_vaults.push(default_vault.clone());
                }
        }
        
        // Add current directory if it's a vault
        if let Ok(current_dir) = std::env::current_dir() {
            if current_dir.join(".obsidian").exists()
                && !self.available_vaults.contains(&current_dir) {
                    self.available_vaults.push(current_dir);
                }
        }
        
        // Scan common locations for additional vaults
        if let Some(home_dir) = dirs::home_dir() {
            let common_vault_locations = [
                home_dir.join("Documents"),
                home_dir.join("Nextcloud"),
                home_dir.join("Dropbox"),
                home_dir.join("OneDrive"),
                home_dir.join("obsidian-vaults"),
            ];
            
            for location in &common_vault_locations {
                if location.exists() {
                    if let Ok(entries) = std::fs::read_dir(location) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.is_dir() && path.join(".obsidian").exists()
                                && !self.available_vaults.contains(&path) {
                                    self.available_vaults.push(path);
                                }
                        }
                    }
                }
            }
        }
    }
    
    pub fn show_vault_switcher(&mut self) {
        self.mode = AppMode::VaultSwitcher;
        self.vault_switcher_selected = 0;
    }
    
    pub fn vault_switcher_navigate_up(&mut self) {
        if self.vault_switcher_selected > 0 {
            self.vault_switcher_selected -= 1;
        }
    }
    
    pub fn vault_switcher_navigate_down(&mut self) {
        if self.vault_switcher_selected < self.available_vaults.len().saturating_sub(1) {
            self.vault_switcher_selected += 1;
        }
    }
    
    pub fn get_selected_vault(&self) -> Option<&std::path::PathBuf> {
        self.available_vaults.get(self.vault_switcher_selected)
    }
    
    pub fn get_vault_display_info(&self) -> Vec<(String, String)> {
        self.available_vaults.iter()
            .map(|vault| {
                let name = vault.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let path = vault.display().to_string();
                (name, path)
            })
            .collect()
    }
    
    pub fn cancel_vault_switcher(&mut self) {
        self.mode = AppMode::Normal;
    }

    /// Ask the main loop to switch to the highlighted vault.
    ///
    /// The app cannot do this itself: storage is owned by the main loop, and
    /// swapping it means flushing whatever is still owed to the *current* vault
    /// first. Requesting it here and performing it there is the same shape as
    /// pending folder relocations.
    pub fn request_vault_switch(&mut self) {
        let Some(vault) = self.get_selected_vault().cloned() else {
            return;
        };
        self.cancel_vault_switcher();

        if self.vault_path.as_ref() == Some(&vault) {
            self.set_message("Already in that vault".to_string());
            return;
        }
        self.disk.pending_vault_switch = Some(vault);
    }

    /// Adopt a freshly-loaded notebook from another vault.
    ///
    /// Everything keyed to the old vault has to go, not merely be ignored.
    /// `deleted_note_paths` is the sharp one: it holds absolute paths, and
    /// carrying it across would delete files in the vault we just left on the next
    /// write. The undo stack, cursor memory and open note are all equally
    /// meaningless against a different set of notes.
    pub fn adopt_vault(&mut self, vault: std::path::PathBuf, notebook: NotebookData) {
        self.notebook = notebook;
        self.vault_path = Some(vault.clone());

        self.disk = DiskState::default();

        self.current_note = None;
        self.editor_content.clear();
        self.editor_cursor = (0, 0);
        self.editor_scroll = 0;
        self.note_cursor_map.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.search_results.clear();
        self.yank_buffer.clear();

        self.palette_query.clear();
        self.palette_items.clear();
        self.palette_selected = 0;
        self.task_items.clear();
        self.task_selected = 0;
        self.outline_headings.clear();
        self.outline_selected = 0;

        self.initialize_tag_manager();
        self.refresh_tree_view();
        self.set_welcome_message();
        self.mode = AppMode::Normal;
        self.focused_pane = FocusedPane::Folders;

        let name = vault
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.set_operation_info(format!("Switched to vault: {}", name), Some("📁".to_string()));
    }
    
    // File watcher functionality
    pub fn initialize_file_watcher(&mut self, vault_path: std::path::PathBuf) {
        match FileWatcher::new(vault_path.clone()) {
            Ok(watcher) => {
                self.file_watcher = Some(watcher);
                self.sync_status = "📁 Watching vault".to_string();
            },
            Err(e) => {
                eprintln!("Failed to initialize file watcher: {}", e);
                self.sync_status = "⚠️  File watching unavailable".to_string();
            }
        }
    }
    
    pub fn poll_file_changes(&mut self) {
        if let Some(watcher) = &self.file_watcher {
            let changes = watcher.poll_changes();
            if changes.is_empty() {
                return;
            }
            // Ignore (but drain) events caused by our own recent save, so saving
            // your note never shows a spurious "modified externally" message.
            if self.wrote_recently() {
                return;
            }
            self.has_external_changes = true;
            self.handle_file_changes(changes);
        }
    }
    
    pub(crate) fn handle_file_changes(&mut self, changes: Vec<FileChangeEvent>) {
        for change in changes {
            match change {
                FileChangeEvent::Modified(path) => {
                    self.handle_file_modified(path);
                },
                FileChangeEvent::Created(path) => {
                    self.handle_file_created(path);
                },
                FileChangeEvent::Deleted(path) => {
                    self.handle_file_deleted(path);
                },
                FileChangeEvent::Renamed(from, to) => {
                    self.handle_file_renamed(from, to);
                },
            }
        }
        
        // Refresh the tree view after processing all changes
        self.refresh_tree_view();
        
        // Update sync status
        self.sync_status = "🔄 External changes detected".to_string();
        self.set_message("File changes detected from external source".to_string());
    }
    
    /// A note's file changed on disk.
    ///
    /// If we have nothing unsaved for that note, take the disk version — there is
    /// nothing to lose, and leaving the in-memory copy stale is how a later edit
    /// silently reverts somebody else's work. If we *do* have unsaved changes, leave
    /// the buffer alone and say so; the write path preserves whichever version it
    /// finds rather than overwriting it, so nothing is riding on this notification.
    pub(crate) fn handle_file_modified(&mut self, path: std::path::PathBuf) {
        let Some(note_id) = self
            .notebook
            .notes
            .values()
            .find(|n| n.file_path.as_deref() == Some(path.as_path()))
            .map(|n| n.id)
        else {
            // Not a file we have a note for. `handle_file_created` covers arrivals.
            return;
        };

        if self.has_unsaved_work_for(note_id) {
            let title = self
                .notebook
                .notes
                .get(&note_id)
                .map(|n| n.title.clone())
                .unwrap_or_default();
            self.set_operation_info(
                format!("'{}' also changed on disk — both versions will be kept", title),
                Some("⚔️".to_string()),
            );
            return;
        }

        self.reload_note_from_disk(note_id, &path);
    }

    /// Whether anything would be lost by replacing this note with the disk version.
    ///
    /// The dirty set is the main answer, but the editor buffer is checked too: it
    /// holds keystrokes that have not been folded back into the note yet, and those
    /// are exactly the edits a reload would throw away.
    fn has_unsaved_work_for(&self, note_id: uuid::Uuid) -> bool {
        if self.disk.dirty_note_ids.contains(&note_id) {
            return true;
        }
        match (&self.current_note, self.notebook.notes.get(&note_id)) {
            (Some(current), Some(stored)) if current.id == note_id => {
                self.editor_content != stored.content
            }
            _ => false,
        }
    }

    /// Replace a note with what is now on disk, refreshing the editor if that note
    /// happens to be open.
    pub(crate) fn reload_note_from_disk(&mut self, note_id: uuid::Uuid, path: &std::path::Path) {
        let Some(vault) = self.vault_path.clone() else {
            return;
        };
        let Ok(storage) = crate::storage::VaultStorage::new(vault) else {
            return;
        };
        let Some(fresh) = storage.load_single_note(path) else {
            return;
        };

        let Some(note) = self.notebook.notes.get_mut(&note_id) else {
            return;
        };
        let title = fresh.title.clone();
        // Keep the identity we already have. The id, and the note's place in the
        // tree, are ours; only the file's contents are the disk's to change.
        note.title = fresh.title;
        note.content = fresh.content;
        note.tags = fresh.tags;
        note.modified_at = fresh.modified_at;
        note.disk_stamp = fresh.disk_stamp;
        let content = note.content.clone();

        if self.current_note.as_ref().map(|n| n.id) == Some(note_id) {
            self.current_note = self.notebook.notes.get(&note_id).cloned();
            self.editor_content = content;
            // The note got shorter while we were looking away; a cursor left past
            // the end would index outside the buffer.
            self.clamp_cursor_to_content();
        }

        self.set_operation_info(
            format!("'{}' reloaded from disk", title),
            Some("🔄".to_string()),
        );
    }
    
    pub(crate) fn handle_file_created(&mut self, path: std::path::PathBuf) {
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            self.set_operation_info(
                format!("New note created: {}", file_stem),
                Some("➕".to_string())
            );
        }
    }
    
    pub(crate) fn handle_file_deleted(&mut self, path: std::path::PathBuf) {
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            self.set_operation_info(
                format!("Note deleted: {}", file_stem),
                Some("🗑️".to_string())
            );
        }
    }
    
    pub(crate) fn handle_file_renamed(&mut self, from: std::path::PathBuf, to: std::path::PathBuf) {
        let from_name = from.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
        let to_name = to.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
        
        self.set_operation_info(
            format!("Note renamed: {} → {}", from_name, to_name),
            Some("🔄".to_string())
        );
    }
    
    #[allow(dead_code)]
    pub fn clear_external_changes_flag(&mut self) {
        self.has_external_changes = false;
        self.sync_status = "📁 Vault in sync".to_string();
    }
    
    // Tag management functionality
}

#[cfg(test)]
mod conflict_tests {
    use super::*;
    use crate::storage::{NotebookStorage, VaultStorage};
    use std::fs;

    /// A vault with one note, loaded into an App the way main.rs does it.
    fn app_on_a_vault(tag: &str, body: &str) -> (App, std::path::PathBuf, uuid::Uuid) {
        let dir = std::env::temp_dir().join(format!("scribble_watch_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Note.md"), body).unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let mut app = App::default();
        app.notebook = storage.load_notebook().unwrap();
        app.vault_path = Some(dir.clone());
        let id = *app.notebook.notes.keys().next().unwrap();
        (app, dir, id)
    }

    /// Nothing unsaved means nothing to lose. Leaving the in-memory copy stale is
    /// how a later edit silently reverts somebody else's work, so take the disk
    /// version without asking.
    #[test]
    fn a_clean_note_is_reloaded_from_disk() {
        let (mut app, dir, id) = app_on_a_vault("clean", "---\ntitle: Note\n---\nbefore\n");
        let path = dir.join("Note.md");

        fs::write(&path, "---\ntitle: Note\n---\nafter\n").unwrap();
        app.handle_file_modified(path);

        let content = app.notebook.notes.get(&id).unwrap().content.clone();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(content, "after\n", "clean note was not refreshed from disk");
    }

    /// The open note must have the editor buffer refreshed too, or the reload is
    /// invisible and the next keystroke saves the stale text straight back.
    #[test]
    fn reloading_the_open_note_refreshes_the_editor() {
        let (mut app, dir, id) = app_on_a_vault("open", "---\ntitle: Note\n---\nbefore\n");
        let path = dir.join("Note.md");
        app.current_note = app.notebook.notes.get(&id).cloned();
        app.editor_content = "before\n".to_string();
        app.editor_cursor = (0, 6);

        fs::write(&path, "---\ntitle: Note\n---\nx\n").unwrap();
        app.handle_file_modified(path);

        let content = app.editor_content.clone();
        let cursor = app.editor_cursor;
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(content, "x\n", "editor buffer kept the stale text");
        assert!(cursor.1 <= 1, "cursor left past the end of the shorter line");
    }

    /// Unsaved work is never thrown away by a notification. The buffer stands, and
    /// the write path is what keeps both versions when the save comes.
    #[test]
    fn a_dirty_note_is_left_alone() {
        let (mut app, dir, id) = app_on_a_vault("dirty", "---\ntitle: Note\n---\nbefore\n");
        let path = dir.join("Note.md");
        app.notebook.notes.get_mut(&id).unwrap().content = "what I typed\n".to_string();
        app.mark_note_dirty(id);

        fs::write(&path, "---\ntitle: Note\n---\nfrom elsewhere\n").unwrap();
        app.handle_file_modified(path);

        let content = app.notebook.notes.get(&id).unwrap().content.clone();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(content, "what I typed\n", "a reload discarded unsaved work");
    }

    /// Keystrokes that have not been folded back into the note yet count as unsaved
    /// work, even though nothing has marked the note dirty.
    #[test]
    fn an_untracked_edit_in_the_buffer_still_blocks_a_reload() {
        let (mut app, dir, id) = app_on_a_vault("buffer", "---\ntitle: Note\n---\nbefore\n");
        let path = dir.join("Note.md");
        app.current_note = app.notebook.notes.get(&id).cloned();
        app.editor_content = "half a sentence I am still typ".to_string();

        fs::write(&path, "---\ntitle: Note\n---\nfrom elsewhere\n").unwrap();
        app.handle_file_modified(path);

        let buffer = app.editor_content.clone();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(
            buffer, "half a sentence I am still typ",
            "a reload interrupted someone mid-sentence"
        );
    }
}

#[cfg(test)]
mod vault_switch_tests {
    use super::*;
    use crate::models::{Note, NotebookData};

    fn app_in_vault(path: &str) -> App {
        let mut app = App::default();
        app.notebook.notes.clear();
        app.notebook.folders.clear();
        app.vault_path = Some(std::path::PathBuf::from(path));
        app.available_vaults = vec![
            std::path::PathBuf::from("/vaults/one"),
            std::path::PathBuf::from("/vaults/two"),
        ];
        app
    }

    #[test]
    fn choosing_a_vault_requests_the_switch_and_closes_the_picker() {
        let mut app = app_in_vault("/vaults/one");
        app.show_vault_switcher();
        app.vault_switcher_selected = 1;
        app.request_vault_switch();

        assert_eq!(
            app.disk.pending_vault_switch.as_deref(),
            Some(std::path::Path::new("/vaults/two"))
        );
        assert_eq!(app.mode, AppMode::Normal, "the picker stayed open");
    }

    /// Reloading the vault you are already in would throw away the open note and
    /// the undo stack for nothing.
    #[test]
    fn choosing_the_current_vault_does_nothing() {
        let mut app = app_in_vault("/vaults/one");
        app.show_vault_switcher();
        app.vault_switcher_selected = 0;
        app.request_vault_switch();

        assert!(app.disk.pending_vault_switch.is_none());
    }

    /// The sharp one. `deleted_note_paths` holds absolute paths into the vault we
    /// are leaving; carrying it across would delete files over there on the next
    /// write in the new vault.
    #[test]
    fn switching_drops_state_belonging_to_the_old_vault() {
        let mut app = app_in_vault("/vaults/one");
        let mut old = Note::new("Old".to_string(), None);
        old.content = "old text".to_string();
        let old_id = old.id;
        app.notebook.add_note(old);
        app.open_note_by_id(old_id);
        app.mark_note_dirty(old_id);
        app.disk
            .deleted_note_paths
            .push(std::path::PathBuf::from("/vaults/one/Gone.md"));
        app.note_cursor_map.insert(old_id, (5, 5));
        app.push_undo_snapshot();

        let mut fresh = NotebookData::new();
        fresh.add_note(Note::new("New".to_string(), None));
        app.adopt_vault(std::path::PathBuf::from("/vaults/two"), fresh);

        assert!(
            app.disk.deleted_note_paths.is_empty(),
            "a delete queued against the old vault survived the switch"
        );
        assert!(app.disk.dirty_note_ids.is_empty(), "old dirty ids survived");
        assert!(!app.disk.pending_disk_save, "a pending write survived");
        assert!(app.note_cursor_map.is_empty(), "cursor memory survived");
        assert!(app.undo_stack.is_empty(), "undo history from the old vault survived");
        assert!(app.current_note.is_none(), "the old note stayed open");
        assert!(app.editor_content.is_empty(), "the old note's text stayed in the editor");
    }

    #[test]
    fn switching_adopts_the_new_notebook_and_its_path() {
        let mut app = app_in_vault("/vaults/one");
        let mut fresh = NotebookData::new();
        fresh.add_note(Note::new("Only In Two".to_string(), None));
        app.adopt_vault(std::path::PathBuf::from("/vaults/two"), fresh);

        assert_eq!(
            app.vault_path.as_deref(),
            Some(std::path::Path::new("/vaults/two"))
        );
        assert!(app.notebook.find_note_by_title("Only In Two").is_some());
        assert_eq!(app.notebook.notes.len(), 1, "notes from the old vault lingered");
    }

    /// The switch is requested, not performed here — the main loop owns storage and
    /// has to flush the old vault first.
    #[test]
    fn requesting_a_switch_does_not_itself_change_the_vault() {
        let mut app = app_in_vault("/vaults/one");
        app.show_vault_switcher();
        app.vault_switcher_selected = 1;
        app.request_vault_switch();

        assert_eq!(
            app.vault_path.as_deref(),
            Some(std::path::Path::new("/vaults/one")),
            "the vault changed before the old one was flushed"
        );
    }
}
