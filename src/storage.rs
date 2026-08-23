use crate::error::{IoResultExt, StorageError};
use crate::models::{FileStamp, NotebookData, Note, Folder};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// YAML frontmatter for markdown files.
//
// Every field is skipped when absent rather than written as `null`: these files
// are read and hand-edited by people (and by Obsidian), so a note with no tags
// should simply not mention tags. Deserialization is unaffected — a missing key
// and an explicit null both produce None.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoteFrontmatter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scribble_id: Option<String>,
    /// The note's real title. Persisted so the filename (which is sanitized and
    /// may carry a disambiguation suffix) never has to round-trip as the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    modified_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    folder_path: Option<String>,
}

/// Write `content` to `path` atomically: fill a sibling temp file, flush it to
/// the device, then rename it over the target. `fs::write` truncates before it
/// writes, so a crash mid-write leaves a zero-length note; rename is atomic on
/// POSIX, so a reader sees either the old file or the complete new one.
///
/// The temp file is dot-prefixed so a crashed run leaves something the vault
/// loader already skips rather than a stray note.
fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;

    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("path has no parent directory: {:?}", path),
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("scribble-note");
    let tmp = dir.join(format!(".{}.tmp", file_name));

    // Scope the handle so it is closed before the rename.
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

// Abstract trait for different storage backends
pub trait NotebookStorage {
    fn load_notebook(&self) -> Result<NotebookData, StorageError>;
    fn save_notebook(&self, notebook: &NotebookData) -> Result<(), StorageError>;

    /// Write only the `dirty` notes and delete `deleted_paths`. Default: fall back
    /// to a full save (correct, just not incremental) — used by single-file backends
    /// where it's already cheap.
    fn save_incremental(
        &self,
        notebook: &NotebookData,
        _dirty: &[Uuid],
        _deleted_paths: &[PathBuf],
    ) -> Result<SaveReport, StorageError> {
        self.save_notebook(notebook)?;
        Ok(SaveReport::default())
    }

    /// Move/rename a folder's directory (and all its files) on disk from one
    /// vault-relative path to another, returning the updated absolute paths for
    /// every note that lived under it. Default: no-op (single-file backends have
    /// no folder directories).
    fn relocate_folder(
        &self,
        _notebook: &NotebookData,
        _old_rel: &Path,
        _new_rel: &Path,
    ) -> Result<Vec<(Uuid, PathBuf)>, StorageError> {
        Ok(Vec::new())
    }
}

/// Split a leading YAML frontmatter block off `content`, returning `(yaml, body)`.
///
/// Both delimiters may end in either LF or CRLF. Requiring a literal `---\n` meant
/// a note that had been through a Windows editor or a sync client parsed as having
/// no frontmatter at all, so it loaded as a brand new note on every single load: a
/// fresh `scribble_id` each time, and `created_at` reset to now. It self-healed on
/// the next save, but the note's identity and true creation date were already gone.
///
/// The closing delimiter is matched as a whole line rather than by searching for a
/// fixed byte sequence, which is also what keeps the body free of the delimiter's
/// own newline — leaving that behind is what used to grow every note by a blank
/// line on each save.
fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))?;

    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == "---" {
            return Some((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// Convert CRLF to LF.
///
/// The editor is LF-only by construction: `get_cursor_byte_index` walks
/// `content.lines()` adding `line.len() + 1` for the terminator, and `lines()` has
/// already stripped the `\r`. A CRLF note would therefore under-count by one byte
/// per line, drifting the cursor further with every line above it and inserting
/// typed characters at the wrong offset. Normalising on load keeps that whole class
/// of corruption out of the editor; the file is rewritten as LF on the next save.
fn normalize_line_endings(content: &str) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n")
    } else {
        content.to_string()
    }
}

/// What a save did, beyond succeeding.
#[derive(Debug, Default)]
pub struct SaveReport {
    /// Paths chosen for notes that did not have one yet, so the caller can store
    /// them back and write to the same file next time.
    pub assigned: Vec<(Uuid, PathBuf)>,
    /// Fresh stamps for every note written. The caller must store these back, or
    /// the next save will believe its own write was somebody else's.
    pub stamps: Vec<(Uuid, FileStamp)>,
    /// Notes whose file had changed underneath us. The version that was on disk was
    /// preserved before ours went down.
    pub conflicts: Vec<Conflict>,
}

/// A file that changed under us, and where the version we found got preserved.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub note_title: String,
    pub preserved_at: PathBuf,
}

/// Whether a filename is a conflict artefact rather than a note the user made.
/// Nextcloud writes `Note (conflicted copy 2026-08-16 120000).md`; Syncthing writes
/// `Note.sync-conflict-20260816-120000-ABCDEFG.md`; we write `(scribble conflict`.
///
/// Ours is included deliberately: a preserved version is a byte copy and therefore
/// carries the original's `scribble_id`, so without this it would be the artefact
/// that keeps the contested id and the note the user is editing that gets re-minted.
fn looks_like_a_sync_conflict(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name.contains("(conflicted copy")
        || name.contains(".sync-conflict-")
        || name.contains("(scribble conflict")
}

/// Where to preserve the version of `path` that we found on disk.
///
/// Named after the note it forked from, with a timestamp, so it sorts beside the
/// original and says for itself what it is and when it happened.
fn conflict_sidecar_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let when = chrono::Local::now().format("%Y-%m-%d %H%M%S");
    let dir = path.parent().unwrap_or(Path::new("."));

    let mut candidate = dir.join(format!("{} (scribble conflict {}).md", stem, when));
    // Two conflicts on the same note within one second is vanishingly unlikely, but
    // silently overwriting the first one would defeat the entire point of this file.
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{} (scribble conflict {}-{}).md", stem, when, n));
        n += 1;
    }
    candidate
}

/// Keep two files that claim the same `scribble_id` as two separate notes.
///
/// A sync client's conflicted copy is a byte-for-byte copy, frontmatter included,
/// so it arrives carrying the *same* id as the note it forked from. `notes` is
/// keyed by id, so the second file to load used to evict the first and the pair
/// silently collapsed into one — and walk order decided the winner. `(` sorts
/// before `.`, so `Meeting (conflicted copy).md` consistently beat `Meeting.md`:
/// the real note disappeared from scribble altogether and the user carried on
/// editing the conflict without being told.
///
/// Nothing here tries to merge or resolve the conflict — that is the user's call.
/// It only guarantees that no note vanishes. The incoming note is re-minted, except
/// when the id is currently held by a conflict artefact and the incoming file is
/// the genuine note, in which case they swap so the original id stays with the
/// original note and its links and history keep resolving.
fn resolve_id_collision(notebook: &mut NotebookData, incoming: &mut Note) {
    let Some(existing) = notebook.notes.get(&incoming.id) else {
        return;
    };

    let existing_is_artefact = existing
        .file_path
        .as_deref()
        .is_some_and(looks_like_a_sync_conflict);
    let incoming_is_artefact = incoming
        .file_path
        .as_deref()
        .is_some_and(looks_like_a_sync_conflict);

    if existing_is_artefact && !incoming_is_artefact {
        // The artefact got here first only because of walk order. Hand the id back.
        let contested = incoming.id;
        let mut displaced = notebook.notes.remove(&contested).expect("just borrowed");
        displaced.id = Uuid::new_v4();
        notebook.add_note(displaced);
    } else {
        incoming.id = Uuid::new_v4();
    }
}

