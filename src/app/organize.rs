use super::*;
use uuid::Uuid;

impl App {
    pub fn start_move_item(&mut self) {
        if let Some(selected_item) = self.get_selected_item().cloned() {
            self.move_item_id = Some(selected_item.id);
            self.move_item_type = Some(selected_item.item_type.clone());
            self.mode = AppMode::Move;
            
            let item_name = &selected_item.name;
            let item_type_str = match selected_item.item_type {
                TreeItemType::Note => "note",
                TreeItemType::Folder => "folder",
            };
            self.set_message(format!("Moving {} '{}' - select destination folder or press Esc to cancel", item_type_str, item_name));
        } else {
            self.set_message("Nothing selected to move".to_string());
        }
    }
    
    pub fn cancel_move(&mut self) {
        self.move_item_id = None;
        self.move_item_type = None;
        self.mode = AppMode::Normal;
        self.set_message("Move cancelled".to_string());
    }
    
    /// Move the pending item to the highlighted row's folder.
    ///
    /// A highlighted *note* means its containing folder, which is what makes the
    /// tree usable as a destination picker.
    pub fn execute_move(&mut self) -> Result<(), String> {
        let destination_folder_id = match self.get_selected_item() {
            Some(item) => match item.item_type {
                TreeItemType::Folder => Some(item.id),
                TreeItemType::Note => self
                    .notebook
                    .notes
                    .get(&item.id)
                    .and_then(|n| n.folder_id),
            },
            None => return Err("No destination selected".to_string()),
        };
        self.execute_move_to(destination_folder_id)
    }

    /// Move the pending item to the vault root.
    ///
    /// Reachable only by aiming at a root-level note otherwise — which fails
    /// outright in a vault that has none, leaving no way to move anything back
    /// out of a folder.
    pub fn execute_move_to_root(&mut self) -> Result<(), String> {
        self.execute_move_to(None)
    }

