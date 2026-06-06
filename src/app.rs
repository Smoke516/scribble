use crate::autocomplete::{AutocompleteState, MarkdownAutocomplete};
use pulldown_cmark;
use crate::models::{Note, Folder, NotebookData, FolderTreeNode};
use crate::search::{EnhancedSearch, SearchQuery, SearchResult};
use crate::tags::TagManager;
use crate::theme::ThemeManager;
use crate::watcher::{FileWatcher, FileChangeEvent};
use crate::config::Config;
use uuid::Uuid;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Insert,
    Search,
    #[allow(dead_code)]  // TODO: advanced-search mode (unfinished)
    SearchAdvanced,
    SearchReplace,
    Command,
    InputNote,
    InputFolder,
    Move,
    Help,
    DeleteConfirm,
    QuickJump,
    RecentFiles,
    VaultSwitcher,
    TagBrowser,
    TagInput,
    ThemeBrowser,
    Rename,
    NoteSearch,
    Backlinks,
    Visual,
    TemplatePicker,
    SpellSuggest,
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
    #[allow(dead_code)]  // TODO: SaveStatus::Error not yet set on save failures
    Error,
}

#[derive(Debug, Clone)]
pub enum OperationResult {
    Success { message: String, icon: String },
    Error { message: String, icon: String },
    Info { message: String, icon: String },
}

#[derive(Debug, Clone)]
pub struct ImportFailure {
    pub file_path: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub successful_imports: usize,
    pub failed_imports: Vec<ImportFailure>,
    pub skipped_duplicates: Vec<String>,
    pub renamed_duplicates: Vec<(String, String)>, // (original_name, new_name)
}

impl ImportResult {
    pub fn new() -> Self {
        Self {
            successful_imports: 0,
            failed_imports: Vec::new(),
            skipped_duplicates: Vec::new(),
            renamed_duplicates: Vec::new(),
        }
    }
    
    #[allow(dead_code)]
    pub fn total_processed(&self) -> usize {
        self.successful_imports + self.failed_imports.len() + self.skipped_duplicates.len()
    }
    
    pub fn has_issues(&self) -> bool {
        !self.failed_imports.is_empty() || !self.skipped_duplicates.is_empty()
    }
    
    pub fn format_summary(&self) -> String {
        let mut summary = format!("Import completed: {} successful", self.successful_imports);
        
        if !self.failed_imports.is_empty() {
            summary.push_str(&format!(", {} failed", self.failed_imports.len()));
        }
        
        if !self.skipped_duplicates.is_empty() {
            summary.push_str(&format!(", {} skipped (duplicates)", self.skipped_duplicates.len()));
        }
        
        if !self.renamed_duplicates.is_empty() {
            summary.push_str(&format!(", {} renamed", self.renamed_duplicates.len()));
        }
        
        summary
    }
}

#[derive(Debug, Clone)]
struct ParsedNote {
    title: String,
    content: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    modified_at: Option<chrono::DateTime<chrono::Utc>>,
    tags: Vec<String>,
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
    #[allow(dead_code)]
    pub last_operation: Option<String>,
    pub operation_result: Option<OperationResult>,
    pub operation_result_time: Option<std::time::Instant>,

    // Disk persistence: set when in-memory changes need writing through to the
    // vault; the main loop performs the actual write. last_self_write lets the
    // file watcher ignore the changes we cause ourselves.
    pub pending_disk_save: bool,
    pub last_self_write: Option<std::time::Instant>,
    /// Notes changed since the last disk write (written incrementally).
    pub dirty_note_ids: HashSet<Uuid>,
    /// Files of deleted notes to remove from the vault on the next write.
    pub deleted_note_paths: Vec<std::path::PathBuf>,
    /// Folder-structure change: fall back to a full save (rare, but correct).
    pub force_full_save: bool,

    // Quick jump and recent files
    pub quick_jump_query: String,
    pub quick_jump_results: Vec<Uuid>,
    pub quick_jump_selected: usize,
    pub recent_files_selected: usize,
    pub show_recent_files: bool,
    
    // Live preview
    pub preview_content: String,
    pub preview_scroll: u16,
    
    // Vault switching
    pub vault_switcher_selected: usize,
    pub available_vaults: Vec<std::path::PathBuf>,
    
    // File watching
    pub file_watcher: Option<FileWatcher>,
    pub sync_status: String,
    pub has_external_changes: bool,
    
    // Tag management
    pub tag_manager: TagManager,
    pub tag_browser_selected: usize,
    pub tag_browser_sort_by_frequency: bool,
    pub tag_filter_active: Vec<String>,
    
    // Theme management
    pub theme_manager: ThemeManager,
    pub config: Config,
    pub theme_browser_selected: usize,
    
    // Help dialog
    pub help_scroll: u16,
    
    // Rename operation
    pub rename_item_id: Option<Uuid>,
    pub rename_item_type: Option<TreeItemType>,
    pub rename_item_name: String,

    // Fuzzy search mode flag (replaces fragile status_message check)
    pub is_fuzzy_search: bool,

    // Text undo stack: (content_snapshot, cursor_position)
    pub undo_stack: Vec<(String, (u16, u16))>,

    // Auto-save: time of last content modification
    pub last_keystroke: Option<std::time::Instant>,

    // Search dialog: live results as user types
    pub search_dialog_note_ids: Vec<Uuid>,
    pub search_dialog_selected: usize,

    // Tag filter: note IDs that pass the active filter (empty = show all)
    pub tag_filter_note_ids: HashSet<Uuid>,

    // Vim motions: pending key for double-key sequences (dd, yy)
    pub pending_key: Option<char>,

    // Yank buffer for y/p operations
    pub yank_buffer: String,

    // Redo stack (mirror of undo_stack, cleared on new edits)
    pub redo_stack: Vec<(String, (u16, u16))>,

    // In-note search
    pub note_search_query: String,
    pub note_search_matches: Vec<(u16, u16)>,  // (row, col) of each match
    pub note_search_selected: usize,
    pub note_search_active: bool,

    // Backlinks panel
    pub backlinks_selected: usize,
    pub backlinks_cache: Vec<(Uuid, String)>,  // (note_id, title)

    // Per-note cursor memory: restore position when revisiting a note
    pub note_cursor_map: HashMap<Uuid, (u16, u16)>,

    // Viewport height hint set by the renderer for scroll clamping
    pub editor_viewport_height: u16,

    // Visual selection mode
    pub visual_anchor: (u16, u16),

    // Template picker
    pub template_picker_selected: usize,

    // Spell check
    pub spell_check_enabled: bool,
    pub aspell_available: bool,
    /// (row, col, word_len) for each misspelled word
    pub spell_errors: Vec<(usize, usize, usize)>,
    /// Suggestions for the word under cursor (shown in SpellSuggest popup)
    pub spell_suggestions: Vec<String>,
    pub spell_suggestions_selected: usize,
    /// The word range being corrected: (row, col, len)
    pub spell_word_range: (usize, usize, usize),
}

