use crate::app::{App, AppMode, FocusedPane, TreeItemType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, Event};

pub fn handle_event(app: &mut App, event: Event) -> Result<(), Box<dyn std::error::Error>> {
    if let Event::Key(key) = event {
        match app.mode {
            AppMode::Normal => handle_normal_mode(app, key),
            AppMode::Insert => handle_insert_mode(app, key),
            AppMode::Search => handle_search_mode(app, key),
            AppMode::SearchAdvanced => handle_advanced_search_mode(app, key),
            AppMode::SearchReplace => handle_replace_mode(app, key),
            AppMode::Command => handle_command_mode(app, key),
            AppMode::InputNote => handle_input_note_mode(app, key),
            AppMode::InputFolder => handle_input_folder_mode(app, key),
            AppMode::Move => handle_move_mode(app, key),
            AppMode::Help => handle_help_mode(app, key),
            AppMode::DeleteConfirm => handle_delete_confirm_mode(app, key),
            AppMode::QuickJump => handle_quick_jump_mode(app, key),
            AppMode::RecentFiles => handle_recent_files_mode(app, key),
            AppMode::VaultSwitcher => handle_vault_switcher_mode(app, key),
        AppMode::TagBrowser => handle_tag_browser_mode(app, key),
        AppMode::TagInput => handle_tag_input_mode(app, key),
        AppMode::ThemeBrowser => handle_theme_browser_mode(app, key),
        AppMode::Rename => handle_rename_mode(app, key),
        AppMode::NoteSearch => handle_note_search_mode(app, key),
        AppMode::Backlinks => handle_backlinks_mode(app, key),
        AppMode::Visual => handle_visual_mode(app, key),
        AppMode::TemplatePicker => handle_template_picker_mode(app, key),
        AppMode::SpellSuggest => handle_spell_suggest_mode(app, key),
        AppMode::Outline => handle_outline_mode(app, key),
        }
    }
    Ok(())
}

/// Handle bracketed paste events (text pasted from system clipboard).
pub fn handle_paste(app: &mut App, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    match app.mode {
        AppMode::Insert => {
            // Insert pasted text at cursor position
            app.push_undo_snapshot();
            let newline_count = text.chars().filter(|&c| c == '\n').count();
            let cursor_index = get_cursor_byte_index(app);
            app.editor_content.insert_str(cursor_index, text);

            // Move cursor to end of pasted text
            if newline_count > 0 {
                app.editor_cursor.0 += newline_count as u16;
                // Set column to length of last line of pasted text
                let last_line_len = text.rsplit('\n').next().map(|l| l.len()).unwrap_or(0);
                app.editor_cursor.1 = last_line_len as u16;
            } else {
                app.editor_cursor.1 += text.len() as u16;
            }

            app.adjust_scroll_to_cursor();
            app.mark_modified();
            app.update_preview_content();
        }
        AppMode::Search | AppMode::SearchAdvanced | AppMode::SearchReplace
        | AppMode::Command | AppMode::InputNote | AppMode::InputFolder
        | AppMode::Rename | AppMode::NoteSearch | AppMode::QuickJump => {
            // Paste into the input buffer for text-input modes
            app.input_buffer.push_str(text);
        }
        _ => {
            // In Normal mode and others, show a hint
            app.set_message("Press 'i' to enter Insert mode before pasting".to_string());
        }
    }
    Ok(())
}

/// Where a binding applies. Keys that only mean something while a note is open
/// and the editor pane has focus are `Editor`; everything else is `Any`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Any,
    Editor,
    /// The landing page: no note is open, so the digits are free to act as
    /// shortcuts into the recent list instead of meaning anything in an editor.
    Welcome,
}

/// What a key does, named independently of which key triggers it.
///
/// Splitting the name from the binding is the point of this table: several keys
/// can share one action (`j` and `Down`), the help screen can be generated from
/// the same data the dispatcher uses, and bindings become loadable from config
/// later without touching any of the behaviour below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    // Navigation
    CursorDown,
    CursorUp,
    GoToTop,
    GoToBottom,
    CursorLeft,
    CursorRight,
    CyclePane,
    ActivateSelected,
    // Editor motions
    AppendAtLineEnd,
    OpenLineBelow,
    OpenLineAbove,
    CursorLineStart,
    CursorLineEnd,
    WordForward,
    WordBackward,
    DeleteCharAtCursor,
    ToggleTaskCheckbox,
    PasteBelow,
    PasteClipboardBelow,
    YankLineSequence,
    DeleteLineOrConfirm,
    EnterInsert,
    EnterVisual,
    UndoText,
    RedoText,
    UndoLastDelete,
    PendingSpellPrefix,
    SpellSuggestions,
    // Items
    NewNoteOrSearchNext,
    NewFolderAtRoot,
    NewSubfolder,
    MoveItem,
    RenameItem,
    SaveNote,
    ExternalEditor,
    TagInput,
    // Panels and search
    QuickJump,
    ShowOutline,
    Backlinks,
    RecentFiles,
    VaultSwitcher,
    TagBrowser,
    ThemeBrowser,
    DailyNote,
    TogglePreview,
    FollowLink,
    SearchInNoteOrGlobal,
    SearchPrevOrTemplates,
    FuzzySearch,
    AdvancedSearch,
    SearchReplace,
    ClearNoteSearch,
    // Scrolling
    ScrollHalfUp,
    ScrollHalfDown,
    ScrollPageUp,
    ScrollPageDown,
    /// Open the Nth entry of the landing page's recent list (1-based).
    OpenRecent(u8),
    // Meta
    CommandMode,
    Help,
    Quit,
}

struct Binding {
    code: KeyCode,
    mods: KeyModifiers,
    ctx: Ctx,
    action: Action,
    /// Human-readable label for this binding.
    ///
    /// Not rendered yet: `draw_help_dialog` is still a hand-curated two-column
    /// layout covering modes this table does not describe, and flattening it into
    /// a generated list would be a downgrade. The text lives here so that when the
    /// help screen is generated, the binding and its description cannot drift
    /// apart. Asserted non-empty by `every_binding_is_described`.
    #[allow(dead_code)]
    desc: &'static str,
}

const fn k(code: KeyCode, ctx: Ctx, action: Action, desc: &'static str) -> Binding {
    Binding { code, mods: KeyModifiers::NONE, ctx, action, desc }
}

const fn ctrl(code: KeyCode, ctx: Ctx, action: Action, desc: &'static str) -> Binding {
    Binding { code, mods: KeyModifiers::CONTROL, ctx, action, desc }
}