    /// Perform the pending move. `destination` of None means the vault root.
    fn execute_move_to(&mut self, destination_folder_id: Option<uuid::Uuid>) -> Result<(), String> {
        let move_id = self.move_item_id.ok_or("No item selected for moving")?;
        let move_type = self.move_item_type.as_ref().ok_or("No item type selected")?;

        {
            match move_type {
                TreeItemType::Note => {
                    // move_note relocates the file and flags the save itself.
                    self.move_note(move_id, destination_folder_id)?;
                },
                TreeItemType::Folder => {
                    // Relocate the directory (and its files) on disk.
                    let old_rel = self.folder_rel_path(move_id);
                    self.move_folder(move_id, destination_folder_id)?;
                    let new_rel = self.folder_rel_path(move_id);
                    self.queue_folder_relocation(old_rel, new_rel);
                },
            }
            
            // Reset move state
            self.move_item_id = None;
            self.move_item_type = None;
            self.mode = AppMode::Normal;
            self.refresh_tree_view();
            
            let dest_name = if let Some(dest_id) = destination_folder_id {
                if let Some(folder) = self.notebook.folders.get(&dest_id) {
                    folder.name.clone()
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Root".to_string()
            };
            
            self.set_operation_success(format!("Item moved to '{}'!", dest_name), Some("📁".to_string()));
            Ok(())
        }
    }
    
    pub(crate) fn move_note(&mut self, note_id: Uuid, destination_folder_id: Option<Uuid>) -> Result<(), String> {
        // Update the note and take its old on-disk path. Clearing file_path makes
        // the saver recompute a fresh path inside the destination folder.
        let old_path = {
            let note = self.notebook.notes.get_mut(&note_id).ok_or("Note not found")?;
            if note.folder_id == destination_folder_id {
                return Err("Note is already in this location".to_string());
            }
            note.folder_id = destination_folder_id;
            note.modified_at = chrono::Utc::now();
            note.file_path.take()
        };

        // Keep the open note in sync (it now has folder_id updated, file_path None).
        if self.current_note.as_ref().map(|n| n.id) == Some(note_id) {
            if let Some(updated) = self.notebook.notes.get(&note_id).cloned() {
                self.current_note = Some(updated);
            }
        }

        // Remove the file from the old folder, then write it into the new one so
        // the file follows the note.
        if let Some(path) = old_path {
            self.mark_note_deleted(path);
        }
        self.mark_note_dirty(note_id);
        Ok(())
    }
    
    pub(crate) fn move_folder(&mut self, folder_id: Uuid, destination_folder_id: Option<Uuid>) -> Result<(), String> {
        // Check for circular dependency
        if let Some(dest_id) = destination_folder_id {
            if self.is_folder_ancestor(folder_id, dest_id) {
                return Err("Cannot move folder into its own subfolder".to_string());
            }
        }
        
        if let Some(folder) = self.notebook.folders.get_mut(&folder_id) {
            // Check if we're actually moving to a different location
            if folder.parent_id == destination_folder_id {
                return Err("Folder is already in this location".to_string());
            }
            
            // Update folder hierarchy
            if folder.parent_id.is_none() {
                // Remove from root folders
                self.notebook.root_folder_ids.retain(|&id| id != folder_id);
            }
            
            folder.parent_id = destination_folder_id;
            
            if destination_folder_id.is_none() {
                // Add to root folders
                if !self.notebook.root_folder_ids.contains(&folder_id) {
                    self.notebook.root_folder_ids.push(folder_id);
                }
            }
            
            Ok(())
        } else {
            Err("Folder not found".to_string())
        }
    }
    
    pub fn start_rename_item(&mut self) {
        if let Some(selected_item) = self.get_selected_item().cloned() {
            self.rename_item_id = Some(selected_item.id);
            self.rename_item_type = Some(selected_item.item_type.clone());
            self.rename_item_name = selected_item.name.clone();
            self.input_buffer = selected_item.name.clone();
            self.mode = AppMode::Rename;
            
            let item_type_str = match selected_item.item_type {
                TreeItemType::Note => "note",
                TreeItemType::Folder => "folder",
            };
            self.set_message(format!("Renaming {} '{}'", item_type_str, selected_item.name));
        } else {
            self.set_message("Nothing selected to rename".to_string());
        }
    }
    
    pub fn cancel_rename(&mut self) {
        self.rename_item_id = None;
        self.rename_item_type = None;
        self.rename_item_name.clear();
        self.input_buffer.clear();
        self.mode = AppMode::Normal;
        self.set_message("Rename cancelled".to_string());
    }
    
    pub fn execute_rename(&mut self) -> Result<(), String> {
        let rename_id = self.rename_item_id.ok_or("No item selected for renaming")?;
        let rename_type = self.rename_item_type.clone().ok_or("No item type selected")?;
        let new_name = self.input_buffer.trim().to_string();

        if new_name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        if new_name == self.rename_item_name {
            return Err("Name unchanged".to_string());
        }

        match rename_type {
            TreeItemType::Note => {
                self.rename_note(rename_id, new_name.clone())?;
                // Rewrite the note so its frontmatter title is updated on disk.
                self.mark_note_dirty(rename_id);
            },
            TreeItemType::Folder => {
                // Rename the directory on disk (and remap contained notes).
                let old_rel = self.folder_rel_path(rename_id);
                self.rename_folder(rename_id, new_name.clone())?;
                let new_rel = self.folder_rel_path(rename_id);
                self.queue_folder_relocation(old_rel, new_rel);
            },
        }

        // Reset rename state
        self.rename_item_id = None;
        self.rename_item_type = None;
        self.rename_item_name.clear();
        self.input_buffer.clear();
        self.mode = AppMode::Normal;
        self.refresh_tree_view();

        self.set_operation_success(format!("Renamed to '{}'!", new_name), Some("✏️".to_string()));
        Ok(())
    }
    
    pub(crate) fn rename_note(&mut self, note_id: Uuid, new_name: String) -> Result<(), String> {
        // Check if a note with this name already exists
        if self.notebook.notes.values().any(|n| n.id != note_id && n.title == new_name) {
            return Err(format!("A note with the name '{}' already exists", new_name));
        }
        
        if let Some(note) = self.notebook.notes.get_mut(&note_id) {
            note.title = new_name.clone();
            note.modified_at = chrono::Utc::now();
            
            // Update current note if it's the one being renamed
            if let Some(ref current_note) = self.current_note {
                if current_note.id == note_id {
                    self.current_note = Some(note.clone());
                }
            }
            
            Ok(())
        } else {
            Err("Note not found".to_string())
        }
    }
    
    pub(crate) fn rename_folder(&mut self, folder_id: Uuid, new_name: String) -> Result<(), String> {
        // Check if a folder with this name already exists at the same level
        if let Some(folder) = self.notebook.folders.get(&folder_id) {
            let parent_id = folder.parent_id;
            
            // Check for name conflicts at the same level
            if self.notebook.folders.values().any(|f| 
                f.id != folder_id && 
                f.name == new_name && 
                f.parent_id == parent_id
            ) {
                return Err(format!("A folder with the name '{}' already exists at this level", new_name));
            }
        }
        
        if let Some(folder) = self.notebook.folders.get_mut(&folder_id) {
            folder.name = new_name;
            Ok(())
        } else {
            Err("Folder not found".to_string())
        }
    }
    
    pub(crate) fn is_folder_ancestor(&self, ancestor_id: Uuid, descendant_id: Uuid) -> bool {
        if ancestor_id == descendant_id {
            return true;
        }
        
        if let Some(descendant) = self.notebook.folders.get(&descendant_id) {
            if let Some(parent_id) = descendant.parent_id {
                return self.is_folder_ancestor(ancestor_id, parent_id);
            }
        }
        
        false
    }
}