impl App {
    pub fn new(config: &Config) -> Self {
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
            pending_disk_save: false,
            last_self_write: None,
            dirty_note_ids: HashSet::new(),
            deleted_note_paths: Vec::new(),
            force_full_save: false,
            last_operation: None,
            operation_result: None,
            operation_result_time: None,
            
            // Quick jump and recent files
            quick_jump_query: String::new(),
            quick_jump_results: Vec::new(),
            quick_jump_selected: 0,
            recent_files_selected: 0,
            show_recent_files: false,
            
            // Live preview
            preview_content: String::new(),
            preview_scroll: 0,
            
            // Vault switching
            vault_switcher_selected: 0,
            available_vaults: Vec::new(),
            
            // File watching
            file_watcher: None,
            sync_status: String::new(),
            has_external_changes: false,
            
            // Tag management
            tag_manager: TagManager::new(),
            tag_browser_selected: 0,
            tag_browser_sort_by_frequency: true,
            tag_filter_active: Vec::new(),
            
            // Theme management
            theme_manager: ThemeManager::new(&config.ui.theme),
            config: config.clone(),
            theme_browser_selected: 0,
            
            // Help dialog
            help_scroll: 0,
            
            // Rename operation
            rename_item_id: None,
            rename_item_type: None,
            rename_item_name: String::new(),

            // Fuzzy search
            is_fuzzy_search: false,

            // Text undo
            undo_stack: Vec::new(),

            // Auto-save debounce
            last_keystroke: None,

            // Search dialog live results
            search_dialog_note_ids: Vec::new(),
            search_dialog_selected: 0,

            // Tag filter
            tag_filter_note_ids: HashSet::new(),

            // Vim motions
            pending_key: None,
            yank_buffer: String::new(),

            // Redo
            redo_stack: Vec::new(),

            // In-note search
            note_search_query: String::new(),
            note_search_matches: Vec::new(),
            note_search_selected: 0,
            note_search_active: false,

            // Backlinks
            backlinks_selected: 0,
            backlinks_cache: Vec::new(),

            // Per-note cursor memory
            note_cursor_map: HashMap::new(),

            // Viewport height
            editor_viewport_height: 20,

            // Visual selection
            visual_anchor: (0, 0),

            // Template picker
            template_picker_selected: 0,

            // Spell check — detect aspell at startup
            spell_check_enabled: config.behavior.spell_check,
            aspell_available: crate::spell::check_available(),
            spell_errors: Vec::new(),
            spell_suggestions: Vec::new(),
            spell_suggestions_selected: 0,
            spell_word_range: (0, 0, 0),
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
            if self.tag_filter_note_ids.is_empty() || self.tag_filter_note_ids.contains(&note.id) {
                self.folder_tree_items.push(TreeItem {
                    id: note.id,
                    name: note.title.clone(),
                    item_type: TreeItemType::Note,
                    depth: 0,
                    expanded: false,
                });
            }
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
                if self.tag_filter_note_ids.is_empty() || self.tag_filter_note_ids.contains(&note.id) {
                    self.folder_tree_items.push(TreeItem {
                        id: note.id,
                        name: note.title.clone(),
                        item_type: TreeItemType::Note,
                        depth: node.depth + 1,
                        expanded: false,
                    });
                }
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
        // Save cursor position for the note we're leaving
        if let Some(ref current) = self.current_note {
            self.note_cursor_map.insert(current.id, self.editor_cursor);
        }
        if let Some(note) = self.notebook.notes.get(&note_id).cloned() {
            self.current_note = Some(note.clone());
            self.editor_content = note.content.clone();
            // Restore saved cursor, clamped to new content bounds
            if let Some(&saved) = self.note_cursor_map.get(&note_id) {
                let line_count = self.editor_content.lines().count() as u16;
                let row = saved.0.min(line_count.saturating_sub(1));
                let col = self.editor_content.lines().nth(row as usize)
                    .map(|l| saved.1.min(l.len() as u16)).unwrap_or(0);
                self.editor_cursor = (row, col);
            } else {
                self.editor_cursor = (0, 0);
            }
            self.editor_scroll = 0;
            self.preview_scroll = 0;
            self.adjust_scroll_to_cursor();
            self.focused_pane = FocusedPane::Editor;
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.pending_key = None;
            self.clear_note_search();
            
            // Track recent file access
            self.notebook.add_recent_file(note_id);
            
            // Update preview if enabled
            self.update_preview_content();

            // Run spell check if enabled
            if self.spell_check_enabled && self.aspell_available {
                self.spell_errors = crate::spell::check_content(&self.editor_content);
            }
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
        self.mark_note_dirty(note_id);
        self.set_message("New note created".to_string());
    }

    pub fn create_new_folder(&mut self, name: String, parent_id: Option<Uuid>) {
        let folder = Folder::new(name, parent_id);
        self.notebook.add_folder(folder);
        self.refresh_tree_view();
        self.request_full_save();
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
            
            // Update preview content
            self.update_preview_content();
            
            self.mark_saved();
            // The actual disk write is performed by the main loop.
            if let Some(id) = self.current_note.as_ref().map(|n| n.id) {
                self.mark_note_dirty(id);
            }
            self.set_operation_success("Note saved".to_string(), Some("💾".to_string()));
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
                    // Capture the file path BEFORE removing so we can delete it
                    // from disk too (otherwise the file lingers and the note
                    // reappears on the next reload).
                    let file_path = self
                        .notebook
                        .notes
                        .get(&item_id)
                        .and_then(|n| n.file_path.clone());
                    self.notebook.remove_note(item_id);
                    self.dirty_note_ids.remove(&item_id);
                    if let Some(path) = file_path {
                        self.mark_note_deleted(path);
                    } else {
                        self.pending_disk_save = true; // unsaved note: nothing on disk
                    }
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
                    self.request_full_save();
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
    
    #[allow(dead_code)]  // TODO: clear-search-history not key-bound
    pub fn clear_search_history(&mut self) {
        self.enhanced_search.clear_history();
        self.set_message("Search history cleared".to_string());
    }

    // New fuzzy search method
    pub fn fuzzy_search_notes(&mut self, query: String) {
        let results = self.notebook.fuzzy_search_notes(&query);
        
        if !results.is_empty() {
            // Convert to search results format for compatibility
            self.search_results = results.iter().map(|(note, _score)| (*note).clone()).collect();
            
            // Extract needed data before mutable operations
            let first_note_id = results[0].0.id;
            let first_note_title = results[0].0.title.clone();
            let first_score = results[0].1;
            let results_count = results.len();
            
            // Automatically navigate to and open the first search result
            self.open_note_by_id(first_note_id);
            self.set_operation_success(
                format!("Found {} notes for '{}' - Opened: '{}' (score: {})", 
                    results_count, query, first_note_title, first_score),
                Some("🔍".to_string())
            );
        } else {
            self.search_results.clear();
            self.set_operation_error(
                format!("No fuzzy matches found for '{}'", query),
                Some("❓".to_string())
            );
        }
    }

    // Undo delete functionality
    pub fn undo_last_delete(&mut self) -> Result<(), String> {
        match self.notebook.undo_last_delete() {
            Ok(message) => {
                self.refresh_tree_view();
                self.set_operation_success(message, Some("↩️".to_string()));
                Ok(())
            }
            Err(message) => {
                self.set_operation_error(message.clone(), Some("❌".to_string()));
                Err(message)
            }
        }
    }

    // Note linking functionality
    pub fn parse_current_note_links(&mut self) {
        if let Some(ref note) = self.current_note {
            let note_id = note.id;
            self.notebook.parse_links_in_note(note_id);
        }
    }

    pub fn follow_link_at_cursor(&mut self) -> Result<(), String> {
        if let Some(ref _note) = self.current_note {
            // Calculate the actual byte position of the cursor
            let cursor_pos = self.calculate_cursor_byte_position();
            
            if let Some(link_title) = self.extract_link_at_position(cursor_pos) {
                if let Some(target_id) = self.notebook.find_note_by_title(&link_title) {
                    self.open_note_by_id(target_id);
                    self.set_operation_success(
                        format!("Followed link to: {}", link_title),
                        Some("🔗".to_string())
                    );
                    Ok(())
                } else {
                    // Offer to create the note
                    self.set_operation_error(
                        format!("Note '{}' not found. Press n to create a new note with this title.", link_title),
                        Some("🔍".to_string())
                    );
                    Err(format!("Note '{}' not found", link_title))
                }
            } else {
                Err("No [[wiki link]] found at cursor position".to_string())
            }
        } else {
            Err("No note currently open".to_string())
        }
    }

    fn calculate_cursor_byte_position(&self) -> usize {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut byte_position = 0;
        
        // Add bytes from all lines before the cursor line
        for i in 0..(self.editor_cursor.0 as usize) {
            if let Some(line) = lines.get(i) {
                byte_position += line.len() + 1; // +1 for newline character
            }
        }
        
        // Add bytes from the cursor column position within the current line
        byte_position += self.editor_cursor.1 as usize;
        
        // Make sure we don't exceed content length
        byte_position.min(self.editor_content.len())
    }
    
    fn extract_link_at_position(&self, pos: usize) -> Option<String> {
        use regex::Regex;
        
        let link_regex = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        
        // First, try to find a link that contains the cursor position
        for captures in link_regex.captures_iter(&self.editor_content) {
            if let Some(full_match) = captures.get(0) {
                // Check if cursor is within the link
                if pos >= full_match.start() && pos <= full_match.end() {
                    if let Some(title) = captures.get(1) {
                        return Some(title.as_str().trim().to_string());
                    }
                }
            }
        }
        
        // If no direct match, try to find the closest link within the current line
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let current_line_index = self.editor_cursor.0 as usize;
        
        if let Some(current_line) = lines.get(current_line_index) {
            // Look for links in the current line
            for captures in link_regex.captures_iter(current_line) {
                if let Some(title) = captures.get(1) {
                    return Some(title.as_str().trim().to_string());
                }
            }
        }
        
        None
    }

    pub fn get_backlinks_for_current_note(&self) -> Vec<String> {
        if let Some(ref note) = self.current_note {
            self.notebook.get_backlinks(note.id)
                .iter()
                .filter_map(|link| {
                    self.notebook.notes.get(&link.source_note_id)
                        .map(|source_note| source_note.title.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    #[allow(dead_code)]  // TODO: outgoing wiki-links not surfaced
    pub fn get_outgoing_links_for_current_note(&self) -> Vec<String> {
        if let Some(ref note) = self.current_note {
            self.notebook.get_outgoing_links(note.id)
                .iter()
                .map(|link| link.target_note_title.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    // Recent Files functionality
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

    // Live Preview functionality
    pub fn update_preview_content(&mut self) {
        if self.preview_enabled {
            self.preview_content = self.render_markdown_preview(&self.editor_content);
        }
    }

    fn render_markdown_preview(&self, content: &str) -> String {
        // Simple markdown-to-text conversion for now
        // In the future, this could use pulldown-cmark for proper rendering
        content.lines()
            .map(|line| {
                if line.starts_with("# ") {
                    format!("▉ {}", &line[2..])
                } else if line.starts_with("## ") {
                    format!("▊ {}", &line[3..])
                } else if line.starts_with("### ") {
                    format!("▋ {}", &line[4..])
                } else if line.starts_with("- ") || line.starts_with("* ") {
                    format!("• {}", &line[2..])
                } else if line.starts_with("> ") {
                    format!("│ {}", &line[2..])
                } else if line.starts_with("```") {
                    "┌─────────────────────────────────────┐".to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn preview_scroll_up(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_sub(1);
    }

    pub fn preview_scroll_down(&mut self) {
        if self.preview_scroll < self.preview_max_scroll() {
            self.preview_scroll = self.preview_scroll.saturating_add(1);
        }
    }

    pub fn preview_scroll_to_bottom(&mut self) {
        self.preview_scroll = self.preview_max_scroll();
    }

    /// Upper bound for the preview scroll offset. Approximated from the editor's
    /// line count (the rendered preview is close in length); keeps scrolling from
    /// running off into blank space.
    fn preview_max_scroll(&self) -> u16 {
        self.editor_content.lines().count() as u16
    }

    // Welcome page functionality
    pub fn set_welcome_message(&mut self) {
        let note_count = self.notebook.notes.len();
        let folder_count = self.notebook.folders.len();
        
        if note_count == 0 {
            self.set_message("Welcome to Scribble! Press 'n' to create your first note or '?' for help".to_string());
        } else {
            self.set_message(format!(
                "Welcome back! Loaded {} notes across {} folders. Press 'n' for new note, '?' for help", 
                note_count, folder_count
            ));
        }
        
        // Start with focus on the folder tree to encourage exploration
        self.focused_pane = FocusedPane::Folders;
        
        // Clear any selected note - show welcome in editor pane
        self.current_note = None;
        self.editor_content.clear();
        self.editor_cursor = (0, 0);
        self.editor_scroll = 0;
        self.preview_scroll = 0;
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
        self.last_keystroke = Some(std::time::Instant::now());
    }
    
    pub fn mark_saved(&mut self) {
        self.save_status = SaveStatus::Saved;
    }
    
    pub fn mark_saving(&mut self) {
        self.save_status = SaveStatus::Saving;
    }

    /// Mark a single note as needing an incremental disk write.
    pub fn mark_note_dirty(&mut self, id: Uuid) {
        self.dirty_note_ids.insert(id);
        self.pending_disk_save = true;
    }

    /// Mark a note's file for removal from the vault on the next write.
    pub fn mark_note_deleted(&mut self, path: std::path::PathBuf) {
        self.deleted_note_paths.push(path);
        self.pending_disk_save = true;
    }

    /// Request a full save (folder-structure changes the incremental path can't
    /// express). Correct but writes every note; used for the rare folder ops.
    pub fn request_full_save(&mut self) {
        self.force_full_save = true;
        self.pending_disk_save = true;
    }

    /// Record a successful disk write: marks the buffer clean and timestamps the
    /// write so the file watcher ignores the events it generated.
    pub fn mark_disk_saved(&mut self) {
        self.last_self_write = Some(std::time::Instant::now());
        self.save_status = SaveStatus::Saved;
    }

    /// Surface a failed disk write and keep the save pending so it retries.
    pub fn report_save_failure(&mut self, err: String) {
        self.save_status = SaveStatus::Error;
        self.pending_disk_save = true;
        self.set_operation_error(format!("Save failed: {}", err), Some("⚠️".to_string()));
    }

    /// Did we write to the vault very recently? Used to suppress the file
    /// watcher's "external change" notifications for our own saves.
    fn wrote_recently(&self) -> bool {
        self.last_self_write
            .map(|t| t.elapsed().as_millis() < 1500)
            .unwrap_or(false)
    }
    
    pub fn update_visual_feedback(&mut self) {
        // Clear operation result after 3 seconds
        if let Some(time) = self.operation_result_time {
            if time.elapsed().as_secs() > 3 {
                self.operation_result = None;
                self.operation_result_time = None;
            }
        }

        // Auto-save debounce: save 2 seconds after last keystroke if modified
        if let Some(last_key) = self.last_keystroke {
            if last_key.elapsed().as_secs() >= 2 && self.save_status == SaveStatus::Modified {
                let _ = self.save_current_note();
                self.last_keystroke = None;
            }
        }
    }
    
    /// Check if autocompletion should be triggered and update state
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

    pub fn export_all_notes(&self) -> Result<usize, String> {
        let storage = crate::storage::Storage::new()
            .map_err(|e| format!("Failed to initialize storage: {}", e))?;
        
        let export_dir = storage.get_notes_dir();
        std::fs::create_dir_all(&export_dir)
            .map_err(|e| format!("Failed to create export directory: {}", e))?;
        
        let mut exported_count = 0;
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
            
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to export note '{}': {}", note.title, e))?;
            
            exported_count += 1;
        }
        
        Ok(exported_count)
    }
    
    pub fn export_notes_to_directory(&self, directory: &str) -> Result<usize, String> {
        use std::fs;
        use std::path::Path;
        
        let export_dir = Path::new(directory);
        fs::create_dir_all(export_dir)
            .map_err(|e| format!("Failed to create export directory: {}", e))?;
        
        let mut exported_count = 0;
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
            
            exported_count += 1;
        }
        
        Ok(exported_count)
    }
    
    pub fn import_notes_from_directory(&mut self, directory: &str) -> Result<ImportResult, String> {
        use std::fs;
        use std::path::Path;
        
        let import_dir = Path::new(directory);
        if !import_dir.exists() {
            return Err("Import directory does not exist".to_string());
        }
        
        // Create backup before importing
        if let Err(e) = self.create_backup() {
            return Err(format!("Failed to create backup before import: {}", e));
        }
        
        let mut result = ImportResult::new();
        
        for entry in fs::read_dir(import_dir)
            .map_err(|e| format!("Failed to read import directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                match self.import_note_from_file(&path, &mut result) {
                    Ok(_) => result.successful_imports += 1,
                    Err(e) => {
                        result.failed_imports.push(ImportFailure {
                            file_path: path.display().to_string(),
                            error: e,
                        });
                    }
                }
            }
        }
        
        if result.successful_imports > 0 {
            self.refresh_tree_view();
        }
        
        Ok(result)
    }
    
    fn import_note_from_file(&mut self, file_path: &std::path::Path, result: &mut ImportResult) -> Result<(), String> {
        use std::fs;
        
        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let filename = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported Note")
            .to_string();
        
        // Parse the imported note content
        let parsed_note = self.parse_imported_note_content(&content, &filename)?;
        
        // Handle potential conflicts
        let final_title = self.resolve_title_conflict(&parsed_note.title, result);
        
        let mut note = Note::new(final_title.clone(), None);
        note.content = parsed_note.content;
        
        // Set metadata if available
        if let Some(created_at) = parsed_note.created_at {
            note.created_at = created_at;
        }
        if let Some(modified_at) = parsed_note.modified_at {
            note.modified_at = modified_at;
        }
        if !parsed_note.tags.is_empty() {
            note.tags = parsed_note.tags;
        }
        
        self.notebook.add_note(note);
        
        // Track if the title was changed due to conflict
        if final_title != parsed_note.title {
            result.renamed_duplicates.push((parsed_note.title, final_title));
        }
        
        Ok(())
    }
    
    fn parse_imported_note_content(&self, content: &str, fallback_title: &str) -> Result<ParsedNote, String> {
        
        let lines: Vec<&str> = content.lines().collect();
        let mut title = fallback_title.to_string();
        let mut created_at = None;
        let mut modified_at = None;
        let mut tags = Vec::new();
        let mut content_start_index = 0;
        
        // Check if first line is a title (# Title)
        if !lines.is_empty() && lines[0].starts_with("# ") {
            title = lines[0].strip_prefix("# ").unwrap_or(fallback_title).to_string();
            content_start_index = 1;
            
            // Look for metadata after the title
            let mut i = content_start_index;
            while i < lines.len() {
                let line = lines[i].trim();
                
                if line.is_empty() {
                    i += 1;
                    continue;
                }
                
                if line.starts_with("Created: ") {
                    if let Some(date_str) = line.strip_prefix("Created: ") {
                        created_at = parse_datetime(date_str);
                    }
                    i += 1;
                    content_start_index = i + 1;
                } else if line.starts_with("Modified: ") {
                    if let Some(date_str) = line.strip_prefix("Modified: ") {
                        modified_at = parse_datetime(date_str);
                    }
                    i += 1;
                    content_start_index = i + 1;
                } else if line.starts_with("Tags: ") {
                    if let Some(tags_str) = line.strip_prefix("Tags: ") {
                        tags = tags_str.split(", ")
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    i += 1;
                    content_start_index = i + 1;
                } else if line == "---" {
                    // Skip the separator line
                    i += 1;
                    content_start_index = i + 1;
                    break;
                } else {
                    // No more metadata, content starts here
                    break;
                }
            }
        }
        
        // Extract the actual content
        let note_content = if content_start_index < lines.len() {
            lines[content_start_index..].join("\n")
        } else {
            String::new()
        };
        
        Ok(ParsedNote {
            title,
            content: note_content,
            created_at,
            modified_at,
            tags,
        })
    }
    
    fn resolve_title_conflict(&self, original_title: &str, _result: &mut ImportResult) -> String {
        let mut title = original_title.to_string();
        let mut counter = 1;
        
        // Check for conflicts and rename if necessary
        while self.notebook.notes.values().any(|note| note.title == title) {
            title = format!("{} ({})", original_title, counter);
            counter += 1;
        }
        
        title
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
    
    pub fn list_backups(&self) -> Result<Vec<std::path::PathBuf>, String> {
        let storage = crate::storage::Storage::new()
            .map_err(|e| format!("Failed to initialize storage: {}", e))?;
        
        storage.list_backups()
            .map_err(|e| format!("Failed to list backups: {}", e))
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
        let rename_type = self.rename_item_type.as_ref().ok_or("No item type selected")?;
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
            },
            TreeItemType::Folder => {
                self.rename_folder(rename_id, new_name.clone())?;
            },
        }
        
        // Reset rename state
        self.rename_item_id = None;
        self.rename_item_type = None;
        self.rename_item_name.clear();
        self.input_buffer.clear();
        self.mode = AppMode::Normal;
        self.refresh_tree_view();
        // Rename can move a directory / change a note's frontmatter title — use a
        // full save to keep the vault consistent (renames are infrequent).
        self.request_full_save();

        self.set_operation_success(format!("Renamed to '{}'!", new_name), Some("✏️".to_string()));
        Ok(())
    }
    
    fn rename_note(&mut self, note_id: Uuid, new_name: String) -> Result<(), String> {
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
    
    fn rename_folder(&mut self, folder_id: Uuid, new_name: String) -> Result<(), String> {
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
        self.preview_scroll = 0;
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
    
    /// Scroll help dialog up
    pub fn help_scroll_up(&mut self) {
        self.help_scroll = self.help_scroll.saturating_sub(1);
    }
    
    /// Scroll help dialog down
    pub fn help_scroll_down(&mut self) {
        if self.help_scroll < 200 {
            self.help_scroll += 1;
        }
    }
    
    /// Reset help scroll when opening help
    pub fn reset_help_scroll(&mut self) {
        self.help_scroll = 0;
    }
    
    // Vault switching functionality
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
    
    fn handle_file_changes(&mut self, changes: Vec<FileChangeEvent>) {
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
    
    fn handle_file_modified(&mut self, path: std::path::PathBuf) {
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
    
    fn handle_file_created(&mut self, path: std::path::PathBuf) {
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            self.set_operation_info(
                format!("New note created: {}", file_stem),
                Some("➕".to_string())
            );
        }
    }
    
    fn handle_file_deleted(&mut self, path: std::path::PathBuf) {
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            self.set_operation_info(
                format!("Note deleted: {}", file_stem),
                Some("🗑️".to_string())
            );
        }
    }
    
    fn handle_file_renamed(&mut self, from: std::path::PathBuf, to: std::path::PathBuf) {
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
    
    fn apply_tag_filter(&mut self) {
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

    /// Tags on the currently open note.
    pub fn current_note_tags(&self) -> Vec<String> {
        self.current_note
            .as_ref()
            .map(|n| n.tags.clone())
            .unwrap_or_default()
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
    pub fn change_theme(&mut self, theme_name: &str) {
        self.theme_manager.set_theme(theme_name);
        
        // Update and save config
        self.config.ui.theme = theme_name.to_string();
        if let Err(e) = self.config.save() {
            self.set_operation_error(
                format!("Theme changed but failed to save config: {}", e),
                Some("⚠️".to_string())
            );
        } else {
            self.set_operation_info(
                format!("Theme changed to: {}", theme_name),
                Some("🎨".to_string())
            );
        }
    }
    
    pub fn get_available_themes() -> Vec<&'static str> {
        use crate::theme::ThemeType;
        ThemeType::available_themes()
    }
    
    pub fn current_theme_name(&self) -> &'static str {
        self.theme_manager.current_theme().to_string()
    }
    
    pub fn show_theme_browser(&mut self) {
        self.mode = AppMode::ThemeBrowser;
        self.theme_browser_selected = 0;
        // Find current theme in list
        let current = self.current_theme_name();
        let themes = Self::get_available_themes();
        if let Some(pos) = themes.iter().position(|&t| t == current) {
            self.theme_browser_selected = pos;
        }
        self.set_message("Opened theme browser - use arrow keys to navigate, Enter to select".to_string());
    }
    
    pub fn navigate_theme_browser(&mut self, direction: i32) {
        let themes = Self::get_available_themes();
        let max_index = themes.len().saturating_sub(1);
        
        if direction < 0 && self.theme_browser_selected > 0 {
            self.theme_browser_selected -= 1;
        } else if direction > 0 && self.theme_browser_selected < max_index {
            self.theme_browser_selected += 1;
        }
    }
    
    pub fn select_theme_from_browser(&mut self) {
        let themes = Self::get_available_themes();
        if let Some(&theme_name) = themes.get(self.theme_browser_selected) {
            self.change_theme(theme_name);
            self.mode = AppMode::Normal;
        }
    }
    
    pub fn cancel_theme_browser(&mut self) {
        self.mode = AppMode::Normal;
    }

    // -------------------------------------------------------------------------
    // Text undo
    // -------------------------------------------------------------------------

    /// Save a snapshot of the current editor content to the undo stack.
    /// Clears the redo stack because a new edit invalidates redo history.
    pub fn push_undo_snapshot(&mut self) {
        if let Some((last_content, _)) = self.undo_stack.last() {
            if last_content == &self.editor_content {
                return; // nothing changed
            }
        }
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push((self.editor_content.clone(), self.editor_cursor));
        self.redo_stack.clear();
    }

    /// Restore the previous editor snapshot. Pushes current state to redo stack.
    pub fn undo_text(&mut self) -> bool {
        if let Some((content, cursor)) = self.undo_stack.pop() {
            // Save current state to redo stack before replacing it
            if self.redo_stack.len() >= 50 {
                self.redo_stack.remove(0);
            }
            self.redo_stack.push((self.editor_content.clone(), self.editor_cursor));
            self.editor_content = content;
            self.editor_cursor = cursor;
            self.mark_modified();
            self.update_preview_content();
            true
        } else {
            false
        }
    }

    /// Re-apply the last undone change. Returns true if a redo snapshot was available.
    pub fn redo_text(&mut self) -> bool {
        if let Some((content, cursor)) = self.redo_stack.pop() {
            // Save current state to undo stack
            if self.undo_stack.len() >= 50 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push((self.editor_content.clone(), self.editor_cursor));
            self.editor_content = content;
            self.editor_cursor = cursor;
            self.mark_modified();
            self.update_preview_content();
            true
        } else {
            false
        }
    }

    // -------------------------------------------------------------------------
    // Vim-style cursor movement (Normal mode, editor focused)
    // -------------------------------------------------------------------------

    /// Move cursor down one line, adjusting scroll so it stays visible.
    pub fn cursor_down_normal(&mut self) {
        let line_count = self.editor_content.lines().count() as u16;
        if self.editor_cursor.0 < line_count.saturating_sub(1) {
            self.editor_cursor.0 += 1;
            let lines: Vec<&str> = self.editor_content.lines().collect();
            if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
                self.editor_cursor.1 = self.editor_cursor.1.min(line.len() as u16);
            }
            self.adjust_scroll_to_cursor();
        }
    }

    /// Move cursor up one line, adjusting scroll so it stays visible.
    pub fn cursor_up_normal(&mut self) {
        if self.editor_cursor.0 > 0 {
            self.editor_cursor.0 -= 1;
            let lines: Vec<&str> = self.editor_content.lines().collect();
            if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
                self.editor_cursor.1 = self.editor_cursor.1.min(line.len() as u16);
            }
            self.adjust_scroll_to_cursor();
        }
    }

    /// Move cursor to the start of the current line (0).
    pub fn cursor_to_line_start(&mut self) {
        self.editor_cursor.1 = 0;
    }

    /// Move cursor to the end of the current line ($).
    pub fn cursor_to_line_end(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
            self.editor_cursor.1 = line.len() as u16;
        }
    }

    /// Move cursor forward one word (w).
    pub fn cursor_word_forward(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.editor_cursor.1 as usize;
            // Skip current non-whitespace
            while col < chars.len() && !chars[col].is_whitespace() { col += 1; }
            // Skip whitespace
            while col < chars.len() && chars[col].is_whitespace() { col += 1; }
            self.editor_cursor.1 = col as u16;
        }
    }

    /// Move cursor backward one word (b).
    pub fn cursor_word_backward(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.editor_cursor.1 as usize;
            // Skip whitespace backward
            while col > 0 && chars[col - 1].is_whitespace() { col -= 1; }
            // Skip word chars backward
            while col > 0 && !chars[col - 1].is_whitespace() { col -= 1; }
            self.editor_cursor.1 = col as u16;
        }
    }

    // -------------------------------------------------------------------------
    // Vim-style editing operations (Normal mode)
    // -------------------------------------------------------------------------

    /// Delete the character under the cursor (x).
    pub fn delete_char_at_cursor(&mut self) {
        let row = self.editor_cursor.0 as usize;
        let col = self.editor_cursor.1 as usize;
        // Collect what we need and drop the borrow before mutating
        let char_exists = self.editor_content.lines()
            .nth(row)
            .map(|l| col < l.len())
            .unwrap_or(false);
        if char_exists {
            let abs_pos = self.get_line_start_position(row) + col;
            self.editor_content.remove(abs_pos);
            // Clamp cursor to new line length
            let new_len = self.editor_content
                .lines()
                .nth(row)
                .map(|l| l.len())
                .unwrap_or(0);
            if self.editor_cursor.1 as usize > new_len && new_len > 0 {
                self.editor_cursor.1 = new_len as u16;
            }
            self.mark_modified();
            self.update_preview_content();
        }
    }

    /// Delete the current line and store it in the yank buffer (dd).
    pub fn delete_current_line(&mut self) {
        let row = self.editor_cursor.0 as usize;
        // Collect owned data before any mutable operations
        let line_info: Option<(usize, String, usize)> = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            lines.get(row).map(|l| (lines.len(), l.to_string(), l.len()))
        };
        let (line_count, line_content, line_len) = match line_info {
            Some(v) => v,
            None => return,
        };

        self.push_undo_snapshot();
        self.yank_buffer = line_content;

        let start = self.get_line_start_position(row);
        if row + 1 < line_count {
            // Remove line including its trailing newline
            let end = start + line_len + 1;
            self.editor_content.drain(start..end);
        } else if start > 0 {
            // Last line: also remove the preceding newline
            self.editor_content.truncate(start.saturating_sub(1));
            self.editor_cursor.0 = self.editor_cursor.0.saturating_sub(1);
        } else {
            self.editor_content.clear();
            self.editor_cursor = (0, 0);
            self.mark_modified();
            self.update_preview_content();
            return;
        }

        let new_line_count = self.editor_content.lines().count() as u16;
        if self.editor_cursor.0 >= new_line_count && new_line_count > 0 {
            self.editor_cursor.0 = new_line_count - 1;
        }
        self.editor_cursor.1 = 0;
        self.mark_modified();
        self.update_preview_content();
    }

    /// Copy the current line into the yank buffer (yy).
    pub fn yank_current_line(&mut self) {
        let row = self.editor_cursor.0 as usize;
        // Collect to owned String before any mutable operations
        let yanked: Option<String> = self.editor_content.lines().nth(row).map(|l| l.to_string());
        if let Some(line) = yanked {
            self.yank_buffer = line;
            let preview: String = self.yank_buffer.chars().take(40).collect();
            self.set_operation_info(format!("Yanked: \"{}\"", preview), Some("📋".to_string()));
        }
    }

    /// Read text from the system clipboard.
    pub fn read_system_clipboard(&self) -> Option<String> {
        // Try wl-paste (Wayland), then xclip, then xsel (X11)
        let commands: &[(&str, &[&str])] = &[
            ("wl-paste", &["--no-newline"]),
            ("xclip", &["-selection", "clipboard", "-o"]),
            ("xsel", &["--clipboard", "--output"]),
        ];
        for (cmd, args) in commands {
            if let Ok(output) = std::process::Command::new(cmd)
                .args(*args)
                .output()
            {
                if output.status.success() {
                    return String::from_utf8(output.stdout).ok();
                }
            }
        }
        None
    }

    /// Paste system clipboard on a new line below the cursor (P).
    pub fn paste_clipboard_below(&mut self) {
        let text = match self.read_system_clipboard() {
            Some(t) if !t.is_empty() => t,
            _ => {
                self.set_message("System clipboard is empty (requires xclip, xsel, or wl-paste)".to_string());
                return;
            }
        };
        self.push_undo_snapshot();
        let insert_pos = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let row = self.editor_cursor.0 as usize;
            if row < lines.len() {
                self.get_line_start_position(row) + lines[row].len()
            } else {
                self.editor_content.len()
            }
        };
        let to_insert = format!("\n{}", text);
        self.editor_content.insert_str(insert_pos, &to_insert);
        self.editor_cursor.0 += 1;
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
        self.update_preview_content();
        let preview: String = text.chars().take(40).collect();
        self.set_operation_info(format!("Pasted: \"{}\"", preview), Some("📋".to_string()));
    }

    /// Paste system clipboard at the cursor position (for Insert mode).
    pub fn paste_clipboard_at_cursor(&mut self, cursor_byte_index: usize) {
        let text = match self.read_system_clipboard() {
            Some(t) if !t.is_empty() => t,
            _ => {
                self.set_message("System clipboard is empty (requires xclip, xsel, or wl-paste)".to_string());
                return;
            }
        };
        self.push_undo_snapshot();
        self.editor_content.insert_str(cursor_byte_index, &text);
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        if newline_count > 0 {
            self.editor_cursor.0 += newline_count as u16;
            let last_line_len = text.rsplit('\n').next().map(|l| l.len()).unwrap_or(0);
            self.editor_cursor.1 = last_line_len as u16;
        } else {
            self.editor_cursor.1 += text.len() as u16;
        }
        self.adjust_scroll_to_cursor();
        self.mark_modified();
        self.update_preview_content();
    }

    /// Paste yank buffer on a new line below the cursor (p).
    pub fn paste_below(&mut self) {
        if self.yank_buffer.is_empty() {
            self.set_message("Nothing in yank buffer".to_string());
            return;
        }
        self.push_undo_snapshot();
        // Compute insert position and drop borrow before mutating
        let insert_pos = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let row = self.editor_cursor.0 as usize;
            if row < lines.len() {
                self.get_line_start_position(row) + lines[row].len()
            } else {
                self.editor_content.len()
            }
        };
        let to_insert = format!("\n{}", self.yank_buffer);
        self.editor_content.insert_str(insert_pos, &to_insert);
        self.editor_cursor.0 += 1;
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
        self.update_preview_content();
    }

    /// Open a new blank line below the cursor and prepare for insert (o).
    pub fn open_line_below(&mut self) {
        self.push_undo_snapshot();
        // Compute insert position and drop borrow before mutating
        let insert_pos = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let row = self.editor_cursor.0 as usize;
            if row < lines.len() {
                self.get_line_start_position(row) + lines[row].len()
            } else {
                self.editor_content.len()
            }
        };
        self.editor_content.insert(insert_pos, '\n');
        self.editor_cursor.0 += 1;
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
    }

    // -------------------------------------------------------------------------
    // In-note search
    // -------------------------------------------------------------------------

    /// Scan editor_content for all occurrences of note_search_query and store
    /// their (row, col) positions in note_search_matches.
    pub fn find_note_search_matches(&mut self) {
        self.note_search_matches.clear();
        self.note_search_selected = 0;
        if self.note_search_query.is_empty() {
            return;
        }
        let query_lower = self.note_search_query.to_lowercase();
        for (row, line) in self.editor_content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;
            while let Some(col) = line_lower[start..].find(&query_lower) {
                self.note_search_matches.push((row as u16, (start + col) as u16));
                start += col + query_lower.len().max(1);
            }
        }
    }

    /// Jump to the next in-note search match (wraps around).
    pub fn note_search_next(&mut self) {
        if self.note_search_matches.is_empty() { return; }
        self.note_search_selected = (self.note_search_selected + 1)
            % self.note_search_matches.len();
        self.jump_to_selected_match_pub();
    }

    /// Jump to the previous in-note search match (wraps around).
    pub fn note_search_prev(&mut self) {
        if self.note_search_matches.is_empty() { return; }
        if self.note_search_selected == 0 {
            self.note_search_selected = self.note_search_matches.len() - 1;
        } else {
            self.note_search_selected -= 1;
        }
        self.jump_to_selected_match_pub();
    }

    pub fn jump_to_selected_match_pub(&mut self) {
        if let Some(&(row, col)) = self.note_search_matches.get(self.note_search_selected) {
            self.editor_cursor = (row, col);
            self.adjust_scroll_to_cursor();
        }
    }

    /// Clear all in-note search state.
    pub fn clear_note_search(&mut self) {
        self.note_search_query.clear();
        self.note_search_matches.clear();
        self.note_search_selected = 0;
        self.note_search_active = false;
    }

    // -------------------------------------------------------------------------
    // Backlinks panel
    // -------------------------------------------------------------------------

    /// Populate backlinks_cache for the current note and enter Backlinks mode.
    pub fn show_backlinks_panel(&mut self) {
        if self.current_note.is_none() {
            self.set_message("No note selected".to_string());
            return;
        }
        self.backlinks_cache = self.get_backlinks_for_current_note()
            .into_iter()
            .filter_map(|title| {
                self.notebook.find_note_by_title(&title)
                    .map(|id| (id, title))
            })
            .collect();
        if self.backlinks_cache.is_empty() {
            self.set_message("No notes link to this note".to_string());
        } else {
            self.backlinks_selected = 0;
            self.mode = AppMode::Backlinks;
        }
    }

    pub fn cancel_backlinks(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn backlinks_navigate_up(&mut self) {
        if self.backlinks_selected > 0 {
            self.backlinks_selected -= 1;
        }
    }

    pub fn backlinks_navigate_down(&mut self) {
        if self.backlinks_selected < self.backlinks_cache.len().saturating_sub(1) {
            self.backlinks_selected += 1;
        }
    }

    /// Open the note selected in the backlinks panel.
    pub fn open_selected_backlink(&mut self) {
        if let Some(note_id) = self.backlinks_cache.get(self.backlinks_selected).map(|(id, _)| *id) {
            self.mode = AppMode::Normal;
            self.open_note_by_id(note_id);
        }
    }

    /// Open a new blank line above the cursor and prepare for insert (O).
    pub fn open_line_above(&mut self) {
        self.push_undo_snapshot();
        let row = self.editor_cursor.0 as usize;
        let insert_pos = self.get_line_start_position(row);
        self.editor_content.insert(insert_pos, '\n');
        // cursor stays on the new (now current) line
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
    }

    // -------------------------------------------------------------------------
    // Jump to line (:N command)
    // -------------------------------------------------------------------------

    pub fn jump_to_line(&mut self, line_num: usize) {
        if self.current_note.is_none() { return; }
        let line_count = self.editor_content.lines().count().max(1);
        let target = (line_num.saturating_sub(1)).min(line_count - 1) as u16;
        self.editor_cursor.0 = target;
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let line_len = lines.get(target as usize).map(|l| l.len() as u16).unwrap_or(0);
        self.editor_cursor.1 = self.editor_cursor.1.min(line_len);
        self.adjust_scroll_to_cursor();
    }

    // -------------------------------------------------------------------------
    // Visual selection mode
    // -------------------------------------------------------------------------

    pub fn enter_visual_mode(&mut self) {
        self.visual_anchor = self.editor_cursor;
        self.mode = AppMode::Visual;
    }

    /// Returns the (start, end) of the visual selection, ordered by position.
    pub fn get_visual_selection(&self) -> ((u16, u16), (u16, u16)) {
        let a = self.visual_anchor;
        let c = self.editor_cursor;
        if a.0 < c.0 || (a.0 == c.0 && a.1 <= c.1) {
            (a, c)
        } else {
            (c, a)
        }
    }

    /// Yank the visual selection into the yank buffer and exit Visual mode.
    pub fn yank_visual_selection(&mut self) {
        let (start, end) = self.get_visual_selection();
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut selected = String::new();
        for row in start.0..=end.0 {
            if let Some(line) = lines.get(row as usize) {
                let from = if row == start.0 { (start.1 as usize).min(line.len()) } else { 0 };
                let to   = if row == end.0   { ((end.1 as usize) + 1).min(line.len()) } else { line.len() };
                selected.push_str(&line[from..to]);
                if row < end.0 { selected.push('\n'); }
            }
        }
        self.yank_buffer = selected;
        let preview: String = self.yank_buffer.chars().take(40).collect();
        self.set_operation_info(format!("Yanked: \"{}\"", preview), Some("📋".to_string()));
        self.mode = AppMode::Normal;
    }

    /// Delete the visual selection and exit Visual mode.
    pub fn delete_visual_selection(&mut self) {
        let (start, end) = self.get_visual_selection();
        // First yank it
        {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let mut selected = String::new();
            for row in start.0..=end.0 {
                if let Some(line) = lines.get(row as usize) {
                    let from = if row == start.0 { (start.1 as usize).min(line.len()) } else { 0 };
                    let to   = if row == end.0   { ((end.1 as usize) + 1).min(line.len()) } else { line.len() };
                    selected.push_str(&line[from..to]);
                    if row < end.0 { selected.push('\n'); }
                }
            }
            self.yank_buffer = selected;
        }
        // Now delete
        self.push_undo_snapshot();
        let mut new_lines: Vec<String> = self.editor_content.lines().map(|l| l.to_string()).collect();
        if start.0 == end.0 {
            if let Some(line) = new_lines.get_mut(start.0 as usize) {
                let from = (start.1 as usize).min(line.len());
                let to   = ((end.1 as usize) + 1).min(line.len());
                line.drain(from..to);
            }
        } else {
            let prefix = new_lines.get(start.0 as usize)
                .map(|l| l[..(start.1 as usize).min(l.len())].to_string())
                .unwrap_or_default();
            let suffix = new_lines.get(end.0 as usize)
                .map(|l| l[((end.1 as usize) + 1).min(l.len())..].to_string())
                .unwrap_or_default();
            let s = start.0 as usize;
            let e = (end.0 as usize).min(new_lines.len().saturating_sub(1));
            new_lines.drain(s..=e);
            new_lines.insert(s, format!("{}{}", prefix, suffix));
        }
        // Preserve trailing newline
        let had_trailing = self.editor_content.ends_with('\n');
        self.editor_content = new_lines.join("\n");
        if had_trailing && !self.editor_content.ends_with('\n') {
            self.editor_content.push('\n');
        }
        self.editor_cursor = start;
        self.mark_modified();
        self.update_preview_content();
        self.mode = AppMode::Normal;
    }

    // -------------------------------------------------------------------------
    // Templates
    // -------------------------------------------------------------------------

    pub fn get_templates() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Blank",       "# Untitled\n\n"),
            ("Daily Note",  "# {date}\n\n## Tasks\n\n- [ ] \n\n## Notes\n\n"),
            ("Meeting",     "# Meeting: {date}\n\n## Attendees\n\n- \n\n## Agenda\n\n1. \n\n## Notes\n\n## Action Items\n\n- [ ] \n"),
            ("Project",     "# Project: Untitled\n\n## Overview\n\n## Goals\n\n- \n\n## Progress\n\n## Notes\n\n"),
        ]
    }

    pub fn show_template_picker(&mut self) {
        if self.current_note.is_none() {
            self.set_message("Create or select a note first".to_string());
            return;
        }
        self.mode = AppMode::TemplatePicker;
        self.template_picker_selected = 0;
    }

    pub fn apply_template(&mut self, index: usize) {
        let templates = Self::get_templates();
        if let Some((_, content)) = templates.get(index) {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let filled = content.replace("{date}", &today);
            self.push_undo_snapshot();
            self.editor_content = filled;
            if let Some(ref mut note) = self.current_note {
                note.content = self.editor_content.clone();
            }
            self.editor_cursor = (0, 0);
            self.editor_scroll = 0;
            self.preview_scroll = 0;
            self.mark_modified();
            self.update_preview_content();
        }
        self.mode = AppMode::Insert;
    }

    // -------------------------------------------------------------------------
    // HTML export
    // -------------------------------------------------------------------------

    // -------------------------------------------------------------------------
    // Spell check
    // -------------------------------------------------------------------------

    /// Re-run aspell on the current note content and update `spell_errors`.
    pub fn run_spell_check(&mut self) {
        if !self.spell_check_enabled || !self.aspell_available {
            return;
        }
        self.spell_errors = crate::spell::check_content(&self.editor_content);
    }

    /// Toggle spell checking on/off.
    #[allow(dead_code)]
    pub fn toggle_spell_check(&mut self) {
        if !self.aspell_available {
            self.set_message("aspell not found — install it with: sudo apt install aspell".to_string());
            return;
        }
        self.spell_check_enabled = !self.spell_check_enabled;
        if self.spell_check_enabled {
            self.run_spell_check();
            self.set_message(format!("Spell check ON — {} error(s) found", self.spell_errors.len()));
        } else {
            self.spell_errors.clear();
            self.set_message("Spell check OFF".to_string());
        }
    }

    /// Return the word under the editor cursor: `(row, col, len, word)`.
    ///
    /// Also checks `col - 1` so this works when the cursor sits just past the
    /// end of a word (the natural position after exiting Insert mode).
    pub fn get_word_at_cursor(&self) -> Option<(usize, usize, usize, String)> {
        let row = self.editor_cursor.0 as usize;
        let col = self.editor_cursor.1 as usize;
        let line = self.editor_content.lines().nth(row)?;
        let bytes = line.as_bytes();
        let len = line.len();

        // Prefer the char at col; fall back to col-1 for the common case where
        // the cursor is one position past the end of a word.
        let search_col = if col < len && bytes[col].is_ascii_alphabetic() {
            col
        } else if col > 0 && bytes[col - 1].is_ascii_alphabetic() {
            col - 1
        } else {
            return None;
        };

        let mut start = search_col;
        while start > 0 && bytes[start - 1].is_ascii_alphabetic() {
            start -= 1;
        }
        let mut end = search_col;
        while end < len && bytes[end].is_ascii_alphabetic() {
            end += 1;
        }
        let word = line[start..end].to_string();
        if word.is_empty() {
            return None;
        }
        Some((row, start, end - start, word))
    }

    /// Enter `SpellSuggest` mode for the word at the cursor.
    pub fn show_spell_suggestions(&mut self) {
        if !self.aspell_available {
            self.set_message("aspell not found".to_string());
            return;
        }
        if let Some((row, col, len, word)) = self.get_word_at_cursor() {
            self.spell_word_range = (row, col, len);
            self.spell_suggestions = crate::spell::get_suggestions(&word);
            self.spell_suggestions_selected = 0;
            self.mode = AppMode::SpellSuggest;
        } else {
            self.set_message("No word at cursor".to_string());
        }
    }

    /// Replace the word at `spell_word_range` with the currently selected suggestion.
    pub fn apply_spell_suggestion(&mut self) {
        let (row, col, wlen) = self.spell_word_range;
        if let Some(suggestion) = self.spell_suggestions.get(self.spell_suggestions_selected).cloned() {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            if row < lines.len() {
                let line = lines[row];
                let before = &line[..col.min(line.len())];
                let after_start = (col + wlen).min(line.len());
                let after = &line[after_start..];
                let new_line = format!("{}{}{}", before, suggestion, after);
                // Rebuild content replacing only that line
                let new_content: String = self.editor_content.lines().enumerate()
                    .map(|(i, l)| if i == row { new_line.as_str() } else { l })
                    .collect::<Vec<_>>()
                    .join("\n");
                self.editor_content = new_content;
                self.mark_modified();
            }
        }
        self.mode = AppMode::Normal;
        self.run_spell_check();
    }

    pub fn export_notes_to_html(&self, path: Option<&str>) -> Result<usize, String> {
        use std::fs;
        use std::path::PathBuf;
        let base_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            dirs::document_dir()
                .or_else(dirs::home_dir)
                .unwrap_or_else(|| PathBuf::from("."))
                .join("scribble_export")
        };
        fs::create_dir_all(&base_path).map_err(|e| e.to_string())?;
        let mut count = 0;
        for note in self.notebook.notes.values() {
            let parser = pulldown_cmark::Parser::new(&note.content);
            let mut html_body = String::new();
            pulldown_cmark::html::push_html(&mut html_body, parser);
            let full_html = format!(
                "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><title>{title}</title>\
                 <style>body{{font-family:sans-serif;max-width:800px;margin:auto;padding:2em;line-height:1.6}}\
                 pre{{background:#1e1e2e;color:#cdd6f4;padding:1em;border-radius:6px;overflow:auto}}\
                 code{{background:#313244;padding:0.2em 0.4em;border-radius:3px}}\
                 blockquote{{border-left:4px solid #89b4fa;margin-left:0;padding-left:1em;color:#6c7086}}</style>\
                 </head>\n<body>\n<h1>{title}</h1>\n{body}</body>\n</html>",
                title = note.title,
                body  = html_body,
            );
            let safe = note.title.chars()
                .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
                .collect::<String>();
            let file_path = base_path.join(format!("{}.html", safe));
            fs::write(&file_path, full_html).map_err(|e| e.to_string())?;
            count += 1;
        }
        Ok(count)
    }
}

impl Default for App {
    fn default() -> Self {
        let default_config = Config::default();
        Self::new(&default_config)
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

pub fn sanitize_filename(filename: &str) -> String {
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

fn parse_datetime(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{DateTime, Utc, NaiveDateTime};
    
    // Try parsing the format used by our export: "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
    }
    
    // Try parsing ISO 8601 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }
    
    // Try parsing other common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S UTC",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d",
        "%m/%d/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ];
    
    for format in &formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
            return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
    }
    
    None
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

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use crate::models::Note;

    #[test]
    fn saving_syncs_memory_and_requests_disk_write() {
        let mut app = App::default();
        let note = Note::new("Test".to_string(), None);
        let id = note.id;
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = "hello world".to_string();
        app.pending_disk_save = false;

        app.save_current_note().unwrap();

        // editor content was committed to the in-memory notebook
        assert_eq!(app.notebook.notes.get(&id).unwrap().content, "hello world");
        // a disk write is now pending, and the note is in the dirty set
        assert!(app.pending_disk_save);
        assert!(app.dirty_note_ids.contains(&id));

        // after a successful write, our own change is suppressed for the watcher
        app.mark_disk_saved();
        assert!(app.wrote_recently());
        assert_eq!(app.save_status, SaveStatus::Saved);
    }

    #[test]
    fn failed_save_keeps_request_pending_for_retry() {
        let mut app = App::default();
        app.report_save_failure("disk full".to_string());
        assert!(app.pending_disk_save, "failed save must stay pending to retry");
        assert_eq!(app.save_status, SaveStatus::Error);
    }
}
