use chrono::{DateTime, Utc};
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
    Visual,
    TemplatePicker,
    SpellSuggest,
    Outline,
    /// One door in front of the six finders.
    Palette,
    /// Every open task in the vault, in one list.
    Tasks,
    /// The folder tree as an overlay, for when the sidebar is not on screen.
    Explorer,
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


/// One row of the "where you left off" list.
#[derive(Debug, Clone)]
pub struct RecentEntry {
    pub id: Uuid,
    pub title: String,
    /// Containing folder name, empty for notes at the vault root.
    pub folder: String,
    /// Coarse "2h ago" style age, for a glanceable column.
    pub age: String,
}

/// What activating a landing-page row does.
///
/// The page returns intent rather than performing it, so the row and the key
/// that does the same thing share one code path in the dispatcher instead of
/// drifting apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeAction {
    OpenNote(Uuid),
    DailyNote,
    NewNote,
    Search,
    QuickJump,
    Explorer,
    Help,
}

/// One selectable row of the landing page.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub detail: String,
    pub key: String,
    pub action: WelcomeAction,
}

/// Everything the landing page shows.
///
/// Computed rather than stored: it is read once per frame from state that is
/// already authoritative, so it cannot drift the way a cached copy would.
#[derive(Debug, Default)]
pub struct Dashboard {
    pub recent: Vec<RecentEntry>,
    /// Every selectable row, recents first, then the fixed actions.
    pub menu: Vec<MenuItem>,
    /// Where the recents end and the actions begin, for the separating gap.
    pub recent_count: usize,
    pub open_tasks: usize,
    pub notes_with_tasks: usize,
    pub note_count: usize,
    pub folder_count: usize,
    pub tag_count: usize,
    pub vault_label: Option<String>,
}

/// Render a duration as the coarsest unit that is still true. Deliberately
/// approximate: the column exists to be scanned, not to be precise.
/// Flip a checkbox on a line, returning the rewritten line.
///
/// `None` when the line has no checkbox, which the task panel treats as "the note
/// changed under us" rather than editing whatever now sits on that line.
fn flip_checkbox(line: &str) -> Option<String> {
    let open = line.find("[ ]").map(|pos| (pos, "[x]"));
    let shut = crate::tasks::parse_task_line(line)
        .filter(|(done, _)| *done)
        .and_then(|_| line.find('[').map(|pos| (pos, "[ ]")));

    let (col, new) = open.or(shut)?;
    let mut out = line.to_string();
    // Every mark is one character and the brackets are ASCII, so the replacement is
    // always the same three bytes.
    out.replace_range(col..col + 3, new);
    Some(out)
}

fn humanize_age(then: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let mins = (now - then).num_minutes();
    if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{}m ago", mins)
    } else if mins < 60 * 24 {
        format!("{}h ago", mins / 60)
    } else if mins < 60 * 24 * 7 {
        format!("{}d ago", mins / (60 * 24))
    } else {
        format!("{}w ago", mins / (60 * 24 * 7))
    }
}

pub struct App {
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
    /// Vault root actually in use, for the landing page. None in JSON mode.
    pub vault_path: Option<std::path::PathBuf>,
    /// Highlighted row of the landing page menu.
    pub welcome_selected: usize,
    /// Where a confirm dialog should return to. Without this, deleting from the
    /// explorer would dump you back to Normal instead of the tree you were in.
    pub modal_return: Option<AppMode>,
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
    /// Whether the yank buffer holds whole lines. `p` pastes a linewise yank onto
    /// its own line and a charwise one inline, which is the difference between `yy`
    /// and `yw` behaving like vim and behaving like a surprise.
    pub yank_linewise: bool,

    /// The operator waiting for a motion, if any — the `d` of a half-typed `dw`.
    pub pending_op: Option<crate::vim::Operator>,
    /// A count being typed, for `3dd` and `d3w`.
    pub pending_count: Option<usize>,
    /// A key that only makes sense as the middle of a longer sequence: the `i` of
    /// `diw`, the `g` of `dgg`.
    pub pending_op_prefix: Option<char>,

    // Redo stack (mirror of undo_stack, cleared on new edits)
    pub redo_stack: Vec<(String, (u16, u16))>,

    // In-note search


    // Outline panel
    pub outline_headings: Vec<(usize, u8, String)>, // (line_index, level, text)
    pub outline_selected: usize,