/// Every Normal-mode binding, as data.
///
/// Order carries no meaning: lookup matches exactly on code + modifiers, and
/// prefers an `Editor` entry only when the editor actually has focus. That is
/// what stops a plain-letter motion from swallowing the Ctrl chord that shares
/// its letter -- which is exactly what used to happen to Ctrl+B, Ctrl+O, Ctrl+P,
/// Ctrl+L and Ctrl+Y whenever the editor was focused.
const NORMAL_BINDINGS: &[Binding] = &[
    // --- navigation (pane-aware inside the action) ---
    k(KeyCode::Char('j'), Ctx::Any, Action::CursorDown, "Move down"),
    k(KeyCode::Down, Ctx::Any, Action::CursorDown, "Move down"),
    k(KeyCode::Char('k'), Ctx::Any, Action::CursorUp, "Move up"),
    k(KeyCode::Up, Ctx::Any, Action::CursorUp, "Move up"),
    k(KeyCode::Char('g'), Ctx::Any, Action::GoToTop, "Go to top"),
    k(KeyCode::Char('G'), Ctx::Any, Action::GoToBottom, "Go to bottom"),
    k(KeyCode::Char('h'), Ctx::Editor, Action::CursorLeft, "Move cursor left"),
    k(KeyCode::Left, Ctx::Editor, Action::CursorLeft, "Move cursor left"),
    k(KeyCode::Char('l'), Ctx::Editor, Action::CursorRight, "Move cursor right"),
    k(KeyCode::Right, Ctx::Editor, Action::CursorRight, "Move cursor right"),
    k(KeyCode::Tab, Ctx::Any, Action::CyclePane, "Switch pane"),
    k(KeyCode::Enter, Ctx::Any, Action::ActivateSelected, "Open note / expand folder"),

    // --- editor motions and edits ---
    k(KeyCode::Char('A'), Ctx::Editor, Action::AppendAtLineEnd, "Append at end of line"),
    k(KeyCode::Char('o'), Ctx::Editor, Action::OpenLineBelow, "Open line below"),
    k(KeyCode::Char('O'), Ctx::Editor, Action::OpenLineAbove, "Open line above"),
    k(KeyCode::Char('0'), Ctx::Editor, Action::CursorLineStart, "Start of line"),
    k(KeyCode::Char('$'), Ctx::Editor, Action::CursorLineEnd, "End of line"),
    k(KeyCode::Char('w'), Ctx::Editor, Action::WordForward, "Word forward"),
    k(KeyCode::Char('b'), Ctx::Editor, Action::WordBackward, "Word backward"),
    k(KeyCode::Char('x'), Ctx::Editor, Action::DeleteCharAtCursor, "Delete character"),
    k(KeyCode::Char(' '), Ctx::Editor, Action::ToggleTaskCheckbox, "Toggle task checkbox"),
    k(KeyCode::Char('p'), Ctx::Editor, Action::PasteBelow, "Paste line below"),
    k(KeyCode::Char('P'), Ctx::Editor, Action::PasteClipboardBelow, "Paste clipboard below"),
    k(KeyCode::Char('y'), Ctx::Editor, Action::YankLineSequence, "yy: yank line"),
    k(KeyCode::Char('v'), Ctx::Editor, Action::EnterVisual, "Visual selection"),
    k(KeyCode::Char('z'), Ctx::Editor, Action::PendingSpellPrefix, "z=: spelling prefix"),
    k(KeyCode::Char('='), Ctx::Editor, Action::SpellSuggestions, "z=: spelling suggestions"),
    ctrl(KeyCode::Char('z'), Ctx::Editor, Action::UndoText, "Undo edit"),
    ctrl(KeyCode::Char('y'), Ctx::Editor, Action::RedoText, "Redo edit"),
    k(KeyCode::Char('d'), Ctx::Any, Action::DeleteLineOrConfirm, "dd: delete line / delete item"),
    k(KeyCode::Char('i'), Ctx::Any, Action::EnterInsert, "Insert mode"),
    k(KeyCode::Char('u'), Ctx::Any, Action::UndoLastDelete, "Undo last delete"),

    // --- items ---
    k(KeyCode::Char('n'), Ctx::Any, Action::NewNoteOrSearchNext, "New note / next match"),
    k(KeyCode::Char('f'), Ctx::Any, Action::NewFolderAtRoot, "New folder at root"),
    k(KeyCode::Char('F'), Ctx::Any, Action::NewSubfolder, "New subfolder"),
    k(KeyCode::Char('m'), Ctx::Any, Action::MoveItem, "Move item"),
    k(KeyCode::Char('r'), Ctx::Any, Action::RenameItem, "Rename item"),
    k(KeyCode::Char('e'), Ctx::Any, Action::ExternalEditor, "Open in external editor"),
    k(KeyCode::Char('t'), Ctx::Any, Action::TagInput, "Edit tags"),
    ctrl(KeyCode::Char('s'), Ctx::Any, Action::SaveNote, "Save note"),

    // --- panels ---
    ctrl(KeyCode::Char('j'), Ctx::Any, Action::QuickJump, "Quick jump to note"),
    ctrl(KeyCode::Char('g'), Ctx::Any, Action::ShowOutline, "Outline panel"),
    ctrl(KeyCode::Char('b'), Ctx::Any, Action::Backlinks, "Backlinks panel"),
    ctrl(KeyCode::Char('o'), Ctx::Any, Action::RecentFiles, "Recent files"),
    ctrl(KeyCode::Char('v'), Ctx::Any, Action::VaultSwitcher, "Switch vault"),
    ctrl(KeyCode::Char('t'), Ctx::Any, Action::TagBrowser, "Tag browser"),
    ctrl(KeyCode::Char('l'), Ctx::Any, Action::FollowLink, "Follow link at cursor"),
    ctrl(KeyCode::Char('p'), Ctx::Any, Action::TogglePreview, "Toggle preview"),
    k(KeyCode::F(2), Ctx::Any, Action::TogglePreview, "Toggle preview"),
    k(KeyCode::F(3), Ctx::Any, Action::ThemeBrowser, "Theme browser"),
    k(KeyCode::F(4), Ctx::Any, Action::DailyNote, "Today's daily note"),

    // --- search ---
    k(KeyCode::Char('/'), Ctx::Any, Action::SearchInNoteOrGlobal, "Search in note / all notes"),
    k(KeyCode::Char('N'), Ctx::Any, Action::SearchPrevOrTemplates, "Previous match / templates"),
    k(KeyCode::Esc, Ctx::Any, Action::ClearNoteSearch, "Clear search highlights"),
    ctrl(KeyCode::Char('f'), Ctx::Any, Action::FuzzySearch, "Fuzzy search"),
    ctrl(KeyCode::Char('a'), Ctx::Any, Action::AdvancedSearch, "Advanced search"),
    ctrl(KeyCode::Char('r'), Ctx::Any, Action::SearchReplace, "Search and replace"),

    // --- scrolling ---
    ctrl(KeyCode::Char('u'), Ctx::Any, Action::ScrollHalfUp, "Half page up"),
    ctrl(KeyCode::Char('d'), Ctx::Any, Action::ScrollHalfDown, "Half page down"),
    k(KeyCode::PageUp, Ctx::Any, Action::ScrollPageUp, "Page up"),
    k(KeyCode::PageDown, Ctx::Any, Action::ScrollPageDown, "Page down"),

    // --- landing page: digits jump straight into the recent list ---
    k(KeyCode::Char('1'), Ctx::Welcome, Action::OpenRecent(1), "Open 1st recent note"),
    k(KeyCode::Char('2'), Ctx::Welcome, Action::OpenRecent(2), "Open 2nd recent note"),
    k(KeyCode::Char('3'), Ctx::Welcome, Action::OpenRecent(3), "Open 3rd recent note"),
    k(KeyCode::Char('4'), Ctx::Welcome, Action::OpenRecent(4), "Open 4th recent note"),
    k(KeyCode::Char('5'), Ctx::Welcome, Action::OpenRecent(5), "Open 5th recent note"),

    // --- meta ---
    k(KeyCode::Char(':'), Ctx::Any, Action::CommandMode, "Command"),
    k(KeyCode::Char('?'), Ctx::Any, Action::Help, "Help"),
    k(KeyCode::Char('q'), Ctx::Any, Action::Quit, "Quit"),
];

/// SHIFT is already baked into the character an uppercase key produces, and
/// terminals disagree about whether they also report the modifier, so it must
/// not take part in matching.
fn effective_mods(key: &KeyEvent) -> KeyModifiers {
    key.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT)
}

/// Exact match on code + modifiers. Editor bindings are consulted first, and only
/// while the editor is focused; everything else falls through to the global set.
fn lookup_exact(
    code: KeyCode,
    mods: KeyModifiers,
    editor_focused: bool,
    on_welcome: bool,
) -> Option<Action> {
    let matches = |b: &&Binding, ctx: Ctx| b.ctx == ctx && b.code == code && b.mods == mods;

    if on_welcome {
        if let Some(b) = NORMAL_BINDINGS.iter().find(|b| matches(b, Ctx::Welcome)) {
            return Some(b.action);
        }
    }
    if editor_focused {
        if let Some(b) = NORMAL_BINDINGS.iter().find(|b| matches(b, Ctx::Editor)) {
            return Some(b.action);
        }
    }
    NORMAL_BINDINGS
        .iter()
        .find(|b| matches(b, Ctx::Any))
        .map(|b| b.action)
}

/// Resolve a key to an action.
fn lookup(key: &KeyEvent, editor_focused: bool, on_welcome: bool) -> Option<Action> {
    let mods = effective_mods(key);
    if let Some(action) = lookup_exact(key.code, mods, editor_focused, on_welcome) {
        return Some(action);
    }

    // Terminals deliver "Esc, then x" as Alt+x, and users press Esc before a
    // command all the time. Nothing binds Alt today, so an Alt-modified key with
    // no binding of its own falls back to the plain key -- otherwise Esc followed
    // by `q` would silently do nothing, which is what the old unguarded match arms
    // (which ignored modifiers entirely) happened to get right. An explicit Alt
    // binding added later still wins, because the exact pass runs first.
    if mods.contains(KeyModifiers::ALT) {
        return lookup_exact(key.code, mods & !KeyModifiers::ALT, editor_focused, on_welcome);
    }
    None
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    // Clear pending vim key if current key doesn't continue the sequence
    let continues_sequence = matches!(
        (app.pending_key, &key.code),
        (Some('d'), KeyCode::Char('d')) | (Some('y'), KeyCode::Char('y')) | (Some('z'), KeyCode::Char('='))
    );
    if app.pending_key.is_some() && !continues_sequence {
        app.pending_key = None;
    }

    let editor_focused = app.focused_pane == FocusedPane::Editor && app.current_note.is_some();
    // The landing page shows exactly when no note is open.
    let on_welcome = app.current_note.is_none();

    if let Some(action) = lookup(&key, editor_focused, on_welcome) {
        run_action(app, action, editor_focused);
    }
}

