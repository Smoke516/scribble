use crate::autocomplete::{AutocompleteState, MarkdownAutocomplete};
use crate::models::{Note, Folder, NotebookData, FolderTreeNode};
use crate::search::{EnhancedSearch, SearchResult};
use crate::tags::TagManager;
use crate::theme::ThemeManager;
use crate::watcher::{FileWatcher, FileChangeEvent};
use crate::config::Config;
use uuid::Uuid;
use std::collections::{HashMap, HashSet, VecDeque};

// Domain method groups split out of this file (all operate on `App`).
mod search;
mod io;
mod organize;
mod helpers;
mod view;
mod editor;
mod vault_watcher;
mod tags;
mod theme;
mod edit_ops;
use helpers::*;
pub use helpers::sanitize_filename;

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
    Outline,
}

/// Which section of the links panel keyboard navigation currently acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklinkFocus {
    Incoming,
    Outgoing,
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
pub(crate) struct ParsedNote {
    title: String,
    content: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    modified_at: Option<chrono::DateTime<chrono::Utc>>,
    tags: Vec<String>,
}

/// What the in-memory notebook still owes the disk.
///
/// These fields are one protocol, not six independent flags: the main loop reads
/// them together on every tick to decide what to write, and a change to one
/// without the others is a bug (a dirty id with no `pending_disk_save` never gets
/// written; clearing `pending_disk_save` without clearing `dirty_note_ids` writes
/// the same notes forever). Grouping them makes that contract visible and gives
/// the invariants somewhere to live.
#[derive(Debug, Default)]
pub struct DiskState {
    /// Set when in-memory changes need writing through to the vault; the main
    /// loop performs the actual write.
    pub pending_disk_save: bool,
    /// Lets the file watcher ignore the changes we cause ourselves.
    pub last_self_write: Option<std::time::Instant>,
    /// Notes changed since the last disk write (written incrementally).
    pub dirty_note_ids: HashSet<Uuid>,
    /// Files of deleted notes to remove from the vault on the next write.
    pub deleted_note_paths: Vec<std::path::PathBuf>,
    /// Folder-structure change: fall back to a full save (rare, but correct).
    pub force_full_save: bool,
    /// Folder directories to move/rename on disk: (old relative path, new).
    pub pending_folder_relocations: Vec<(std::path::PathBuf, std::path::PathBuf)>,
}

impl DiskState {
    /// True when anything is still owed to the disk. The exit path uses this to
    /// avoid rewriting an untouched vault.
    pub fn has_pending_work(&self) -> bool {
        self.pending_disk_save
            || !self.dirty_note_ids.is_empty()
            || !self.deleted_note_paths.is_empty()
            || !self.pending_folder_relocations.is_empty()
    }

    /// Record that everything owed has now been written.
    pub fn clear_after_write(&mut self) {
        self.dirty_note_ids.clear();
        self.deleted_note_paths.clear();
        self.force_full_save = false;
        self.pending_disk_save = false;
    }
}

/// Spell-check state: whether it is on, what aspell found, and the suggestion
/// popup's contents. Inert as a group when aspell is missing.
#[derive(Debug, Default)]
pub struct SpellState {
    pub enabled: bool,
    pub aspell_available: bool,
    pub errors: Vec<(usize, usize, usize)>,
    pub suggestions: Vec<String>,
    pub suggestions_selected: usize,
    pub word_range: (usize, usize, usize),
}

/// In-note search (`/` with the editor focused): the query, every match in the
/// current note, and which one is highlighted. Distinct from the global note
/// search, which lives in `search_query`/`search_results`.
#[derive(Debug, Default)]
pub struct NoteSearchState {
    pub query: String,
    pub matches: Vec<(u16, u16)>,
    pub selected: usize,
    pub active: bool,
}

/// The links panel: notes pointing here, links pointing out, and which of the
/// two sections keyboard navigation currently drives.
#[derive(Debug)]
pub struct LinksState {
    pub incoming: Vec<(Uuid, String)>,
    pub incoming_selected: usize,
    pub outgoing: Vec<(Option<Uuid>, String)>,
    pub outgoing_selected: usize,
    pub focus: BacklinkFocus,
}

impl Default for LinksState {
    fn default() -> Self {
        Self {
            incoming: Vec::new(),
            incoming_selected: 0,
            outgoing: Vec::new(),
            outgoing_selected: 0,
            focus: BacklinkFocus::Incoming,
        }
    }
}