    // Command palette
    pub palette_query: String,
    pub palette_items: Vec<crate::palette::Item>,
    pub palette_selected: usize,
    /// Every task in the vault, as of the last time the panel was opened.
    pub task_items: Vec<crate::tasks::Task>,
    pub task_selected: usize,
    /// Whether the panel is also listing completed tasks. Off by default: the
    /// question the panel answers is what is still outstanding.
    pub tasks_show_done: bool,

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
            vault_path: None,
            welcome_selected: 0,
            modal_return: None,
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
            yank_linewise: true,
            pending_op: None,
            pending_count: None,
            pending_op_prefix: None,

            // Redo
            redo_stack: Vec::new(),

            outline_headings: Vec::new(),
            outline_selected: 0,
            palette_query: String::new(),
            palette_items: Vec::new(),
            palette_selected: 0,
            task_items: Vec::new(),
            task_selected: 0,
            tasks_show_done: false,

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
    /// Build the landing-page view of the notebook.
    ///
    /// The landing page answers "what was I doing, and what should I do next" —
    /// so it is built from the notebook's actual state (recency, open tasks,
    /// today's note) rather than from a list of the app's features, which is
    /// what `?` is for.
    pub fn dashboard(&self) -> Dashboard {
        let now = Utc::now();

        let mut by_recency: Vec<&Note> = self.notebook.notes.values().collect();
        // Ties broken by id so the list never reshuffles between frames.
        by_recency.sort_by(|a, b| b.modified_at.cmp(&a.modified_at).then(a.id.cmp(&b.id)));

        let recent = by_recency
            .iter()
            .take(8)
            .map(|n| RecentEntry {
                id: n.id,
                title: n.title.clone(),
                folder: n
                    .folder_id
                    .and_then(|fid| self.notebook.folders.get(&fid))
                    .map(|f| f.name.clone())
                    .unwrap_or_default(),
                age: humanize_age(n.modified_at, now),
            })
            .collect();

        let today_title = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_exists = self.notebook.find_note_by_title(&today_title).is_some();

        // Shared with the task panel, so the landing page's count and the panel's
        // list can never disagree about what a task is. The inline version this
        // replaces also counted checkboxes inside fenced code blocks.
        let open = crate::tasks::collect(&self.notebook, false);
        let open_tasks = open.len();
        let notes_with_tasks = crate::tasks::notes_covered(&open);

        let recent: Vec<RecentEntry> = recent;
        let mut menu: Vec<MenuItem> = recent
            .iter()
            .enumerate()
            .map(|(i, e)| MenuItem {
                label: e.title.clone(),
                detail: if e.folder.is_empty() {
                    e.age.clone()
                } else {
                    format!("{} · {}", e.folder, e.age)
                },
                key: format!("{}", i + 1),
                action: WelcomeAction::OpenNote(e.id),
            })
            .collect();
        let recent_count = menu.len();

        let today_detail = if today_exists {
            format!("{} · open", today_title)
        } else {
            format!("{} · new", today_title)
        };
        for (label, detail, key, action) in [
            ("Today's daily note", today_detail.as_str(), "F4", WelcomeAction::DailyNote),
            ("Browse the vault", "", "e", WelcomeAction::Explorer),
            ("New note", "", "n", WelcomeAction::NewNote),
            ("Search all notes", "", "/", WelcomeAction::Search),
            ("Jump to note", "", "Ctrl+J", WelcomeAction::QuickJump),
            ("Help", "", "?", WelcomeAction::Help),
        ] {
            menu.push(MenuItem {
                label: label.to_string(),
                detail: detail.to_string(),
                key: key.to_string(),
                action,
            });
        }

        Dashboard {
            menu,
            recent_count,
            recent,
            open_tasks,
            notes_with_tasks,
            note_count: self.notebook.notes.len(),
            folder_count: self.notebook.folders.len(),
            tag_count: self.tag_manager.get_tags_alphabetical().len(),
            vault_label: self.vault_path.as_ref().and_then(|p| {
                p.file_name().map(|n| n.to_string_lossy().to_string())
            }),
        }
    }

    /// Move the landing-page highlight, clamped rather than wrapped: running off
    /// the end of a short list and silently landing back at the top is
    /// disorienting on a page you are only glancing at.
    /// Point the tree selection at the note currently open.
    ///
    /// The rename/move/delete commands all act on the tree selection. With the
    /// sidebar hidden that selection is invisible, so acting on it blind is a
    /// footgun — retarget it at the note actually on screen first, and the
    /// existing commands keep working unchanged.
    pub fn focus_tree_on_current_note(&mut self) -> bool {
        let Some(id) = self.current_note.as_ref().map(|n| n.id) else {
            return false;
        };
        if let Some(idx) = self
            .folder_tree_items
            .iter()
            .position(|t| t.id == id && t.item_type == TreeItemType::Note)
        {
            self.selected_folder_index = idx;
            true
        } else {
            false
        }
    }