fn run_action(app: &mut App, action: Action, editor_focused: bool) {
    let preview_focused = app.focused_pane == FocusedPane::Preview;

    match action {
        // --- navigation ---
        Action::CursorDown => {
            if editor_focused {
                app.cursor_down_normal();
            } else if preview_focused {
                app.preview_scroll_down();
            } else {
                app.navigate_down();
            }
        }
        Action::CursorUp => {
            if editor_focused {
                app.cursor_up_normal();
            } else if preview_focused {
                app.preview_scroll_up();
            } else {
                app.navigate_up();
            }
        }
        Action::GoToTop => {
            if editor_focused {
                app.editor_cursor = (0, 0);
                app.scroll_to_top();
            } else if preview_focused {
                app.preview_scroll = 0;
            } else {
                app.navigate_to_top();
            }
        }
        Action::GoToBottom => {
            if editor_focused {
                let line_count = app.editor_content.lines().count() as u16;
                app.editor_cursor.0 = line_count.saturating_sub(1);
                app.editor_cursor.1 = app.editor_content.lines()
                    .last().map(|l| l.len() as u16).unwrap_or(0);
                app.scroll_to_bottom();
            } else if preview_focused {
                app.preview_scroll_to_bottom();
            } else {
                app.navigate_to_bottom();
            }
        }
        Action::CursorLeft => {
            if app.editor_cursor.1 > 0 { app.editor_cursor.1 -= 1; }
        }
        Action::CursorRight => {
            let line_len = app.editor_content.lines()
                .nth(app.editor_cursor.0 as usize)
                .map(|l| l.len() as u16).unwrap_or(0);
            if app.editor_cursor.1 < line_len { app.editor_cursor.1 += 1; }
        }
        Action::CyclePane => {
            app.focused_pane = if app.preview_enabled {
                match app.focused_pane {
                    FocusedPane::Folders => FocusedPane::Editor,
                    FocusedPane::Editor => FocusedPane::Preview,
                    FocusedPane::Preview => FocusedPane::Folders,
                }
            } else {
                match app.focused_pane {
                    FocusedPane::Folders => FocusedPane::Editor,
                    FocusedPane::Editor => FocusedPane::Folders,
                    FocusedPane::Preview => FocusedPane::Editor, // Fallback if preview gets disabled
                }
            };
        }
        Action::ActivateSelected => {
            if let Some(item) = app.get_selected_item() {
                match item.item_type {
                    TreeItemType::Note => {
                        app.select_note(item.id);
                    }
                    TreeItemType::Folder => {
                        app.toggle_folder_expansion();
                    }
                }
            }
        }

        // --- editor motions and edits ---
        Action::AppendAtLineEnd => {
            app.push_undo_snapshot();
            app.cursor_to_line_end();
            app.mode = AppMode::Insert;
        }
        Action::OpenLineBelow => {
            app.push_undo_snapshot();
            app.open_line_below();
            app.mode = AppMode::Insert;
        }
        Action::OpenLineAbove => {
            app.push_undo_snapshot();
            app.open_line_above();
            app.mode = AppMode::Insert;
        }
        Action::CursorLineStart => app.cursor_to_line_start(),
        Action::CursorLineEnd => app.cursor_to_line_end(),
        Action::WordForward => app.cursor_word_forward(),
        Action::WordBackward => app.cursor_word_backward(),
        Action::DeleteCharAtCursor => {
            app.push_undo_snapshot();
            app.delete_char_at_cursor();
            app.mark_modified();
        }
        Action::ToggleTaskCheckbox => app.toggle_task_checkbox(),
        Action::PasteBelow => {
            app.push_undo_snapshot();
            app.paste_below();
            app.mark_modified();
        }
        Action::PasteClipboardBelow => app.paste_clipboard_below(),
        Action::YankLineSequence => {
            if app.pending_key == Some('y') {
                app.pending_key = None;
                app.yank_current_line();
                app.set_message("Line yanked".to_string());
            } else {
                app.pending_key = Some('y');
            }
        }
        Action::DeleteLineOrConfirm => {
            if editor_focused {
                if app.pending_key == Some('d') {
                    app.pending_key = None;
                    app.push_undo_snapshot();
                    app.delete_current_line();
                    app.mark_modified();
                } else {
                    app.pending_key = Some('d');
                }
            } else if let Err(e) = app.start_delete_confirmation() {
                app.set_message(e);
            }
        }
        Action::EnterInsert => {
            if app.current_note.is_some() {
                app.push_undo_snapshot();
                app.mode = AppMode::Insert;
                app.focused_pane = FocusedPane::Editor;
            } else {
                app.set_message("No note selected".to_string());
            }
        }
        Action::EnterVisual => app.enter_visual_mode(),
        Action::UndoText => {
            if app.undo_text() {
                app.set_operation_info("Undo".to_string(), Some("↩".to_string()));
            } else {
                app.set_message("Nothing to undo".to_string());
            }
        }
        Action::RedoText => {
            if app.redo_text() {
                app.set_operation_info("Redo".to_string(), Some("↪".to_string()));
            } else {
                app.set_message("Nothing to redo".to_string());
            }
        }
        Action::UndoLastDelete => {
            if let Err(e) = app.undo_last_delete() {
                app.set_message(e);
            }
        }
        Action::PendingSpellPrefix => {
            app.pending_key = Some('z');
        }
        Action::SpellSuggestions => {
            // Only the tail of the `z=` sequence does anything; a bare `=` is inert.
            if app.pending_key == Some('z') {
                app.pending_key = None;
                app.show_spell_suggestions();
            }
        }

        // --- items ---
        Action::NewNoteOrSearchNext => {
            if editor_focused && app.note_search.active {
                app.note_search_next();
            } else {
                let folder_id = if let Some(item) = app.get_selected_item() {
                    match item.item_type {
                        TreeItemType::Folder => Some(item.id),
                        TreeItemType::Note => app.notebook.notes
                            .get(&item.id)
                            .and_then(|n| n.folder_id),
                    }
                } else {
                    None
                };
                app.start_new_note_input(folder_id);
            }
        }
        Action::NewFolderAtRoot => {
            // Default behavior: create folder at root level
            // Use Shift+F to create subfolder in selected folder
            app.start_new_folder_input(None);
        }
        Action::NewSubfolder => {
            let parent_id = if let Some(item) = app.get_selected_item() {
                match item.item_type {
                    TreeItemType::Folder => Some(item.id),
                    TreeItemType::Note => {
                        // Find the parent folder of the selected note
                        app.notebook.notes.get(&item.id).and_then(|note| note.folder_id)
                    }
                }
            } else {
                None
            };
            app.start_new_folder_input(parent_id);
        }
        Action::MoveItem => app.start_move_item(),
        Action::RenameItem => app.start_rename_item(),
        Action::SaveNote => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        Action::ExternalEditor => {
            if let Err(e) = app.open_in_external_editor() {
                app.set_message(e);
            }
        }
        Action::TagInput => app.start_tag_input(),

        // --- panels ---
        Action::QuickJump => app.start_quick_jump(),
        Action::ShowOutline => app.show_outline(),
        Action::Backlinks => app.show_backlinks_panel(),
        Action::RecentFiles => app.toggle_recent_files(),
        Action::VaultSwitcher => app.show_vault_switcher(),
        Action::TagBrowser => app.show_tag_browser(),
        Action::ThemeBrowser => app.show_theme_browser(),
        Action::DailyNote => app.open_daily_note(),
        Action::TogglePreview => app.toggle_preview(),
        Action::FollowLink => {
            if let Err(e) = app.follow_link_at_cursor() {
                app.set_message(e);
            }
        }

        // --- search ---
        Action::SearchInNoteOrGlobal => {
            if editor_focused {
                app.note_search.query.clear();
                app.note_search.matches.clear();
                app.note_search.active = true;
                app.mode = AppMode::NoteSearch;
            } else {
                app.mode = AppMode::Search;
                app.is_fuzzy_search = false;
                app.input_buffer.clear();
                app.search_dialog_note_ids.clear();
                app.search_dialog_selected = 0;
            }
        }
        Action::SearchPrevOrTemplates => {
            if editor_focused && app.note_search.active {
                app.note_search_prev();
            } else if !editor_focused {
                app.show_template_picker();
            }
        }
        Action::FuzzySearch => {
            app.mode = AppMode::Search;
            app.is_fuzzy_search = true;
            app.input_buffer.clear();
            app.search_dialog_note_ids.clear();
            app.search_dialog_selected = 0;
        }
        Action::AdvancedSearch => app.start_advanced_search(),
        Action::SearchReplace => {
            if app.current_note.is_some() {
                app.mode = AppMode::SearchReplace;
                app.input_buffer.clear();
            } else {
                app.set_message("No note selected for replace".to_string());
            }
        }
        Action::ClearNoteSearch => {
            // Inert unless an in-note search is actually highlighted.
            if app.note_search.active {
                app.clear_note_search();
            }
        }

        // --- scrolling ---
        Action::ScrollHalfUp => {
            if (editor_focused || preview_focused) && app.current_note.is_some() {
                app.scroll_half_page_up();
            }
        }
        Action::ScrollHalfDown => {
            if (editor_focused || preview_focused) && app.current_note.is_some() {
                app.scroll_half_page_down();
            }
        }
        Action::ScrollPageUp => {
            if (editor_focused || preview_focused) && app.current_note.is_some() {
                app.scroll_page_up();
            }
        }
        Action::ScrollPageDown => {
            if (editor_focused || preview_focused) && app.current_note.is_some() {
                app.scroll_page_down();
            }
        }

        Action::OpenRecent(n) => {
            if let Some(entry) = app.dashboard().recent.get(n as usize - 1) {
                let id = entry.id;
                app.select_note(id);
                app.focused_pane = FocusedPane::Editor;
            }
        }

        // --- meta ---
        Action::CommandMode => {
            app.mode = AppMode::Command;
            app.command_buffer.clear();
        }
        Action::Help => {
            app.mode = AppMode::Help;
            app.reset_help_scroll();
        }
        Action::Quit => app.quit(),
    }
}