pub struct App {
    pub links: LinksState,
    pub note_search: NoteSearchState,
    pub spell: SpellState,
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

    /// Everything tracking what still needs to reach the disk.
    pub disk: DiskState,

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

    // Backlinks panel

    // Outline panel
    pub outline_headings: Vec<(usize, u8, String)>, // (line_index, level, text)
    pub outline_selected: usize,

    // Per-note cursor memory: restore position when revisiting a note
    pub note_cursor_map: HashMap<Uuid, (u16, u16)>,

    // Viewport height hint set by the renderer for scroll clamping
    pub editor_viewport_height: u16,

    // Visual selection mode
    pub visual_anchor: (u16, u16),

    // Template picker
    pub template_picker_selected: usize,

    // Spell check
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
            // aspell is probed once at startup; everything else starts empty.
            spell: SpellState {
                aspell_available: crate::spell::check_available(),
                ..SpellState::default()
            },
            note_search: NoteSearchState::default(),
            links: LinksState::default(),
            save_status: SaveStatus::Saved,
            disk: DiskState::default(),
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

            outline_headings: Vec::new(),
            outline_selected: 0,

            // Per-note cursor memory
            note_cursor_map: HashMap::new(),

            // Viewport height
            editor_viewport_height: 20,

            // Visual selection
            visual_anchor: (0, 0),

            // Template picker
            template_picker_selected: 0,

