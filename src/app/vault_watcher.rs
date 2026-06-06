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
            if default_vault.exists() && default_vault.join(".obsidian").exists() {
                if !self.available_vaults.contains(default_vault) {
                    self.available_vaults.push(default_vault.clone());
                }
            }
        }
        
        // Add current directory if it's a vault
        if let Ok(current_dir) = std::env::current_dir() {
            if current_dir.join(".obsidian").exists() {
                if !self.available_vaults.contains(&current_dir) {
                    self.available_vaults.push(current_dir);
                }
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
                            if path.is_dir() && path.join(".obsidian").exists() {
                                if !self.available_vaults.contains(&path) {
                                    self.available_vaults.push(path);
                                }
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
    
    pub(crate) fn handle_file_modified(&mut self, path: std::path::PathBuf) {
        // If the modified file is the currently open note, offer to reload
        if let Some(current_note) = &self.current_note {
            if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                let note_filename = sanitize_filename(&current_note.title);
                if file_name == note_filename {
                    // Note: In a more sophisticated implementation, we might show a dialog
                    // asking the user if they want to reload the file
                    self.set_operation_info(
                        format!("Note '{}' was modified externally", current_note.title),
                        Some("🔄".to_string())
                    );
                }
            }
        }
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