fn handle_insert_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if app.autocomplete_state.active {
                app.cancel_autocomplete();
            } else {
                // Vim behaviour: cursor moves back one column when leaving Insert mode
                if app.editor_cursor.1 > 0 {
                    app.editor_cursor.1 -= 1;
                }
                app.mode = AppMode::Normal;
                // Auto-save on exit insert mode
                if let Err(e) = app.save_current_note() {
                    app.set_message(e);
                }
                // Parse links when exiting insert mode
                app.parse_current_note_links();
                // Refresh spell errors after editing
                app.run_spell_check();
            }
        }
        
        KeyCode::Tab => {
            if app.autocomplete_state.active {
                app.apply_autocomplete();
            } else {
                insert_str_at_cursor(app, "    "); // 4 spaces
                app.mark_modified();
                app.update_autocompletion();
            }
        }
        
        KeyCode::Up => {
            if app.autocomplete_state.active {
                app.previous_autocomplete_suggestion();
            } else {
                if app.editor_cursor.0 > 0 {
                    app.editor_cursor.0 -= 1;
                    // Ensure cursor column doesn't exceed the new line length
                    let lines: Vec<&str> = app.editor_content.lines().collect();
                    if let Some(current_line) = lines.get(app.editor_cursor.0 as usize) {
                        if app.editor_cursor.1 > current_line.len() as u16 {
                            app.editor_cursor.1 = current_line.len() as u16;
                        }
                    }
                    app.adjust_scroll_to_cursor();
                }
            }
        }
        
        KeyCode::Down => {
            if app.autocomplete_state.active {
                app.next_autocomplete_suggestion();
            } else {
                let lines: Vec<&str> = app.editor_content.lines().collect();
                if app.editor_cursor.0 < (lines.len() as u16).saturating_sub(1) {
                    app.editor_cursor.0 += 1;
                    // Ensure cursor column doesn't exceed the new line length
                    if let Some(current_line) = lines.get(app.editor_cursor.0 as usize) {
                        if app.editor_cursor.1 > current_line.len() as u16 {
                            app.editor_cursor.1 = current_line.len() as u16;
                        }
                    }
                    app.adjust_scroll_to_cursor();
                }
            }
        }
        
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    's' => {
                        if let Err(e) = app.save_current_note() {
                            app.set_message(e);
                        }
                    }
                    'p' => {
                        app.toggle_preview();
                    }
                    'u' => {
                        app.scroll_half_page_up();
                    }
                    'd' => {
                        app.scroll_half_page_down();
                    }
                    'l' => {
                        // Follow link at cursor (also works in insert mode)
                        if let Err(e) = app.follow_link_at_cursor() {
                            app.set_message(e);
                        }
                    }
                    'v' => {
                        // Ctrl+V paste from system clipboard in insert mode
                        let idx = get_cursor_byte_index(app);
                        app.paste_clipboard_at_cursor(idx);
                    }
                    'z' => {
                        // Ctrl+Z undo in insert mode
                        if app.undo_text() {
                            app.set_operation_info("Undo".to_string(), Some("↩".to_string()));
                        } else {
                            app.set_message("Nothing to undo".to_string());
                        }
                    }
                    'y' => {
                        // Ctrl+Y redo in insert mode
                        if app.redo_text() {
                            app.set_operation_info("Redo".to_string(), Some("↪".to_string()));
                        } else {
                            app.set_message("Nothing to redo".to_string());
                        }
                    }
                    _ => {}
                }
            } else {
                // Push snapshot at word boundaries for granular undo
                if c == ' ' {
                    app.push_undo_snapshot();
                }
                insert_char_at_cursor(app, c);
                app.mark_modified();
                app.update_autocompletion();
                app.update_preview_content(); // Update preview as we type
            }
        }

        KeyCode::Enter => {
            if app.autocomplete_state.active {
                app.apply_autocomplete();
            } else {
                app.push_undo_snapshot(); // word boundary
                insert_char_at_cursor(app, '\n');
                app.mark_modified();
                app.editor_cursor.0 += 1;
                app.editor_cursor.1 = 0;
                app.adjust_scroll_to_cursor();
                app.update_autocompletion();
                app.update_preview_content();
            }
        }
        
        KeyCode::Backspace => {
            delete_char_before_cursor(app);
            app.mark_modified();
            app.update_autocompletion();
            app.update_preview_content();
        }
        
        // Page Up/Down scrolling in insert mode
        KeyCode::PageUp => {
            app.scroll_page_up();
        }
        KeyCode::PageDown => {
            app.scroll_page_down();
        }
        
        KeyCode::Left => {
            if app.editor_cursor.1 > 0 {
                app.editor_cursor.1 -= 1;
                app.update_autocompletion();
            }
        }
        
        KeyCode::Right => {
            let current_line = app.editor_content
                .lines()
                .nth(app.editor_cursor.0 as usize)
                .unwrap_or("");
            if (app.editor_cursor.1 as usize) < current_line.len() {
                app.editor_cursor.1 += 1;
                app.update_autocompletion();
            }
        }
        
        // Function keys work in insert mode too
        KeyCode::F(2) => {
            app.toggle_preview();
        }
        
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
            app.search_dialog_note_ids.clear();
            app.search_dialog_selected = 0;
            app.is_fuzzy_search = false;
        }

        KeyCode::Enter => {
            // Open the currently selected live result, or fall back to full search
            if let Some(&note_id) = app.search_dialog_note_ids.get(app.search_dialog_selected) {
                app.open_note_by_id(note_id);
            } else if !app.input_buffer.is_empty() {
                if app.is_fuzzy_search {
                    app.fuzzy_search_notes(app.input_buffer.clone());
                } else {
                    app.search_notes(app.input_buffer.clone());
                }
            }
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
            app.search_dialog_note_ids.clear();
            app.search_dialog_selected = 0;
            app.is_fuzzy_search = false;
        }

        // Navigate through live results
        KeyCode::Up => {
            if app.search_dialog_selected > 0 {
                app.search_dialog_selected -= 1;
            }
        }
        KeyCode::Down => {
            if !app.search_dialog_note_ids.is_empty() {
                app.search_dialog_selected = (app.search_dialog_selected + 1)
                    .min(app.search_dialog_note_ids.len().saturating_sub(1));
            }
        }

        // Tab toggles fuzzy / regular and re-runs current query
        KeyCode::Tab => {
            app.is_fuzzy_search = !app.is_fuzzy_search;
            run_live_search(app);
        }

        KeyCode::Char(c) => {
            app.input_buffer.push(c);
            run_live_search(app);
        }

        KeyCode::Backspace => {
            app.input_buffer.pop();
            run_live_search(app);
        }

        _ => {}
    }
}