            // Spell check — detect aspell at startup
        };
        
        // Create default folder structure
        app.create_default_structure();
        app.refresh_tree_view();
        
        app
    }

    pub(crate) fn create_default_structure(&mut self) {
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

    pub(crate) fn add_tree_node_to_items(&mut self, node: &FolderTreeNode) {
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
            if self.spell.enabled && self.spell.aspell_available {
                self.spell.errors = crate::spell::check_content(&self.editor_content);
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
    
    pub(crate) fn navigate_to_note(&mut self, note_id: Uuid) {
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
                    self.disk.dirty_note_ids.remove(&item_id);
                    if let Some(path) = file_path {
                        self.mark_note_deleted(path);
                    } else {
                        self.disk.pending_disk_save = true; // unsaved note: nothing on disk
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

    // Recent Files functionality
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
        self.disk.dirty_note_ids.insert(id);
        self.disk.pending_disk_save = true;
    }

    /// Mark a note's file for removal from the vault on the next write.
    pub fn mark_note_deleted(&mut self, path: std::path::PathBuf) {
        self.disk.deleted_note_paths.push(path);
        self.disk.pending_disk_save = true;
    }

    /// Request a full save (folder-structure changes the incremental path can't
    /// express). Correct but writes every note; used for the rare folder ops.
    pub fn request_full_save(&mut self) {
        self.disk.force_full_save = true;
        self.disk.pending_disk_save = true;
    }

    /// A folder's path relative to the vault root (its chain of folder names).
    pub fn folder_rel_path(&self, folder_id: Uuid) -> std::path::PathBuf {
        let mut components = Vec::new();
        let mut current = self.notebook.folders.get(&folder_id);
        while let Some(folder) = current {
            components.push(folder.name.clone());
            current = folder.parent_id.and_then(|pid| self.notebook.folders.get(&pid));
        }
        components.reverse();
        components.iter().collect()
    }

    /// Queue a folder directory move/rename to be applied on the next disk write.
    pub fn queue_folder_relocation(
        &mut self,
        old_rel: std::path::PathBuf,
        new_rel: std::path::PathBuf,
    ) {
        if old_rel != new_rel && !old_rel.as_os_str().is_empty() {
            self.disk.pending_folder_relocations.push((old_rel, new_rel));
            self.disk.pending_disk_save = true;
        }
    }

    /// Record a successful disk write: marks the buffer clean and timestamps the
    /// write so the file watcher ignores the events it generated.
    pub fn mark_disk_saved(&mut self) {
        self.disk.last_self_write = Some(std::time::Instant::now());
        self.save_status = SaveStatus::Saved;
    }

    /// Surface a failed disk write and keep the save pending so it retries.
    pub fn report_save_failure(&mut self, err: String) {
        self.save_status = SaveStatus::Error;
        self.disk.pending_disk_save = true;
        self.set_operation_error(format!("Save failed: {}", err), Some("⚠️".to_string()));
    }

    /// Did we write to the vault very recently? Used to suppress the file
    /// watcher's "external change" notifications for our own saves.
    pub(crate) fn wrote_recently(&self) -> bool {
        self.disk.last_self_write
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
    // -------------------------------------------------------------------------
    // In-note search
    // -------------------------------------------------------------------------

    /// Scan editor_content for all occurrences of note_search_query and store
    /// their (row, col) positions in note_search_matches.
    pub fn find_note_search_matches(&mut self) {
        self.note_search.matches.clear();
        self.note_search.selected = 0;
        if self.note_search.query.is_empty() {
            return;
        }
        let query_lower = self.note_search.query.to_lowercase();
        for (row, line) in self.editor_content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;
            while let Some(col) = line_lower[start..].find(&query_lower) {
                self.note_search.matches.push((row as u16, (start + col) as u16));
                start += col + query_lower.len().max(1);
            }
        }
    }

    /// Jump to the next in-note search match (wraps around).
    pub fn note_search_next(&mut self) {
        if self.note_search.matches.is_empty() { return; }
        self.note_search.selected = (self.note_search.selected + 1)
            % self.note_search.matches.len();
        self.jump_to_selected_match_pub();
    }

    /// Jump to the previous in-note search match (wraps around).
    pub fn note_search_prev(&mut self) {
        if self.note_search.matches.is_empty() { return; }
        if self.note_search.selected == 0 {
            self.note_search.selected = self.note_search.matches.len() - 1;
        } else {
            self.note_search.selected -= 1;
        }
        self.jump_to_selected_match_pub();
    }

    pub fn jump_to_selected_match_pub(&mut self) {
        if let Some(&(row, col)) = self.note_search.matches.get(self.note_search.selected) {
            self.editor_cursor = (row, col);
            self.adjust_scroll_to_cursor();
        }
    }

    /// Clear all in-note search state.
    pub fn clear_note_search(&mut self) {
        self.note_search.query.clear();
        self.note_search.matches.clear();
        self.note_search.selected = 0;
        self.note_search.active = false;
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
        self.links.incoming = self.get_backlinks_for_current_note()
            .into_iter()
            .filter_map(|title| {
                self.notebook.find_note_by_title(&title)
                    .map(|id| (id, title))
            })
            .collect();
        self.links.outgoing = self.get_outgoing_links_for_current_note();
        if self.links.incoming.is_empty() && self.links.outgoing.is_empty() {
            self.set_message("No links to or from this note".to_string());
        } else {
            self.links.incoming_selected = 0;
            self.links.outgoing_selected = 0;
            // Start focus on whichever section has entries (prefer incoming).
            self.links.focus = if self.links.incoming.is_empty() {
                BacklinkFocus::Outgoing
            } else {
                BacklinkFocus::Incoming
            };
            self.mode = AppMode::Backlinks;
        }
    }

    pub fn cancel_backlinks(&mut self) {
        self.mode = AppMode::Normal;
    }

    /// Toggle navigation focus between the incoming and outgoing sections,
    /// but only when the other section actually has entries.
    pub fn backlinks_toggle_focus(&mut self) {
        self.links.focus = match self.links.focus {
            BacklinkFocus::Incoming if !self.links.outgoing.is_empty() => BacklinkFocus::Outgoing,
            BacklinkFocus::Outgoing if !self.links.incoming.is_empty() => BacklinkFocus::Incoming,
            other => other,
        };
    }

    pub fn backlinks_navigate_up(&mut self) {
        match self.links.focus {
            BacklinkFocus::Incoming => {
                self.links.incoming_selected = self.links.incoming_selected.saturating_sub(1);
            }
            BacklinkFocus::Outgoing => {
                self.links.outgoing_selected = self.links.outgoing_selected.saturating_sub(1);
            }
        }
    }

    pub fn backlinks_navigate_down(&mut self) {
        match self.links.focus {
            BacklinkFocus::Incoming => {
                if self.links.incoming_selected < self.links.incoming.len().saturating_sub(1) {
                    self.links.incoming_selected += 1;
                }
            }
            BacklinkFocus::Outgoing => {
                if self.links.outgoing_selected < self.links.outgoing.len().saturating_sub(1) {
                    self.links.outgoing_selected += 1;
                }
            }
        }
    }

    /// Open the link selected in whichever section currently has focus.
    /// For an outgoing link whose target note doesn't exist yet, create it first.
    pub fn open_selected_backlink(&mut self) {
        match self.links.focus {
            BacklinkFocus::Incoming => {
                if let Some(note_id) = self.links.incoming.get(self.links.incoming_selected).map(|(id, _)| *id) {
                    self.mode = AppMode::Normal;
                    self.open_note_by_id(note_id);
                }
            }
            BacklinkFocus::Outgoing => {
                let Some((target_id, title)) = self.links.outgoing.get(self.links.outgoing_selected).cloned() else {
                    return;
                };
                self.mode = AppMode::Normal;
                match target_id {
                    Some(id) => self.open_note_by_id(id),
                    None => {
                        // Broken link: create the missing note in the same folder, then
                        // refresh links so the source note's link now resolves.
                        let folder_id = self.current_note.as_ref().and_then(|n| n.folder_id);
                        self.create_new_note(title.clone(), folder_id);
                        self.notebook.rebuild_links();
                        self.set_message(format!("Created note '{}'", title));
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Outline panel
    // -------------------------------------------------------------------------

    /// Populate the outline from the current note's headings and enter Outline mode.
    pub fn show_outline(&mut self) {
        if self.current_note.is_none() {
            self.set_message("No note selected".to_string());
            return;
        }
        self.outline_headings = Self::parse_headings(&self.editor_content);
        if self.outline_headings.is_empty() {
            self.set_message("No headings in this note".to_string());
            return;
        }
        self.outline_selected = 0;
        self.mode = AppMode::Outline;
    }

    /// Parse markdown ATX headings (`#`..`######`), skipping fenced code blocks.
    fn parse_headings(content: &str) -> Vec<(usize, u8, String)> {
        let mut headings = Vec::new();
        let mut in_code = false;
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if in_code {
                continue;
            }
            let level = trimmed.bytes().take_while(|&b| b == b'#').count();
            if (1..=6).contains(&level) && trimmed[level..].starts_with(' ') {
                headings.push((i, level as u8, trimmed[level..].trim().to_string()));
            }
        }
        headings
    }

    pub fn outline_navigate_up(&mut self) {
        self.outline_selected = self.outline_selected.saturating_sub(1);
    }

    pub fn outline_navigate_down(&mut self) {
        if self.outline_selected < self.outline_headings.len().saturating_sub(1) {
            self.outline_selected += 1;
        }
    }

    pub fn cancel_outline(&mut self) {
        self.mode = AppMode::Normal;
    }

    /// Jump the editor to the selected heading and return to Normal mode.
    pub fn outline_select(&mut self) {
        if let Some(&(line_idx, _, _)) = self.outline_headings.get(self.outline_selected) {
            self.mode = AppMode::Normal;
            self.focused_pane = FocusedPane::Editor;
            self.jump_to_line(line_idx + 1); // jump_to_line is 1-based
        }
    }

    // -------------------------------------------------------------------------
    // Task checkboxes & daily notes
    // -------------------------------------------------------------------------

    /// Toggle a markdown task checkbox (`[ ]` <-> `[x]`) on the current line.
    pub fn toggle_task_checkbox(&mut self) {
        if self.current_note.is_none() {
            return;
        }
        let row = self.editor_cursor.0 as usize;
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let Some(line) = lines.get(row).copied() else { return; };

        // Prefer flipping an unchecked box; otherwise uncheck a checked one.
        let replacement = line.find("[ ]").map(|pos| (pos, "[x]"))
            .or_else(|| line.to_ascii_lowercase().find("[x]").map(|pos| (pos, "[ ]")));

        let Some((col, new)) = replacement else {
            self.set_message("No checkbox on this line".to_string());
            return;
        };

        self.push_undo_snapshot();
        let start = self.get_line_start_position(row) + col;
        // `[ ]` and `[x]` are both 3 bytes, so the cursor stays valid.
        self.editor_content.replace_range(start..start + 3, new);
        self.update_preview_content();
        self.mark_modified();
    }

    /// Open today's daily note (`YYYY-MM-DD`), creating it at the root if absent.
    pub fn open_daily_note(&mut self) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        if let Some(id) = self.notebook.find_note_by_title(&today) {
            self.open_note_by_id(id);
            self.set_message(format!("Daily note: {}", today));
        } else {
            self.create_new_note(today.clone(), None);
            self.focused_pane = FocusedPane::Editor;
            self.set_message(format!("Created daily note: {}", today));
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
        if !self.spell.enabled || !self.spell.aspell_available {
            return;
        }
        self.spell.errors = crate::spell::check_content(&self.editor_content);
    }

    /// Toggle spell checking on/off.
    #[allow(dead_code)]
    pub fn toggle_spell_check(&mut self) {
        if !self.spell.aspell_available {
            self.set_message("aspell not found — install it with: sudo apt install aspell".to_string());
            return;
        }
        self.spell.enabled = !self.spell.enabled;
        if self.spell.enabled {
            self.run_spell_check();
            self.set_message(format!("Spell check ON — {} error(s) found", self.spell.errors.len()));
        } else {
            self.spell.errors.clear();
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
        if !self.spell.aspell_available {
            self.set_message("aspell not found".to_string());
            return;
        }
        if let Some((row, col, len, word)) = self.get_word_at_cursor() {
            self.spell.word_range = (row, col, len);
            self.spell.suggestions = crate::spell::get_suggestions(&word);
            self.spell.suggestions_selected = 0;
            self.mode = AppMode::SpellSuggest;
        } else {
            self.set_message("No word at cursor".to_string());
        }
    }

    /// Replace the word at `spell_word_range` with the currently selected suggestion.
    pub fn apply_spell_suggestion(&mut self) {
        let (row, col, wlen) = self.spell.word_range;
        if let Some(suggestion) = self.spell.suggestions.get(self.spell.suggestions_selected).cloned() {
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
        app.disk.pending_disk_save = false;

        app.save_current_note().unwrap();

        // editor content was committed to the in-memory notebook
        assert_eq!(app.notebook.notes.get(&id).unwrap().content, "hello world");
        // a disk write is now pending, and the note is in the dirty set
        assert!(app.disk.pending_disk_save);
        assert!(app.disk.dirty_note_ids.contains(&id));

        // after a successful write, our own change is suppressed for the watcher
        app.mark_disk_saved();
        assert!(app.wrote_recently());
        assert_eq!(app.save_status, SaveStatus::Saved);
    }

    #[test]
    fn failed_save_keeps_request_pending_for_retry() {
        let mut app = App::default();
        app.report_save_failure("disk full".to_string());
        assert!(app.disk.pending_disk_save, "failed save must stay pending to retry");
        assert_eq!(app.save_status, SaveStatus::Error);
    }

    #[test]
    fn moving_a_note_relocates_its_file() {
        use crate::models::Folder;
        let mut app = App::default();

        let folder = Folder::new("Work".to_string(), None);
        let fid = folder.id;
        app.notebook.add_folder(folder);

        let mut note = Note::new("Memo".to_string(), None);
        let nid = note.id;
        let old_path = std::path::PathBuf::from("/vault/Memo.md");
        note.file_path = Some(old_path.clone());
        app.notebook.add_note(note);

        app.move_note(nid, Some(fid)).unwrap();

        let moved = app.notebook.notes.get(&nid).unwrap();
        assert_eq!(moved.folder_id, Some(fid));
        // path cleared so the saver rewrites it inside the destination folder
        assert!(moved.file_path.is_none());
        // old file queued for deletion, note queued for (re)write
        assert!(app.disk.deleted_note_paths.contains(&old_path));
        assert!(app.disk.dirty_note_ids.contains(&nid));
    }
}

#[cfg(test)]
mod backlink_graph_tests {
    use super::*;
    use crate::models::Note;

    /// Build an app holding `source` (which links out via `[[..]]` in its content)
    /// with `source` loaded as the current note and its links parsed.
    fn app_with_source(content: &str) -> (App, Uuid) {
        let mut app = App::default();
        let mut source = Note::new("Source".to_string(), None);
        source.content = content.to_string();
        let sid = source.id;
        app.notebook.add_note(source.clone());
        app.current_note = Some(source);
        app.notebook.rebuild_links();
        (app, sid)
    }

    #[test]
    fn outgoing_link_to_existing_note_resolves_and_opens() {
        let (mut app, _) = app_with_source("see [[Target]] for details");
        let target = Note::new("Target".to_string(), None);
        let tid = target.id;
        app.notebook.add_note(target);
        app.notebook.rebuild_links();

        app.show_backlinks_panel();
        assert_eq!(app.mode, AppMode::Backlinks);
        // Incoming empty, so focus lands on the outgoing section.
        assert_eq!(app.links.focus, BacklinkFocus::Outgoing);
        assert_eq!(app.links.outgoing, vec![(Some(tid), "Target".to_string())]);

        app.open_selected_backlink();
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.current_note.as_ref().unwrap().id, tid);
    }

    #[test]
    fn opening_broken_outgoing_link_creates_the_missing_note() {
        let (mut app, sid) = app_with_source("a link to [[Ghost]]");

        app.show_backlinks_panel();
        // Target doesn't exist yet → flagged as broken (None id).
        assert_eq!(app.links.outgoing, vec![(None, "Ghost".to_string())]);

        app.open_selected_backlink();

        // The missing note now exists and the source link resolves to it.
        let ghost_id = app.notebook.find_note_by_title("Ghost");
        assert!(ghost_id.is_some(), "broken link should create the note");
        let resolved: Vec<_> = app.notebook.get_outgoing_links(sid)
            .iter()
            .map(|l| l.target_note_id)
            .collect();
        assert_eq!(resolved, vec![ghost_id]);
    }

    #[test]
    fn duplicate_outgoing_links_are_deduped_by_title() {
        let (mut app, _) = app_with_source("[[Target]] and again [[target]]");
        app.notebook.add_note(Note::new("Target".to_string(), None));
        app.notebook.rebuild_links();

        let outgoing = app.get_outgoing_links_for_current_note();
        assert_eq!(outgoing.len(), 1, "case-insensitive duplicate titles collapse to one");
    }

    #[test]
    fn tab_only_switches_focus_to_a_non_empty_section() {
        let (mut app, _) = app_with_source("links to [[Ghost]]");
        app.show_backlinks_panel();
        // No incoming links, so Tab must keep focus on outgoing.
        assert_eq!(app.links.focus, BacklinkFocus::Outgoing);
        app.backlinks_toggle_focus();
        assert_eq!(app.links.focus, BacklinkFocus::Outgoing);
    }
}

#[cfg(test)]
mod outline_and_task_tests {
    use super::*;
    use crate::models::Note;

    /// App with a single note loaded into the editor with the given content.
    fn app_editing(content: &str) -> App {
        let mut app = App::default();
        let mut note = Note::new("Doc".to_string(), None);
        note.content = content.to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = content.to_string();
        app
    }

    #[test]
    fn parse_headings_collects_levels_and_skips_code_fences() {
        let content = "\
# Title
intro text
## Section A
```
# not a heading (inside code)
```
### Sub A1
###### Deep
####### too many hashes
#no-space";
        let headings = App::parse_headings(content);
        assert_eq!(headings, vec![
            (0, 1, "Title".to_string()),
            (2, 2, "Section A".to_string()),
            (6, 3, "Sub A1".to_string()),
            (7, 6, "Deep".to_string()),
        ]);
    }

    #[test]
    fn outline_select_jumps_cursor_to_heading_line() {
        let mut app = app_editing("# A\n\ntext\n## B\nmore");
        app.show_outline();
        assert_eq!(app.mode, AppMode::Outline);
        // Two headings: "# A" at line 0, "## B" at line 3.
        assert_eq!(app.outline_headings.len(), 2);
        app.outline_navigate_down();
        app.outline_select();
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.editor_cursor.0, 3); // jumped to "## B"
    }

    #[test]
    fn toggle_task_checkbox_flips_both_ways() {
        let mut app = app_editing("- [ ] buy milk");
        app.editor_cursor = (0, 0);

        app.toggle_task_checkbox();
        assert_eq!(app.editor_content, "- [x] buy milk");

        app.toggle_task_checkbox();
        assert_eq!(app.editor_content, "- [ ] buy milk");
    }

    #[test]
    fn toggle_task_checkbox_unchecks_uppercase_and_preserves_other_lines() {
        let mut app = app_editing("first\n- [X] done\nlast");
        app.editor_cursor = (1, 0); // on the checkbox line
        app.toggle_task_checkbox();
        assert_eq!(app.editor_content, "first\n- [ ] done\nlast");
    }

    #[test]
    fn toggle_task_checkbox_is_a_noop_without_a_checkbox() {
        let mut app = app_editing("just a paragraph");
        app.editor_cursor = (0, 0);
        app.toggle_task_checkbox();
        assert_eq!(app.editor_content, "just a paragraph");
    }

    #[test]
    fn daily_note_creates_then_reopens_the_same_note() {
        let mut app = App::default();
        let before = app.notebook.notes.len();

        app.open_daily_note();
        let after_create = app.notebook.notes.len();
        assert_eq!(after_create, before + 1);

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let id = app.notebook.find_note_by_title(&today).expect("daily note exists");

        // Opening again must reuse the existing note, not create a duplicate.
        app.open_daily_note();
        assert_eq!(app.notebook.notes.len(), after_create);
        assert_eq!(app.notebook.find_note_by_title(&today), Some(id));
    }
}