    pub fn welcome_move(&mut self, delta: isize) {
        let len = self.dashboard().menu.len();
        if len == 0 {
            self.welcome_selected = 0;
            return;
        }
        let next = self.welcome_selected as isize + delta;
        self.welcome_selected = next.clamp(0, len as isize - 1) as usize;
    }

    /// What the highlighted row would do. The dispatcher performs it, so a row
    /// and its shortcut key cannot diverge.
    pub fn welcome_action_at_cursor(&self) -> Option<WelcomeAction> {
        self.dashboard().menu.get(self.welcome_selected).map(|m| m.action)
    }

    pub fn set_welcome_message(&mut self) {
        let note_count = self.notebook.notes.len();
        let folder_count = self.notebook.folders.len();
        
        // The landing page already shows the counts and the keys, so repeating
        // them in the status bar is just noise. Only the empty vault needs a nudge.
        if note_count == 0 {
            self.set_message("Press 'n' to write your first note, or '?' for help".to_string());
        } else {
            self.set_message(String::new());
        }
        let _ = folder_count;
        
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
    /// Absorb what a save reported: paths chosen for new notes, fresh disk stamps,
    /// and any file that had to be preserved before we wrote over it.
    ///
    /// Storing the stamps back is not bookkeeping — it is what stops the *next*
    /// save from mistaking our own write for somebody else's and preserving a copy
    /// of it on every single autosave.
    pub fn apply_save_report(&mut self, report: crate::storage::SaveReport) {
        // Store back the path chosen for any newly-written note so it writes to the
        // same file next time.
        for (id, path) in report.assigned {
            if let Some(n) = self.notebook.notes.get_mut(&id) {
                n.file_path = Some(path.clone());
            }
            if self.current_note.as_ref().map(|n| n.id) == Some(id) {
                if let Some(cn) = self.current_note.as_mut() {
                    cn.file_path = Some(path);
                }
            }
        }

        for (id, stamp) in report.stamps {
            if let Some(n) = self.notebook.notes.get_mut(&id) {
                n.disk_stamp = Some(stamp);
            }
            if let Some(cn) = self.current_note.as_mut() {
                if cn.id == id {
                    cn.disk_stamp = Some(stamp);
                }
            }
        }

        // Say so plainly. The user's own text is still in the note they are editing;
        // what they need to know is that a second file now exists and where.
        if let Some(conflict) = report.conflicts.last() {
            let name = conflict
                .preserved_at
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| conflict.preserved_at.display().to_string());
            let extra = report.conflicts.len().saturating_sub(1);
            let message = if extra > 0 {
                format!(
                    "'{}' changed on disk — kept that version as {} ({} more)",
                    conflict.note_title, name, extra
                )
            } else {
                format!(
                    "'{}' changed on disk — kept that version as {}",
                    conflict.note_title, name
                )
            };
            self.set_operation_info(message, Some("⚔️".to_string()));
        }
    }

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
    
