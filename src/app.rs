use crate::autocomplete::{AutocompleteState, MarkdownAutocomplete};
use crate::models::{Note, Folder, NotebookData, FolderTreeNode};
use crate::search::{EnhancedSearch, SearchQuery, SearchResult};
use uuid::Uuid;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Insert,
    Search,
    SearchAdvanced,
    SearchReplace,
    Command,
    InputNote,
    InputFolder,
    Move,
    Help,
    DeleteConfirm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPane {
    Folders,
    Editor,
    Preview,
}

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: TreeItemType,
    pub depth: usize,
    pub expanded: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeItemType {
    Folder,
    Note,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SaveStatus {
    Saved,
    Modified,
    Saving,
    Error,
}

#[derive(Debug, Clone)]
pub enum OperationResult {
    Success { message: String, icon: String },
    Error { message: String, icon: String },
    Info { message: String, icon: String },
}

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub focused_pane: FocusedPane,
    pub notebook: NotebookData,
    
    // UI State
    pub folder_tree_items: Vec<TreeItem>,
    pub selected_folder_index: usize,
    pub current_note: Option<Note>,
    pub editor_content: String,
    pub editor_cursor: (u16, u16), // (row, col)
    pub editor_scroll: u16,
    
    // Search
    pub search_query: String,
    pub search_results: Vec<Note>,
    pub enhanced_search: EnhancedSearch,
    pub enhanced_search_results: Vec<SearchResult>,
    
    // Status and messages
    pub status_message: String,
    pub message_history: VecDeque<String>,
    
    // Input handling
    pub input_buffer: String,
    pub command_buffer: String,
    pub pending_folder_parent: Option<Uuid>,
    
    // External editor
    pub external_editor: Option<String>,
    pub just_returned_from_editor: bool,
    
    // Move operation
    pub move_item_id: Option<Uuid>,
    pub move_item_type: Option<TreeItemType>,
    
    // Delete confirmation
    pub delete_item_id: Option<Uuid>,
    pub delete_item_type: Option<TreeItemType>,
    pub delete_item_name: String,
    
    // Preview mode
    pub preview_enabled: bool,
    
    // Autocompletion
    pub autocomplete_state: AutocompleteState,
    pub markdown_autocomplete: MarkdownAutocomplete,
    
    // Visual feedback
    pub save_status: SaveStatus,
    pub last_operation: Option<String>,
    pub operation_result: Option<OperationResult>,
    pub operation_result_time: Option<std::time::Instant>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            should_quit: false,
            mode: AppMode::Normal,
            focused_pane: FocusedPane::Folders,
            notebook: NotebookData::new(),
            
            folder_tree_items: Vec::new(),
            selected_folder_index: 0,
            current_note: None,
            editor_content: String::new(),
            editor_cursor: (0, 0),
            editor_scroll: 0,
            
            search_query: String::new(),
            search_results: Vec::new(),
            enhanced_search: EnhancedSearch::new(),
            enhanced_search_results: Vec::new(),
            
            status_message: "Welcome to Scribble! Press ? for help".to_string(),
            message_history: VecDeque::with_capacity(50),
            
            input_buffer: String::new(),
            command_buffer: String::new(),
            pending_folder_parent: None,
            
            // Try to detect helix, then fall back to other editors
            external_editor: detect_external_editor(),
            just_returned_from_editor: false,
            
            // Move operation
            move_item_id: None,
            move_item_type: None,
            
            // Delete confirmation
            delete_item_id: None,
            delete_item_type: None,
            delete_item_name: String::new(),
            
            // Preview mode
            preview_enabled: false,
            
            // Autocompletion
            autocomplete_state: AutocompleteState::new(),
            markdown_autocomplete: MarkdownAutocomplete::new(),
            
            // Visual feedback
            save_status: SaveStatus::Saved,
            last_operation: None,
            operation_result: None,
            operation_result_time: None,
        };
        
        // Create default folder structure
        app.create_default_structure();
        app.refresh_tree_view();
        
        app
    }

    fn create_default_structure(&mut self) {
        // Create some default folders
        let general_folder = Folder::new("General".to_string(), None);
        let projects_folder = Folder::new("Projects".to_string(), None);
        let daily_folder = Folder::new("Daily Notes".to_string(), None);
        
        self.notebook.add_folder(general_folder);
        self.notebook.add_folder(projects_folder);
        self.notebook.add_folder(daily_folder);
        
        // Create a sample note
        let welcome_note = Note::new("Welcome to Scribble".to_string(), None);
        self.notebook.add_note(welcome_note);
    }

    pub fn refresh_tree_view(&mut self) {
        self.folder_tree_items.clear();
        let tree = self.notebook.build_folder_tree();
        
        // Add root level notes first
        let root_notes = self.notebook.get_folder_notes(None);
        for note in root_notes {
            self.folder_tree_items.push(TreeItem {
                id: note.id,
                name: note.title.clone(),
                item_type: TreeItemType::Note,
                depth: 0,
                expanded: false,
            });
        }
        
        // Add folder tree
        for node in tree {
            self.add_tree_node_to_items(&node);
        }
    }

    fn add_tree_node_to_items(&mut self, node: &FolderTreeNode) {
        // Add the folder
        self.folder_tree_items.push(TreeItem {
            id: node.folder.id,
            name: node.folder.name.clone(),
            item_type: TreeItemType::Folder,
            depth: node.depth,
            expanded: node.folder.expanded,
        });
        
        // Add notes in this folder if expanded
        if node.folder.expanded {
            for note in &node.notes {
                self.folder_tree_items.push(TreeItem {
                    id: note.id,
                    name: note.title.clone(),
                    item_type: TreeItemType::Note,
                    depth: node.depth + 1,
                    expanded: false,
                });
            }
            
            // Add child folders recursively
            for child in &node.children {
                self.add_tree_node_to_items(child);
            }
        }
    }

    pub fn get_selected_item(&self) -> Option<&TreeItem> {
        self.folder_tree_items.get(self.selected_folder_index)
    }

    pub fn select_note(&mut self, note_id: Uuid) {
        if let Some(note) = self.notebook.notes.get(&note_id).cloned() {
            self.current_note = Some(note.clone());
            self.editor_content = note.content;
            self.editor_cursor = (0, 0);
            self.editor_scroll = 0;
            self.focused_pane = FocusedPane::Editor;
        }
    }
    
    pub fn open_note_by_id(&mut self, note_id: Uuid) {
        // First, select the note (load it into the editor)
        self.select_note(note_id);
        
        // Then, update the tree selection to highlight the note
        self.navigate_to_note(note_id);
        
        // Make sure editor is focused
        self.focused_pane = FocusedPane::Editor;
    }
    
    fn navigate_to_note(&mut self, note_id: Uuid) {
        // Find the note in the tree items and select it
        for (index, item) in self.folder_tree_items.iter().enumerate() {
            if item.id == note_id && item.item_type == TreeItemType::Note {
                self.selected_folder_index = index;
                
                // If the note is in a folder, make sure the folder is expanded
                if let Some(note) = self.notebook.notes.get(&note_id) {
                    if let Some(folder_id) = note.folder_id {
                        if let Some(folder) = self.notebook.folders.get_mut(&folder_id) {
                            folder.expanded = true;
                            self.refresh_tree_view(); // Refresh to show the expanded folder
                            
                            // Re-find the note index after refresh
                            for (idx, item) in self.folder_tree_items.iter().enumerate() {
                                if item.id == note_id && item.item_type == TreeItemType::Note {
                                    self.selected_folder_index = idx;
                                    break;
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    pub fn create_new_note(&mut self, title: String, folder_id: Option<Uuid>) {
        let note = Note::new(title, folder_id);
        let note_id = note.id;
        self.notebook.add_note(note);
        self.refresh_tree_view();
        self.select_note(note_id);
        self.set_message("New note created".to_string());
    }

    pub fn create_new_folder(&mut self, name: String, parent_id: Option<Uuid>) {
        let folder = Folder::new(name, parent_id);
        self.notebook.add_folder(folder);
        self.refresh_tree_view();
        self.set_message("New folder created".to_string());
    }

    pub fn start_new_note_input(&mut self, folder_id: Option<Uuid>) {
        self.pending_folder_parent = folder_id;
        self.mode = AppMode::InputNote;
        self.input_buffer.clear();
    }

    pub fn start_new_folder_input(&mut self, parent_id: Option<Uuid>) {
        self.pending_folder_parent = parent_id;
        self.mode = AppMode::InputFolder;
        self.input_buffer.clear();
    }

    pub fn finish_new_note_input(&mut self) {
        let title = if self.input_buffer.trim().is_empty() {
            "Untitled Note".to_string()
        } else {
            self.input_buffer.trim().to_string()
        };
        
        let folder_id = self.pending_folder_parent;
        self.create_new_note(title, folder_id);
        self.pending_folder_parent = None;
        self.input_buffer.clear();
        self.mode = AppMode::Insert; // Go directly to editing the new note
    }

    pub fn finish_new_folder_input(&mut self) {
        let name = if self.input_buffer.trim().is_empty() {
            "New Folder".to_string()
        } else {
            self.input_buffer.trim().to_string()
        };
        
        let parent_id = self.pending_folder_parent;
        self.create_new_folder(name, parent_id);
        self.pending_folder_parent = None;
        self.input_buffer.clear();
        self.mode = AppMode::Normal;
    }

    pub fn cancel_input(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
        self.pending_folder_parent = None;
    }

    pub fn save_current_note(&mut self) -> Result<(), String> {
        if let Some(ref note) = self.current_note.clone() {
            self.mark_saving();
            
            let mut updated_note = note.clone();
            updated_note.update_content(self.editor_content.clone());
            
            // Update the note in the notebook
            self.notebook.notes.insert(updated_note.id, updated_note.clone());
            self.current_note = Some(updated_note);
            self.refresh_tree_view();
            
            self.mark_saved();
            self.set_operation_success("Note saved successfully".to_string(), Some("💾".to_string()));
            Ok(())
        } else {
            self.set_operation_error("No note to save".to_string(), None);
            Err("No note to save".to_string())
        }
    }

    pub fn start_delete_confirmation(&mut self) -> Result<(), String> {
        if let Some(item) = self.get_selected_item().cloned() {
            self.delete_item_id = Some(item.id);
            self.delete_item_type = Some(item.item_type.clone());
            self.delete_item_name = item.name.clone();
            self.mode = AppMode::DeleteConfirm;
            Ok(())
        } else {
            Err("Nothing to delete".to_string())
        }
    }

    pub fn confirm_delete(&mut self) -> Result<(), String> {
        if let (Some(item_id), Some(item_type)) = (self.delete_item_id, self.delete_item_type.clone()) {
            match item_type {
                TreeItemType::Note => {
                    self.notebook.remove_note(item_id);
                    if let Some(ref current_note) = self.current_note {
                        if current_note.id == item_id {
                            self.current_note = None;
                            self.editor_content.clear();
                        }
                    }
                    self.set_message(format!("Note '{}' deleted", self.delete_item_name));
                }
                TreeItemType::Folder => {
                    self.notebook.remove_folder(item_id)?;
                    self.set_message(format!("Folder '{}' deleted", self.delete_item_name));
                }
            }
            
            // Clear deletion state
            self.delete_item_id = None;
            self.delete_item_type = None;
            self.delete_item_name.clear();
            self.mode = AppMode::Normal;
            
            self.refresh_tree_view();
            
            // Adjust selection if needed
            if self.selected_folder_index >= self.folder_tree_items.len() {
                self.selected_folder_index = self.folder_tree_items.len().saturating_sub(1);
            }
            
            Ok(())
        } else {
            Err("No item selected for deletion".to_string())
        }
    }

    pub fn cancel_delete(&mut self) {
        self.delete_item_id = None;
        self.delete_item_type = None;
        self.delete_item_name.clear();
        self.mode = AppMode::Normal;
        self.set_message("Deletion cancelled".to_string());
    }

    pub fn toggle_folder_expansion(&mut self) {
        if let Some(item) = self.get_selected_item().cloned() {
            if item.item_type == TreeItemType::Folder {
                if let Some(folder) = self.notebook.folders.get_mut(&item.id) {
                    folder.expanded = !folder.expanded;
                    self.refresh_tree_view();
                }
            }
        }
    }

    pub fn search_notes(&mut self, query: String) {
        self.search_query = query.clone();
        
        // Use basic search for backward compatibility
        self.search_results = self.notebook.search_notes(&query).into_iter().cloned().collect();
        
        // Also perform enhanced search
        let search_query = SearchQuery::new(query.clone());
        match self.enhanced_search.search(&self.notebook, search_query) {
            Ok(results) => {
                self.enhanced_search_results = results;
                let total_matches: usize = self.enhanced_search_results.iter()
                    .map(|r| r.matches.len())
                    .sum();
                    
                if !self.enhanced_search_results.is_empty() {
                    // Extract needed data first to avoid borrowing issues
                    let first_note_id = self.enhanced_search_results[0].note.id;
                    let first_note_title = self.enhanced_search_results[0].note.title.clone();
                    let results_count = self.enhanced_search_results.len();
                    
                    // Automatically navigate to and open the first search result
                    self.open_note_by_id(first_note_id);
                    self.set_message(format!("Found {} notes with {} matches for '{}' - Opened first result: '{}'", 
                        results_count, total_matches, query, first_note_title));
                } else {
                    self.set_message(format!("No matches found for '{}'", query));
                }
            }
            Err(e) => {
                self.set_message(format!("Search error: {}", e));
            }
        }
    }
    
    pub fn enhanced_search_notes(&mut self, query: SearchQuery) {
        match self.enhanced_search.search(&self.notebook, query) {
            Ok(results) => {
                self.enhanced_search_results = results;
                let total_matches: usize = self.enhanced_search_results.iter()
                    .map(|r| r.matches.len())
                    .sum();
                    
                if !self.enhanced_search_results.is_empty() {
                    // Extract needed data first to avoid borrowing issues
                    let first_note_id = self.enhanced_search_results[0].note.id;
                    let first_note_title = self.enhanced_search_results[0].note.title.clone();
                    let results_count = self.enhanced_search_results.len();
                    
                    // Automatically navigate to and open the first search result
                    self.open_note_by_id(first_note_id);
                    self.set_message(format!("Enhanced search found {} notes with {} matches - Opened first result: '{}'", 
                        results_count, total_matches, first_note_title));
                } else {
                    self.set_message("No matches found".to_string());
                }
            }
            Err(e) => {
                self.set_message(format!("Search error: {}", e));
            }
        }
    }
    
    pub fn get_search_history(&self) -> Vec<&String> {
        self.enhanced_search.get_search_history()
    }
    
    pub fn clear_search_history(&mut self) {
        self.enhanced_search.clear_history();
        self.set_message("Search history cleared".to_string());
    }

    pub fn navigate_up(&mut self) {
        if self.selected_folder_index > 0 {
            self.selected_folder_index -= 1;
        }
    }

    pub fn navigate_down(&mut self) {
        if self.selected_folder_index < self.folder_tree_items.len().saturating_sub(1) {
            self.selected_folder_index += 1;
        }
    }

    pub fn navigate_to_top(&mut self) {
        self.selected_folder_index = 0;
    }

    pub fn navigate_to_bottom(&mut self) {
        self.selected_folder_index = self.folder_tree_items.len().saturating_sub(1);
    }

    pub fn set_message(&mut self, message: String) {
        self.status_message = message.clone();
        self.message_history.push_front(message);
        if self.message_history.len() > 50 {
            self.message_history.pop_back();
        }
    }
    
    pub fn set_operation_success(&mut self, message: String, icon: Option<String>) {
        self.operation_result = Some(OperationResult::Success {
            message: message.clone(),
            icon: icon.unwrap_or("✅".to_string()),
        });
        self.operation_result_time = Some(std::time::Instant::now());
        self.set_message(message);
    }
    
    pub fn set_operation_error(&mut self, message: String, icon: Option<String>) {
        self.operation_result = Some(OperationResult::Error {
            message: message.clone(),
            icon: icon.unwrap_or("❌".to_string()),
        });
        self.operation_result_time = Some(std::time::Instant::now());
        self.set_message(message);
    }
    
    pub fn set_operation_info(&mut self, message: String, icon: Option<String>) {
        self.operation_result = Some(OperationResult::Info {
            message: message.clone(),
            icon: icon.unwrap_or("ℹ️".to_string()),
        });
        self.operation_result_time = Some(std::time::Instant::now());
        self.set_message(message);
    }
    
    pub fn mark_modified(&mut self) {
        self.save_status = SaveStatus::Modified;
    }
    
    pub fn mark_saved(&mut self) {
        self.save_status = SaveStatus::Saved;
    }
    
    pub fn mark_saving(&mut self) {
        self.save_status = SaveStatus::Saving;
    }
    
    pub fn update_visual_feedback(&mut self) {
        // Clear operation result after 3 seconds
        if let Some(time) = self.operation_result_time {
            if time.elapsed().as_secs() > 3 {
                self.operation_result = None;
                self.operation_result_time = None;
            }
        }
    }
    
    /// Check if autocompletion should be triggered and update state
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
    fn get_line_start_position(&self, line_index: usize) -> usize {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut pos = 0;
        for i in 0..line_index.min(lines.len()) {
            pos += lines[i].len() + 1; // +1 for the newline character
        }
        pos
    }
    
    /// Update cursor position from absolute character position
    fn update_cursor_from_absolute_position(&mut self, abs_pos: usize) {
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

    pub fn export_all_notes(&self) -> Result<(), String> {
        let storage = crate::storage::Storage::new()
            .map_err(|e| format!("Failed to initialize storage: {}", e))?;
        
        let mut _exported_count = 0;
        for note in self.notebook.notes.values() {
            match storage.export_note_to_file(&note.id.to_string(), &note.content) {
                Ok(_) => _exported_count += 1,
                Err(e) => return Err(format!("Failed to export note '{}': {}", note.title, e)),
            }
        }
        
        // Set message with export count (this will be handled by the caller)
        Ok(())
    }
    
    pub fn export_notes_to_directory(&self, directory: &str) -> Result<(), String> {
        use std::fs;
        use std::path::Path;
        
        let export_dir = Path::new(directory);
        fs::create_dir_all(export_dir)
            .map_err(|e| format!("Failed to create export directory: {}", e))?;
        
        let mut _exported_count = 0;
        for note in self.notebook.notes.values() {
            let filename = sanitize_filename(&note.title);
            let file_path = export_dir.join(format!("{}.md", filename));
            
            let content = format!("# {}\n\nCreated: {}\nModified: {}\nTags: {}\n\n---\n\n{}", 
                note.title,
                note.created_at.format("%Y-%m-%d %H:%M:%S"),
                note.modified_at.format("%Y-%m-%d %H:%M:%S"),
                note.tags.join(", "),
                note.content
            );
            
            fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write note '{}': {}", note.title, e))?;
            
            _exported_count += 1;
        }
        
        // Could return exported_count in future for user feedback
        Ok(())
    }
    
    pub fn import_notes_from_directory(&mut self, directory: &str) -> Result<(), String> {
        use std::fs;
        use std::path::Path;
        
        let import_dir = Path::new(directory);
        if !import_dir.exists() {
            return Err("Import directory does not exist".to_string());
        }
        
        let mut imported_count = 0;
        for entry in fs::read_dir(import_dir)
            .map_err(|e| format!("Failed to read import directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                match self.import_note_from_file(&path) {
                    Ok(_) => imported_count += 1,
                    Err(e) => eprintln!("Warning: Failed to import {}: {}", path.display(), e),
                }
            }
        }
        
        if imported_count > 0 {
            self.refresh_tree_view();
        }
        
        Ok(())
    }
    
    fn import_note_from_file(&mut self, file_path: &std::path::Path) -> Result<(), String> {
        use std::fs;
        
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let filename = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported Note")
            .to_string();
        
        // Try to extract title from first line if it's a header
        let (title, note_content) = if content.starts_with("# ") {
            let lines: Vec<&str> = content.lines().collect();
            let title = lines[0].strip_prefix("# ").unwrap_or(&filename).to_string();
            let content = lines.iter().skip(1).cloned().collect::<Vec<_>>().join("\n");
            (title, content)
        } else {
            (filename, content)
        };
        
        // Check if note already exists
        let existing_note = self.notebook.notes.values()
            .find(|note| note.title == title);
        
        if existing_note.is_some() {
            return Err(format!("Note with title '{}' already exists", title));
        }
        
        let mut note = Note::new(title, None);
        note.content = note_content;
        self.notebook.add_note(note);
        
        Ok(())
    }
    
    pub fn create_backup(&self) -> Result<(), String> {
        let storage = crate::storage::Storage::new()
            .map_err(|e| format!("Failed to initialize storage: {}", e))?;
        
        match storage.backup_data() {
            Ok(_backup_path) => {
                Ok(())
            }
            Err(e) => Err(format!("Backup failed: {}", e))
        }
    }
    
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
    
    pub fn execute_move(&mut self) -> Result<(), String> {
        let move_id = self.move_item_id.ok_or("No item selected for moving")?;
        let move_type = self.move_item_type.as_ref().ok_or("No item type selected")?;
        
        // Get the selected destination
        if let Some(selected_item) = self.get_selected_item() {
            let destination_folder_id = match selected_item.item_type {
                TreeItemType::Folder => Some(selected_item.id),
                TreeItemType::Note => {
                    // Find the parent folder of the selected note
                    if let Some(note) = self.notebook.notes.get(&selected_item.id) {
                        note.folder_id
                    } else {
                        None
                    }
                },
            };
            
            match move_type {
                TreeItemType::Note => {
                    self.move_note(move_id, destination_folder_id)?;
                },
                TreeItemType::Folder => {
                    self.move_folder(move_id, destination_folder_id)?;
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
        } else {
            Err("No destination selected".to_string())
        }
    }
    
    fn move_note(&mut self, note_id: Uuid, destination_folder_id: Option<Uuid>) -> Result<(), String> {
        if let Some(note) = self.notebook.notes.get_mut(&note_id) {
            // Check if we're actually moving to a different location
            if note.folder_id == destination_folder_id {
                return Err("Note is already in this location".to_string());
            }
            
            // Update the note's folder_id
            note.folder_id = destination_folder_id;
            note.modified_at = chrono::Utc::now();
            
            // Update current note if it's the one being moved
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
    
    fn move_folder(&mut self, folder_id: Uuid, destination_folder_id: Option<Uuid>) -> Result<(), String> {
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
    
    fn is_folder_ancestor(&self, ancestor_id: Uuid, descendant_id: Uuid) -> bool {
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
    
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
    
    /// Toggle markdown preview mode
    pub fn toggle_preview(&mut self) {
        self.preview_enabled = !self.preview_enabled;
        
        let message = if self.preview_enabled {
            "Preview enabled - showing markdown preview"
        } else {
            "Preview disabled - showing editor only"
        };
        self.set_message(message.to_string());
        
        // If we're turning on preview and currently in normal mode, switch focus to editor
        if self.preview_enabled && self.focused_pane == FocusedPane::Folders {
            self.focused_pane = FocusedPane::Editor;
        }
    }
    
    /// Scroll editor up by one line
    pub fn scroll_up(&mut self) {
        self.editor_scroll = self.editor_scroll.saturating_sub(1);
    }
    
    /// Scroll editor down by one line
    pub fn scroll_down(&mut self) {
        let content_lines = self.editor_content.lines().count() as u16;
        if self.editor_scroll < content_lines.saturating_sub(1) {
            self.editor_scroll += 1;
        }
    }
    
    /// Scroll editor up by half a page (Ctrl+U)
    pub fn scroll_half_page_up(&mut self) {
        self.editor_scroll = self.editor_scroll.saturating_sub(10);
    }
    
    /// Scroll editor down by half a page (Ctrl+D)
    pub fn scroll_half_page_down(&mut self) {
        let content_lines = self.editor_content.lines().count() as u16;
        let new_scroll = self.editor_scroll + 10;
        self.editor_scroll = new_scroll.min(content_lines.saturating_sub(1));
    }
    
    /// Scroll editor up by a full page
    pub fn scroll_page_up(&mut self) {
        self.editor_scroll = self.editor_scroll.saturating_sub(20);
    }
    
    /// Scroll editor down by a full page
    pub fn scroll_page_down(&mut self) {
        let content_lines = self.editor_content.lines().count() as u16;
        let new_scroll = self.editor_scroll + 20;
        self.editor_scroll = new_scroll.min(content_lines.saturating_sub(1));
    }
    
    /// Jump to top of editor
    pub fn scroll_to_top(&mut self) {
        self.editor_scroll = 0;
    }
    
    /// Jump to bottom of editor
    pub fn scroll_to_bottom(&mut self) {
        let content_lines = self.editor_content.lines().count() as u16;
        self.editor_scroll = content_lines.saturating_sub(1);
    }
    
    /// Ensure cursor is visible after scrolling
    pub fn adjust_scroll_to_cursor(&mut self) {
        let visible_height = 20; // Approximate visible lines in editor
        
        // If cursor is above the visible area, scroll up
        if self.editor_cursor.0 < self.editor_scroll {
            self.editor_scroll = self.editor_cursor.0;
        }
        
        // If cursor is below the visible area, scroll down
        if self.editor_cursor.0 >= self.editor_scroll + visible_height {
            self.editor_scroll = self.editor_cursor.0.saturating_sub(visible_height - 1);
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for external editor support
fn detect_external_editor() -> Option<String> {
    // Check environment variables first
    if let Ok(editor) = std::env::var("EDITOR") {
        return Some(editor);
    }
    
    // Try to find helix first (preferred)
    if command_exists("hx") {
        return Some("hx".to_string());
    }
    
    if command_exists("helix") {
        return Some("helix".to_string());
    }
    
    // Fallback to other popular editors
    let editors = ["nvim", "vim", "nano", "emacs"];
    for editor in &editors {
        if command_exists(editor) {
            return Some(editor.to_string());
        }
    }
    
    None
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn create_temp_file(title: &str, content: &str) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    
    let temp_dir = std::env::temp_dir();
    let sanitized_title = title.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();
    
    let temp_file = temp_dir.join(format!("scribble_{}_{}.md", 
        sanitized_title, 
        std::process::id()));
    
    let mut file = std::fs::File::create(&temp_file)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    
    Ok(temp_file)
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn run_external_editor(editor: &str, file_path: &std::path::PathBuf) -> Result<(), String> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
        cursor::Show
    };
    use std::io::{stdout, Write};
    
    // Fully reset terminal to normal mode
    let mut stdout = stdout();
    
    // Disable raw mode first
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;
    
    // Clear screen and show cursor
    execute!(stdout, Clear(ClearType::All), Show)
        .map_err(|e| format!("Failed to clear screen: {}", e))?;
    
    // Flush to ensure terminal is ready
    stdout.flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;
    
    // Run the external editor with proper stdio inheritance
    let status = std::process::Command::new(editor)
        .arg(file_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to start {}: {}", editor, e))?;
    
    // Give terminal a moment to settle after editor exits
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Re-enable raw mode for TUI
    enable_raw_mode().map_err(|e| format!("Failed to re-enable raw mode: {}", e))?;
    
    // Clear and reset for our TUI
    execute!(stdout, Clear(ClearType::All))
        .map_err(|e| format!("Failed to clear screen for TUI: {}", e))?;
    
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with code {:?}", editor, status.code()))
    }
}