/// Run a live search against the notebook and store results in search_dialog_note_ids.
fn run_live_search(app: &mut App) {
    let query = app.input_buffer.clone();
    if query.is_empty() {
        app.search_dialog_note_ids.clear();
        app.search_dialog_selected = 0;
        return;
    }
    app.search_dialog_selected = 0;
    if app.is_fuzzy_search {
        let results = app.notebook.fuzzy_search_notes(&query);
        app.search_dialog_note_ids = results.iter().map(|(n, _)| n.id).collect();
    } else {
        let search_query = crate::search::SearchQuery::new(query);
        match app.enhanced_search.search(&app.notebook, search_query) {
            Ok(results) => {
                app.enhanced_search_results = results;
                app.search_dialog_note_ids = app.enhanced_search_results
                    .iter().map(|r| r.note.id).collect();
            }
            Err(_) => app.search_dialog_note_ids.clear(),
        }
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.command_buffer.clear();
        }
        
        KeyCode::Enter => {
            let old_mode = app.mode.clone();
            execute_command(app, &app.command_buffer.clone());
            
            // Only reset to Normal if the command didn't change the mode to a special state
            if matches!(old_mode, AppMode::Command) && matches!(app.mode, AppMode::Command) {
                app.mode = AppMode::Normal;
            }
            
            app.command_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.command_buffer.pop();
        }
        
        _ => {}
    }
}

fn execute_command(app: &mut App, command: &str) {
    let command = command.trim();
    match command {
        "q" | "quit" => app.quit(),
        "w" | "write" => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        "wq" => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            } else {
                app.quit();
            }
        }
        "export" => {
            match app.export_all_notes() {
                Ok(count) => {
                    let storage = crate::storage::Storage::new().unwrap();
                    let export_path = storage.get_notes_dir();
                    app.set_operation_success(format!("Exported {} notes to {:?}", count, export_path), Some("📦".to_string()));
                },
                Err(e) => app.set_operation_error(format!("Export failed: {}", e), Some("🚨".to_string())),
            }
        },
        "backup" => {
            match app.create_backup() {
                Ok(_) => app.set_operation_success("Backup created successfully".to_string(), Some("💾".to_string())),
                Err(e) => app.set_operation_error(format!("Backup failed: {}", e), Some("🚨".to_string())),
            }
        },
        "list-backups" | "backups" => {
            match app.list_backups() {
                Ok(backups) => {
                    if backups.is_empty() {
                        app.set_message("No backups found".to_string());
                    } else {
                        app.set_message(format!("Found {} backup(s)", backups.len()));
                        for (i, backup) in backups.iter().enumerate().take(5) {
                            app.message_history.push_back(format!("{}: {}", i + 1, backup.display()));
                        }
                        if backups.len() > 5 {
                            app.message_history.push_back(format!("... and {} more", backups.len() - 5));
                        }
                    }
                },
                Err(e) => app.set_operation_error(format!("Failed to list backups: {}", e), Some("🚨".to_string())),
            }
        },
        "help" | "h" => {
            // Switch to help mode to show the comprehensive help dialog
            app.mode = AppMode::Help;
            app.reset_help_scroll();
            app.set_message("Showing comprehensive help - press Esc, q, or ? to close".to_string());
        },
        "vault" => {
            // Show vault switcher
            app.show_vault_switcher();
        },
        "daily" | "today" => {
            app.open_daily_note();
        },
        _ => {
            if command.starts_with("theme ") {
                let theme_arg = command.strip_prefix("theme ").unwrap_or("").trim();
                match theme_arg {
                    "list" => {
                        app.show_theme_browser();
                    },
                    "current" => {
                        app.set_message(format!("Current theme: {}", app.current_theme_name()));
                    },
                    "" => {
                        app.set_message("Usage: theme <name> | theme list | theme current".to_string());
                    },
                    theme_name => {
                        let available_themes = crate::app::App::get_available_themes();
                        if available_themes.contains(&theme_name) {
                            app.change_theme(theme_name);
                        } else {
                            app.set_operation_error(
                                format!("Unknown theme '{}'. Use 'theme list' to see available themes.", theme_name),
                                Some("🚨".to_string())
                            );
                        }
                    },
                }
            } else if command.starts_with("export html") {
                let path_arg = command.strip_prefix("export html").unwrap_or("").trim();
                let path = if path_arg.is_empty() { None } else { Some(path_arg) };
                match app.export_notes_to_html(path) {
                    Ok(count) => {
                        let dest = path.unwrap_or("~/Documents/scribble_export");
                        app.set_operation_success(
                            format!("Exported {} notes as HTML to '{}'", count, dest),
                            Some("🌐".to_string()),
                        );
                    }
                    Err(e) => app.set_operation_error(format!("HTML export failed: {}", e), Some("🚨".to_string())),
                }
            } else if command.starts_with("export ") {
                let path = command.strip_prefix("export ").unwrap_or("").trim();
                match app.export_notes_to_directory(path) {
                    Ok(count) => app.set_operation_success(format!("Exported {} notes to '{}'", count, path), Some("📦".to_string())),
                    Err(e) => app.set_operation_error(format!("Export failed: {}", e), Some("🚨".to_string())),
                }
            } else if command.starts_with("import ") {
                let path = command.strip_prefix("import ").unwrap_or("").trim();
                match app.import_notes_from_directory(path) {
                    Ok(result) => {
                        let summary = result.format_summary();
                        if result.has_issues() {
                            // Show warning for partial success
                            app.set_operation_error(summary, Some("⚠️".to_string()));
                            
                            // Add details to message history
                            for failure in &result.failed_imports {
                                app.message_history.push_back(format!("Failed to import {}: {}", failure.file_path, failure.error));
                            }
                            for skip in &result.skipped_duplicates {
                                app.message_history.push_back(format!("Skipped duplicate: {}", skip));
                            }
                        } else {
                            app.set_operation_success(summary, Some("📦".to_string()));
                        }
                        
                        // Show renamed files in message history
                        for (old_name, new_name) in &result.renamed_duplicates {
                            app.message_history.push_back(format!("Renamed '{}' to '{}' (duplicate)", old_name, new_name));
                        }
                    },
                    Err(e) => app.set_operation_error(format!("Import failed: {}", e), Some("🚨".to_string())),
                }
        } else if command == "spell" || command == "spellon" {
            if !app.spell.aspell_available {
                app.set_message("aspell not found — install it: sudo apt install aspell".to_string());
            } else {
                app.spell.enabled = true;
                app.run_spell_check();
                app.set_message(format!("Spell check ON — {} error(s)", app.spell.errors.len()));
            }
        } else if command == "nospell" || command == "spelloff" {
            app.spell.enabled = false;
            app.spell.errors.clear();
            app.set_message("Spell check OFF".to_string());
        } else if let Ok(line_num) = command.parse::<usize>() {
                // :N — jump to line number
                app.jump_to_line(line_num);
                app.set_message(format!("Jumped to line {}", line_num));
                app.focused_pane = crate::app::FocusedPane::Editor;
            } else {
                app.set_message(format!("Unknown command: {}", command));
            }
        }
    }
}

fn handle_input_note_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_input();
        }
        
        KeyCode::Enter => {
            app.finish_new_note_input();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

fn handle_input_folder_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_input();
        }
        
        KeyCode::Enter => {
            app.finish_new_folder_input();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}