    /// Pull the cursor back inside the buffer.
    ///
    /// Needed whenever the content changes underneath the cursor rather than
    /// because of it — a reload from disk, or an operator that deleted the rest of
    /// the line. `get_cursor_byte_index` trusts the row and column it is given, so
    /// a cursor left past the end of a shortened note indexes into nothing.
    pub fn clamp_cursor_to_content(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let last_row = lines.len().saturating_sub(1) as u16;
        self.editor_cursor.0 = self.editor_cursor.0.min(last_row);
        let line_len = lines
            .get(self.editor_cursor.0 as usize)
            .map(|l| l.chars().count())
            .unwrap_or(0) as u16;
        self.editor_cursor.1 = self.editor_cursor.1.min(line_len);
        self.adjust_scroll_to_cursor();
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
    // Command palette
    // -------------------------------------------------------------------------

    /// Open the palette with an empty query, showing recent notes.
    pub fn show_palette(&mut self) {
        self.palette_query.clear();
        self.palette_selected = 0;
        self.refresh_palette();
        self.mode = AppMode::Palette;
    }

    pub fn cancel_palette(&mut self) {
        self.mode = AppMode::Normal;
        self.palette_query.clear();
        self.palette_items.clear();
        self.palette_selected = 0;
    }

    /// Re-rank against the current query.
    ///
    /// Cheap enough to run on every keystroke at this vault size, and doing so is
    /// what makes the palette feel like one thing rather than a search you submit.
    pub fn refresh_palette(&mut self) {
        let recent: Vec<Uuid> = self
            .notebook
            .get_recent_files()
            .iter()
            .map(|r| r.note_id)
            .collect();
        let current_content = self.editor_content.clone();
        let ctx = crate::palette::Context {
            notebook: &self.notebook,
            tags: &self.tag_manager,
            recent: &recent,
            current_note: self.current_note.as_ref().map(|n| n.id),
            current_content: &current_content,
        };
        self.palette_items = crate::palette::resolve(&self.palette_query, &ctx);
        // The old selection means nothing against a new list; anchoring to the top
        // keeps Enter landing on the best match rather than wherever the cursor was.
        self.palette_selected = 0;
    }

    pub fn palette_type(&mut self, c: char) {
        self.palette_query.push(c);
        self.refresh_palette();
    }

    pub fn palette_backspace(&mut self) {
        self.palette_query.pop();
        self.refresh_palette();
    }

    pub fn palette_navigate_up(&mut self) {
        self.palette_selected = self.palette_selected.saturating_sub(1);
    }

    pub fn palette_navigate_down(&mut self) {
        if self.palette_selected < self.palette_items.len().saturating_sub(1) {
            self.palette_selected += 1;
        }
    }

    /// Act on the selected row.
    ///
    /// Returns the command to run, if the row was one — the caller dispatches it,
    /// because commands map onto key actions that live in the event layer.
    pub fn palette_select(&mut self) -> Option<crate::palette::Command> {
        use crate::palette::PaletteAction;

        let item = self.palette_items.get(self.palette_selected).cloned()?;

        match item.action {
            PaletteAction::OpenNote(id) => {
                self.cancel_palette();
                self.open_note_by_id(id);
                None
            }
            PaletteAction::JumpTo(id, line) => {
                self.cancel_palette();
                self.open_note_by_id(id);
                self.focused_pane = FocusedPane::Editor;
                self.jump_to_line(line + 1); // jump_to_line is 1-based
                None
            }
            // Narrows the palette rather than closing it: picking a tag is choosing
            // where to look, not what to open. The trailing space is what the
            // resolver reads as "within this tag", so the state stays entirely in
            // the query and typing more keeps narrowing.
            PaletteAction::FilterTag(tag) => {
                self.palette_query = format!("#{} ", tag);
                self.refresh_palette();
                None
            }
            PaletteAction::Run(cmd) => {
                self.cancel_palette();
                Some(cmd)
            }
        }
    }


    // Task panel
    // -------------------------------------------------------------------------

    /// Collect every open task in the vault and enter Tasks mode.
    ///
    /// Vault-wide rather than per-note on purpose: the point is seeing what is
    /// outstanding without having to remember which note you wrote it in.
    pub fn show_task_panel(&mut self) {
        self.task_items = crate::tasks::collect(&self.notebook, self.tasks_show_done);
        if self.task_items.is_empty() {
            let what = if self.tasks_show_done { "tasks" } else { "open tasks" };
            self.set_message(format!("No {} in the vault", what));
            return;
        }
        self.task_selected = self.task_selected.min(self.task_items.len() - 1);
        self.mode = AppMode::Tasks;
    }

    pub fn task_navigate_up(&mut self) {
        self.task_selected = self.task_selected.saturating_sub(1);
    }

    pub fn task_navigate_down(&mut self) {
        if self.task_selected < self.task_items.len().saturating_sub(1) {
            self.task_selected += 1;
        }
    }

    pub fn cancel_task_panel(&mut self) {
        self.mode = AppMode::Normal;
    }

    /// Show or hide completed tasks, keeping the panel open.
    pub fn toggle_task_panel_done(&mut self) {
        self.tasks_show_done = !self.tasks_show_done;
        self.task_items = crate::tasks::collect(&self.notebook, self.tasks_show_done);
        if self.task_items.is_empty() {
            self.task_selected = 0;
        } else {
            self.task_selected = self.task_selected.min(self.task_items.len() - 1);
        }
    }

    /// Open the note the selected task lives in, with the cursor on that line.
    pub fn task_select(&mut self) {
        let Some(task) = self.task_items.get(self.task_selected).cloned() else {
            return;
        };
        self.mode = AppMode::Normal;
        self.open_note_by_id(task.note_id);
        self.focused_pane = FocusedPane::Editor;
        self.jump_to_line(task.line + 1); // jump_to_line is 1-based
    }

    /// Tick or untick the selected task where it lives, without leaving the panel.
    ///
    /// This is what makes the panel somewhere you work rather than just an index:
    /// clearing the morning's list should not mean opening five notes.
    pub fn task_toggle_selected(&mut self) {
        let Some(task) = self.task_items.get(self.task_selected).cloned() else {
            return;
        };
        let Some(note) = self.notebook.notes.get_mut(&task.note_id) else {
            return;
        };

        let mut lines: Vec<String> = note.content.lines().map(|l| l.to_string()).collect();
        let Some(line) = lines.get_mut(task.line) else {
            return;
        };
        let Some(flipped) = flip_checkbox(line) else {
            // The note changed under us since the list was built; rebuild rather
            // than editing whatever now happens to sit on that line.
            self.task_items = crate::tasks::collect(&self.notebook, self.tasks_show_done);
            self.set_message("Task list was out of date; refreshed".to_string());
            return;
        };
        *line = flipped;

        let trailing_newline = note.content.ends_with('\n');
        let mut rebuilt = lines.join("\n");
        if trailing_newline {
            rebuilt.push('\n');
        }
        note.update_content(rebuilt.clone());
        let note_id = task.note_id;

        // Keep the open editor in step if this is the note on screen.
        if self.current_note.as_ref().map(|n| n.id) == Some(note_id) {
            self.editor_content = rebuilt;
            if let Some(cn) = self.current_note.as_mut() {
                cn.content = self.editor_content.clone();
            }
            self.clamp_cursor_to_content();
        }

        // mark_note_dirty also sets the pending-write flag.
        self.mark_note_dirty(note_id);
        self.mark_modified();

        // Rebuild so a ticked task leaves the list when done items are hidden, and
        // hold the selection at the same position rather than snapping to the top.
        let previous = self.task_selected;
        self.task_items = crate::tasks::collect(&self.notebook, self.tasks_show_done);
        self.task_selected = previous.min(self.task_items.len().saturating_sub(1));
        if self.task_items.is_empty() {
            self.mode = AppMode::Normal;
            self.set_message("All tasks done".to_string());
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

    /// Open today's daily note, creating it if absent.
    ///
    /// Title and folder come from `[capture]`, the same settings `scribble --today`
    /// uses, so the two entry points can never disagree about which file today's
    /// note is. They default to `YYYY-MM-DD` at the vault root, which is what this
    /// has always done.
    pub fn open_daily_note(&mut self) {
        let today = chrono::Local::now()
            .format(&self.config.capture.daily_format)
            .to_string();
        if let Some(id) = self.notebook.find_note_by_title(&today) {
            self.open_note_by_id(id);
            self.set_message(format!("Daily note: {}", today));
        } else {
            let folder = self.config.capture.daily_folder.clone();
            let folder_id = crate::capture::daily_folder_id(&mut self.notebook, &folder);
            self.create_new_note(today.clone(), folder_id);
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
        // Existing behaviour: a visual yank pastes onto its own line.
        self.yank_linewise = true;
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

    /// Storing the report's stamps back is what stops the next save mistaking our
    /// own write for somebody else's. Dropping them would put a conflict file beside
    /// every note on the second autosave.
    #[test]
    fn a_save_report_stores_paths_and_stamps_back_onto_the_notes() {
        use crate::models::FileStamp;
        let mut app = App::default();
        let note = Note::new("Test".to_string(), None);
        let id = note.id;
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);

        let path = std::path::PathBuf::from("/vault/Test.md");
        let stamp = FileStamp::of_bytes(b"written");
        app.apply_save_report(crate::storage::SaveReport {
            assigned: vec![(id, path.clone())],
            stamps: vec![(id, stamp)],
            conflicts: Vec::new(),
        });

        let stored = app.notebook.notes.get(&id).unwrap();
        assert_eq!(stored.file_path.as_deref(), Some(path.as_path()));
        assert_eq!(stored.disk_stamp, Some(stamp), "stamp was not stored back");
        // The open copy of the note has to be updated too, or it saves with a stale
        // stamp and conflicts with itself.
        let open = app.current_note.as_ref().unwrap();
        assert_eq!(open.file_path.as_deref(), Some(path.as_path()));
        assert_eq!(open.disk_stamp, Some(stamp));
    }

    /// A preserved file the user never hears about is a file they will not think to
    /// look for, so a conflict has to reach the status line.
    #[test]
    fn a_conflict_is_reported_to_the_user() {
        let mut app = App::default();
        app.apply_save_report(crate::storage::SaveReport {
            assigned: Vec::new(),
            stamps: Vec::new(),
            conflicts: vec![crate::storage::Conflict {
                note_title: "Meeting".to_string(),
                preserved_at: std::path::PathBuf::from(
                    "/vault/Meeting (scribble conflict 2026-08-16 131829).md",
                ),
            }],
        });

        let shown = format!("{:?}", app.operation_result);
        assert!(shown.contains("Meeting"), "note not named: {}", shown);
        assert!(
            shown.contains("scribble conflict"),
            "the preserved file was not named: {}",
            shown
        );
    }

    /// A reload can shorten the note under the cursor, and get_cursor_byte_index
    /// trusts whatever row and column it is handed.
    #[test]
    fn the_cursor_is_pulled_back_inside_a_shortened_note() {
        let mut app = App::default();
        app.editor_content = "a much longer first line\nand a second\n".to_string();
        app.editor_cursor = (1, 12);

        app.editor_content = "short\n".to_string();
        app.clamp_cursor_to_content();

        assert_eq!(app.editor_cursor.0, 0, "cursor left past the last line");
        assert!(app.editor_cursor.1 <= 5, "cursor left past the end of its line");
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

    /// An app whose vault holds exactly the given `(title, content)` notes.
    ///
    /// `App::new` seeds a sample note, and `notes` is a HashMap with no order of
    /// its own — so leaving it in place makes any "the first note" lookup a coin
    /// flip, and lets a stray sample task into the panel's list.
    fn app_with_notes(notes: &[(&str, &str)]) -> App {
        let mut app = App::default();
        app.notebook.notes.clear();
        app.notebook.folders.clear();
        for (title, content) in notes {
            let mut note = Note::new(title.to_string(), None);
            note.content = content.to_string();
            app.notebook.add_note(note);
        }
        app
    }

    #[test]
    fn the_task_panel_lists_open_tasks_from_every_note() {
        let mut app = app_with_notes(&[
            ("Alpha", "- [ ] one\n- [x] done\n"),
            ("Beta", "- [ ] two\n"),
        ]);
        app.show_task_panel();

        assert_eq!(app.mode, AppMode::Tasks);
        let texts: Vec<&str> = app.task_items.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, vec!["one", "two"], "completed task should be hidden");
    }

    /// Opening a panel with nothing in it would be a dead end, so it says so and
    /// stays put instead.
    #[test]
    fn an_empty_task_panel_does_not_open() {
        let mut app = app_with_notes(&[("Alpha", "no tasks here\n")]);
        app.show_task_panel();
        assert_eq!(app.mode, AppMode::Normal, "opened an empty panel");
    }

    #[test]
    fn selecting_a_task_opens_its_note_at_that_line() {
        let mut app = app_with_notes(&[("Alpha", "intro\n\n- [ ] find me\n")]);
        app.show_task_panel();
        app.task_select();

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(
            app.current_note.as_ref().map(|n| n.title.as_str()),
            Some("Alpha")
        );
        assert_eq!(app.editor_cursor.0, 2, "cursor did not land on the task line");
    }

    /// Ticking from the panel is what makes it somewhere to work rather than just
    /// an index — clearing the morning's list should not mean opening five notes.
    #[test]
    fn toggling_from_the_panel_edits_the_note_it_lives_in() {
        let mut app = app_with_notes(&[("Alpha", "- [ ] one\n- [ ] two\n")]);
        app.show_task_panel();
        app.task_toggle_selected();

        let id = app.notebook.find_note_by_title("Alpha").unwrap();
        let content = app.notebook.notes.get(&id).unwrap().content.clone();
        assert!(content.starts_with("- [x] one"), "checkbox not ticked: {:?}", content);
        assert!(content.contains("- [ ] two"), "the other task was disturbed: {:?}", content);
        assert!(app.disk.dirty_note_ids.contains(&id), "edit was not queued for saving");
    }

    /// A ticked task leaves the list when completed ones are hidden, and the
    /// selection holds its position rather than snapping back to the top.
    #[test]
    fn a_ticked_task_leaves_the_list_and_the_selection_holds() {
        let mut app = app_with_notes(&[("Alpha", "- [ ] one\n- [ ] two\n- [ ] three\n")]);
        app.show_task_panel();
        app.task_navigate_down();
        assert_eq!(app.task_items[app.task_selected].text, "two");

        app.task_toggle_selected();

        let texts: Vec<&str> = app.task_items.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, vec!["one", "three"]);
        assert_eq!(app.task_selected, 1, "selection jumped away from where it was");
    }

    #[test]
    fn showing_done_tasks_includes_them_without_closing_the_panel() {
        let mut app = app_with_notes(&[("Alpha", "- [ ] open\n- [x] closed\n")]);
        app.show_task_panel();
        assert_eq!(app.task_items.len(), 1);

        app.toggle_task_panel_done();
        assert_eq!(app.task_items.len(), 2);
        assert!(app.tasks_show_done);
    }

    /// The editor must not keep showing the old text when the panel edits the note
    /// currently on screen.
    #[test]
    fn toggling_the_open_note_updates_the_editor_too() {
        let mut app = app_with_notes(&[("Alpha", "- [ ] one\n")]);
        let id = app.notebook.find_note_by_title("Alpha").unwrap();
        app.open_note_by_id(id);
        app.show_task_panel();
        app.task_toggle_selected();

        assert!(
            app.editor_content.starts_with("- [x] one"),
            "editor kept the stale text: {:?}",
            app.editor_content
        );
    }

    /// Ticking the last open task empties the panel, which should close rather than
    /// leave an empty box on screen.
    #[test]
    fn clearing_the_last_task_closes_the_panel() {
        let mut app = app_with_notes(&[("Alpha", "- [ ] only one\n")]);
        app.show_task_panel();
        app.task_toggle_selected();
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn flip_checkbox_handles_both_directions_and_leaves_prose_alone() {
        assert_eq!(flip_checkbox("- [ ] a").as_deref(), Some("- [x] a"));
        assert_eq!(flip_checkbox("- [x] a").as_deref(), Some("- [ ] a"));
        assert_eq!(flip_checkbox("  - [X] indented").as_deref(), Some("  - [ ] indented"));
        assert_eq!(flip_checkbox("just prose"), None);
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

#[cfg(test)]
mod dashboard_tests {
    use super::*;
    use crate::models::{Folder, Note};

    /// App::new seeds a sample note and three folders, which would skew every
    /// count here — start from a genuinely empty notebook instead.
    fn empty_app() -> App {
        let mut app = App::default();
        app.notebook.notes.clear();
        app.notebook.folders.clear();
        app
    }

    fn note_at(title: &str, minutes_ago: i64, folder: Option<Uuid>) -> Note {
        let mut n = Note::new(title.to_string(), folder);
        n.modified_at = Utc::now() - chrono::Duration::minutes(minutes_ago);
        n
    }

    /// The list answers "what was I doing", so it is newest-first and capped
    /// however large the vault is.
    #[test]
    fn recent_list_is_newest_first_and_capped() {
        let mut app = empty_app();
        let titles = ["oldest", "h", "g", "f", "e", "d", "c", "b", "newest"];
        for (i, title) in titles.iter().enumerate() {
            app.notebook.add_note(note_at(title, (titles.len() - i) as i64 * 10, None));
        }
        let d = app.dashboard();
        assert_eq!(d.recent.len(), 8, "capped at eight");
        assert_eq!(d.recent[0].title, "newest");
        assert!(
            !d.recent.iter().any(|e| e.title == "oldest"),
            "the oldest note must fall off the list"
        );
    }

    /// The folder column names the containing folder, and root notes say so
    /// rather than rendering an empty gap.
    #[test]
    fn recent_entries_carry_their_folder() {
        let mut app = empty_app();
        let folder = Folder::new("Cheat-Sheets".to_string(), None);
        let fid = folder.id;
        app.notebook.add_folder(folder);
        app.notebook.add_note(note_at("yazi", 1, Some(fid)));
        app.notebook.add_note(note_at("loose", 2, None));

        let d = app.dashboard();
        assert_eq!(d.recent[0].folder, "Cheat-Sheets");
        assert_eq!(d.recent[1].folder, "", "root notes carry no folder name");
    }

    /// Only unchecked boxes count — a finished list is not outstanding work.
    #[test]
    fn only_unchecked_tasks_are_counted() {
        let mut app = empty_app();
        let mut a = Note::new("Plan".to_string(), None);
        a.content = "- [ ] one\n- [x] done\n  - [ ] indented\n* [ ] star bullet\n- not a task".into();
        let mut b = Note::new("Done".to_string(), None);
        b.content = "- [x] all\n- [X] finished".into();
        app.notebook.add_note(a);
        app.notebook.add_note(b);

        let d = app.dashboard();
        assert_eq!(d.open_tasks, 3, "two dash bullets plus the star bullet");
        assert_eq!(d.notes_with_tasks, 1, "the fully-checked note does not count");
    }

    #[test]
    fn ages_read_as_the_coarsest_true_unit() {
        let now = Utc::now();
        let ago = |m: i64| humanize_age(now - chrono::Duration::minutes(m), now);
        assert_eq!(ago(0), "just now");
        assert_eq!(ago(5), "5m ago");
        assert_eq!(ago(90), "1h ago");
        assert_eq!(ago(60 * 30), "1d ago");
        assert_eq!(ago(60 * 24 * 10), "1w ago");
    }

    /// An empty vault has no recency to show; the renderer branches on this.
    #[test]
    fn empty_vault_yields_an_empty_recent_list() {
        let d = empty_app().dashboard();
        assert!(d.recent.is_empty());
        assert_eq!(d.open_tasks, 0);
        assert_eq!(d.recent_count, 0, "no recents means the menu is actions only");
        assert!(
            d.menu.iter().any(|m| m.action == WelcomeAction::DailyNote),
            "the daily-note action is offered even in an empty vault"
        );
    }

    /// The menu is recents then actions, with recent_count marking the seam the
    /// renderer puts a gap at.
    #[test]
    fn menu_is_recents_then_actions() {
        let mut app = empty_app();
        app.notebook.add_note(note_at("Alpha", 5, None));
        app.notebook.add_note(note_at("Beta", 9, None));
        let d = app.dashboard();

        assert_eq!(d.recent_count, 2);
        assert!(matches!(d.menu[0].action, WelcomeAction::OpenNote(_)));
        assert!(matches!(d.menu[1].action, WelcomeAction::OpenNote(_)));
        assert!(
            d.menu[d.recent_count..]
                .iter()
                .all(|m| !matches!(m.action, WelcomeAction::OpenNote(_))),
            "everything after the seam is a fixed action"
        );
        assert_eq!(d.menu[0].key, "1", "recents keep their digit shortcut");
    }

    /// Clamped, not wrapped: silently jumping from the last row back to the first
    /// is disorienting on a page you only glance at.
    #[test]
    fn welcome_selection_clamps_at_both_ends() {
        let mut app = empty_app();
        app.notebook.add_note(note_at("Alpha", 5, None));
        let last = app.dashboard().menu.len() - 1;

        app.welcome_move(-1);
        assert_eq!(app.welcome_selected, 0, "cannot go above the first row");

        for _ in 0..50 {
            app.welcome_move(1);
        }
        assert_eq!(app.welcome_selected, last, "cannot go past the last row");
    }

    /// The highlighted row and its shortcut key must resolve to the same thing.
    #[test]
    fn activating_a_recent_row_targets_that_note() {
        let mut app = empty_app();
        app.notebook.add_note(note_at("Older", 90, None));
        let newest = note_at("Newest", 1, None);
        let newest_id = newest.id;
        app.notebook.add_note(newest);

        app.welcome_selected = 0;
        assert_eq!(
            app.welcome_action_at_cursor(),
            Some(WelcomeAction::OpenNote(newest_id))
        );
    }

    /// Rename/move act on the tree selection. With the sidebar hidden that
    /// selection is invisible, so they retarget it at the open note first.
    #[test]
    fn focus_tree_on_current_note_targets_the_open_note() {
        let mut app = empty_app();
        let a = note_at("Alpha", 5, None);
        let b = note_at("Beta", 9, None);
        let b_id = b.id;
        app.notebook.add_note(a);
        app.notebook.add_note(b.clone());
        app.refresh_tree_view();

        app.current_note = Some(b);
        app.selected_folder_index = 0;
        assert!(app.focus_tree_on_current_note());

        let sel = app.get_selected_item().expect("something must be selected");
        assert_eq!(sel.id, b_id, "selection follows the note on screen");
    }

    /// Nothing open means nothing to retarget, and it must not move the cursor.
    #[test]
    fn focus_tree_on_current_note_is_a_noop_with_no_note() {
        let mut app = empty_app();
        app.notebook.add_note(note_at("Alpha", 5, None));
        app.refresh_tree_view();
        app.current_note = None;
        app.selected_folder_index = 0;

        assert!(!app.focus_tree_on_current_note());
        assert_eq!(app.selected_folder_index, 0);
    }
}
