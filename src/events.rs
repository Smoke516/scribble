use crate::app::{App, AppMode, FocusedPane, TreeItemType, WelcomeAction};
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
        AppMode::Visual => handle_visual_mode(app, key),
        AppMode::TemplatePicker => handle_template_picker_mode(app, key),
        AppMode::SpellSuggest => handle_spell_suggest_mode(app, key),
        AppMode::Outline => handle_outline_mode(app, key),
        AppMode::Palette => handle_palette_mode(app, key),
        AppMode::Tasks => handle_tasks_mode(app, key),
        AppMode::Conflicts => handle_conflicts_mode(app, key),
        AppMode::Explorer => handle_explorer_mode(app, key),
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
                let last_line_len = text.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0);
                app.editor_cursor.1 = last_line_len as u16;
            } else {
                app.editor_cursor.1 += text.chars().count() as u16;
            }

            app.adjust_scroll_to_cursor();
            app.mark_modified();
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
    /// Vim's `<CR>`: down a line, to its first non-blank character.
    CursorNextLineStart,
    WordForward,
    WordBackward,
    DeleteCharAtCursor,
    ToggleTaskCheckbox,
    PasteBelow,
    PasteClipboardBelow,
    ShowPalette,
    ShowTasks,
    DeleteToLineEnd,
    ChangeToLineEnd,
    YankLine,
    NoOp,
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
    MoveItem,
    RenameItem,
    SaveNote,
    ExternalEditor,
    TagInput,
    // Panels and search
    QuickJump,
    ShowOutline,
    RecentFiles,
    VaultSwitcher,
    TagBrowser,
    ThemeBrowser,
    DailyNote,
    TogglePreview,
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
    WelcomeUp,
    WelcomeDown,
    WelcomeActivate,
    ShowExplorer,
    /// Close the note and return to the landing page.
    GoHome,
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
    k(KeyCode::Enter, Ctx::Editor, Action::CursorNextLineStart, "Next line"),

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
    k(KeyCode::Char('D'), Ctx::Editor, Action::DeleteToLineEnd, "D: delete to end of line"),
    k(KeyCode::Char('C'), Ctx::Editor, Action::ChangeToLineEnd, "C: change to end of line"),
    k(KeyCode::Char('Y'), Ctx::Editor, Action::YankLine, "Y: yank line"),
    k(KeyCode::Char('v'), Ctx::Editor, Action::EnterVisual, "Visual selection"),
    k(KeyCode::Char('z'), Ctx::Editor, Action::PendingSpellPrefix, "z=: spelling prefix"),
    k(KeyCode::Char('='), Ctx::Editor, Action::SpellSuggestions, "z=: spelling suggestions"),
    ctrl(KeyCode::Char('z'), Ctx::Editor, Action::UndoText, "Undo edit"),
    ctrl(KeyCode::Char('y'), Ctx::Editor, Action::RedoText, "Redo edit"),
    k(KeyCode::Char('d'), Ctx::Any, Action::DeleteLineOrConfirm, "d{motion}: delete / delete item"),
    k(KeyCode::Char('c'), Ctx::Editor, Action::NoOp, "c{motion}: change"),
    k(KeyCode::Char('y'), Ctx::Editor, Action::NoOp, "y{motion}: yank"),
    k(KeyCode::Char('i'), Ctx::Any, Action::EnterInsert, "Insert mode"),
    k(KeyCode::Char('u'), Ctx::Any, Action::UndoLastDelete, "Undo last delete"),

    // --- items ---
    k(KeyCode::Char('n'), Ctx::Any, Action::NewNoteOrSearchNext, "New note / next match"),
    k(KeyCode::Char('m'), Ctx::Any, Action::MoveItem, "Move item"),
    k(KeyCode::Char('r'), Ctx::Any, Action::RenameItem, "Rename item"),
    k(KeyCode::Char('e'), Ctx::Any, Action::ExternalEditor, "Open in external editor"),
    k(KeyCode::Char('t'), Ctx::Any, Action::TagInput, "Edit tags"),
    ctrl(KeyCode::Char('s'), Ctx::Any, Action::SaveNote, "Save note"),

    // --- panels ---
    ctrl(KeyCode::Char('j'), Ctx::Any, Action::QuickJump, "Quick jump to note"),
    ctrl(KeyCode::Char('g'), Ctx::Any, Action::ShowOutline, "Outline panel"),
    ctrl(KeyCode::Char('k'), Ctx::Any, Action::ShowTasks, "Tasks: every open task in the vault"),
    ctrl(KeyCode::Char('e'), Ctx::Any, Action::ShowExplorer, "Explorer: browse the vault"),
    ctrl(KeyCode::Char('o'), Ctx::Any, Action::RecentFiles, "Recent files"),
    ctrl(KeyCode::Char('v'), Ctx::Any, Action::VaultSwitcher, "Switch vault"),
    ctrl(KeyCode::Char('t'), Ctx::Any, Action::TagBrowser, "Tag browser"),
    // The palette takes Ctrl+P, the near-universal chord for "open anything by
    // typing". Preview keeps F2, which it already answered to, so nothing is lost.
    ctrl(KeyCode::Char('p'), Ctx::Any, Action::ShowPalette, "Go to: notes, tags, headings, commands"),
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

    // --- landing page: it owns the whole screen, so it owns the arrow keys ---
    k(KeyCode::Char('j'), Ctx::Welcome, Action::WelcomeDown, "Next item"),
    k(KeyCode::Down, Ctx::Welcome, Action::WelcomeDown, "Next item"),
    k(KeyCode::Char('k'), Ctx::Welcome, Action::WelcomeUp, "Previous item"),
    k(KeyCode::Up, Ctx::Welcome, Action::WelcomeUp, "Previous item"),
    k(KeyCode::Enter, Ctx::Welcome, Action::WelcomeActivate, "Activate item"),

    // --- landing page: digits jump straight into the recent list ---
    k(KeyCode::Char('1'), Ctx::Welcome, Action::OpenRecent(1), "Open 1st recent note"),
    k(KeyCode::Char('2'), Ctx::Welcome, Action::OpenRecent(2), "Open 2nd recent note"),
    k(KeyCode::Char('3'), Ctx::Welcome, Action::OpenRecent(3), "Open 3rd recent note"),
    k(KeyCode::Char('4'), Ctx::Welcome, Action::OpenRecent(4), "Open 4th recent note"),
    k(KeyCode::Char('5'), Ctx::Welcome, Action::OpenRecent(5), "Open 5th recent note"),
    k(KeyCode::Char('6'), Ctx::Welcome, Action::OpenRecent(6), "Open 6th recent note"),
    k(KeyCode::Char('7'), Ctx::Welcome, Action::OpenRecent(7), "Open 7th recent note"),
    k(KeyCode::Char('8'), Ctx::Welcome, Action::OpenRecent(8), "Open 8th recent note"),

    // --- meta ---
    k(KeyCode::Char(':'), Ctx::Any, Action::CommandMode, "Command"),
    k(KeyCode::Char('?'), Ctx::Any, Action::Help, "Help"),
    // `q` backs out one level: a note returns to the landing page, and the
    // landing page quits. `Q` skips the ladder for when you just want out.
    k(KeyCode::Char('q'), Ctx::Welcome, Action::Quit, "Quit"),
    k(KeyCode::Char('q'), Ctx::Any, Action::GoHome, "Close note, back to the landing page"),
    k(KeyCode::Char('Q'), Ctx::Any, Action::Quit, "Quit immediately"),
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