// Vault-based storage for Obsidian compatibility
#[derive(Debug)]
pub struct VaultStorage {
    vault_path: PathBuf,
}

impl VaultStorage {
    pub fn new(vault_path: PathBuf) -> Result<Self, StorageError> {
        if !vault_path.exists() {
            return Err(StorageError::VaultMissing(vault_path));
        }
        if !vault_path.is_dir() {
            return Err(StorageError::VaultNotDirectory(vault_path));
        }
        
        Ok(Self { vault_path })
    }
    
    fn parse_markdown_with_frontmatter(&self, content: &str) -> (Option<NoteFrontmatter>, String) {
        if let Some((yaml_content, markdown_content)) = split_frontmatter(content) {
            if let Ok(frontmatter) = serde_yaml::from_str::<NoteFrontmatter>(yaml_content) {
                return (Some(frontmatter), normalize_line_endings(markdown_content));
            }
        }
        (None, normalize_line_endings(content))
    }
    
    /// Read one note back off disk, for picking up an external change without
    /// reloading the whole vault.
    ///
    /// Returns a note with a fresh `disk_stamp` and no `folder_id`: placing a note
    /// in the tree is the caller's business, and the caller already knows where this
    /// one lives.
    pub fn load_single_note(&self, path: &Path) -> Option<Note> {
        let content = fs::read_to_string(path).ok()?;
        let stamp = Some(FileStamp::of_bytes(content.as_bytes()));
        let (frontmatter, markdown) = self.parse_markdown_with_frontmatter(&content);
        let title = path.file_stem()?.to_string_lossy().to_string();

        let mut note = Note::new(title.clone(), None);
        if let Some(fm) = frontmatter {
            if let Some(id) = fm.scribble_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()) {
                note.id = id;
            }
            note.title = fm.title.unwrap_or(title);
            note.created_at = fm.created_at.unwrap_or(note.created_at);
            note.modified_at = fm.modified_at.unwrap_or(note.modified_at);
            note.tags = fm.tags.unwrap_or_default();
        }
        note.content = markdown;
        note.file_path = Some(path.to_path_buf());
        note.disk_stamp = stamp;
        Some(note)
    }

    /// `file_path` is the path the note is actually being written to, which is not
    /// always `note.file_path`: a note being saved for the first time has none yet,
    /// and passing it in is what lets `folder_path` be correct on the first write
    /// rather than only on the second.
    fn create_markdown_with_frontmatter(&self, note: &Note, file_path: &Path, content: &str) -> String {
        let frontmatter = NoteFrontmatter {
            scribble_id: Some(note.id.to_string()),
            title: Some(note.title.clone()),
            created_at: Some(note.created_at),
            modified_at: Some(note.modified_at),
            tags: if note.tags.is_empty() { None } else { Some(note.tags.clone()) },
            // Vault-RELATIVE, and omitted entirely for notes at the vault root.
            // This used to be the absolute path, which pinned every note to one
            // machine and one user's home directory, and leaked that username into
            // any note that got shared or published.
            folder_path: file_path.parent().and_then(|parent| {
                let relative = self.get_relative_path(parent);
                if relative.as_os_str().is_empty() {
                    None
                } else {
                    Some(relative.to_string_lossy().to_string())
                }
            }),
        };
        
        if let Ok(yaml) = serde_yaml::to_string(&frontmatter) {
            format!("---\n{}---\n{}", yaml, content)
        } else {
            content.to_string()
        }
    }
    
    fn get_relative_path(&self, full_path: &Path) -> PathBuf {
        full_path.strip_prefix(&self.vault_path).unwrap_or(full_path).to_path_buf()
    }
    
    #[allow(dead_code)]
    fn create_folder_structure(&self, notebook: &NotebookData) -> HashMap<String, Uuid> {
        let mut path_to_folder_id = HashMap::new();
        
        // Walk through actual filesystem directories
        for entry in WalkDir::new(&self.vault_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path() != self.vault_path)
        {
            let relative_path = self.get_relative_path(entry.path());
            let path_str = relative_path.to_string_lossy().to_string();
            
            // Skip .obsidian and other hidden directories
            if path_str.starts_with('.') {
                continue;
            }
            
            // Find or create folder for this path
            if let Some(folder) = notebook.folders.values().find(|f| {
                f.name == relative_path.file_name().unwrap_or_default().to_string_lossy()
            }) {
                path_to_folder_id.insert(path_str, folder.id);
            }
        }
        
        path_to_folder_id
    }

    /// Absolute directory for a folder, following its parent chain.
    fn folder_dir(&self, folder: &Folder, notebook: &NotebookData) -> PathBuf {
        let mut components = Vec::new();
        let mut current = folder;
        loop {
            components.push(current.name.clone());
            match current.parent_id.and_then(|pid| notebook.folders.get(&pid)) {
                Some(parent) => current = parent,
                None => break,
            }
        }
        components.reverse();
        let mut dir = self.vault_path.clone();
        for c in components {
            dir.push(c);
        }
        dir
    }

    /// Absolute directory a note lives in (vault root, or its folder chain).
    fn note_dir(&self, folder_id: Option<Uuid>, notebook: &NotebookData) -> PathBuf {
        match folder_id.and_then(|fid| notebook.folders.get(&fid)) {
            Some(folder) => self.folder_dir(folder, notebook),
            None => self.vault_path.clone(),
        }
    }

    /// Create every folder directory in the notebook.
    fn ensure_folders(&self, notebook: &NotebookData) -> Result<(), StorageError> {
        for folder in notebook.folders.values() {
            let dir = self.folder_dir(folder, notebook);
            fs::create_dir_all(&dir).create_dir_ctx(&dir)?;
        }
        Ok(())
    }

    /// Resolve a note's on-disk path: its existing path, or a sanitized,
    /// collision-free name (disambiguated with a short id suffix). Records the
    /// chosen path in `claimed`.
    fn resolve_note_path(
        &self,
        note: &Note,
        notebook: &NotebookData,
        claimed: &mut HashSet<PathBuf>,
    ) -> PathBuf {
        let path = if let Some(existing) = &note.file_path {
            existing.clone()
        } else {
            let dir = self.note_dir(note.folder_id, notebook);
            let mut base = crate::app::sanitize_filename(&note.title);
            if base.is_empty() {
                base = "untitled".to_string();
            }
            let mut candidate = dir.join(format!("{}.md", base));
            if claimed.contains(&candidate) {
                let short = &note.id.to_string()[..8];
                candidate = dir.join(format!("{}-{}.md", base, short));
                let mut n = 2;
                while claimed.contains(&candidate) {
                    candidate = dir.join(format!("{}-{}-{}.md", base, short, n));
                    n += 1;
                }
            }
            candidate
        };
        claimed.insert(path.clone());
        path
    }

    /// Write a single note to disk, returning its resolved path, a fresh stamp, and
    /// a conflict if the file had to be preserved before we wrote over it.
    ///
    /// The rule is: **never overwrite a file we cannot account for.** If what is on
    /// disk does not match what we last read or wrote there, it is somebody else's
    /// work — another device via the sync client, a `scribble -n` capture, an
    /// editor — and it gets copied aside before our version goes down.
    ///
    /// Our version keeps the original filename rather than being diverted itself,
    /// because the person at the keyboard is mid-sentence in this note. Moving the
    /// file out from under them, or reloading somebody else's text into the buffer
    /// they are typing into, is the disruptive choice. This way nothing is lost, the
    /// editor never jumps, and the preserved file is waiting whenever they get to it.
    fn write_note(
        &self,
        note: &Note,
        notebook: &NotebookData,
        claimed: &mut HashSet<PathBuf>,
    ) -> Result<(PathBuf, Option<FileStamp>, Option<Conflict>), StorageError> {
        let file_path = self.resolve_note_path(note, notebook, claimed);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).create_dir_ctx(parent)?;
        }

        let on_disk = FileStamp::of(&file_path);
        let diverged = match (note.disk_stamp, on_disk) {
            // Nothing there: the note is new, or its file was removed. Either way
            // there is nothing to lose by writing.
            (_, None) => false,
            // We have seen this file. Is it still the one we saw?
            (Some(expected), Some(actual)) => actual != expected,
            // A file we have never read, under a name we are about to take. Writing
            // would destroy it, and we have no idea what it is.
            (None, Some(_)) => true,
        };

        let content = self.create_markdown_with_frontmatter(note, &file_path, &note.content);

        // If what is on disk is byte-for-byte what we were about to write, there is
        // no conflict to preserve — only a stale stamp. This is what keeps an
        // external edit that merely matches our own from leaving a pointless file.
        let identical = diverged && on_disk == Some(FileStamp::of_bytes(content.as_bytes()));

        let conflict = if diverged && !identical {
            let preserved_at = conflict_sidecar_path(&file_path);
            fs::copy(&file_path, &preserved_at).write_ctx(&preserved_at)?;
            Some(Conflict {
                note_title: note.title.clone(),
                preserved_at,
            })
        } else {
            None
        };

        write_atomic(&file_path, &content).write_ctx(&file_path)?;
        // Stamped from what we wrote, not by re-reading: this is the exact content
        // now on disk, and re-reading would only add a window for it to change in.
        let stamp = FileStamp::of_bytes(content.as_bytes());
        Ok((file_path, Some(stamp), conflict))
    }
}

