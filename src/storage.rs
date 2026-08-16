use crate::error::{IoResultExt, StorageError};
use crate::models::{NotebookData, Note, Folder};
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

    /// Write only the `dirty` notes and delete `deleted_paths`, returning the
    /// on-disk path assigned to any note that did not have one yet (so the caller
    /// can store it back). Default: fall back to a full save (correct, just not
    /// incremental) — used by single-file backends where it's already cheap.
    fn save_incremental(
        &self,
        notebook: &NotebookData,
        _dirty: &[Uuid],
        _deleted_paths: &[PathBuf],
    ) -> Result<Vec<(Uuid, PathBuf)>, StorageError> {
        self.save_notebook(notebook)?;
        Ok(Vec::new())
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
        if let Some(rest) = content.strip_prefix("---\n") {
            if let Some(end_pos) = rest.find("\n---\n") {
                let yaml_content = &rest[..end_pos];
                // "\n---\n" is five bytes. Skipping only four left the closing
                // delimiter's newline at the head of the body, and since saving
                // re-adds the separator, every load/save round trip grew the note
                // by one blank line.
                let markdown_content = &rest[end_pos + 5..];
                
                if let Ok(frontmatter) = serde_yaml::from_str::<NoteFrontmatter>(yaml_content) {
                    return (Some(frontmatter), markdown_content.to_string());
                }
            }
        }
        (None, content.to_string())
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

    /// Write a single note to disk, returning its resolved path.
    fn write_note(
        &self,
        note: &Note,
        notebook: &NotebookData,
        claimed: &mut HashSet<PathBuf>,
    ) -> Result<PathBuf, StorageError> {
        let file_path = self.resolve_note_path(note, notebook, claimed);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).create_dir_ctx(parent)?;
        }
        let content = self.create_markdown_with_frontmatter(note, &file_path, &note.content);
        write_atomic(&file_path, &content).write_ctx(&file_path)?;
        Ok(file_path)
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
                        }
                    } else {
                        // Create new note without frontmatter
                        let mut note = Note::new(title, folder_id);
                        note.content = markdown_content;
                        note.file_path = Some(path.to_path_buf());
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
                    
                    notebook.add_note(note);
                }
            }
        }
        
        // Rebuild note links after loading all notes
        notebook.rebuild_links();
        
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
    ) -> Result<Vec<(Uuid, PathBuf)>, StorageError> {
        // Remove files for deleted notes (ignore if already gone).
        for path in deleted_paths {
            let _ = fs::remove_file(path);
        }

        // Claim every known path so a brand-new dirty note can't collide with one.
        let mut claimed: HashSet<PathBuf> =
            notebook.notes.values().filter_map(|n| n.file_path.clone()).collect();

        let mut assigned = Vec::new();
        for id in dirty {
            if let Some(note) = notebook.notes.get(id) {
                let had_path = note.file_path.is_some();
                let path = self.write_note(note, notebook, &mut claimed)?;
                if !had_path {
                    assigned.push((*id, path));
                }
            }
        }
        Ok(assigned)
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
        assert_eq!(assigned.len(), 1, "new note should report its assigned path");
        assert_eq!(assigned[0].0, aid);
        assert!(dir.join("Alpha.md").exists());
        assert!(!dir.join("Beta.md").exists(), "Beta must not be written");

        // Store the path back (as the app does), then delete Alpha's file.
        let path = assigned[0].1.clone();
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
