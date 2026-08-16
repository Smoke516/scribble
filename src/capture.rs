//! Headless capture: creating notes and appending to today's daily note straight
//! from the command line, without starting the TUI.
//!
//! Everything here goes through the same `VaultStorage` the app uses, so a captured
//! note is an ordinary note — same frontmatter, same atomic write, same identity
//! rules. The point is only to skip the terminal setup, because a thought worth
//! capturing rarely survives waiting for an editor to open.

use chrono::Local;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::models::{Folder, Note, NotebookData};
use crate::storage::{NotebookStorage, VaultStorage};

/// How far a title may run before it stops being a title and starts being the note.
const MAX_TITLE_LEN: usize = 60;

#[derive(Debug)]
pub enum CaptureError {
    NoVault,
    NoText,
    Storage(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoVault => write!(
                f,
                "no vault to capture into.\n\
                 Set one with --vault <path>, or set vaults.default in the config."
            ),
            Self::NoText => write!(
                f,
                "nothing to capture. Pass the text as an argument, or pipe it in."
            ),
            Self::Storage(e) => write!(f, "{}", e),
        }
    }
}

/// Take the capture text from the argument, or from stdin when it is piped.
///
/// The piped form is what makes capture composable — `git log -1 | scribble -n`,
/// or a shell function that pipes in the output of something. Reading stdin
/// unconditionally would hang an interactive `scribble -n` with no argument, so the
/// terminal check is what keeps that an error rather than a freeze.
pub fn resolve_text(arg: Option<String>) -> Result<String, CaptureError> {
    if let Some(text) = arg {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }

    if std::io::stdin().is_terminal() {
        return Err(CaptureError::NoText);
    }

    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| CaptureError::Storage(e.to_string()))?;

    if buf.trim().is_empty() {
        return Err(CaptureError::NoText);
    }
    Ok(buf)
}

/// Derive a note title from its opening line.
///
/// Truncation prefers a word boundary, because a title cut mid-word reads like a
/// bug every time you scroll past it in the note list. The full text still lands in
/// the body, so nothing is lost to the shortening.
fn title_from(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("").trim();
    let stripped = first_line.trim_start_matches(['#', '-', '*', ' ']).trim();
    let source = if stripped.is_empty() { first_line } else { stripped };

    if source.chars().count() <= MAX_TITLE_LEN {
        return source.to_string();
    }

    let cut: String = source.chars().take(MAX_TITLE_LEN).collect();
    match cut.rsplit_once(' ') {
        Some((head, _)) if head.chars().count() >= MAX_TITLE_LEN / 2 => head.to_string(),
        _ => cut,
    }
}

/// Open the vault and read the notebook, in one step, with errors the CLI can print.
fn open_vault(vault: &Path) -> Result<(VaultStorage, NotebookData), CaptureError> {
    let storage =
        VaultStorage::new(vault.to_path_buf()).map_err(|e| CaptureError::Storage(e.to_string()))?;
    let notebook = storage
        .load_notebook()
        .map_err(|e| CaptureError::Storage(e.to_string()))?;
    Ok((storage, notebook))
}

/// Write one dirty note and return the path it landed on.
fn commit(
    storage: &VaultStorage,
    notebook: &NotebookData,
    note_id: uuid::Uuid,
) -> Result<PathBuf, CaptureError> {
    let assigned = storage
        .save_incremental(notebook, &[note_id], &[])
        .map_err(|e| CaptureError::Storage(e.to_string()))?;

    if let Some((_, path)) = assigned.into_iter().find(|(id, _)| *id == note_id) {
        return Ok(path);
    }
    // An existing note keeps the path it already had; save_incremental only reports
    // paths it had to invent.
    notebook
        .notes
        .get(&note_id)
        .and_then(|n| n.file_path.clone())
        .ok_or_else(|| CaptureError::Storage("note was written but has no path".into()))
}

/// `scribble -n "..."` — create a note and return where it landed.
pub fn new_note(vault: &Path, text: &str) -> Result<PathBuf, CaptureError> {
    let (storage, mut notebook) = open_vault(vault)?;

    let mut note = Note::new(title_from(text), None);
    note.content = ensure_trailing_newline(text);
    let id = note.id;
    notebook.add_note(note);

    commit(&storage, &notebook, id)
}