impl NotebookStorage for VaultStorage {
    fn load_notebook(&self) -> Result<NotebookData, StorageError> {
        let mut notebook = NotebookData::new();
        let mut folders_created = HashMap::new();
        
        // Walk through the vault directory
        for entry in WalkDir::new(&self.vault_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip .obsidian and other hidden directories/files. Only the
            // vault-RELATIVE components may disqualify an entry: the vault's own
            // absolute path may legitimately sit under a dot-directory (a vault in
            // ~/.notes, or inside a dotfiles checkout), and testing those
            // components would silently skip every file in the vault.
            let relative_to_vault = self.get_relative_path(path);
            if relative_to_vault
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            {
                continue;
            }
            
            if path.is_dir() && path != self.vault_path {
                // Create folder if it doesn't exist
                let relative_path = self.get_relative_path(path);
                let folder_name = path.file_name().unwrap().to_string_lossy().to_string();
                let parent_path = relative_path.parent();
                
                let parent_id = if let Some(parent) = parent_path {
                    let parent_str = parent.to_string_lossy().to_string();
                    folders_created.get(&parent_str).copied()
                } else {
                    None
                };
                
                let folder = Folder::new(folder_name, parent_id);
                let folder_id = folder.id;
                folders_created.insert(relative_path.to_string_lossy().to_string(), folder_id);
                notebook.add_folder(folder);
            } else if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                // Process markdown file
                if let Ok(content) = fs::read_to_string(path) {
                    // Stamped from the bytes we just read, so it describes exactly
                    // the content we are about to parse — there is no window in
                    // which the file could change without us noticing.
                    let stamp = Some(FileStamp::of_bytes(content.as_bytes()));
                    let (frontmatter, markdown_content) = self.parse_markdown_with_frontmatter(&content);
                    
                    // Determine folder_id from path
                    let relative_path = self.get_relative_path(path);
                    let parent_path = relative_path.parent();
                    let folder_id = if let Some(parent) = parent_path {
                        let parent_str = parent.to_string_lossy().to_string();
                        folders_created.get(&parent_str).copied()
                    } else {
                        None
                    };
                    
                    // Create note
                    let title = path.file_stem().unwrap().to_string_lossy().to_string();
                    // Whether the note itself records when it was last modified.
                    // Checked before `fm` is consumed below.
                    let has_recorded_modified_at =
                        frontmatter.as_ref().and_then(|fm| fm.modified_at).is_some();
                    let mut note = if let Some(fm) = frontmatter {
                        // Use existing frontmatter data
                        let note_id = fm.scribble_id
                            .and_then(|s| Uuid::parse_str(&s).ok())
                            .unwrap_or_else(Uuid::new_v4);
                        
                        Note {
                            id: note_id,
                            // Prefer the real title from frontmatter; fall back to
                            // the filename for notes created before this field existed.
                            title: fm.title.unwrap_or(title),
                            content: markdown_content,
                            folder_id,
                            created_at: fm.created_at.unwrap_or_else(Utc::now),
                            modified_at: fm.modified_at.unwrap_or_else(Utc::now),
                            tags: fm.tags.unwrap_or_default(),
                            file_path: Some(path.to_path_buf()),
                            disk_stamp: stamp,
                        }
                    } else {
                        // Create new note without frontmatter
                        let mut note = Note::new(title, folder_id);
                        note.content = markdown_content;
                        note.file_path = Some(path.to_path_buf());
                        note.disk_stamp = stamp;
                        note
                    };
                    
                    // Fall back to filesystem mtime only for notes that don't record
                    // their own modified_at. A recorded timestamp must win: sync
                    // clients, fresh clones and restores rewrite mtimes wholesale,
                    // and letting the filesystem override would reset every note's
                    // modified date to whenever the files last happened to be touched.
                    if !has_recorded_modified_at {
                        if let Ok(metadata) = fs::metadata(path) {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(modified_utc) = modified.duration_since(std::time::UNIX_EPOCH) {
                                    note.modified_at = DateTime::from_timestamp(
                                        modified_utc.as_secs() as i64,
                                        modified_utc.subsec_nanos()
                                    ).unwrap_or(note.modified_at);
                                }
                            }
                        }
                    }
                    
                    resolve_id_collision(&mut notebook, &mut note);
                    notebook.add_note(note);
                }
            }
        }

        Ok(notebook)
    }
    
    fn save_notebook(&self, notebook: &NotebookData) -> Result<(), StorageError> {
        self.ensure_folders(notebook)?;

        // Iterate in id order so collision-disambiguation is stable across saves
        // (HashMap order is otherwise random, which would churn filenames).
        let mut notes: Vec<&Note> = notebook.notes.values().collect();
        notes.sort_by_key(|n| n.id);
        let mut claimed: HashSet<PathBuf> =
            notes.iter().filter_map(|n| n.file_path.clone()).collect();

        for note in notes {
            self.write_note(note, notebook, &mut claimed)?;
        }
        Ok(())
    }

    fn save_incremental(
        &self,
        notebook: &NotebookData,
        dirty: &[Uuid],
        deleted_paths: &[PathBuf],
    ) -> Result<SaveReport, StorageError> {
        // Remove files for deleted notes (ignore if already gone).
        for path in deleted_paths {
            let _ = fs::remove_file(path);
        }

        // Claim every known path so a brand-new dirty note can't collide with one.
        let mut claimed: HashSet<PathBuf> =
            notebook.notes.values().filter_map(|n| n.file_path.clone()).collect();

        let mut report = SaveReport::default();
        for id in dirty {
            if let Some(note) = notebook.notes.get(id) {
                let had_path = note.file_path.is_some();
                let (path, stamp, conflict) = self.write_note(note, notebook, &mut claimed)?;
                if !had_path {
                    report.assigned.push((*id, path));
                }
                if let Some(stamp) = stamp {
                    report.stamps.push((*id, stamp));
                }
                if let Some(conflict) = conflict {
                    report.conflicts.push(conflict);
                }
            }
        }
        Ok(report)
    }

    fn relocate_folder(
        &self,
        notebook: &NotebookData,
        old_rel: &Path,
        new_rel: &Path,
    ) -> Result<Vec<(Uuid, PathBuf)>, StorageError> {
        let old_abs = self.vault_path.join(old_rel);
        let new_abs = self.vault_path.join(new_rel);
        if old_abs == new_abs {
            return Ok(Vec::new());
        }

        if let Some(parent) = new_abs.parent() {
            fs::create_dir_all(parent).create_dir_ctx(parent)?;
        }
        if old_abs.exists() {
            // A full save may have pre-created the (empty) destination; remove it
            // so the rename can land there.
            if new_abs.exists() {
                let _ = fs::remove_dir(&new_abs);
            }
            fs::rename(&old_abs, &new_abs).map_err(|source| StorageError::Rename {
                from: old_abs.clone(),
                to: new_abs.clone(),
                source,
            })?;
        } else {
            // Folder had no directory yet (e.g. empty/never-written): just create.
            fs::create_dir_all(&new_abs).create_dir_ctx(&new_abs)?;
        }

        // Remap every note path that was under the old directory.
        let mut updated = Vec::new();
        for note in notebook.notes.values() {
            if let Some(p) = &note.file_path {
                if let Ok(rel) = p.strip_prefix(&old_abs) {
                    updated.push((note.id, new_abs.join(rel)));
                }
            }
        }
        Ok(updated)
    }
}