/// Whether a command aimed at "the selected item" should mean the tree's
/// highlight rather than the note on screen.
///
/// Only when the tree is both focused and drawn: with the sidebar hidden its
/// highlight cannot be seen, so acting on it is acting blind.
fn acting_on_the_tree(app: &App) -> bool {
    app.focused_pane == FocusedPane::Folders && app.tree_is_visible()
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

/// Feed a key to the operator-pending state machine.
///
/// Returns true when the key was consumed by an operator sequence, in which case
/// the keymap must not also see it — the `w` of `dw` is a motion for the operator,
/// not a cursor movement.
///
/// This exists because `dd` and `yy` used to be hard-coded two-key sequences, so
/// the only thing either operator could combine with was itself. Modelling the
/// operator as pending state instead means every operator gets every motion from
/// one table, rather than needing a binding per pair.
fn feed_operator_pending(app: &mut App, key: &KeyEvent) -> bool {
    use crate::vim::{Motion, Operator, Target, TextObject};

    let KeyCode::Char(c) = key.code else {
        // Esc abandons a half-typed sequence; any other non-character key is not
        // part of one, so it clears the state and falls through to the keymap.
        if key.code == KeyCode::Esc && app.pending_op.is_some() {
            app.clear_pending_operator();
            return true;
        }
        app.clear_pending_operator();
        return false;
    };

    // Counts. A leading `0` is the start-of-line motion, not a digit — it only
    // becomes one once a count is already being typed.
    if c.is_ascii_digit() && !(c == '0' && app.pending_count.is_none()) {
        let digit = c.to_digit(10).unwrap() as usize;
        app.pending_count = Some(app.pending_count.unwrap_or(0) * 10 + digit);
        return true;
    }

    let Some(operator) = app.pending_op else {
        // Not pending yet: an operator key arms one, anything else is a normal key.
        let operator = match c {
            'd' => Operator::Delete,
            'c' => Operator::Change,
            'y' => Operator::Yank,
            _ => return false,
        };
        app.pending_op = Some(operator);
        app.pending_op_prefix = None;
        return true;
    };

    let count = app.pending_count.unwrap_or(1);

    // Mid-sequence keys: the `i`/`a` of `diw`, the `g` of `dgg`.
    match (app.pending_op_prefix, c) {
        (Some('i'), 'w') => {
            app.clear_pending_operator();
            app.apply_operator(operator, Target::Object(TextObject::InnerWord), count);
            return true;
        }
        (Some('a'), 'w') => {
            app.clear_pending_operator();
            app.apply_operator(operator, Target::Object(TextObject::AWord), count);
            return true;
        }
        (Some('g'), 'g') => {
            app.clear_pending_operator();
            app.apply_operator(operator, Target::Motion(Motion::FileStart), count);
            return true;
        }
        (Some(_), _) => {
            // A prefix that led nowhere — abandon the whole sequence rather than
            // guessing at what was meant.
            app.clear_pending_operator();
            return true;
        }
        (None, 'i') | (None, 'a') | (None, 'g') => {
            app.pending_op_prefix = Some(c);
            return true;
        }
        _ => {}
    }

    // The doubled form: dd, cc, yy.
    let doubled = matches!(
        (operator, c),
        (Operator::Delete, 'd') | (Operator::Change, 'c') | (Operator::Yank, 'y')
    );
    let motion = if doubled {
        Some(Motion::WholeLine)
    } else {
        match c {
            'w' => Some(Motion::WordForward),
            'b' => Some(Motion::WordBackward),
            'e' => Some(Motion::WordEnd),
            '$' => Some(Motion::LineEnd),
            '0' => Some(Motion::LineStart),
            '^' => Some(Motion::FirstNonBlank),
            'j' => Some(Motion::Down),
            'k' => Some(Motion::Up),
            'G' => Some(Motion::FileEnd),
            _ => None,
        }
    };

    app.clear_pending_operator();
    if let Some(motion) = motion {
        app.apply_operator(operator, Target::Motion(motion), count);
    }
    // Consumed either way: a key that is not a motion cancels the operator rather
    // than acting on its own, which is what vim does and what stops a mistyped `dq`
    // from quitting.
    true
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    // Clear pending vim key if current key doesn't continue the sequence
    let continues_sequence = matches!(
        (app.pending_key, &key.code),
        (Some('z'), KeyCode::Char('='))
    );
    if app.pending_key.is_some() && !continues_sequence {
        app.pending_key = None;
    }

    let editor_focused = app.focused_pane == FocusedPane::Editor && app.current_note.is_some();
    // The landing page shows exactly when no note is open.
    let on_welcome = app.current_note.is_none();

    // Operators only exist in the editor. Outside it `d` still means delete-item,
    // and a stray count would swallow the digit shortcuts on the landing page.
    // Modified keys are never part of a sequence — Ctrl+Y is redo, not a yank.
    if editor_focused && key.modifiers.is_empty() && feed_operator_pending(app, &key) {
        return;
    }
    if !editor_focused {
        app.clear_pending_operator();
    }

    // A count that reaches the keymap belongs to a plain motion — `3w`, `5j`. Take
    // it and clear it here: leaving it armed made the next operator silently
    // inherit it, so `3w` followed by `dd` deleted three lines.
    let count = app.pending_count.take().unwrap_or(1);

    if let Some(action) = lookup(&key, editor_focused, on_welcome) {
        // Only movement repeats. Repeating anything else would turn `3` followed by
        // a mistyped key into three of whatever that key does.
        let repeats = if matches!(
            action,
            Action::CursorUp
                | Action::CursorDown
                | Action::CursorLeft
                | Action::CursorRight
                | Action::WordForward
                | Action::WordBackward
        ) {
            count.clamp(1, 1000)
        } else {
            1
        };
        for _ in 0..repeats {
            run_action(app, action, editor_focused);
        }
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
            // A pane that is not drawn is not in the cycle. With the sidebar
            // hidden, Tab used to move focus into an invisible tree, where j/k
            // moved a highlight nobody could see and Enter opened whatever it
            // had landed on.
            let tree = if app.tree_is_visible() {
                FocusedPane::Folders
            } else {
                FocusedPane::Editor
            };
            app.focused_pane = if app.preview_enabled {
                match app.focused_pane {
                    FocusedPane::Folders => FocusedPane::Editor,
                    FocusedPane::Editor => FocusedPane::Preview,
                    FocusedPane::Preview => tree,
                }
            } else {
                match app.focused_pane {
                    FocusedPane::Folders => FocusedPane::Editor,
                    FocusedPane::Editor => tree,
                    FocusedPane::Preview => FocusedPane::Editor, // Fallback if preview gets disabled
                }
            };
        }
        Action::ActivateSelected => {
            // Only from the tree. With a note open, the tree's selection is
            // usually some other note — it does not follow the palette, the task
            // panel or the landing page — so activating it from the editor or the
            // preview jumped you somewhere you never asked to go, and took the
            // unsaved buffer with it.
            if app.focused_pane == FocusedPane::Folders {
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
        Action::CursorNextLineStart => app.cursor_to_next_line_start(),
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
        // D, C and Y are vim's shorthand for d$, c$ and yy. They earn their own
        // bindings because they are single keystrokes, not operator sequences.
        Action::DeleteToLineEnd => app.apply_operator(
            crate::vim::Operator::Delete,
            crate::vim::Target::Motion(crate::vim::Motion::LineEnd),
            1,
        ),
        Action::ChangeToLineEnd => app.apply_operator(
            crate::vim::Operator::Change,
            crate::vim::Target::Motion(crate::vim::Motion::LineEnd),
            1,
        ),
        Action::YankLine => app.apply_operator(
            crate::vim::Operator::Yank,
            crate::vim::Target::Motion(crate::vim::Motion::WholeLine),
            1,
        ),
        // In the editor `d` is an operator and never reaches here — the pending
        // layer consumes it. Outside it, it still confirms deleting the selected
        // note or folder.
        Action::DeleteLineOrConfirm => {
            if !editor_focused {
                // Same target as rename and move: the highlight when the tree is
                // the thing you are looking at, the open note otherwise. It used
                // to offer to delete the tree's highlight from the preview pane,
                // while `r` in that same pane renamed the note on screen.
                if !acting_on_the_tree(app) {
                    app.focus_tree_on_current_note();
                }
                if let Err(e) = app.start_delete_confirmation() {
                    app.set_message(e);
                }
            }
        }
        // Present only so the help screen can document the operator; the pending
        // layer handles the key itself.
        Action::NoOp => {}
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
        Action::MoveItem => {
            if !acting_on_the_tree(app) {
                app.focus_tree_on_current_note();
            }
            app.start_move_item();
        }
        Action::RenameItem => {
            if !acting_on_the_tree(app) {
                app.focus_tree_on_current_note();
            }
            app.start_rename_item();
        }
        Action::SaveNote => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        Action::ExternalEditor => {
            if let Err(e) = app.request_external_edit() {
                app.set_message(e);
            }
        }
        Action::TagInput => app.start_tag_input(),

        // --- panels ---
        Action::QuickJump => app.start_quick_jump(),
        Action::ShowOutline => app.show_outline(),
        Action::ShowPalette => app.show_palette(),
        Action::ShowTasks => app.show_task_panel(),
        Action::RecentFiles => app.toggle_recent_files(),
        Action::VaultSwitcher => app.show_vault_switcher(),
        Action::TagBrowser => app.show_tag_browser(),
        Action::ThemeBrowser => app.show_theme_browser(),
        Action::DailyNote => app.open_daily_note(),
        Action::TogglePreview => app.toggle_preview(),

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
            // `N` steps backwards through an in-note search while one is running,
            // and otherwise offers a template. It used to require NOT being focused
            // on the editor, while the picker itself required a note to be open —
            // and having a note open is most of what "editor focused" means. The
            // obvious route in (`n` to create a note, `N` to fill it) hit that gap
            // and silently did nothing.
            if editor_focused && app.note_search.active {
                app.note_search_prev();
            } else {
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
            if app.current_note.is_none() {
            } else if preview_focused {
                // j/k scroll the preview pane; paging has to follow the same pane
                // or the keys disagree about which text they are moving through.
                app.preview_scroll_page_up();
            } else if editor_focused {
                app.scroll_half_page_up();
            }
        }
        Action::ScrollHalfDown => {
            if app.current_note.is_none() {
            } else if preview_focused {
                // j/k scroll the preview pane; paging has to follow the same pane
                // or the keys disagree about which text they are moving through.
                app.preview_scroll_page_down();
            } else if editor_focused {
                app.scroll_half_page_down();
            }
        }
        Action::ScrollPageUp => {
            if app.current_note.is_none() {
            } else if preview_focused {
                // j/k scroll the preview pane; paging has to follow the same pane
                // or the keys disagree about which text they are moving through.
                app.preview_scroll_page_up();
            } else if editor_focused {
                app.scroll_page_up();
            }
        }
        Action::ScrollPageDown => {
            if app.current_note.is_none() {
            } else if preview_focused {
                // j/k scroll the preview pane; paging has to follow the same pane
                // or the keys disagree about which text they are moving through.
                app.preview_scroll_page_down();
            } else if editor_focused {
                app.scroll_page_down();
            }
        }

        Action::WelcomeUp => app.welcome_move(-1),
        Action::WelcomeDown => app.welcome_move(1),
        Action::WelcomeActivate => {
            // The row hands back intent; the same arms below perform it, so a row
            // and its shortcut key can never drift apart.
            if let Some(w) = app.welcome_action_at_cursor() {
                let follow = match w {
                    WelcomeAction::OpenNote(id) => {
                        app.select_note(id);
                        app.focused_pane = FocusedPane::Editor;
                        return;
                    }
                    WelcomeAction::DailyNote => Action::DailyNote,
                    WelcomeAction::NewNote => Action::NewNoteOrSearchNext,
                    WelcomeAction::Search => Action::SearchInNoteOrGlobal,
                    WelcomeAction::QuickJump => Action::QuickJump,
                    WelcomeAction::Explorer => Action::ShowExplorer,
                    WelcomeAction::Help => Action::Help,
                };
                run_action(app, follow, false);
            }
        }
        Action::ShowExplorer => {
            app.mode = AppMode::Explorer;
            app.focused_pane = FocusedPane::Folders;
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
        Action::GoHome => {
            // Save on the way out: closing a note must never be a way to lose the
            // last edit. The landing page is rebuilt from the notebook, so the
            // note has to be committed to it first.
            if app.current_note.is_some() {
                if let Err(e) = app.save_current_note() {
                    app.set_message(e);
                }
            }
            app.set_welcome_message();
            app.welcome_selected = 0;
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
                // Refresh spell errors after editing
                app.run_spell_check();
            }
        }
        
        KeyCode::Tab => {
            if app.autocomplete_state.active {
                app.apply_autocomplete();
            } else if app.indent_list_item(false) {
                // Inside a list, Tab nests the item — which is what it is for
                // there, and what the popup used to swallow.
            } else {
                insert_str_at_cursor(app, "    "); // 4 spaces
                app.mark_modified();
                app.update_autocompletion();
            }
        }

        KeyCode::BackTab => {
            app.indent_list_item(true);
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
                        let chars = current_line.chars().count() as u16;
                        if app.editor_cursor.1 > chars {
                            app.editor_cursor.1 = chars;
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
                        let chars = current_line.chars().count() as u16;
                        if app.editor_cursor.1 > chars {
                            app.editor_cursor.1 = chars;
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
            }
        }

        KeyCode::Enter => {
            {
                // Enter always breaks the line. Accepting a snippet is Tab's job:
                // a key that sometimes makes a newline and sometimes does not is
                // the kind of surprise you cannot plan around while typing.
                app.cancel_autocomplete();
                app.push_undo_snapshot(); // word boundary
                insert_char_at_cursor(app, '\n');
                app.mark_modified();
                app.editor_cursor.0 += 1;
                app.editor_cursor.1 = 0;
                app.adjust_scroll_to_cursor();
                app.update_autocompletion();
            }
        }
        
        KeyCode::Backspace => {
            delete_char_before_cursor(app);
            app.mark_modified();
            app.update_autocompletion();
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
            if (app.editor_cursor.1 as usize) < current_line.chars().count() {
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
                    let where_to = crate::storage::Storage::new()
                        .map(|s| s.get_notes_dir().display().to_string())
                        .unwrap_or_else(|_| "the notes directory".to_string());
                    app.set_operation_success(
                        format!("Exported {} notes to {}", count, where_to),
                        Some("📦".to_string()),
                    );
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
        } else if matches!(command, "spell" | "spellon" | "spell on") {
            if !app.spell.aspell_available {
                app.set_message("aspell not found — install it: sudo apt install aspell".to_string());
            } else {
                app.spell.enabled = true;
                app.run_spell_check();
                app.set_message(format!("Spell check ON — {} error(s)", app.spell.errors.len()));
            }
        } else if matches!(command, "nospell" | "spelloff" | "spell off") {
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
    app.cursor_byte_index()
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

    // Forward by characters: the column counts those, not bytes.
    app.editor_cursor.1 += s.chars().count() as u16;
}

fn delete_char_before_cursor(app: &mut App) {
    if app.editor_content.is_empty() {
        return;
    }
    
    let cursor_index = get_cursor_byte_index(app);
    if cursor_index > 0 {
        // Remove the previous character, which may be several bytes wide.
        let prev = app.editor_content[..cursor_index]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        app.editor_content.replace_range(prev..cursor_index, "");

        // Move cursor back
        if app.editor_cursor.1 > 0 {
            app.editor_cursor.1 -= 1;
        } else if app.editor_cursor.0 > 0 {
            // We deleted a newline, move to end of previous line
            app.editor_cursor.0 -= 1;
            let lines: Vec<&str> = app.editor_content.lines().collect();
            if let Some(prev_line) = lines.get(app.editor_cursor.0 as usize) {
                app.editor_cursor.1 = prev_line.chars().count() as u16;
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
                
                if let Some(mut note) = app.current_note.clone() {
                    let is_regex = app.command_buffer.contains("regex");
                    let case_sensitive = app.command_buffer.contains("case");

                    match app.enhanced_search.replace_in_note(&mut note, &find, &replace, is_regex, case_sensitive) {
                        Ok(count) => {
                            if count > 0 {
                                let id = note.id;
                                app.editor_content = note.content.clone();
                                // Update the note in the notebook
                                app.notebook.notes.insert(id, note.clone());
                                app.current_note = Some(note);
                                // Queue the write. Without this the replacement lived
                                // in memory only: the message said it had happened,
                                // and quitting threw it away unless something else
                                // had happened to mark the note dirty first.
                                app.mark_note_dirty(id);
                                app.mark_modified();
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
        KeyCode::Char('h') | KeyCode::Left => {
            // Collapse a folder to get past it quickly while hunting for the
            // destination. Nothing here may modify the vault: you are choosing
            // where something lands, not editing.
            if let Some(item) = app.get_selected_item() {
                if item.item_type == TreeItemType::Folder && item.expanded {
                    app.toggle_folder_expansion();
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(item) = app.get_selected_item() {
                if item.item_type == TreeItemType::Folder && !item.expanded {
                    app.toggle_folder_expansion();
                }
            }
        }

        KeyCode::Enter => {
            if let Err(e) = app.execute_move() {
                app.set_message(e);
                app.cancel_move();
            }
        }

        // The vault root is not a row in the tree, so it needs a key of its own.
        // Aiming at a root-level note happens to work, but fails outright in a
        // vault that has none — leaving nothing to move back out of a folder to.
        KeyCode::Char('~') => {
            if let Err(e) = app.execute_move_to_root() {
                app.set_message(e);
                app.cancel_move();
            }
        }

        // Help in move mode
        KeyCode::Char('?') => {
            app.set_message(
                "Move mode: j/k navigate · h/l fold · Enter drop here · ~ vault root · Esc cancel"
                    .to_string(),
            );
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
        _ => {
            app.cancel_delete();
        }
    }

    // Both paths leave the confirm; return to whatever opened it. Deleting from
    // the explorer and being dropped back to Normal loses your place in the tree.
    if let Some(back) = app.modal_return.take() {
        app.mode = back;
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
            app.request_vault_switch();
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
                    app.request_vault_switch();
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

/// The palette is a text field with a list under it, so it takes characters
/// directly rather than going through the normal-mode keymap.
fn handle_palette_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_palette(),
        KeyCode::Enter => {
            if let Some(cmd) = app.palette_select() {
                run_palette_command(app, cmd);
            }
        }
        KeyCode::Up => app.palette_navigate_up(),
        KeyCode::Down => app.palette_navigate_down(),
        KeyCode::Backspace => app.palette_backspace(),
        // Ctrl+N / Ctrl+P move the selection, so the hands never leave the keys —
        // and plain n/p stay available as text, which they have to be when the
        // query is a note title.
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.palette_navigate_down()
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.palette_navigate_up()
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => app.palette_type(c),
        _ => {}
    }
}

/// Run a command chosen by name. Each one is the same thing its chord does.
fn run_palette_command(app: &mut App, cmd: crate::palette::Command) {
    use crate::palette::Command;
    match cmd {
        Command::DailyNote => app.open_daily_note(),
        Command::Tasks => app.show_task_panel(),
        Command::Conflicts => app.show_conflicts(),
        Command::Outline => app.show_outline(),
        Command::Explorer => {
            app.mode = AppMode::Explorer;
            app.focused_pane = FocusedPane::Folders;
        }
        Command::RecentFiles => app.toggle_recent_files(),
        Command::TagBrowser => app.show_tag_browser(),
        Command::ThemeBrowser => app.show_theme_browser(),
        Command::VaultSwitcher => app.show_vault_switcher(),
        Command::TogglePreview => app.toggle_preview(),
        // Same folder rule as `n`: whatever the tree selection is sitting in.
        Command::NewNote => {
            let folder_id = app.get_selected_item().and_then(|item| match item.item_type {
                TreeItemType::Folder => Some(item.id),
                TreeItemType::Note => app.notebook.notes.get(&item.id).and_then(|n| n.folder_id),
            });
            app.start_new_note_input(folder_id);
        }
        Command::SaveNote => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        Command::Help => app.mode = AppMode::Help,
        Command::Quit => app.should_quit = true,
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

/// The resolve view: a list of conflicts, and three answers for the selected one.
fn handle_conflicts_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.cancel_conflicts(),
        KeyCode::Up | KeyCode::Char('k') => app.conflict_navigate_up(),
        KeyCode::Down | KeyCode::Char('j') => app.conflict_navigate_down(),
        // Tab picks the answer, Enter commits it. Two keys rather than one so a
        // stray keystroke cannot rewrite a note that took work to get right.
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => app.conflict_cycle_action(true),
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => app.conflict_cycle_action(false),
        KeyCode::Enter => app.resolve_selected_conflict(),
        _ => {}
    }
}

fn handle_tasks_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.cancel_task_panel(),
        KeyCode::Enter => app.task_select(),
        KeyCode::Up | KeyCode::Char('k') => app.task_navigate_up(),
        KeyCode::Down | KeyCode::Char('j') => app.task_navigate_down(),
        // Tick it where it lives, without leaving the list — the same key that
        // toggles a checkbox in the editor.
        KeyCode::Char(' ') => app.task_toggle_selected(),
        KeyCode::Char('a') => app.toggle_task_panel_done(),
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


/// The folder tree as an overlay. Navigation mirrors the sidebar exactly — this
/// is the same tree, just floating — so muscle memory carries over.
fn handle_explorer_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('e') | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => app.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => app.navigate_up(),
        KeyCode::Char('g') => app.navigate_to_top(),
        KeyCode::Char('G') => app.navigate_to_bottom(),
        KeyCode::Char('h') | KeyCode::Left => {
            // Collapse rather than leave: the overlay is the only tree on screen.
            if let Some(item) = app.get_selected_item() {
                if item.item_type == TreeItemType::Folder && item.expanded {
                    app.toggle_folder_expansion();
                }
            }
        }

        // --- structural edits, acting on the highlighted row ---
        KeyCode::Char('n') => {
            let folder_id = app.get_selected_item().and_then(|item| match item.item_type {
                TreeItemType::Folder => Some(item.id),
                TreeItemType::Note => app.notebook.notes.get(&item.id).and_then(|n| n.folder_id),
            });
            app.start_new_note_input(folder_id);
        }
        KeyCode::Char('f') => {
            let parent = app.get_selected_item().and_then(|item| match item.item_type {
                TreeItemType::Folder => Some(item.id),
                TreeItemType::Note => app.notebook.notes.get(&item.id).and_then(|n| n.folder_id),
            });
            app.start_new_folder_input(parent);
        }
        KeyCode::Char('F') => app.start_new_folder_input(None),
        KeyCode::Char('r') => app.start_rename_item(),
        KeyCode::Char('m') => app.start_move_item(),
        KeyCode::Char('d') => {
            // Stack the confirm over the tree and come back to it afterwards:
            // being dumped out of the explorer to delete one file is annoying.
            app.modal_return = Some(AppMode::Explorer);
            if let Err(e) = app.start_delete_confirmation() {
                app.modal_return = None;
                app.set_message(e);
            }
        }

        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
            if let Some(item) = app.get_selected_item() {
                match item.item_type {
                    TreeItemType::Note => {
                        let id = item.id;
                        app.select_note(id);
                        app.mode = AppMode::Normal;
                        app.focused_pane = FocusedPane::Editor;
                    }
                    // Folders expand in place; only a note dismisses the overlay.
                    TreeItemType::Folder => app.toggle_folder_expansion(),
                }
            }
        }
        _ => {}
    }
}

/// Typing anything outside ASCII used to panic the app.
///
/// The column counts characters and the buffer is indexed in bytes; the two
/// agree until you type an accent, an arrow, a CJK character or an emoji, and
/// then every slice at the cursor lands mid-character. `String::insert` and the
/// autocomplete scan both assert on that, so the process died on the keystroke.
#[cfg(test)]
mod utf8_cursor_tests {
    use super::*;
    use crate::models::Note;

    fn typing_app(content: &str) -> App {
        let mut app = App::default();
        let mut note = Note::new("N".to_string(), None);
        note.content = content.to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = content.to_string();
        app.editor_cursor = (0, content.chars().count() as u16);
        app.mode = AppMode::Insert;
        app
    }

    fn type_str(app: &mut App, s: &str) {
        for ch in s.chars() {
            handle_insert_mode(app, KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
    }

    #[test]
    fn typing_past_a_multi_byte_character_does_not_panic() {
        for word in ["café", "naïve", "日本語", "a→b", "🎉"] {
            let mut app = typing_app("");
            type_str(&mut app, word);
            type_str(&mut app, " tail");
            assert_eq!(
                app.editor_content,
                format!("{} tail", word),
                "typing after {:?} went wrong",
                word
            );
            assert_eq!(
                app.editor_cursor.1 as usize,
                app.editor_content.chars().count(),
                "the column stopped counting characters after {:?}",
                word
            );
        }
    }

    #[test]
    fn typing_in_the_middle_of_multi_byte_text_inserts_where_the_cursor_is() {
        let mut app = typing_app("café");
        app.editor_cursor = (0, 3); // between 'f' and 'é'
        type_str(&mut app, "X");
        assert_eq!(app.editor_content, "cafXé");
    }

    #[test]
    fn backspace_removes_a_whole_multi_byte_character() {
        let mut app = typing_app("café");
        handle_insert_mode(&mut app, KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.editor_content, "caf");
        assert_eq!(app.editor_cursor.1, 3);
    }

    #[test]
    fn backspace_over_a_newline_lands_at_the_end_of_the_line_above() {
        let mut app = typing_app("café\n");
        app.editor_cursor = (1, 0);
        handle_insert_mode(&mut app, KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.editor_content, "café");
        assert_eq!(app.editor_cursor, (0, 4), "column counted bytes, not characters");
    }

    #[test]
    fn end_of_line_is_a_character_count() {
        let mut app = typing_app("日本語");
        app.cursor_to_line_end();
        assert_eq!(app.editor_cursor.1, 3);
    }

    #[test]
    fn moving_between_lines_clamps_by_characters() {
        let mut app = typing_app("a→b→c\nxy");
        app.editor_cursor = (0, 5);
        app.cursor_down_normal();
        assert_eq!(app.editor_cursor, (1, 2), "clamped to a byte length");
        app.cursor_up_normal();
        assert_eq!(app.editor_cursor.0, 0);
    }

    #[test]
    fn the_right_arrow_stops_at_the_last_character() {
        let mut app = typing_app("é");
        app.editor_cursor = (0, 1);
        handle_insert_mode(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.editor_cursor.1, 1, "the cursor ran past the end of the line");
    }

    #[test]
    fn autocomplete_scans_a_multi_byte_line_without_panicking() {
        let mut app = typing_app("");
        type_str(&mut app, "café ");
        assert_eq!(app.editor_content, "café ");
        type_str(&mut app, "[");
        assert!(app.autocomplete_state.active, "the trigger stopped being found");
    }

    #[test]
    fn applying_a_completion_after_multi_byte_text_keeps_the_text() {
        let mut app = typing_app("");
        type_str(&mut app, "café [");
        assert!(app.autocomplete_state.active);
        handle_insert_mode(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(
            app.editor_content, "café [](url)",
            "the completion ate the text before it"
        );
        // The cursor sits between the brackets, counted in characters.
        assert_eq!(app.editor_cursor, (0, 6));
    }

    #[test]
    fn pasting_multi_byte_text_advances_the_column_by_characters() {
        let mut app = typing_app("");
        handle_paste(&mut app, "héllo").unwrap();
        assert_eq!(app.editor_cursor.1, 5, "the column counted bytes");
    }

    /// Round-tripping a byte offset back to a cursor has to land on characters.
    #[test]
    fn a_byte_offset_maps_back_to_a_character_column() {
        let mut app = typing_app("日本語");
        app.update_cursor_from_absolute_position(6); // after two 3-byte characters
        assert_eq!(app.editor_cursor, (0, 2));
        assert_eq!(app.cursor_byte_index(), 6, "the round trip did not close");
    }
}

/// What `Tab` and the popup do while a note is being written.
#[cfg(test)]
mod typing_tests {
    use super::*;
    use crate::models::Note;

    fn typing_app(content: &str) -> App {
        let mut app = App::default();
        let mut note = Note::new("N".to_string(), None);
        note.content = content.to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = content.to_string();
        app.editor_cursor = (0, content.chars().count() as u16);
        app.mode = AppMode::Insert;
        app
    }

    fn type_str(app: &mut App, s: &str) {
        for ch in s.chars() {
            handle_insert_mode(app, KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        handle_insert_mode(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// The one that made nested lists untypeable: after `- ` the popup was up,
    /// so Tab applied a completion that replaced `- ` with `- `.
    #[test]
    fn tab_indents_a_list_item() {
        let mut app = typing_app("");
        type_str(&mut app, "- milk");
        assert!(!app.autocomplete_state.active, "the popup is back over list items");
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.editor_content, "  - milk");
        assert_eq!(app.editor_cursor.1, 8, "the cursor did not move with the text");
    }

    #[test]
    fn shift_tab_outdents_a_list_item() {
        let mut app = typing_app("    - milk");
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.editor_content, "  - milk");
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.editor_content, "- milk");
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.editor_content, "- milk", "outdenting past the margin");
    }

    #[test]
    fn tab_indents_every_kind_of_list_item() {
        for line in ["- a", "* a", "+ a", "1. a", "12. a", "- [ ] a", "  - a"] {
            let mut app = typing_app(line);
            press(&mut app, KeyCode::Tab);
            assert_eq!(
                app.editor_content,
                format!("  {}", line),
                "{:?} was not indented",
                line
            );
        }
    }

    #[test]
    fn tab_outside_a_list_still_inserts_spaces() {
        let mut app = typing_app("prose");
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.editor_content, "prose    ");
    }

    /// Accepting a snippet has to leave the note different from what was typed.
    #[test]
    fn accepting_a_snippet_inserts_it_and_places_the_cursor() {
        let cases = [
            ("[", "[](url)", (0u16, 1u16)),
            ("**", "****", (0, 2)),
            ("`", "``", (0, 1)),
            ("![", "![](image.png)", (0, 2)),
        ];
        for (typed, expected, cursor) in cases {
            let mut app = typing_app("");
            type_str(&mut app, typed);
            assert!(app.autocomplete_state.active, "{:?} did not open the popup", typed);
            press(&mut app, KeyCode::Tab);
            assert_eq!(app.editor_content, expected, "{:?} expanded wrong", typed);
            assert_eq!(app.editor_cursor, cursor, "{:?} left the cursor wrong", typed);
        }
    }

    #[test]
    fn a_fence_puts_the_cursor_on_the_empty_line_inside_it() {
        let mut app = typing_app("");
        type_str(&mut app, "```");
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.editor_content, "```

```");
        assert_eq!(app.editor_cursor, (1, 0));
    }

    #[test]
    fn a_table_lands_the_cursor_in_the_first_header() {
        let mut app = typing_app("");
        type_str(&mut app, "|");
        press(&mut app, KeyCode::Tab);
        assert!(app.editor_content.starts_with("| "), "{:?}", app.editor_content);
        assert_eq!(app.editor_cursor, (0, 2), "the cursor missed the first header");
    }

    /// With the no-op popups gone, the first Esc leaves insert mode again.
    #[test]
    fn esc_after_a_list_marker_leaves_insert_mode() {
        let mut app = typing_app("");
        type_str(&mut app, "- milk");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, AppMode::Normal, "Esc was eaten by a popup");
    }

    /// And where a popup is genuinely up, Esc still dismisses it first.
    #[test]
    fn esc_dismisses_a_real_popup_before_leaving_insert_mode() {
        let mut app = typing_app("");
        type_str(&mut app, "[");
        assert!(app.autocomplete_state.active);
        press(&mut app, KeyCode::Esc);
        assert!(!app.autocomplete_state.active);
        assert_eq!(app.mode, AppMode::Insert);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, AppMode::Normal);
    }

    /// Enter is for breaking the line, always — it used to accept the popup
    /// instead, so after typing `- ` it did not even make a newline.
    #[test]
    fn enter_breaks_the_line_even_with_the_popup_up() {
        let mut app = typing_app("");
        type_str(&mut app, "[");
        assert!(app.autocomplete_state.active);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor_content, "[\n");
        assert_eq!(app.editor_cursor, (1, 0));
        assert!(!app.autocomplete_state.active);
    }

    /// Typing on past a trigger has to put the popup away again.
    #[test]
    fn typing_past_a_trigger_closes_the_popup() {
        let mut app = typing_app("");
        type_str(&mut app, "[");
        assert!(app.autocomplete_state.active);
        type_str(&mut app, "x");
        assert!(!app.autocomplete_state.active, "the popup outstayed its trigger");
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::models::Note;
    use uuid::Uuid;

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

    /// Two notes, the first open, the tree's highlight moved to the second by hand
    /// — j/k in the tree moves the highlight without opening anything, so the two
    /// can legitimately differ.
    fn two_notes_with_selection_elsewhere() -> (App, Uuid, Uuid) {
        let mut app = App::default();
        let mut open = Note::new("Open".to_string(), None);
        open.content = "first line\nsecond line".to_string();
        let mut other = Note::new("Other".to_string(), None);
        other.content = "somewhere else".to_string();
        app.notebook.add_note(open.clone());
        app.notebook.add_note(other.clone());
        app.refresh_tree_view();
        app.select_note(open.id);
        // Point the tree at the note that is *not* open.
        app.selected_folder_index = app
            .folder_tree_items
            .iter()
            .position(|i| i.id == other.id)
            .expect("other note missing from the tree");
        app.mode = AppMode::Normal;
        (app, open.id, other.id)
    }

    fn press_enter(app: &mut App) {
        handle_normal_mode(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }

    /// Enter with the editor focused used to activate whatever the tree had
    /// selected, throwing away up to two seconds of typing with it — the autosave
    /// debounce had not fired, and select_note overwrites the buffer.
    #[test]
    fn enter_in_the_editor_does_not_jump_to_another_note() {
        let (mut app, open_id, _) = two_notes_with_selection_elsewhere();
        app.focused_pane = FocusedPane::Editor;
        app.editor_content.push_str(" and an unsaved edit");
        app.mark_modified();

        press_enter(&mut app);

        assert_eq!(
            app.current_note.as_ref().map(|n| n.id),
            Some(open_id),
            "Enter jumped to the note the tree had selected"
        );
        assert!(
            app.editor_content.ends_with("and an unsaved edit"),
            "unsaved edits were discarded: {:?}",
            app.editor_content
        );
    }

    /// Same surprise from the preview pane, which is not the editor as far as the
    /// keymap is concerned.
    #[test]
    fn enter_in_the_preview_does_not_jump_to_another_note() {
        let (mut app, open_id, _) = two_notes_with_selection_elsewhere();
        app.focused_pane = FocusedPane::Preview;

        press_enter(&mut app);

        assert_eq!(app.current_note.as_ref().map(|n| n.id), Some(open_id));
    }

    /// The binding still has to do its job where it was meant to.
    #[test]
    fn enter_in_the_tree_opens_the_selected_note() {
        let (mut app, _, other_id) = two_notes_with_selection_elsewhere();
        app.focused_pane = FocusedPane::Folders;

        press_enter(&mut app);

        assert_eq!(
            app.current_note.as_ref().map(|n| n.id),
            Some(other_id),
            "Enter no longer opens the selected note from the tree"
        );
    }

    /// Vim's `<CR>`: down a line, to its first non-blank character.
    #[test]
    fn enter_in_the_editor_moves_to_the_next_line() {
        let (mut app, _, _) = two_notes_with_selection_elsewhere();
        app.focused_pane = FocusedPane::Editor;
        app.editor_content = "first line\n    indented".to_string();
        app.editor_cursor = (0, 8);

        press_enter(&mut app);

        assert_eq!(app.editor_cursor, (1, 4), "Enter should land on the first non-blank");
    }

    /// A note opened while the tree was highlighting a different one — what the
    /// landing page, its 1-9 shortcuts, the explorer and a freshly created note
    /// all do, none of which touch the tree.
    fn note_opened_while_the_tree_pointed_elsewhere() -> (App, Uuid) {
        let mut app = App::default();
        let open = Note::new("Open".to_string(), None);
        let other = Note::new("Other".to_string(), None);
        let (open_id, other_id) = (open.id, other.id);
        app.notebook.add_note(open);
        app.notebook.add_note(other);
        app.refresh_tree_view();
        app.selected_folder_index = app
            .folder_tree_items
            .iter()
            .position(|i| i.id == other_id)
            .expect("other note missing from the tree");

        app.select_note(open_id);
        app.mode = AppMode::Normal;
        (app, open_id)
    }

    /// The tree's highlight must land on whatever note is open, whichever route
    /// opened it — otherwise the highlight is pointing at a note you have not
    /// looked at, and every command that acts on the selection acts on that one.
    #[test]
    fn opening_a_note_points_the_tree_at_it() {
        let (app, open_id) = note_opened_while_the_tree_pointed_elsewhere();

        assert_eq!(
            app.get_selected_item().map(|i| i.id),
            Some(open_id),
            "the tree is still highlighting a note that is not open"
        );
    }

    /// The reported sequence: a note open in normal mode, Tab into the folder
    /// pane, Enter — and you are looking at a different note, because the
    /// highlight never followed the note you opened.
    #[test]
    fn tab_into_the_tree_then_enter_stays_on_the_open_note() {
        let (mut app, open_id) = note_opened_while_the_tree_pointed_elsewhere();
        app.config.ui.show_sidebar = true; // Tab only reaches a tree that is drawn
        app.focused_pane = FocusedPane::Editor;

        handle_normal_mode(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focused_pane, FocusedPane::Folders, "Tab did not reach the tree");
        press_enter(&mut app);

        assert_eq!(
            app.current_note.as_ref().map(|n| n.id),
            Some(open_id),
            "Tab then Enter jumped to whatever row the tree had left selected"
        );
    }

    /// A note inside a collapsed folder has no row to highlight, so revealing it
    /// has to expand the folder first.
    #[test]
    fn opening_a_note_reveals_it_inside_a_collapsed_folder() {
        use crate::models::Folder;
        let mut app = App::default();
        let folder = Folder::new("Work".to_string(), None);
        let folder_id = folder.id;
        app.notebook.add_folder(folder);
        if let Some(f) = app.notebook.folders.get_mut(&folder_id) {
            f.expanded = false;
        }
        let buried = Note::new("Buried".to_string(), Some(folder_id));
        let buried_id = buried.id;
        app.notebook.add_note(buried);
        app.refresh_tree_view();

        app.select_note(buried_id);

        assert_eq!(
            app.get_selected_item().map(|i| i.id),
            Some(buried_id),
            "the note was not revealed in the tree"
        );
    }

    /// Global Ctrl-shortcuts must keep working while the editor pane is focused.
    /// They share a letter with an editor motion, and the motion arms do not check
    /// modifiers, so a first-match-wins `match` lets the motion swallow the Ctrl key.
    #[test]
    fn ctrl_p_opens_the_palette_even_when_editor_focused() {
        let mut app = editor_focused_app();
        press(&mut app, 'p', KeyModifiers::CONTROL);
        assert_eq!(
            app.mode,
            AppMode::Palette,
            "Ctrl+P was swallowed by the 'p' paste motion"
        );
    }

    /// Preview kept F2 when the palette took Ctrl+P, so the feature is still
    /// reachable by a single key.
    #[test]
    fn f2_still_toggles_preview() {
        let mut app = editor_focused_app();
        let before = app.preview_enabled;
        handle_normal_mode(&mut app, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_ne!(app.preview_enabled, before, "F2 no longer toggles preview");
    }

    #[test]
    fn ctrl_o_opens_recent_files_even_when_editor_focused() {
        let mut app = editor_focused_app();
        press(&mut app, 'o', KeyModifiers::CONTROL);
        assert_eq!(app.mode, AppMode::RecentFiles, "Ctrl+O was swallowed by the 'o' open-line motion");
    }

    /// Ctrl+B lost its binding when wiki links were retired. An unbound Ctrl chord
    /// must still not fall through to the plain key — the modifier has to be part
    /// of the match, or every freed chord silently becomes its own motion.
    #[test]
    fn an_unbound_ctrl_chord_does_not_fall_through_to_the_motion() {
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
        assert_ne!(app.pending_op, Some(crate::vim::Operator::Yank), "Ctrl+Y armed the yank operator instead of redoing");
    }

    // --- the plain motions these share a letter with must still work ---

    #[test]
    fn plain_b_still_moves_a_word_back() {
        let mut app = editor_focused_app();
        press(&mut app, 'b', KeyModifiers::NONE);
        assert!(app.editor_cursor.1 < 10, "plain 'b' should move the cursor back a word");
    }

    #[test]
    fn plain_y_arms_the_yank_operator() {
        let mut app = editor_focused_app();
        press(&mut app, 'y', KeyModifiers::NONE);
        assert_eq!(
            app.pending_op,
            Some(crate::vim::Operator::Yank),
            "plain 'y' should arm the yank operator"
        );
    }

    // --- operator sequences, driven the way a user types them ---

    /// An editor with `content`, cursor at (row, col), ready for normal-mode keys.
    fn editor_with(content: &str, cursor: (u16, u16)) -> App {
        let mut app = editor_focused_app();
        app.editor_content = content.to_string();
        app.editor_cursor = cursor;
        app
    }

    fn type_keys(app: &mut App, keys: &str) {
        for c in keys.chars() {
            press(app, c, KeyModifiers::NONE);
        }
    }


    /// Replace edited the note in memory and told you it had, but queued no disk
    /// write — so quitting threw the replacement away unless something else had
    /// happened to mark the note dirty first. It persisted by luck.
    #[test]
    fn replacing_text_queues_the_write() {
        let mut app = editor_focused_app();
        let id = app.current_note.as_ref().unwrap().id;
        for target in [
            app.notebook.notes.get_mut(&id).unwrap(),
            app.current_note.as_mut().unwrap(),
        ] {
            target.content = "alpha beta alpha".to_string();
        }
        app.editor_content = "alpha beta alpha".to_string();

        // Nothing owed to the disk, so anything queued came from the replace.
        app.disk = crate::app::DiskState::default();
        app.save_status = crate::app::SaveStatus::Saved;

        app.mode = AppMode::SearchReplace;
        app.input_buffer = "alpha|OMEGA".to_string();
        handle_replace_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.editor_content, "OMEGA beta OMEGA");
        assert_eq!(
            app.notebook.notes.get(&id).unwrap().content,
            "OMEGA beta OMEGA",
            "the notebook kept the old text"
        );
        assert!(app.disk.dirty_note_ids.contains(&id), "the replacement was never queued");
        assert!(app.disk.pending_disk_save, "no disk write was requested");
    }

    /// A replace that matches nothing must not dirty the note.
    #[test]
    fn a_replace_with_no_matches_queues_nothing() {
        let mut app = editor_focused_app();
        app.disk = crate::app::DiskState::default();
        app.mode = AppMode::SearchReplace;
        app.input_buffer = "zzzznotpresent|x".to_string();
        handle_replace_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.disk.dirty_note_ids.is_empty(), "a no-op replace dirtied the note");
    }

    #[test]
    fn dw_deletes_a_word() {
        let mut app = editor_with("alpha beta gamma\n", (0, 6));
        type_keys(&mut app, "dw");
        assert_eq!(app.editor_content, "alpha gamma\n");
    }

    #[test]
    fn dd_deletes_the_line() {
        let mut app = editor_with("one\ntwo\nthree\n", (1, 0));
        type_keys(&mut app, "dd");
        assert_eq!(app.editor_content, "one\nthree\n");
    }

    /// The whole point of the operator-pending model: a count typed between the
    /// operator and the motion, which the old two-key sequences could not express.
    #[test]
    fn a_count_between_operator_and_motion_works() {
        let mut app = editor_with("alpha beta gamma delta\n", (0, 0));
        type_keys(&mut app, "d3w");
        assert_eq!(app.editor_content, "delta\n");
    }

    #[test]
    fn a_count_before_the_operator_works_too() {
        let mut app = editor_with("one\ntwo\nthree\nfour\n", (0, 0));
        type_keys(&mut app, "3dd");
        assert_eq!(app.editor_content, "four\n");
    }

    #[test]
    fn ciw_changes_the_word_and_enters_insert() {
        let mut app = editor_with("alpha beta gamma\n", (0, 7));
        type_keys(&mut app, "ciw");
        assert_eq!(app.editor_content, "alpha  gamma\n");
        assert_eq!(app.mode, AppMode::Insert, "c should leave you in insert mode");
    }

    /// `cc` empties the line rather than removing it — vim leaves you a line to
    /// type on rather than pulling the next one up under the cursor.
    #[test]
    fn cc_empties_the_line_without_removing_it() {
        let mut app = editor_with("one\ntwo\nthree\n", (1, 1));
        type_keys(&mut app, "cc");
        assert_eq!(app.editor_content, "one\n\nthree\n");
        assert_eq!(app.mode, AppMode::Insert);
    }

    #[test]
    fn yank_leaves_the_text_alone_and_fills_the_register() {
        let mut app = editor_with("alpha beta gamma\n", (0, 6));
        type_keys(&mut app, "yw");
        assert_eq!(app.editor_content, "alpha beta gamma\n", "yank must not edit");
        assert_eq!(app.yank_buffer, "beta ");
        assert!(!app.yank_linewise, "a word yank is charwise");
    }

    /// A charwise yank pastes back inline. Pasting it onto its own line — the way a
    /// linewise yank goes — would mangle the sentence it came out of.
    #[test]
    fn a_charwise_yank_pastes_inline() {
        let mut app = editor_with("alpha beta\n", (0, 0));
        type_keys(&mut app, "yiw");
        app.editor_cursor = (0, 4);
        app.paste_below();
        assert_eq!(app.editor_content, "alphaalpha beta\n");
    }

    #[test]
    fn yy_still_yanks_a_whole_line_and_pastes_as_one() {
        let mut app = editor_with("one\ntwo\n", (0, 0));
        type_keys(&mut app, "yy");
        assert_eq!(app.yank_buffer, "one");
        assert!(app.yank_linewise);
        app.paste_below();
        assert_eq!(app.editor_content, "one\none\ntwo\n");
    }

    #[test]
    fn shift_d_deletes_to_the_end_of_the_line() {
        let mut app = editor_with("keep this drop this\n", (0, 10));
        press(&mut app, 'D', KeyModifiers::NONE);
        assert_eq!(app.editor_content, "keep this \n");
    }

    /// Esc has to abandon a half-typed operator, or the next motion key silently
    /// deletes something.
    #[test]
    fn esc_abandons_a_pending_operator() {
        let mut app = editor_with("alpha beta\n", (0, 0));
        press(&mut app, 'd', KeyModifiers::NONE);
        handle_normal_mode(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.pending_op.is_none(), "Esc left the operator armed");
        type_keys(&mut app, "w");
        assert_eq!(app.editor_content, "alpha beta\n", "the w after Esc deleted something");
    }

    /// A key that is not a motion cancels the operator rather than acting on its
    /// own — otherwise a mistyped `dq` quits the app.
    #[test]
    fn a_non_motion_cancels_the_operator_instead_of_acting() {
        let mut app = editor_with("alpha beta\n", (0, 0));
        type_keys(&mut app, "dq");
        assert!(app.pending_op.is_none());
        assert!(!app.should_quit, "the q of a mistyped dq quit the app");
        assert_eq!(app.editor_content, "alpha beta\n");
    }

    /// Outside the editor `d` still means "delete the selected item", and digits
    /// still belong to the landing page rather than to a count.
    #[test]
    fn operators_do_not_leak_outside_the_editor() {
        let mut app = editor_focused_app();
        app.focused_pane = FocusedPane::Folders;
        press(&mut app, 'd', KeyModifiers::NONE);
        assert!(app.pending_op.is_none(), "an operator armed outside the editor");
    }

    /// A motion with nowhere to go must not register as an edit, or `dw` at the end
    /// of a note costs an undo step for having done nothing.
    #[test]
    fn an_operator_that_does_nothing_pushes_no_undo_snapshot() {
        let mut app = editor_with("word", (0, 4));
        let before = app.undo_stack.len();
        type_keys(&mut app, "dw");
        assert_eq!(app.editor_content, "word");
        assert_eq!(app.undo_stack.len(), before, "a no-op edit pushed an undo snapshot");
    }

    /// A count typed for a plain motion must be consumed by that motion. Leaving it
    /// armed made the *next* operator silently inherit it: `3w` then `dd` deleted
    /// three lines instead of one. Caught by driving the real binary, not by the
    /// unit tests, because each piece behaved correctly on its own.
    #[test]
    fn a_count_used_by_a_plain_motion_does_not_leak_into_the_next_operator() {
        let mut app = editor_with("one\ntwo\nthree\nfour\n", (0, 0));
        type_keys(&mut app, "3j");
        assert_eq!(app.editor_cursor.0, 3, "3j should move three lines down");
        assert!(app.pending_count.is_none(), "the count survived the motion");

        app.editor_cursor = (0, 0);
        type_keys(&mut app, "dd");
        assert_eq!(app.editor_content, "two\nthree\nfour\n", "dd deleted more than one line");
    }

    /// Even a count that goes nowhere must not linger.
    #[test]
    fn a_count_on_a_non_motion_key_is_still_cleared() {
        let mut app = editor_with("one\ntwo\nthree\nfour\n", (0, 0));
        type_keys(&mut app, "3");
        press(&mut app, 'x', KeyModifiers::NONE);
        assert!(app.pending_count.is_none(), "the count survived a non-motion key");
        type_keys(&mut app, "dd");
        assert_eq!(app.editor_content, "two\nthree\nfour\n");
    }

    #[test]
    fn an_operator_edit_can_be_undone_in_one_step() {
        let mut app = editor_with("alpha beta gamma\n", (0, 0));
        type_keys(&mut app, "d2w");
        assert_eq!(app.editor_content, "gamma\n");
        app.undo_text();
        assert_eq!(app.editor_content, "alpha beta gamma\n");
    }

    // --- the palette, driven the way a user types into it ---

    fn palette_app() -> App {
        let mut app = App::default();
        app.notebook.notes.clear();
        app.notebook.folders.clear();
        for (title, content, tags) in [
            ("Meeting Notes", "# Agenda\nbudget forecast\n", vec!["work"]),
            ("Grocery List", "milk\n", vec!["home"]),
        ] {
            let mut n = Note::new(title.to_string(), None);
            n.content = content.to_string();
            n.tags = tags.into_iter().map(String::from).collect();
            app.notebook.add_note(n);
        }
        app.focused_pane = FocusedPane::Editor;
        app
    }

    fn type_into_palette(app: &mut App, text: &str) {
        for c in text.chars() {
            handle_palette_mode(app, KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    #[test]
    fn typing_narrows_the_palette_and_enter_opens_the_note() {
        let mut app = palette_app();
        app.show_palette();
        type_into_palette(&mut app, "grocery");
        assert_eq!(app.palette_items[0].label, "Grocery List");

        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal, "palette stayed open after opening a note");
        assert_eq!(
            app.current_note.as_ref().map(|n| n.title.as_str()),
            Some("Grocery List")
        );
    }

    #[test]
    fn backspace_widens_the_search_again() {
        let mut app = palette_app();
        app.show_palette();
        type_into_palette(&mut app, "grocery");
        let narrow = app.palette_items.len();
        for _ in 0..7 {
            handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        }
        assert!(app.palette_query.is_empty());
        assert!(app.palette_items.len() > narrow, "backspace did not widen the list");
    }

    /// A new query invalidates the old selection, so Enter must land on the best
    /// match rather than on whatever row the cursor happened to be sitting at.
    #[test]
    fn the_selection_returns_to_the_top_as_the_query_changes() {
        let mut app = palette_app();
        app.show_palette();
        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.palette_selected, 1);
        type_into_palette(&mut app, "m");
        assert_eq!(app.palette_selected, 0, "stale selection survived a new query");
    }

    /// Picking a tag narrows rather than closing, and the narrowing lives entirely
    /// in the query so typing more keeps filtering.
    #[test]
    fn choosing_a_tag_narrows_to_its_notes() {
        let mut app = palette_app();
        app.show_palette();
        type_into_palette(&mut app, "#work");
        assert_eq!(app.palette_items[0].label, "#work");

        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Palette, "palette closed on choosing a tag");
        assert_eq!(app.palette_query, "#work ");
        assert_eq!(app.palette_items[0].label, "Meeting Notes");
    }

    /// The task panel is reachable from the palette as well as by chord — the
    /// palette is meant to be the one door for everything, not for most things.
    #[test]
    fn the_task_panel_can_be_opened_from_the_palette() {
        let mut app = palette_app();
        let mut n = Note::new("Chores".to_string(), None);
        n.content = "- [ ] something to do\n".to_string();
        app.notebook.add_note(n);

        app.show_palette();
        type_into_palette(&mut app, ">open tasks");
        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Tasks, "the palette did not open the task panel");
    }

    #[test]
    fn a_command_can_be_run_by_name() {
        let mut app = palette_app();
        app.show_palette();
        type_into_palette(&mut app, ">quit");
        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.should_quit, "the quit command did not run");
    }

    #[test]
    fn esc_closes_the_palette_and_forgets_the_query() {
        let mut app = palette_app();
        app.show_palette();
        type_into_palette(&mut app, "groc");
        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.palette_query.is_empty());
        assert!(app.palette_items.is_empty());
    }

    /// Ctrl+N and Ctrl+P move the selection; plain n and p have to stay typable,
    /// because a note title contains them.
    #[test]
    fn ctrl_n_navigates_while_plain_n_types() {
        let mut app = palette_app();
        app.show_palette();
        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert_eq!(app.palette_selected, 1);
        assert!(app.palette_query.is_empty(), "Ctrl+N was typed into the query");

        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(app.palette_query, "n", "plain n did not reach the query");
    }

    /// Full text is a different question from titles, and has to point at the line.
    #[test]
    fn full_text_search_jumps_to_the_matching_line() {
        let mut app = palette_app();
        app.show_palette();
        type_into_palette(&mut app, "?budget");
        assert_eq!(app.palette_items[0].detail, "Meeting Notes");

        handle_palette_mode(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            app.current_note.as_ref().map(|n| n.title.as_str()),
            Some("Meeting Notes")
        );
        assert_eq!(app.editor_cursor.0, 1, "cursor did not land on the matching line");
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
    /// The palette prints the chord each command already has, so it teaches the
    /// shortcut rather than replacing it — which only works if the chord it prints
    /// is the one the keymap actually binds. It drifted the moment Ctrl+P moved
    /// from the preview toggle to the palette, and a screenshot caught it rather
    /// than a test.
    #[test]
    fn every_palette_command_prints_the_chord_the_keymap_binds() {
        use crate::palette::Command;

        // The action each palette command stands for, where the keymap has one.
        let pairs = [
            (Command::DailyNote, Action::DailyNote),
            (Command::Tasks, Action::ShowTasks),
            (Command::Outline, Action::ShowOutline),
            (Command::Explorer, Action::ShowExplorer),
            (Command::RecentFiles, Action::RecentFiles),
            (Command::TagBrowser, Action::TagBrowser),
            (Command::ThemeBrowser, Action::ThemeBrowser),
            (Command::VaultSwitcher, Action::VaultSwitcher),
            (Command::TogglePreview, Action::TogglePreview),
            (Command::SaveNote, Action::SaveNote),
        ];

        for (cmd, action) in pairs {
            let bound: Vec<String> = NORMAL_BINDINGS
                .iter()
                .filter(|b| b.action == action)
                .filter_map(|b| match (b.code, b.mods) {
                    (KeyCode::Char(c), KeyModifiers::CONTROL) => {
                        Some(format!("Ctrl+{}", c.to_ascii_uppercase()))
                    }
                    (KeyCode::F(n), _) => Some(format!("F{}", n)),
                    (KeyCode::Char(c), KeyModifiers::NONE) => Some(c.to_string()),
                    _ => None,
                })
                .collect();
            assert!(
                bound.iter().any(|k| k == cmd.chord()),
                "palette shows {:?} for {:?}, but the keymap binds {:?}",
                cmd.chord(),
                cmd,
                bound
            );
        }
    }

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

    /// The landing page owns j/k/Enter while it is up, and gives them back the
    /// moment a note is open — otherwise Enter in the tree would stop working.
    #[test]
    fn landing_page_claims_navigation_only_while_it_is_up() {
        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);

        assert_eq!(lookup(&j, false, true), Some(Action::WelcomeDown));
        assert_eq!(lookup(&enter, false, true), Some(Action::WelcomeActivate));

        // The explorer is Ctrl+E everywhere; plain `e` stays external-editor, so
        // the landing page must not shadow it.
        assert_eq!(lookup(&ctrl_e, false, true), Some(Action::ShowExplorer));
        assert_eq!(lookup(&ctrl_e, false, false), Some(Action::ShowExplorer));
        assert_eq!(lookup(&e, false, true), Some(Action::ExternalEditor));

        // With a note open the same keys revert to their normal meanings.
        assert_eq!(lookup(&j, false, false), Some(Action::CursorDown));
        assert_eq!(lookup(&enter, false, false), Some(Action::ActivateSelected));
        assert_eq!(lookup(&e, false, false), Some(Action::ExternalEditor));
    }

    /// `q` backs out one level rather than always quitting, so a note returns to
    /// the landing page and only the landing page exits. `Q` skips the ladder.
    #[test]
    fn q_backs_out_one_level_and_shift_q_always_quits() {
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let shift_q = KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE);

        assert_eq!(lookup(&q, false, true), Some(Action::Quit), "q on the landing page quits");
        assert_eq!(lookup(&q, false, false), Some(Action::GoHome), "q in a note goes home");
        assert_eq!(lookup(&q, true, false), Some(Action::GoHome), "including with the editor focused");

        assert_eq!(lookup(&shift_q, false, true), Some(Action::Quit));
        assert_eq!(lookup(&shift_q, false, false), Some(Action::Quit));
        assert_eq!(lookup(&shift_q, true, false), Some(Action::Quit));
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
            ('o', Action::RecentFiles),
            ('p', Action::ShowPalette),
            ('g', Action::ShowOutline),
            ('e', Action::ShowExplorer),
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
        // Whatever plain `q` resolves to in that context, Alt+q must reach it too.
        let alt_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT);
        let plain_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(
            lookup(&alt_q, false, true),
            lookup(&plain_q, false, true),
            "Esc-then-q on the landing page must still quit"
        );
        assert_eq!(
            lookup(&alt_q, false, false),
            lookup(&plain_q, false, false),
            "and must still back out of a note"
        );
        assert_eq!(lookup(&alt_q, false, true), Some(Action::Quit));

        // The fallback must not paper over Ctrl: Ctrl+Alt+P is not Ctrl+P.
        let ctrl_alt_p = KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_eq!(lookup(&ctrl_alt_p, false, false), Some(Action::ShowPalette));
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

/// What a key does depends on which pane it was meant for. These are the ones
/// that were aimed at the wrong one.
#[cfg(test)]
mod pane_targeting_tests {
    use super::*;
    use crate::models::Note;

    /// Two notes, the first open, the tree's highlight moved to the other by hand.
    fn app_with(pane: FocusedPane, sidebar: bool) -> (App, String, String) {
        let mut app = App::default();
        let open = Note::new("Open".to_string(), None);
        let other = Note::new("Other".to_string(), None);
        let (open_id, other_id) = (open.id, other.id);
        app.notebook.add_note(open);
        app.notebook.add_note(other);
        app.refresh_tree_view();
        app.select_note(open_id);
        app.editor_content = "line one\nline two\nline three".to_string();
        app.selected_folder_index = app
            .folder_tree_items
            .iter()
            .position(|i| i.id == other_id)
            .unwrap();
        app.config.ui.show_sidebar = sidebar;
        app.focused_pane = pane;
        app.mode = AppMode::Normal;
        (app, "Open".to_string(), "Other".to_string())
    }

    fn press(app: &mut App, c: char) {
        handle_normal_mode(app, KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    /// Re-opening the note already open must keep what is in the buffer. The
    /// autosave is a two-second debounce, so reloading from the notebook throws
    /// away everything typed since it last fired.
    #[test]
    fn reopening_the_open_note_keeps_unsaved_edits() {
        let (mut app, _, _) = app_with(FocusedPane::Folders, true);
        app.editor_content = "typed but not saved".to_string();
        app.mark_modified();
        let id = app.current_note.as_ref().unwrap().id;

        app.select_note(id);

        assert_eq!(app.editor_content, "typed but not saved", "the buffer was reloaded");
    }

    /// And switching away has to take the buffer with it, or the note being left
    /// loses everything since the last autosave.
    #[test]
    fn switching_notes_keeps_the_buffer_of_the_one_being_left() {
        let (mut app, _, _) = app_with(FocusedPane::Folders, true);
        let leaving = app.current_note.as_ref().unwrap().id;
        app.editor_content = "typed but not saved".to_string();
        app.mark_modified();
        let target = app
            .notebook
            .notes
            .keys()
            .copied()
            .find(|id| *id != leaving)
            .unwrap();

        app.select_note(target);

        assert_eq!(
            app.notebook.notes.get(&leaving).unwrap().content,
            "typed but not saved",
            "the note being left lost the buffer"
        );
        assert!(app.disk.dirty_note_ids.contains(&leaving), "no write was queued");
    }

    /// From the tree, `r` and `m` mean the row you highlighted.
    #[test]
    fn rename_from_the_tree_acts_on_the_highlighted_item() {
        let (mut app, _, other) = app_with(FocusedPane::Folders, true);
        press(&mut app, 'r');
        assert_eq!(app.input_buffer, other, "renamed the note on screen instead");
    }

    #[test]
    fn move_from_the_tree_acts_on_the_highlighted_item() {
        let (mut app, _, other) = app_with(FocusedPane::Folders, true);
        press(&mut app, 'm');
        assert_eq!(
            app.get_selected_item().map(|i| i.name.clone()),
            Some(other),
            "moved the note on screen instead"
        );
    }

    /// From the editor or the preview the tree's highlight may be invisible, so
    /// these act on the note in front of you.
    #[test]
    fn rename_away_from_the_tree_acts_on_the_open_note() {
        for pane in [FocusedPane::Editor, FocusedPane::Preview] {
            let (mut app, open, _) = app_with(pane.clone(), true);
            press(&mut app, 'r');
            assert_eq!(app.input_buffer, open, "renamed something else from {:?}", pane);
        }
    }

    /// `d` used to disagree with its siblings: from the preview it offered to
    /// delete the tree's highlight while `r` renamed the note on screen.
    #[test]
    fn delete_from_the_preview_targets_the_note_on_screen() {
        let (mut app, open, _) = app_with(FocusedPane::Preview, true);
        press(&mut app, 'd');
        assert_eq!(app.mode, AppMode::DeleteConfirm);
        assert_eq!(
            app.get_selected_item().map(|i| i.name.clone()),
            Some(open),
            "offered to delete a note that is not on screen"
        );
    }

    #[test]
    fn delete_from_the_tree_targets_the_highlighted_item() {
        let (mut app, _, other) = app_with(FocusedPane::Folders, true);
        press(&mut app, 'd');
        assert_eq!(app.mode, AppMode::DeleteConfirm);
        assert_eq!(app.get_selected_item().map(|i| i.name.clone()), Some(other));
    }

    /// With the sidebar hidden — the default — the tree is a Ctrl+E overlay and
    /// is never drawn. Focus has no business there.
    #[test]
    fn tab_skips_a_sidebar_that_is_not_drawn() {
        let (mut app, _, _) = app_with(FocusedPane::Editor, false);
        for _ in 0..4 {
            handle_normal_mode(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
            assert_ne!(
                app.focused_pane,
                FocusedPane::Folders,
                "Tab moved focus into a pane nobody can see"
            );
        }
    }

    #[test]
    fn tab_still_reaches_a_sidebar_that_is_drawn() {
        let (mut app, _, _) = app_with(FocusedPane::Editor, true);
        let mut seen = false;
        for _ in 0..4 {
            handle_normal_mode(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
            seen |= app.focused_pane == FocusedPane::Folders;
        }
        assert!(seen, "Tab never reached the tree");
    }

    /// And with it hidden, a command aimed at the selection means the open note
    /// even if focus somehow sits on the tree.
    #[test]
    fn a_hidden_tree_never_becomes_the_target() {
        let (mut app, open, _) = app_with(FocusedPane::Folders, false);
        press(&mut app, 'r');
        assert_eq!(app.input_buffer, open, "acted on an invisible highlight");
    }

    /// j/k scroll the preview pane, so paging must too.
    #[test]
    fn paging_follows_the_focused_pane() {
        let keys = [
            KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        ];
        for key in keys {
            let (mut app, _, _) = app_with(FocusedPane::Preview, true);
            app.editor_content = (1..=40).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
            handle_normal_mode(&mut app, key);
            assert_eq!(app.editor_scroll, 0, "paging in the preview scrolled the editor");
            assert!(app.preview_scroll > 0, "the preview did not move");
        }
    }

    #[test]
    fn paging_back_up_follows_the_focused_pane_too() {
        let (mut app, _, _) = app_with(FocusedPane::Preview, true);
        app.editor_content = (1..=40).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        app.editor_scroll = 10;
        app.preview_scroll = 10;
        handle_normal_mode(&mut app, KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        assert_eq!(app.editor_scroll, 10, "paging in the preview scrolled the editor");
        assert!(app.preview_scroll < 10, "the preview did not move");
    }
}

/// The other half of the performance guardrails; see `storage::perf_tests` for
/// why each one is measured two ways.
#[cfg(test)]
mod perf_tests {
    use super::*;
    use crate::models::Note;
    use std::time::{Duration, Instant};

    fn app_with_lines(lines: usize) -> App {
        let content: String = (1..=lines)
            .map(|i| format!("line {} with a reasonable amount of text on it\n", i))
            .collect();
        let mut app = App::default();
        let mut note = Note::new("Big".to_string(), None);
        note.content = content.clone();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = content;
        // The far end of the note is the expensive place to type: everything that
        // converts the cursor to a byte offset walks the lines above it.
        app.editor_cursor = (lines as u16 - 1, 5);
        app.mode = AppMode::Insert;
        app
    }

    /// Median cost of one keystroke, sampled under a deadline.
    ///
    /// The deadline is what makes a bad regression fail fast: a keystroke that
    /// has gone quadratic takes seconds, and collecting a full sample set of
    /// those would hang the run instead of failing it.
    fn median_keystroke(app: &mut App, samples: usize) -> Duration {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let deadline = Instant::now() + Duration::from_secs(2);
        for _ in 0..20 {
            if Instant::now() >= deadline {
                break;
            }
            handle_insert_mode(app, key);
        }
        let mut times: Vec<Duration> = Vec::with_capacity(samples);
        for _ in 0..samples {
            let t = Instant::now();
            handle_insert_mode(app, key);
            times.push(t.elapsed());
            if Instant::now() >= deadline {
                break;
            }
        }
        times.sort();
        times[times.len() / 2]
    }

    /// Typing has to stay imperceptible in a long note. Measured at ~0.7ms at the
    /// end of a 5,000-line note in a debug build; a keystroke you can feel is
    /// somewhere north of 15ms, and the budget sits between the two.
    #[test]
    fn a_keystroke_in_a_long_note_stays_within_budget() {
        let mut app = app_with_lines(5000);
        let took = median_keystroke(&mut app, 101);

        assert!(
            took < Duration::from_millis(10),
            "a keystroke at the end of a 5,000-line note took {:?}, budget is 10ms",
            took
        );
    }

    /// Ten times the note must not cost far more than ten times the keystroke.
    /// Measured at 9.8x — very nearly linear — against a threshold of 25. An
    /// injected cursor conversion per line took it to 95x, and to 1.8s per
    /// keystroke.
    #[test]
    fn keystroke_cost_scales_with_the_length_of_the_note() {
        let short = median_keystroke(&mut app_with_lines(500), 101);
        let long = median_keystroke(&mut app_with_lines(5000), 101);

        let ratio = long.as_secs_f64() / short.as_secs_f64().max(1e-9);
        assert!(
            ratio < 25.0,
            "10x the note cost {:.1}x the keystroke ({:?} -> {:?}); typing is no longer linear in note size",
            ratio,
            short,
            long
        );
    }
}