/// Find the folder daily notes live in, creating it if this is the first one.
///
/// An empty `daily_folder` means the vault root, which is the escape hatch for
/// anyone who does not want their dailies filed away.
pub fn daily_folder_id(notebook: &mut NotebookData, folder_name: &str) -> Option<uuid::Uuid> {
    if folder_name.is_empty() {
        return None;
    }

    let existing = notebook
        .folders
        .values()
        .find(|f| f.parent_id.is_none() && f.name == folder_name)
        .map(|f| f.id);

    if let Some(id) = existing {
        return Some(id);
    }

    let folder = Folder::new(folder_name.to_string(), None);
    let id = folder.id;
    notebook.add_folder(folder);
    Some(id)
}

/// `scribble --today [text]` — resolve today's note, appending `text` if given.
///
/// Returns the note's path either way, so the caller can print it for a capture or
/// open the TUI on it when there was nothing to append.
pub fn today_note(
    vault: &Path,
    config: &Config,
    text: Option<&str>,
) -> Result<PathBuf, CaptureError> {
    let (storage, mut notebook) = open_vault(vault)?;

    let now = Local::now();
    let title = now.format(&config.capture.daily_format).to_string();

    // Match by title wherever the note lives, exactly as F4 does in the app. Keying
    // on the folder as well would mean that a daily note started with F4 (at the
    // root) is invisible to `--today`, which would then cheerfully create a second
    // note for the same day — and daily_folder could never be changed without
    // orphaning every daily note already written.
    let id = match notebook.find_note_by_title(&title) {
        Some(id) => id,
        None => {
            let folder_id = daily_folder_id(&mut notebook, &config.capture.daily_folder);
            let mut note = Note::new(title, folder_id);
            note.content = String::new();
            let id = note.id;
            notebook.add_note(note);
            id
        }
    };

    if let Some(text) = text {
        let entry = format_entry(text, config.capture.timestamp_entries, now);
        let note = notebook.notes.get_mut(&id).expect("just inserted or found");
        let mut content = std::mem::take(&mut note.content);
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&entry);
        note.update_content(content);
    }

    commit(&storage, &notebook, id)
}