#[derive(Debug)]
pub struct Storage {
    data_dir: PathBuf,
    notebook_file: PathBuf,
}


impl Storage {
    pub fn new() -> Result<Self, StorageError> {
        let data_dir = Self::get_data_dir()?;
        fs::create_dir_all(&data_dir).create_dir_ctx(&data_dir)?;
        
        let notebook_file = data_dir.join("notebook.json");
        
        Ok(Self {
            data_dir,
            notebook_file,
        })
    }

    fn get_data_dir() -> Result<PathBuf, StorageError> {
        let data_dir = if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("scribble")
        } else {
            // Fallback to home directory if data_dir is not available
            if let Some(home_dir) = dirs::home_dir() {
                home_dir.join(".scribble")
            } else {
                PathBuf::from(".scribble")
            }
        };
        Ok(data_dir)
    }

    pub fn load_notebook(&self) -> Result<NotebookData, StorageError> {
        if self.notebook_file.exists() {
            let contents =
                fs::read_to_string(&self.notebook_file).read_ctx(&self.notebook_file)?;
            let notebook: NotebookData =
                serde_json::from_str(&contents).map_err(|source| StorageError::Parse {
                    path: self.notebook_file.clone(),
                    source,
                })?;
            Ok(notebook)
        } else {
            // Return empty notebook if file doesn't exist
            Ok(NotebookData::new())
        }
    }

    pub fn save_notebook(&self, notebook: &NotebookData) -> Result<(), StorageError> {
        let json = serde_json::to_string_pretty(notebook).map_err(StorageError::Serialize)?;
        // Single-file backend: a truncated write here loses the whole notebook,
        // not one note, so atomicity matters even more than in vault mode.
        write_atomic(&self.notebook_file, &json).write_ctx(&self.notebook_file)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_notes_dir(&self) -> PathBuf {
        self.data_dir.join("notes")
    }

    #[allow(dead_code)]
    pub fn export_note_to_file(&self, note_id: &str, content: &str) -> Result<PathBuf, StorageError> {
        let notes_dir = self.get_notes_dir();
        fs::create_dir_all(&notes_dir).create_dir_ctx(&notes_dir)?;
        
        let file_path = notes_dir.join(format!("{}.md", note_id));
        write_atomic(&file_path, content).write_ctx(&file_path)?;
        Ok(file_path)
    }

    #[allow(dead_code)]
    pub fn import_note_from_file(&self, file_path: &PathBuf) -> Result<String, StorageError> {
        let content = fs::read_to_string(file_path).read_ctx(file_path)?;
        Ok(content)
    }

    #[allow(dead_code)]
    pub fn backup_data(&self) -> Result<PathBuf, StorageError> {
        let backup_dir = self.data_dir.join("backups");
        fs::create_dir_all(&backup_dir).create_dir_ctx(&backup_dir)?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_file = backup_dir.join(format!("notebook_backup_{}.json", timestamp));
        
        if self.notebook_file.exists() {
            fs::copy(&self.notebook_file, &backup_file).write_ctx(&backup_file)?;
        }
        
        Ok(backup_file)
    }

    #[allow(dead_code)]
    pub fn list_backups(&self) -> Result<Vec<PathBuf>, StorageError> {
        let backup_dir = self.data_dir.join("backups");
        
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(&backup_dir).read_ctx(&backup_dir)? {
            let entry = entry.read_ctx(&backup_dir)?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map(|s| s == "json").unwrap_or(false) {
                if let Some(file_name) = path.file_name() {
                    if file_name.to_string_lossy().starts_with("notebook_backup_") {
                        backups.push(path);
                    }
                }
            }
        }
        
        // Sort backups by filename (which includes timestamp)
        backups.sort();
        backups.reverse(); // Most recent first
        
        Ok(backups)
    }

    #[allow(dead_code)]
    pub fn restore_from_backup(&self, backup_file: &PathBuf) -> Result<(), StorageError> {
        if backup_file.exists() {
            fs::copy(backup_file, &self.notebook_file).write_ctx(&self.notebook_file)?;
        }
        Ok(())
    }
}

impl NotebookStorage for Storage {
    fn load_notebook(&self) -> Result<NotebookData, StorageError> {
        self.load_notebook()
    }
    