fn get_cursor_byte_index(app: &App) -> usize {
    let lines: Vec<&str> = app.editor_content.lines().collect();
    let mut byte_index = 0;
    
    // Add up all the characters from previous lines
    for (i, line) in lines.iter().enumerate() {
        if i < app.editor_cursor.0 as usize {
            byte_index += line.len() + 1; // +1 for the newline character
        } else {
            break;
        }
    }
    
    // Add the column position within the current line
    if let Some(current_line) = lines.get(app.editor_cursor.0 as usize) {
        byte_index += (app.editor_cursor.1 as usize).min(current_line.len());
    }
    
    // Make sure we don't exceed the content length
    byte_index.min(app.editor_content.len())
}

fn insert_char_at_cursor(app: &mut App, c: char) {
    let cursor_index = get_cursor_byte_index(app);
    app.editor_content.insert(cursor_index, c);
    
    // Move cursor forward
    app.editor_cursor.1 += 1;
}

fn insert_str_at_cursor(app: &mut App, s: &str) {
    let cursor_index = get_cursor_byte_index(app);
    app.editor_content.insert_str(cursor_index, s);
    
    // Move cursor forward by the length of inserted string
    app.editor_cursor.1 += s.len() as u16;
}

fn delete_char_before_cursor(app: &mut App) {
    if app.editor_content.is_empty() {
        return;
    }
    
    let cursor_index = get_cursor_byte_index(app);
    if cursor_index > 0 {
        app.editor_content.remove(cursor_index - 1);
        
        // Move cursor back
        if app.editor_cursor.1 > 0 {
            app.editor_cursor.1 -= 1;
        } else if app.editor_cursor.0 > 0 {
            // We deleted a newline, move to end of previous line
            app.editor_cursor.0 -= 1;
            let lines: Vec<&str> = app.editor_content.lines().collect();
            if let Some(prev_line) = lines.get(app.editor_cursor.0 as usize) {
                app.editor_cursor.1 = prev_line.len() as u16;
            }
        }
    }
}

fn handle_advanced_search_mode(app: &mut App, key: KeyEvent) {
    use crate::search::SearchQuery;
    
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        
        KeyCode::Enter => {
            if !app.input_buffer.is_empty() {
                let mut query = SearchQuery::new(app.input_buffer.clone());
                let mut run_search = true;

                // Check for special modifiers in the query
                if app.input_buffer.starts_with("regex:") {
                    let pattern = app.input_buffer.strip_prefix("regex:").unwrap_or("").trim();
                    query = SearchQuery::new(pattern.to_string()).with_regex();
                } else if app.input_buffer.starts_with("case:") {
                    let pattern = app.input_buffer.strip_prefix("case:").unwrap_or("").trim();
                    query = SearchQuery::new(pattern.to_string()).case_sensitive();
                } else if app.input_buffer.starts_with("folder:") {
                    // `folder:Name term` — restrict to a folder by name; the rest is
                    // the search text (empty text lists every note in the folder).
                    let rest = app.input_buffer.strip_prefix("folder:").unwrap_or("").trim_start();
                    let (folder_name, term) = match rest.split_once(char::is_whitespace) {
                        Some((name, t)) => (name, t.trim()),
                        None => (rest, ""),
                    };
                    match app.notebook.find_folder_by_name(folder_name) {
                        Some(fid) => {
                            query = SearchQuery::new(term.to_string()).in_folder(Some(fid));
                        }
                        None => {
                            app.set_message(format!("No folder named '{}'", folder_name));
                            run_search = false;
                        }
                    }
                }

                if run_search {
                    app.enhanced_search_notes(query);
                }
            }
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        // Up/Down to navigate search history
        KeyCode::Up => {
            let history = app.get_search_history();
            if !history.is_empty() {
                app.input_buffer = history[0].clone();
            }
        }
        
        _ => {}
    }
}

fn handle_replace_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
            app.command_buffer.clear();
        }
        
        KeyCode::Enter => {
            // Parse input as "find|replace"
            if let Some(pos) = app.input_buffer.find('|') {
                let find = app.input_buffer[..pos].to_string();
                let replace = app.input_buffer[pos + 1..].to_string();
                
                if let Some(ref mut note) = app.current_note {
                    let is_regex = app.command_buffer.contains("regex");
                    let case_sensitive = app.command_buffer.contains("case");
                    
                    match app.enhanced_search.replace_in_note(note, &find, &replace, is_regex, case_sensitive) {
                        Ok(count) => {
                            if count > 0 {
                                app.editor_content = note.content.clone();
                                // Update the note in the notebook
                                app.notebook.notes.insert(note.id, note.clone());
                                app.set_message(format!("Replaced {} occurrences", count));
                            } else {
                                app.set_message("No matches found to replace".to_string());
                            }
                        }
                        Err(e) => {
                            app.set_message(format!("Replace error: {}", e));
                        }
                    }
                } else {
                    app.set_message("No note selected".to_string());
                }
            } else {
                app.set_message("Format: find_text|replace_text".to_string());
            }
            
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
            app.command_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'r' => app.command_buffer.push_str("regex "),
                    'c' => app.command_buffer.push_str("case "),
                    _ => {}
                }
            } else {
                app.input_buffer.push(c);
            }
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

fn handle_move_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_move();
        }
        
        // Navigation in move mode
        KeyCode::Char('j') | KeyCode::Down => app.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => app.navigate_up(),
        KeyCode::Char('g') => app.navigate_to_top(),
        KeyCode::Char('G') => app.navigate_to_bottom(),
        
        // Execute move
        KeyCode::Enter => {
            if let Err(e) = app.execute_move() {
                app.set_message(e);
                app.cancel_move();
            }
        }
        
        // Help in move mode
        KeyCode::Char('?') => {
            app.set_message("Move mode: j/k=navigate, Enter=move to selected location, Esc=cancel".to_string());
        }
        
        _ => {}
    }
}

fn handle_help_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        
        // Scrolling in help dialog
        KeyCode::Char('j') | KeyCode::Down => {
            app.help_scroll_down();
        }
        
        KeyCode::Char('k') | KeyCode::Up => {
            app.help_scroll_up();
        }
        
        KeyCode::PageDown | KeyCode::Char('d') => {
            for _ in 0..10 {
                app.help_scroll_down();
            }
        }
        
        KeyCode::PageUp | KeyCode::Char('u') => {
            for _ in 0..10 {
                app.help_scroll_up();
            }
        }
        
        KeyCode::Char('g') => {
            app.help_scroll = 0;
        }
        
        KeyCode::Char('G') => {
            app.help_scroll = 200;
        }
        
        _ => {}
    }
}

fn handle_delete_confirm_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        // Confirm deletion with 'y' or Enter
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Err(e) = app.confirm_delete() {
                app.set_message(e);
            }
        }
        
        // Cancel deletion with 'n', Esc, or any other key
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_delete();
        }
        
        // Any other key cancels the operation
        _ => {
            app.cancel_delete();
        }
    }
}

fn handle_quick_jump_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_quick_jump();
        }
        
        KeyCode::Enter => {
            app.quick_jump_select();
        }
        
        KeyCode::Up => {
            app.quick_jump_navigate_up();
        }
        
        KeyCode::Down => {
            app.quick_jump_navigate_down();
        }
        
        KeyCode::Char(c) => {
            app.quick_jump_query.push(c);
            app.update_quick_jump_results();
        }
        
        KeyCode::Backspace => {
            app.quick_jump_query.pop();
            app.update_quick_jump_results();
        }
        
        _ => {}
    }
}

fn handle_recent_files_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.show_recent_files = false;
            app.mode = AppMode::Normal;
        }
        
        KeyCode::Enter => {
            app.select_recent_file(app.recent_files_selected);
        }
        
        KeyCode::Up => {
            if app.recent_files_selected > 0 {
                app.recent_files_selected -= 1;
            }
        }
        
        KeyCode::Down => {
            let max_index = app.get_recent_files_display().len().saturating_sub(1);
            if app.recent_files_selected < max_index {
                app.recent_files_selected += 1;
            }
        }
        
        // Numbers for quick selection
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(digit) = c.to_digit(10) {
                let index = (digit as usize).saturating_sub(1);
                app.select_recent_file(index);
            }
        }
        
        _ => {}
    }
}