/// Render one captured entry as it will appear in the daily note.
///
/// Entries are bulleted so a day's captures read as a list rather than as one
/// run-on paragraph, and timestamped because in a daily log *when* is usually half
/// of what the entry means. Multi-line text is indented under its own bullet so the
/// list structure survives.
fn format_entry(text: &str, timestamp: bool, now: chrono::DateTime<Local>) -> String {
    let mut lines = text.trim_end().lines();
    let first = lines.next().unwrap_or("").trim_end();

    let mut out = String::new();
    if timestamp {
        out.push_str(&format!("- {} {}\n", now.format("%H:%M"), first));
    } else {
        out.push_str(&format!("- {}\n", first));
    }
    for line in lines {
        out.push_str(&format!("  {}\n", line.trim_end()));
    }
    out
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{}\n", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scribble_cap_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn title_comes_from_the_first_line() {
        assert_eq!(title_from("buy milk\nand eggs"), "buy milk");
    }

    /// Markdown leaders are how people naturally type a capture; keeping them would
    /// put a literal "# " or "- " in the note list.
    #[test]
    fn title_drops_markdown_leaders() {
        assert_eq!(title_from("## Standup notes"), "Standup notes");
        assert_eq!(title_from("- buy milk"), "buy milk");
    }

    /// A title cut mid-word reads like a bug every time it scrolls past.
    #[test]
    fn long_titles_break_on_a_word_boundary() {
        let text = "the quick brown fox jumps over the lazy dog and keeps on running well past sixty";
        let title = title_from(text);
        assert!(title.chars().count() <= MAX_TITLE_LEN, "title too long: {:?}", title);
        assert!(!title.ends_with(' '));
        assert!(
            text.starts_with(&title),
            "title is not a prefix of the text: {:?}",
            title
        );
        assert!(
            text[title.len()..].starts_with(' '),
            "title stopped mid-word: {:?}",
            title
        );
    }

    /// A word longer than the limit has no boundary to break on, and must still be
    /// cut rather than left to run.
    #[test]
    fn an_unbroken_run_is_still_truncated() {
        let text = "a".repeat(200);
        assert_eq!(title_from(&text).chars().count(), MAX_TITLE_LEN);
    }

    #[test]
    fn a_captured_note_is_an_ordinary_note() {
        let dir = scratch("new");
        let path = new_note(&dir, "buy milk\nand eggs").unwrap();

        let on_disk = fs::read_to_string(&path).unwrap();
        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();
        let note = nb.notes.values().next().unwrap().clone();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(path.file_name().unwrap(), "buy milk.md");
        assert!(on_disk.starts_with("---\n"), "no frontmatter: {:?}", on_disk);
        assert_eq!(note.title, "buy milk");
        assert_eq!(note.content, "buy milk\nand eggs\n");
    }

    /// By default the daily note goes to the vault root, which is where F4 has
    /// always put it. Anything else would split one day across two files depending
    /// on which entry point got there first.
    #[test]
    fn today_creates_the_daily_note_at_the_root_by_default() {
        let dir = scratch("today");
        let config = Config::default();
        let path = today_note(&dir, &config, Some("first thought")).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(path.parent().unwrap(), dir, "daily note was not at the vault root");
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), format!("{}.md", today));
        assert!(body.contains("first thought"), "entry missing: {:?}", body);
        assert!(body.contains("- "), "entry was not bulleted: {:?}", body);
    }

    /// Setting `daily_folder` files new daily notes away, creating the folder on
    /// the first one.
    #[test]
    fn daily_folder_is_configurable() {
        let dir = scratch("dailyfolder");
        let mut config = Config::default();
        config.capture.daily_folder = "daily".to_string();
        let path = today_note(&dir, &config, Some("filed away")).unwrap();

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(path.parent().unwrap().file_name().unwrap(), "daily");
    }

    /// A daily note started with F4 lives at the root. `--today` has to find *that*
    /// note rather than starting a second one, and must keep doing so even once
    /// daily_folder has been set — otherwise changing the setting orphans every
    /// daily note already written.
    #[test]
    fn today_finds_an_existing_daily_note_outside_the_configured_folder() {
        let dir = scratch("existing");
        let today = Local::now().format("%Y-%m-%d").to_string();
        // Stand in for what F4 leaves behind: the daily note, at the vault root.
        fs::write(
            dir.join(format!("{}.md", today)),
            format!("---\ntitle: {}\n---\nwritten in the app\n", today),
        )
        .unwrap();

        let mut config = Config::default();
        config.capture.daily_folder = "daily".to_string();
        let path = today_note(&dir, &config, Some("captured from the shell")).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        let md_count = walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
            .count();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(md_count, 1, "a second daily note was created for the same day");
        assert_eq!(path.parent().unwrap(), dir, "the note moved out from under the app");
        assert!(body.contains("written in the app"), "existing content lost: {:?}", body);
        assert!(body.contains("captured from the shell"), "entry missing: {:?}", body);
    }

    /// The second capture of the day must land in the same note, under the first —
    /// not create a second file, and not replace what is already there.
    #[test]
    fn a_second_capture_appends_to_the_same_note() {
        let dir = scratch("append");
        let config = Config::default();
        let first = today_note(&dir, &config, Some("first thought")).unwrap();
        let second = today_note(&dir, &config, Some("second thought")).unwrap();

        let body = fs::read_to_string(&second).unwrap();
        let md_count = walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
            .count();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(first, second, "capture created a second file for the same day");
        assert_eq!(md_count, 1, "expected exactly one daily note");
        assert!(body.contains("first thought"), "earlier entry lost: {:?}", body);
        assert!(body.contains("second thought"), "later entry missing: {:?}", body);
        let first_at = body.find("first thought").unwrap();
        let second_at = body.find("second thought").unwrap();
        assert!(first_at < second_at, "entries are out of order: {:?}", body);
    }

    /// Opening today's note with nothing to add must not write an entry, but must
    /// still hand back a real path for the TUI to open.
    #[test]
    fn today_without_text_creates_an_empty_note() {
        let dir = scratch("empty");
        let config = Config::default();
        let path = today_note(&dir, &config, None).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert!(path.exists() || !body.is_empty());
        assert!(!body.contains("- "), "an entry was written anyway: {:?}", body);
    }

    /// Multi-line capture must stay under its own bullet rather than flattening
    /// into the day's list as separate entries.
    #[test]
    fn multi_line_entries_are_indented_under_their_bullet() {
        let now = Local::now();
        let entry = format_entry("first line\nsecond line", false, now);
        assert_eq!(entry, "- first line\n  second line\n");
    }

    #[test]
    fn timestamps_can_be_turned_off() {
        let now = Local::now();
        assert!(format_entry("x", true, now).starts_with("- "));
        assert_eq!(format_entry("x", false, now), "- x\n");
        assert!(
            format_entry("x", true, now).len() > format_entry("x", false, now).len(),
            "timestamp had no effect"
        );
    }
}