    fn save_notebook(&self, notebook: &NotebookData) -> Result<(), StorageError> {
        self.save_notebook(notebook)
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;

    /// Two same-titled notes must produce two distinct files (no overwrite), and
    /// a title containing '/' must be sanitized rather than nested into a dir.
    #[test]
    fn save_sanitizes_and_disambiguates_filenames() {
        let dir = std::env::temp_dir().join(format!("scribble_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut nb = NotebookData::new();
        nb.add_note(Note::new("Meeting Notes".to_string(), None));
        nb.add_note(Note::new("Meeting Notes".to_string(), None)); // duplicate title
        nb.add_note(Note::new("06/2026 plan".to_string(), None)); // path-illegal title

        let storage = VaultStorage::new(dir.clone()).unwrap();
        storage.save_notebook(&nb).unwrap();

        let mut files: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        files.sort();

        // 3 notes → 3 distinct files (bug #2: no silent overwrite)
        assert_eq!(files.len(), 3, "expected 3 distinct files, got {:?}", files);
        // bug #1: the '/' did not create a subdirectory
        assert!(!dir.join("06").exists(), "'/' in title created a subdir");
        assert!(
            files.iter().any(|f| f.starts_with("06_2026 plan")),
            "sanitized slash title missing in {:?}",
            files
        );
        // the two identical titles disambiguated to two names
        let meeting = files.iter().filter(|f| f.starts_with("Meeting Notes")).count();
        assert_eq!(meeting, 2, "duplicate titles collapsed: {:?}", files);

        // Round-trip: titles must survive intact (the sanitized filename and the
        // disambiguation suffix must NOT leak into the reloaded title).
        let loaded = storage.load_notebook().unwrap();
        let mut titles: Vec<String> = loaded.notes.values().map(|n| n.title.clone()).collect();
        titles.sort();
        assert_eq!(
            titles,
            vec!["06/2026 plan", "Meeting Notes", "Meeting Notes"],
            "titles did not round-trip cleanly"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn incremental_save_writes_only_dirty_and_removes_deleted() {
        let dir = std::env::temp_dir().join(format!("scribble_inc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut nb = NotebookData::new();
        let a = Note::new("Alpha".to_string(), None);
        let b = Note::new("Beta".to_string(), None);
        let aid = a.id;
        nb.add_note(a);
        nb.add_note(b);

        let storage = VaultStorage::new(dir.clone()).unwrap();

        // Save ONLY Alpha incrementally → its file exists, Beta's does not.
        let assigned = storage.save_incremental(&nb, &[aid], &[]).unwrap();
        assert_eq!(assigned.assigned.len(), 1, "new note should report its assigned path");
        assert_eq!(assigned.assigned[0].0, aid);
        assert!(dir.join("Alpha.md").exists());
        assert!(!dir.join("Beta.md").exists(), "Beta must not be written");

        // Store the path back (as the app does), then delete Alpha's file.
        let path = assigned.assigned[0].1.clone();
        nb.notes.get_mut(&aid).unwrap().file_path = Some(path.clone());
        storage.save_incremental(&nb, &[], &[path]).unwrap();
        assert!(!dir.join("Alpha.md").exists(), "deleted note's file must be removed");

        let _ = fs::remove_dir_all(&dir);
    }

    /// A vault whose absolute path sits under a dot-directory (~/.notes, a dotfiles
    /// checkout) must still load. Regression: the hidden-entry filter used to test
    /// every component of the absolute path, so such a vault loaded as empty — and
    /// the next save wrote that empty state back over it.
    #[test]
    fn vault_under_hidden_ancestor_still_loads_notes() {
        let root = std::env::temp_dir().join(format!(".scribble_hidden_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Note.md"), "hello world").unwrap();
        // A genuinely hidden entry *inside* the vault must still be skipped.
        fs::create_dir_all(root.join(".obsidian")).unwrap();
        fs::write(root.join(".obsidian").join("workspace.md"), "config").unwrap();

        let storage = VaultStorage::new(root.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();

        let titles: Vec<String> = nb.notes.values().map(|n| n.title.clone()).collect();
        let _ = fs::remove_dir_all(&root);

        assert_eq!(titles, vec!["Note"], "expected exactly the vault's own note");
    }

    /// Saving must never leave a truncated note or a stray temp file behind.
    #[test]
    fn writes_are_atomic_and_leave_no_temp_files() {
        let dir = std::env::temp_dir().join(format!("scribble_atomic_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut nb = NotebookData::new();
        let mut note = Note::new("Draft".to_string(), None);
        note.content = "first version".to_string();
        let id = note.id;
        nb.add_note(note);

        let storage = VaultStorage::new(dir.clone()).unwrap();
        storage.save_notebook(&nb).unwrap();

        // Overwrite the same note to exercise the rename-over-existing path.
        nb.notes.get_mut(&id).unwrap().content = "second version".to_string();
        nb.notes.get_mut(&id).unwrap().file_path = Some(dir.join("Draft.md"));
        storage.save_notebook(&nb).unwrap();

        let body = fs::read_to_string(dir.join("Draft.md")).unwrap();
        let leftovers: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".tmp"))
            .collect();

        let _ = fs::remove_dir_all(&dir);

        assert!(body.contains("second version"), "content not replaced: {:?}", body);
        assert!(!body.contains("first version"), "stale content survived: {:?}", body);
        assert!(leftovers.is_empty(), "temp files left behind: {:?}", leftovers);
    }

    /// A modified_at recorded in frontmatter must survive a load. Regression: the
    /// filesystem mtime was applied unconditionally, so syncing or re-cloning a
    /// vault silently reset every note's modified date.
    #[test]
    fn recorded_modified_at_beats_filesystem_mtime() {
        let dir = std::env::temp_dir().join(format!("scribble_mtime_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Old.md"),
            "---\ntitle: Old\nmodified_at: 2020-01-02T03:04:05Z\n---\nbody\n",
        )
        .unwrap();
        // No frontmatter at all: this one SHOULD fall back to the filesystem.
        fs::write(dir.join("Bare.md"), "just a body\n").unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();

        let recorded = nb.notes.values().find(|n| n.title == "Old").unwrap();
        let bare = nb.notes.values().find(|n| n.title == "Bare").unwrap();
        let recorded_year = recorded.modified_at.format("%Y").to_string();
        let bare_year = bare.modified_at.format("%Y").to_string();
        let now_year = Utc::now().format("%Y").to_string();

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(recorded_year, "2020", "frontmatter modified_at was overwritten");
        assert_eq!(bare_year, now_year, "note without frontmatter should use file mtime");
    }

    /// A CRLF note must keep its identity. Regression: the parser required a
    /// literal `---\n`, so a note saved by a Windows editor or touched by a sync
    /// client had no frontmatter as far as scribble was concerned, and was handed a
    /// fresh id and a fresh creation date on every load.
    #[test]
    fn crlf_frontmatter_keeps_id_and_created_at() {
        let dir = std::env::temp_dir().join(format!("scribble_crlf_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let id = "6f1c9b3e-51a0-4a5f-9d2e-8c7b4a1f0e33";
        fs::write(
            dir.join("Windows.md"),
            format!(
                "---\r\nscribble_id: {}\r\ntitle: Windows\r\ncreated_at: 2019-05-06T07:08:09Z\r\n---\r\nfirst\r\nsecond\r\n",
                id
            ),
        )
        .unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();
        let note = nb.notes.values().find(|n| n.title == "Windows").unwrap().clone();

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(note.id.to_string(), id, "CRLF note was given a new id");
        assert_eq!(
            note.created_at.format("%Y").to_string(),
            "2019",
            "CRLF note lost its creation date"
        );
        assert_eq!(
            note.content, "first\nsecond\n",
            "CRLF survived into the editor buffer, where byte offsets assume LF"
        );
    }

    /// The same normalisation has to apply to a note with no frontmatter at all,
    /// which takes the other branch of the parser.
    #[test]
    fn crlf_body_without_frontmatter_is_normalized() {
        let dir = std::env::temp_dir().join(format!("scribble_crlf_bare_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Bare.md"), "alpha\r\nbeta\r\n").unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();
        let note = nb.notes.values().find(|n| n.title == "Bare").unwrap().clone();

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(note.content, "alpha\nbeta\n");
    }

    /// A CRLF note must also be round-trip stable once normalised, the same way an
    /// LF one is — no growth, no re-parsing as new.
    #[test]
    fn crlf_note_round_trips_without_growing() {
        let dir = std::env::temp_dir().join(format!("scribble_crlf_rt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Round.md"),
            "---\r\ntitle: Round\r\n---\r\nbody line\r\n",
        )
        .unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();
        storage.save_notebook(&nb).unwrap();
        let reloaded = storage.load_notebook().unwrap();
        let once = reloaded.notes.values().find(|n| n.title == "Round").unwrap().clone();
        storage.save_notebook(&reloaded).unwrap();
        let twice_nb = storage.load_notebook().unwrap();
        let twice = twice_nb.notes.values().find(|n| n.title == "Round").unwrap().clone();

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(once.content, twice.content, "note changed on a second round trip");
        assert_eq!(once.id, twice.id, "note was given a new id on reload");
        assert!(!once.content.contains('\r'), "CRLF survived the round trip");
    }

    /// Write a vault of `(filename, body)` notes that all share one scribble_id,
    /// load it, and return `(filename, body)` for each note that survived.
    fn load_sharing_one_id(tag: &str, files: &[(&str, &str)]) -> Vec<(String, String)> {
        let dir = std::env::temp_dir().join(format!("scribble_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let fm = "---\nscribble_id: 6f1c9b3e-51a0-4a5f-9d2e-8c7b4a1f0e33\ntitle: Meeting\n---\n";
        for (name, body) in files {
            fs::write(dir.join(name), format!("{}{}", fm, body)).unwrap();
        }

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();
        let mut survivors: Vec<(String, String)> = nb
            .notes
            .values()
            .map(|n| {
                let name = n
                    .file_path
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                (name, n.content.clone())
            })
            .collect();
        survivors.sort();

        let _ = fs::remove_dir_all(&dir);
        survivors
    }

    /// A sync client's conflicted copy carries the original's frontmatter verbatim,
    /// so both files claim one scribble_id. Regression: `notes` is keyed by id, so
    /// the pair collapsed to a single note and walk order picked the winner — `(`
    /// sorts before `.`, so the conflicted copy consistently evicted the real note,
    /// which then vanished from scribble entirely.
    #[test]
    fn conflicted_copy_does_not_evict_the_real_note() {
        let survivors = load_sharing_one_id(
            "conflict",
            &[
                ("Meeting.md", "the original\n"),
                ("Meeting (conflicted copy 2026-08-16).md", "the conflicted copy\n"),
            ],
        );

        assert_eq!(survivors.len(), 2, "a note was silently dropped: {:?}", survivors);
        assert!(
            survivors.iter().any(|(n, b)| n == "Meeting.md" && b == "the original\n"),
            "the real note did not survive: {:?}",
            survivors
        );
    }

    /// Syncthing names its artefacts differently, and sorts the other way round —
    /// `.sync-conflict-` lands after `Meeting.md`, so here the real note loads
    /// first and the artefact is the one that must yield.
    #[test]
    fn syncthing_conflict_does_not_evict_the_real_note() {
        let survivors = load_sharing_one_id(
            "syncthing",
            &[
                ("Meeting.md", "the original\n"),
                ("Meeting.sync-conflict-20260816-120000-ABCDEFG.md", "the fork\n"),
            ],
        );

        assert_eq!(survivors.len(), 2, "a note was silently dropped: {:?}", survivors);
        assert!(
            survivors.iter().any(|(n, b)| n == "Meeting.md" && b == "the original\n"),
            "the real note did not survive: {:?}",
            survivors
        );
    }

    /// The real note must keep the contested id whichever order the two files load
    /// in, so links and history carry on resolving to it rather than to the artefact.
    #[test]
    fn the_real_note_keeps_the_contested_id() {
        let contested: Uuid = "6f1c9b3e-51a0-4a5f-9d2e-8c7b4a1f0e33".parse().unwrap();
        let dir = std::env::temp_dir().join(format!("scribble_keepid_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let fm = "---\nscribble_id: 6f1c9b3e-51a0-4a5f-9d2e-8c7b4a1f0e33\ntitle: Meeting\n---\n";
        fs::write(dir.join("Meeting.md"), format!("{}the original\n", fm)).unwrap();
        fs::write(
            dir.join("Meeting (conflicted copy 2026-08-16).md"),
            format!("{}the conflicted copy\n", fm),
        )
        .unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let nb = storage.load_notebook().unwrap();
        let holder = nb.notes.get(&contested).map(|n| n.content.clone());

        let _ = fs::remove_dir_all(&dir);

        assert_eq!(
            holder.as_deref(),
            Some("the original\n"),
            "the contested id ended up on the conflict artefact"
        );
    }

    /// Two ordinary notes that share an id — a copy-pasted file, a restored backup —
    /// must both survive too. Neither is an artefact, so the rule is simply that the
    /// one already loaded keeps the id.
    #[test]
    fn two_ordinary_notes_sharing_an_id_both_survive() {
        let survivors = load_sharing_one_id(
            "dupe",
            &[("Meeting.md", "first\n"), ("Meeting copy.md", "second\n")],
        );

        assert_eq!(survivors.len(), 2, "a note was silently dropped: {:?}", survivors);
    }

    /// Save a note the way the app does: write it, then store the report's stamps
    /// back onto the notebook. Skipping that second half is what would make every
    /// autosave look like somebody else's edit.
    fn save_like_the_app(
        storage: &VaultStorage,
        notebook: &mut NotebookData,
        id: Uuid,
    ) -> SaveReport {
        let report = storage.save_incremental(notebook, &[id], &[]).unwrap();
        for (nid, path) in &report.assigned {
            if let Some(n) = notebook.notes.get_mut(nid) {
                n.file_path = Some(path.clone());
            }
        }
        for (nid, stamp) in &report.stamps {
            if let Some(n) = notebook.notes.get_mut(nid) {
                n.disk_stamp = Some(*stamp);
            }
        }
        report
    }

    fn conflict_files(dir: &Path) -> Vec<String> {
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains("(scribble conflict"))
            .collect()
    }

    /// The whole point: a change that arrived under us is never overwritten. Our
    /// version keeps the filename, because the user is mid-sentence in it; theirs is
    /// copied aside first.
    #[test]
    fn an_external_change_is_preserved_before_we_write() {
        let dir = std::env::temp_dir().join(format!("scribble_conf_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Note.md"), "---\ntitle: Note\n---\noriginal\n").unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let mut nb = storage.load_notebook().unwrap();
        let id = *nb.notes.keys().next().unwrap();

        // We type. Meanwhile the sync client lands somebody else's version.
        nb.notes.get_mut(&id).unwrap().content = "what I typed\n".to_string();
        fs::write(
            dir.join("Note.md"),
            "---\ntitle: Note\n---\nwhat the other device wrote\n",
        )
        .unwrap();

        let report = save_like_the_app(&storage, &mut nb, id);

        let ours = fs::read_to_string(dir.join("Note.md")).unwrap();
        let preserved: Vec<String> = conflict_files(&dir);
        let theirs = preserved
            .first()
            .map(|n| fs::read_to_string(dir.join(n)).unwrap())
            .unwrap_or_default();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(report.conflicts.len(), 1, "conflict was not reported");
        assert_eq!(preserved.len(), 1, "expected one preserved file: {:?}", preserved);
        assert!(ours.contains("what I typed"), "our edit did not land: {:?}", ours);
        assert!(
            theirs.contains("what the other device wrote"),
            "their version was not preserved: {:?}",
            theirs
        );
    }

    /// The anti-noise test, and the one that matters most in practice. Autosave runs
    /// constantly; if a save could mistake its own previous write for an external
    /// change, ordinary typing would bury the vault in conflict files.
    #[test]
    fn ordinary_repeated_saves_never_create_a_conflict_file() {
        let dir = std::env::temp_dir().join(format!("scribble_noconf_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Note.md"), "---\ntitle: Note\n---\nstart\n").unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let mut nb = storage.load_notebook().unwrap();
        let id = *nb.notes.keys().next().unwrap();

        for i in 0..5 {
            nb.notes.get_mut(&id).unwrap().content = format!("keystroke {}\n", i);
            let report = save_like_the_app(&storage, &mut nb, id);
            assert!(
                report.conflicts.is_empty(),
                "save {} invented a conflict with its own write",
                i
            );
        }

        let leftovers = conflict_files(&dir);
        let _ = fs::remove_dir_all(&dir);
        assert!(leftovers.is_empty(), "conflict files appeared: {:?}", leftovers);
    }

    /// An external write that happens to say exactly what we were going to say is
    /// not a conflict. Preserving a byte-identical copy would be pure noise.
    #[test]
    fn an_identical_external_write_is_not_a_conflict() {
        let dir = std::env::temp_dir().join(format!("scribble_ident_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Note.md"), "---\ntitle: Note\n---\nbody\n").unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let mut nb = storage.load_notebook().unwrap();
        let id = *nb.notes.keys().next().unwrap();

        // Write once so the file holds exactly what we would write, then blank the
        // stamp to stand in for "something touched this file behind our back".
        save_like_the_app(&storage, &mut nb, id);
        nb.notes.get_mut(&id).unwrap().disk_stamp = None;

        let report = save_like_the_app(&storage, &mut nb, id);
        let leftovers = conflict_files(&dir);
        let _ = fs::remove_dir_all(&dir);

        assert!(report.conflicts.is_empty(), "identical content reported as a conflict");
        assert!(leftovers.is_empty(), "conflict files appeared: {:?}", leftovers);
    }

    /// A brand new note whose filename is already taken by a file we have never
    /// read. We have no idea what that file is, so it does not get destroyed.
    #[test]
    fn a_file_we_have_never_read_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("scribble_unknown_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let mut nb = storage.load_notebook().unwrap();

        // Something lands in the vault after the load — a capture, a sync, a copy.
        fs::write(dir.join("Ideas.md"), "someone else's work\n").unwrap();

        let note = Note::new("Ideas".to_string(), None);
        let id = note.id;
        nb.add_note(note);
        let report = save_like_the_app(&storage, &mut nb, id);

        let preserved = conflict_files(&dir);
        let theirs = preserved
            .first()
            .map(|n| fs::read_to_string(dir.join(n)).unwrap())
            .unwrap_or_default();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(report.conflicts.len(), 1, "unknown file was overwritten silently");
        assert!(
            theirs.contains("someone else's work"),
            "the unknown file's contents were lost: {:?}",
            theirs
        );
    }

    /// A preserved version is a byte copy, so it carries the original's
    /// scribble_id. On the next load it must not evict the note it was forked from —
    /// which is only true because the sidecar name is recognised as an artefact.
    #[test]
    fn a_preserved_version_does_not_evict_the_note_it_forked_from() {
        let dir = std::env::temp_dir().join(format!("scribble_confid_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Note.md"),
            "---\nscribble_id: 6f1c9b3e-51a0-4a5f-9d2e-8c7b4a1f0e33\ntitle: Note\n---\noriginal\n",
        )
        .unwrap();

        let storage = VaultStorage::new(dir.clone()).unwrap();
        let mut nb = storage.load_notebook().unwrap();
        let id = *nb.notes.keys().next().unwrap();

        nb.notes.get_mut(&id).unwrap().content = "mine\n".to_string();
        fs::write(
            dir.join("Note.md"),
            "---\nscribble_id: 6f1c9b3e-51a0-4a5f-9d2e-8c7b4a1f0e33\ntitle: Note\n---\ntheirs\n",
        )
        .unwrap();
        save_like_the_app(&storage, &mut nb, id);

        // Reload the vault from scratch, as a fresh run of the app would.
        let reloaded = storage.load_notebook().unwrap();
        let contents: Vec<String> = reloaded.notes.values().map(|n| n.content.clone()).collect();
        let kept_the_id = reloaded.notes.get(&id).map(|n| n.content.clone());
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(reloaded.notes.len(), 2, "a note was lost on reload: {:?}", contents);
        assert_eq!(
            kept_the_id.as_deref(),
            Some("mine\n"),
            "the preserved copy stole the note's identity"
        );
    }

    /// Frontmatter must not embed absolute paths: they pin a note to one machine
    /// and leak the user's home directory into anything shared.
    #[test]
    fn frontmatter_folder_path_is_vault_relative() {
        use crate::models::Folder;
        let dir = std::env::temp_dir().join(format!("scribble_relpath_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut nb = NotebookData::new();
        let outer = Folder::new("Projects".to_string(), None);
        let outer_id = outer.id;
        nb.add_folder(outer);
        let inner = Folder::new("Cheat-Sheets".to_string(), Some(outer_id));
        let inner_id = inner.id;
        nb.add_folder(inner);
        nb.add_note(Note::new("Nested".to_string(), Some(inner_id)));
        nb.add_note(Note::new("AtRoot".to_string(), None));

        let storage = VaultStorage::new(dir.clone()).unwrap();
        storage.save_notebook(&nb).unwrap();

        let nested = fs::read_to_string(dir.join("Projects").join("Cheat-Sheets").join("Nested.md")).unwrap();
        let at_root = fs::read_to_string(dir.join("AtRoot.md")).unwrap();
        let vault_str = dir.to_string_lossy().to_string();
        let _ = fs::remove_dir_all(&dir);

        assert!(
            nested.contains("folder_path: Projects/Cheat-Sheets"),
            "expected a vault-relative folder_path, got:\n{}",
            nested
        );
        assert!(
            !nested.contains(&vault_str),
            "absolute vault path leaked into frontmatter:\n{}",
            nested
        );
        assert!(
            !at_root.contains("folder_path:"),
            "a note at the vault root should carry no folder_path:\n{}",
            at_root
        );
        // Absent fields are omitted, not written as `null`.
        assert!(
            !nested.contains("null"),
            "frontmatter should omit empty fields rather than write null:\n{}",
            nested
        );
    }

    /// The point of the typed error is that a failure says which file and which
    /// operation. `Box<dyn Error>` produced a bare "Permission denied" with no
    /// indication of what could not be written.
    #[test]
    fn storage_errors_name_the_path_and_the_operation() {
        let missing = std::env::temp_dir().join("scribble_definitely_not_here_xyz");
        let _ = fs::remove_dir_all(&missing);
        let err = VaultStorage::new(missing.clone()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("vault path does not exist"), "got: {}", msg);
        assert!(msg.contains(missing.to_str().unwrap()), "message omits the path: {}", msg);

        // A write into a read-only directory must name the file it failed on.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir = std::env::temp_dir().join(format!("scribble_ro_{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();

            let mut nb = NotebookData::new();
            nb.add_note(Note::new("Blocked".to_string(), None));
            let storage = VaultStorage::new(dir.clone()).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();

            // Root ignores the permission bits, so confirm the directory really is
            // unwritable before asserting on the failure. Cheaper and clearer than
            // asking for the uid.
            let actually_read_only = fs::write(dir.join(".probe"), "x").is_err();
            let outcome = storage.save_notebook(&nb).map(|_| ()).map_err(|e| e.to_string());

            fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
            let _ = fs::remove_dir_all(&dir);

            if actually_read_only {
                let msg = outcome.expect_err("save into a read-only vault must fail");
                assert!(
                    msg.contains("could not write") || msg.contains("could not create"),
                    "message does not name the operation: {}",
                    msg
                );
                assert!(msg.contains("Blocked.md"), "message does not name the file: {}", msg);
            }
        }
    }

    /// Saving and reloading must be idempotent. It was not: the frontmatter
    /// delimiter is five bytes and the parser skipped four, so each round trip
    /// left the delimiter's newline on the body and the next save added another.
    /// Notes silently grew a blank line every time they were opened and saved.
    #[test]
    fn save_load_round_trip_does_not_grow_the_note() {
        let dir = std::env::temp_dir().join(format!("scribble_rt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let storage = VaultStorage::new(dir.clone()).unwrap();

        let mut nb = NotebookData::new();
        let mut note = Note::new("Round".to_string(), None);
        note.content = "first line\nsecond line\n".to_string();
        nb.add_note(note);
        storage.save_notebook(&nb).unwrap();

        let mut bodies = Vec::new();
        for _ in 0..4 {
            let loaded = storage.load_notebook().unwrap();
            let body = loaded.notes.values().next().unwrap().content.clone();
            bodies.push(body);
            storage.save_notebook(&loaded).unwrap();
        }

        let _ = fs::remove_dir_all(&dir);
        assert!(
            bodies.windows(2).all(|w| w[0] == w[1]),
            "body changed across round trips: {:?}",
            bodies.iter().map(|b| b.len()).collect::<Vec<_>>()
        );
        assert!(
            !bodies[0].starts_with('\n'),
            "body must not start with the delimiter's newline: {:?}",
            bodies[0]
        );
    }

    #[test]
    fn relocate_folder_moves_dir_and_remaps_note_paths() {
        use crate::models::Folder;
        let dir = std::env::temp_dir().join(format!("scribble_reloc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut nb = NotebookData::new();
        let folder = Folder::new("Work".to_string(), None);
        let fid = folder.id;
        nb.add_folder(folder);
        let note = Note::new("Task".to_string(), Some(fid));
        let nid = note.id;
        nb.add_note(note);

        let storage = VaultStorage::new(dir.clone()).unwrap();
        storage.save_notebook(&nb).unwrap();
        assert!(dir.join("Work").join("Task.md").exists());
        // store the assigned path back (as the app does)
        nb.notes.get_mut(&nid).unwrap().file_path = Some(dir.join("Work").join("Task.md"));

        // Rename the folder Work -> Projects.
        let updated = storage
            .relocate_folder(&nb, Path::new("Work"), Path::new("Projects"))
            .unwrap();

        assert!(!dir.join("Work").exists(), "old folder dir must be gone");
        assert!(
            dir.join("Projects").join("Task.md").exists(),
            "the file must have followed into the renamed dir"
        );
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].0, nid);
        assert_eq!(updated[0].1, dir.join("Projects").join("Task.md"));

        let _ = fs::remove_dir_all(&dir);
    }
}

/// Guardrails for the two things that have to stay fast, and that nothing else
/// was watching: opening a vault, and typing in a note (see `events.rs`).
///
/// Two guards each, because one alone is a bad trade. An absolute ceiling is the
/// only thing that catches "everything got slower", but it has to be loose enough
/// to survive a shared CI runner, which makes it blind to a 5x regression. A
/// ratio between two sizes on the same machine is immune to how fast that machine
/// is, and catches the change that actually hurts: work that stops being linear.
#[cfg(test)]
mod perf_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Build a vault of `count` notes and return its path.
    fn synthetic_vault(tag: &str, count: usize) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("scribble_perf_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let body = "Some body text on a line.\n".repeat(20);
        for i in 0..count {
            let note = format!(
                "---\nscribble_id: {}\ntitle: Note {}\ncreated_at: 2026-08-23T00:00:00Z\n\
                 modified_at: 2026-08-23T00:00:00Z\ntags:\n- perf\n---\n# Note {}\n\n{}",
                uuid::Uuid::new_v4(),
                i,
                i,
                body
            );
            fs::write(dir.join(format!("note_{:05}.md", i)), note).unwrap();
        }
        dir
    }

    /// Median of several loads, so one scheduling hiccup cannot fail the run.
    fn median_load(dir: &std::path::Path, runs: usize) -> Duration {
        let storage = VaultStorage::new(dir.to_path_buf()).unwrap();
        let _ = storage.load_notebook().unwrap(); // warm the page cache
        // Bounded, so a load that has gone quadratic fails the run instead of
        // hanging it.
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut times: Vec<Duration> = Vec::with_capacity(runs);
        for _ in 0..runs {
            let t = Instant::now();
            let _ = storage.load_notebook().unwrap();
            times.push(t.elapsed());
            if Instant::now() >= deadline {
                break;
            }
        }
        times.sort();
        times[times.len() / 2]
    }

    /// Opening a vault is the first thing that happens and the easiest thing to
    /// make slow without noticing. Measured at ~36ms for 1,000 notes in a debug
    /// build; the ceiling is far above that so a loaded runner cannot trip it,
    /// and far below a vault that has become unusable.
    #[test]
    fn a_thousand_note_vault_loads_within_budget() {
        let dir = synthetic_vault("load", 1000);
        let took = median_load(&dir, 5);
        let _ = fs::remove_dir_all(&dir);

        assert!(
            took < Duration::from_secs(3),
            "loading 1,000 notes took {:?}, budget is 3s",
            took
        );
    }

    /// The guard that survives a slow machine: four times the notes must not cost
    /// far more than four times the work. Measured at 3.9x, and stable across
    /// runs; quadratic loading would be 16x. The threshold leaves room for a
    /// loaded runner without leaving room for an accidental scan-per-note, which
    /// measured 9.6x when injected deliberately — the absolute budget above did
    /// not notice that at all, which is why both guards are here.
    #[test]
    fn vault_load_scales_with_the_number_of_notes() {
        let small_dir = synthetic_vault("scale_small", 250);
        let large_dir = synthetic_vault("scale_large", 1000);
        let small = median_load(&small_dir, 5);
        let large = median_load(&large_dir, 5);
        let _ = fs::remove_dir_all(&small_dir);
        let _ = fs::remove_dir_all(&large_dir);

        let ratio = large.as_secs_f64() / small.as_secs_f64().max(1e-6);
        assert!(
            ratio < 8.0,
            "4x the notes cost {:.1}x the time ({:?} -> {:?}); loading is no longer linear",
            ratio,
            small,
            large
        );
    }
}