fn handle_vault_switcher_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_vault_switcher();
        }
        
        KeyCode::Enter => {
            // Note: Actual vault switching would need to be implemented
            // in the main loop since it requires reinitializing storage
            if let Some(vault) = app.get_selected_vault() {
                app.set_operation_info(
                    format!("Vault selected: {}\nRestart scribble with: scribble --vault {:?}", 
                        vault.file_name().unwrap_or_default().to_string_lossy(),
                        vault
                    ),
                    Some("📁".to_string())
                );
            }
            app.cancel_vault_switcher();
        }
        
        KeyCode::Up | KeyCode::Char('k') => {
            app.vault_switcher_navigate_up();
        }
        
        KeyCode::Down | KeyCode::Char('j') => {
            app.vault_switcher_navigate_down();
        }
        
        // Numbers for quick selection
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(digit) = c.to_digit(10) {
                let index = (digit as usize).saturating_sub(1);
                if index < app.available_vaults.len() {
                    app.vault_switcher_selected = index;
                    if let Some(vault) = app.get_selected_vault() {
                        app.set_operation_info(
                            format!("Vault selected: {}\nRestart scribble with: scribble --vault {:?}", 
                                vault.file_name().unwrap_or_default().to_string_lossy(),
                                vault
                            ),
                            Some("📁".to_string())
                        );
                    }
                    app.cancel_vault_switcher();
                }
            }
        }
        
        _ => {}
    }
}

fn handle_tag_input_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_tag_input(),
        KeyCode::Enter => app.submit_tag_input(),
        KeyCode::Tab => {
            // Autocomplete with the first matching suggestion.
            let suggestions = app.get_tag_suggestions(&app.input_buffer);
            if let Some(first) = suggestions.first() {
                app.input_buffer = first.clone();
            }
        }
        KeyCode::Backspace => {
            if app.input_buffer.is_empty() {
                app.remove_last_tag_from_current_note();
            } else {
                app.input_buffer.pop();
            }
        }
        KeyCode::Char(c) => app.input_buffer.push(c),
        _ => {}
    }
}

fn handle_tag_browser_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_tag_browser();
        }
        
        KeyCode::Enter => {
            app.add_tag_filter();
        }
        
        KeyCode::Up | KeyCode::Char('k') => {
            app.tag_browser_navigate_up();
        }
        
        KeyCode::Down | KeyCode::Char('j') => {
            app.tag_browser_navigate_down();
        }
        
        // Toggle sort mode (s)
        KeyCode::Char('s') => {
            app.toggle_tag_browser_sort();
        }
        
        // Clear all filters (c)
        KeyCode::Char('c') => {
            app.clear_tag_filters();
        }
        
        // Numbers for quick selection/filtering
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(digit) = c.to_digit(10) {
                let index = (digit as usize).saturating_sub(1);
                let tag_count = if app.tag_browser_sort_by_frequency {
                    app.tag_manager.get_tags_by_frequency().len()
                } else {
                    app.tag_manager.get_tags_alphabetical().len()
                };
                
                if index < tag_count {
                    app.tag_browser_selected = index;
                    app.add_tag_filter();
                }
            }
        }
        
        // Remove active filter (Backspace)
        KeyCode::Backspace => {
            if let Some(last_filter) = app.tag_filter_active.last().cloned() {
                app.remove_tag_filter(&last_filter);
            }
        }
        
        _ => {}
    }
}

fn handle_theme_browser_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.cancel_theme_browser();
        }
        
        KeyCode::Up | KeyCode::Char('k') => {
            app.navigate_theme_browser(-1);
        }
        
        KeyCode::Down | KeyCode::Char('j') => {
            app.navigate_theme_browser(1);
        }
        
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.select_theme_from_browser();
        }
        
        // Numbers for quick selection
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(digit) = c.to_digit(10) {
                let index = (digit as usize).saturating_sub(1);
                let themes = crate::app::App::get_available_themes();
                if index < themes.len() {
                    app.theme_browser_selected = index;
                    app.select_theme_from_browser();
                }
            }
        }
        
        _ => {}
    }
}

fn handle_note_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        // Esc: exit search mode, keep highlights for n/N navigation
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            // Leave note_search_active true so n/N still work
        }

        // Enter: jump to current match and exit input mode
        KeyCode::Enter => {
            if !app.note_search.matches.is_empty() {
                app.jump_to_selected_match_pub();
            }
            app.mode = AppMode::Normal;
        }

        // Up/Down navigate matches while still typing
        KeyCode::Up => {
            app.note_search_prev();
        }
        KeyCode::Down => {
            app.note_search_next();
        }

        KeyCode::Char(c) => {
            app.note_search.query.push(c);
            app.find_note_search_matches();
            // Auto-jump to first match
            if !app.note_search.matches.is_empty() {
                app.jump_to_selected_match_pub();
            }
        }

        KeyCode::Backspace => {
            app.note_search.query.pop();
            app.find_note_search_matches();
            if !app.note_search.matches.is_empty() {
                app.jump_to_selected_match_pub();
            }
        }

        _ => {}
    }
}

fn handle_outline_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.cancel_outline(),
        KeyCode::Enter => app.outline_select(),
        KeyCode::Up | KeyCode::Char('k') => app.outline_navigate_up(),
        KeyCode::Down | KeyCode::Char('j') => app.outline_navigate_down(),
        _ => {}
    }
}

fn handle_backlinks_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.cancel_backlinks();
        }

        KeyCode::Enter => {
            app.open_selected_backlink();
        }

        KeyCode::Tab | KeyCode::BackTab => {
            app.backlinks_toggle_focus();
        }

        KeyCode::Up | KeyCode::Char('k') => {
            app.backlinks_navigate_up();
        }

        KeyCode::Down | KeyCode::Char('j') => {
            app.backlinks_navigate_down();
        }

        _ => {}
    }
}

fn handle_visual_mode(app: &mut App, key: KeyEvent) {
    let editor_focused = app.focused_pane == crate::app::FocusedPane::Editor;
    match key.code {
        KeyCode::Esc | KeyCode::Char('v') => {
            app.mode = AppMode::Normal;
        }
        // Movement (reuses normal-mode motion methods)
        KeyCode::Char('j') | KeyCode::Down => { app.cursor_down_normal(); }
        KeyCode::Char('k') | KeyCode::Up   => { app.cursor_up_normal(); }
        KeyCode::Char('h') | KeyCode::Left => {
            if app.editor_cursor.1 > 0 { app.editor_cursor.1 -= 1; }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let len = app.editor_content.lines()
                .nth(app.editor_cursor.0 as usize).map(|l| l.len() as u16).unwrap_or(0);
            if app.editor_cursor.1 < len { app.editor_cursor.1 += 1; }
        }
        KeyCode::Char('0') => { app.cursor_to_line_start(); }
        KeyCode::Char('$') => { app.cursor_to_line_end(); }
        KeyCode::Char('w') => { app.cursor_word_forward(); }
        KeyCode::Char('b') => { app.cursor_word_backward(); }
        KeyCode::Char('g') => { app.editor_cursor = (0, 0); app.scroll_to_top(); }
        KeyCode::Char('G') => {
            let last = app.editor_content.lines().count().saturating_sub(1) as u16;
            app.editor_cursor.0 = last;
            app.scroll_to_bottom();
        }
        // Yank selection
        KeyCode::Char('y') => {
            app.yank_visual_selection();
        }
        // Delete selection
        KeyCode::Char('d') => {
            app.delete_visual_selection();
        }
        // Replace selection with typed text (enter insert mode after deleting)
        KeyCode::Char('c') => {
            app.delete_visual_selection();
            app.mode = AppMode::Insert;
        }
        _ => {}
    }
    let _ = editor_focused; // suppress unused warning
}

fn handle_template_picker_mode(app: &mut App, key: KeyEvent) {
    let count = crate::app::App::get_templates().len();
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.template_picker_selected > 0 { app.template_picker_selected -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.template_picker_selected + 1 < count { app.template_picker_selected += 1; }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let idx = app.template_picker_selected;
            app.apply_template(idx);
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(d) = c.to_digit(10) {
                let idx = (d as usize).saturating_sub(1);
                if idx < count {
                    app.apply_template(idx);
                }
            }
        }
        _ => {}
    }
}

fn handle_spell_suggest_mode(app: &mut App, key: KeyEvent) {
    let count = app.spell.suggestions.len();
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.spell.suggestions_selected > 0 {
                app.spell.suggestions_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.spell.suggestions_selected + 1 < count {
                app.spell.suggestions_selected += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.apply_spell_suggestion();
        }
        // Quick pick by digit
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(d) = c.to_digit(10) {
                let idx = (d as usize).saturating_sub(1);
                if idx < count {
                    app.spell.suggestions_selected = idx;
                    app.apply_spell_suggestion();
                }
            }
        }
        _ => {}
    }
}

fn handle_rename_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_rename();
        }
        
        KeyCode::Enter => {
            if let Err(e) = app.execute_rename() {
                app.set_message(e);
            }
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::models::Note;

    /// An app with a note open and the editor pane focused.
    fn editor_focused_app() -> App {
        let mut app = App::default();
        let mut note = Note::new("Test".to_string(), None);
        note.content = "alpha beta gamma".to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = "alpha beta gamma".to_string();
        app.editor_cursor = (0, 10);
        app.focused_pane = FocusedPane::Editor;
        app.mode = AppMode::Normal;
        app
    }

    fn press(app: &mut App, c: char, mods: KeyModifiers) {
        handle_normal_mode(app, KeyEvent::new(KeyCode::Char(c), mods));
    }

    /// Global Ctrl-shortcuts must keep working while the editor pane is focused.
    /// They share a letter with an editor motion, and the motion arms do not check
    /// modifiers, so a first-match-wins `match` lets the motion swallow the Ctrl key.
    #[test]
    fn ctrl_p_toggles_preview_even_when_editor_focused() {
        let mut app = editor_focused_app();
        let before = app.preview_enabled;
        press(&mut app, 'p', KeyModifiers::CONTROL);
        assert_ne!(app.preview_enabled, before, "Ctrl+P was swallowed by the 'p' paste motion");
    }

    #[test]
    fn ctrl_o_opens_recent_files_even_when_editor_focused() {
        let mut app = editor_focused_app();
        press(&mut app, 'o', KeyModifiers::CONTROL);
        assert_eq!(app.mode, AppMode::RecentFiles, "Ctrl+O was swallowed by the 'o' open-line motion");
    }

    #[test]
    fn ctrl_b_opens_backlinks_even_when_editor_focused() {
        let mut app = editor_focused_app();
        let col_before = app.editor_cursor.1;
        press(&mut app, 'b', KeyModifiers::CONTROL);
        assert_eq!(
            app.editor_cursor.1, col_before,
            "Ctrl+B moved the cursor: it was swallowed by the 'b' word-back motion"
        );
    }

    #[test]
    fn ctrl_y_is_not_swallowed_by_the_yank_pending_key() {
        let mut app = editor_focused_app();
        press(&mut app, 'y', KeyModifiers::CONTROL);
        assert_ne!(app.pending_key, Some('y'), "Ctrl+Y set the 'yy' pending key instead of redoing");
    }

    // --- the plain motions these share a letter with must still work ---

    #[test]
    fn plain_b_still_moves_a_word_back() {
        let mut app = editor_focused_app();
        press(&mut app, 'b', KeyModifiers::NONE);
        assert!(app.editor_cursor.1 < 10, "plain 'b' should move the cursor back a word");
    }

    #[test]
    fn plain_y_still_arms_the_yank_sequence() {
        let mut app = editor_focused_app();
        press(&mut app, 'y', KeyModifiers::NONE);
        assert_eq!(app.pending_key, Some('y'), "plain 'y' should arm the 'yy' sequence");
    }

    /// The structural guarantee the table exists to provide: no two bindings can
    /// claim the same key in the same context. This is what a `match` could not
    /// enforce, and what let five Ctrl chords be silently shadowed.
    #[test]
    fn no_two_bindings_claim_the_same_key_and_context() {
        let mut seen: Vec<(KeyCode, KeyModifiers, Ctx)> = Vec::new();
        for b in NORMAL_BINDINGS {
            let key = (b.code, b.mods, b.ctx);
            assert!(
                !seen.contains(&key),
                "duplicate binding for {:?} with {:?} in {:?} ({})",
                b.code, b.mods, b.ctx, b.desc
            );
            seen.push(key);
        }
    }

    /// The help dialog is organised by topic, not by mode, and covers modes this
    /// table does not describe — so it is hand-written on purpose and generating
    /// it would be a downgrade. What can be enforced is that it stays complete:
    /// every Ctrl chord and function key in the table must appear in the help text.
    ///
    /// The source is pulled in with `include_str!` rather than restructuring 251
    /// lines of carefully aligned UI into data.
    #[test]
    fn every_ctrl_chord_and_fkey_is_documented_in_help() {
        const UI_SRC: &str = include_str!("ui.rs");
        let help = {
            let start = UI_SRC.find("fn draw_help_dialog").expect("help dialog not found");
            &UI_SRC[start..]
        };
        // The help text abbreviates pairs as "Ctrl+U/D"; expand so both halves
        // count as documented.
        let expanded = regex::Regex::new(r"Ctrl\+([A-Z])/([A-Z])")
            .unwrap()
            .replace_all(help, "Ctrl+$1 Ctrl+$2")
            .to_string();

        let mut undocumented = Vec::new();
        for b in NORMAL_BINDINGS {
            let label = match (b.code, b.mods) {
                (KeyCode::Char(c), KeyModifiers::CONTROL) => {
                    format!("Ctrl+{}", c.to_ascii_uppercase())
                }
                (KeyCode::F(n), _) => format!("F{}", n),
                _ => continue, // plain letters are too short to match reliably
            };
            if !expanded.contains(&label) {
                undocumented.push(format!("{} ({})", label, b.desc));
            }
        }
        assert!(
            undocumented.is_empty(),
            "bindings missing from the help screen: {:?}",
            undocumented
        );
    }

    /// The digits are only shortcuts while the landing page is up. Anywhere else
    /// they must stay unbound, or typing a number in Normal mode would teleport
    /// you into another note.
    #[test]
    fn recent_note_digits_bind_only_on_the_landing_page() {
        let two = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
        assert_eq!(lookup(&two, false, true), Some(Action::OpenRecent(2)));
        assert_eq!(lookup(&two, false, false), None, "inert in the tree");
        assert_eq!(lookup(&two, true, false), None, "inert with the editor focused");
    }

    /// A binding with no label cannot be documented, so refuse to add one.
    #[test]
    fn every_binding_is_described() {
        for b in NORMAL_BINDINGS {
            assert!(!b.desc.trim().is_empty(), "binding {:?} has no description", b.code);
        }
    }

    /// An `Editor` binding is inert unless the editor pane actually has focus,
    /// and must not fall through to a global binding on the same key.
    #[test]
    fn editor_bindings_do_not_fire_outside_the_editor() {
        assert_eq!(
            lookup(&KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE), false, false),
            None,
            "'w' is an editor motion and has no meaning in the tree"
        );
        assert_eq!(
            lookup(&KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE), true, false),
            Some(Action::WordForward)
        );
    }

    /// Every Ctrl chord resolves to its own action regardless of focus.
    #[test]
    fn ctrl_chords_resolve_identically_in_both_contexts() {
        for (c, expected) in [
            ('b', Action::Backlinks),
            ('o', Action::RecentFiles),
            ('p', Action::TogglePreview),
            ('l', Action::FollowLink),
        ] {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
            assert_eq!(lookup(&key, false, false), Some(expected), "Ctrl+{} in tree", c);
            assert_eq!(lookup(&key, true, false), Some(expected), "Ctrl+{} with editor focused", c);
        }
    }

    /// Pressing Esc and then a command key arrives as Alt+key. Nothing binds Alt,
    /// so it must still reach the plain binding. Caught by a pty run where `Esc q`
    /// quit on the old code but hung on the first version of this table.
    #[test]
    fn alt_modified_key_falls_back_to_the_plain_binding() {
        let alt_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT);
        assert_eq!(lookup(&alt_q, false, false), Some(Action::Quit), "Esc-then-q must still quit");

        // The fallback must not paper over Ctrl: Ctrl+Alt+P is not Ctrl+P.
        let ctrl_alt_p = KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_eq!(lookup(&ctrl_alt_p, false, false), Some(Action::TogglePreview));
    }

    /// Terminals disagree about whether SHIFT is reported alongside an uppercase
    /// character, so matching must not depend on it.
    #[test]
    fn uppercase_binding_matches_with_and_without_shift_reported() {
        let mut a = editor_focused_app();
        a.focused_pane = FocusedPane::Folders;
        handle_normal_mode(&mut a, KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE));

        let mut b = editor_focused_app();
        b.focused_pane = FocusedPane::Folders;
        handle_normal_mode(&mut b, KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));

        assert_eq!(
            a.selected_folder_index, b.selected_folder_index,
            "SHIFT must not change how 'G' dispatches"
        );
    }
}
