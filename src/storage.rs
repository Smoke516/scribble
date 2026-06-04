use crate::models::{NotebookData, Note, Folder};
use dirs;
use serde_json;
use serde_yaml;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use walkdir::WalkDir;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// YAML frontmatter for markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoteFrontmatter {
    scribble_id: Option<String>,
    created_at: Option<DateTime<Utc>>,
    modified_at: Option<DateTime<Utc>>,
    tags: Option<Vec<String>>,
    folder_path: Option<String>,
}

// Abstract trait for different storage backends
pub trait NotebookStorage {
    fn load_notebook(&self) -> Result<NotebookData, Box<dyn std::error::Error>>;
    fn save_notebook(&self, notebook: &NotebookData) -> Result<(), Box<dyn std::error::Error>>;
}

// Vault-based storage for Obsidian compatibility
pub struct VaultStorage {
    vault_path: PathBuf,
}

impl VaultStorage {
    pub fn new(vault_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if !vault_path.exists() {
            return Err(format!("Vault path does not exist: {:?}", vault_path).into());
        }
        if !vault_path.is_dir() {
            return Err(format!("Vault path is not a directory: {:?}", vault_path).into());
        }
        
        Ok(Self { vault_path })
    }
    
    fn parse_markdown_with_frontmatter(&self, content: &str) -> (Option<NoteFrontmatter>, String) {
        if content.starts_with("---\n") {
            if let Some(end_pos) = content[4..].find("\n---\n") {
                let yaml_content = &content[4..end_pos + 4];
                let markdown_content = &content[end_pos + 8..];
                
                if let Ok(frontmatter) = serde_yaml::from_str::<NoteFrontmatter>(yaml_content) {
                    return (Some(frontmatter), markdown_content.to_string());
                }
            }
        }
        (None, content.to_string())
    }
    
    fn create_markdown_with_frontmatter(&self, note: &Note, content: &str) -> String {
        let frontmatter = NoteFrontmatter {
            scribble_id: Some(note.id.to_string()),
            created_at: Some(note.created_at),
            modified_at: Some(note.modified_at),
            tags: if note.tags.is_empty() { None } else { Some(note.tags.clone()) },
            folder_path: note.file_path.as_ref().and_then(|p| p.parent().map(|parent| parent.to_string_lossy().to_string())),
        };
        
        if let Ok(yaml) = serde_yaml::to_string(&frontmatter) {
            format!("---\n{}---\n{}", yaml, content)
        } else {
            content.to_string()
        }
    }
    
    fn get_relative_path(&self, full_path: &PathBuf) -> PathBuf {
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
            let relative_path = self.get_relative_path(&entry.path().to_path_buf());
            let path_str = relative_path.to_string_lossy().to_string();
            
            // Skip .obsidian and other hidden directories
            if path_str.starts_with('.') {
                continue;
            }
            
            // Find or create folder for this path
            if let Some(folder) = notebook.folders.values().find(|f| {
                f.name == relative_path.file_name().unwrap_or_default().to_string_lossy().to_string()
            }) {
                path_to_folder_id.insert(path_str, folder.id);
            }
        }
        
        path_to_folder_id
    }
}

impl NotebookStorage for VaultStorage {
    fn load_notebook(&self) -> Result<NotebookData, Box<dyn std::error::Error>> {
        let mut notebook = NotebookData::new();
        let mut folders_created = HashMap::new();
        
        // Walk through the vault directory
        for entry in WalkDir::new(&self.vault_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip .obsidian and other hidden directories/files
            if path.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.')) {
                continue;
            }
            
            if path.is_dir() && path != self.vault_path {
                // Create folder if it doesn't exist
                let relative_path = self.get_relative_path(&path.to_path_buf());
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
            } else if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                // Process markdown file
                if let Ok(content) = fs::read_to_string(path) {
                    let (frontmatter, markdown_content) = self.parse_markdown_with_frontmatter(&content);
                    
                    // Determine folder_id from path
                    let relative_path = self.get_relative_path(&path.to_path_buf());
                    let parent_path = relative_path.parent();
                    let folder_id = if let Some(parent) = parent_path {
                        let parent_str = parent.to_string_lossy().to_string();
                        folders_created.get(&parent_str).copied()
                    } else {
                        None
                    };
                    
                    // Create note
                    let title = path.file_stem().unwrap().to_string_lossy().to_string();
                    let mut note = if let Some(fm) = frontmatter {
                        // Use existing frontmatter data
                        let note_id = fm.scribble_id
                            .and_then(|s| Uuid::parse_str(&s).ok())
                            .unwrap_or_else(Uuid::new_v4);
                        
                        Note {
                            id: note_id,
                            title,
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
                    
                    // Use filesystem metadata for timestamps if not in frontmatter
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
                    
                    notebook.add_note(note);
                }
            }
        }
        
        // Rebuild note links after loading all notes
        notebook.rebuild_links();
        
        Ok(notebook)
    }
    
    fn save_notebook(&self, notebook: &NotebookData) -> Result<(), Box<dyn std::error::Error>> {
        // Create directories for folders
        for folder in notebook.folders.values() {
            let mut folder_path = self.vault_path.clone();
            
            // Build full folder path
            let mut path_components = Vec::new();
            let mut current_folder = folder;
            
            loop {
                path_components.push(current_folder.name.clone());
                if let Some(parent_id) = current_folder.parent_id {
                    if let Some(parent_folder) = notebook.folders.get(&parent_id) {
                        current_folder = parent_folder;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            
            path_components.reverse();
            for component in path_components {
                folder_path.push(component);
            }
            
            fs::create_dir_all(&folder_path)?;
        }
        
        // Save notes as markdown files
        for note in notebook.notes.values() {
            let file_path = if let Some(existing_path) = &note.file_path {
                existing_path.clone()
            } else {
                // Create new file path
                let mut note_path = self.vault_path.clone();
                
                // Add folder path if note belongs to a folder
                if let Some(folder_id) = note.folder_id {
                    if let Some(folder) = notebook.folders.get(&folder_id) {
                        // Build folder path
                        let mut path_components = Vec::new();
                        let mut current_folder = folder;
                        
                        loop {
                            path_components.push(current_folder.name.clone());
                            if let Some(parent_id) = current_folder.parent_id {
                                if let Some(parent_folder) = notebook.folders.get(&parent_id) {
                                    current_folder = parent_folder;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        
                        path_components.reverse();
                        for component in path_components {
                            note_path.push(component);
                        }
                    }
                }
                
                note_path.push(format!("{}.md", note.title));
                note_path
            };
            
            // Create directory if it doesn't exist
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            // Create content with frontmatter
            let content_with_frontmatter = self.create_markdown_with_frontmatter(note, &note.content);
            
            // Write file
            fs::write(&file_path, content_with_frontmatter)?;
        }
        
        Ok(())
    }
}

pub struct Storage {
    data_dir: PathBuf,
    notebook_file: PathBuf,
}


impl Storage {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = Self::get_data_dir()?;
        fs::create_dir_all(&data_dir)?;
        
        let notebook_file = data_dir.join("notebook.json");
        
        Ok(Self {
            data_dir,
            notebook_file,
        })
    }

    fn get_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
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

    pub fn load_notebook(&self) -> Result<NotebookData, Box<dyn std::error::Error>> {
        if self.notebook_file.exists() {
            let contents = fs::read_to_string(&self.notebook_file)?;
            let notebook: NotebookData = serde_json::from_str(&contents)?;
            Ok(notebook)
        } else {
            // Return empty notebook if file doesn't exist
            Ok(NotebookData::new())
        }
    }

    pub fn save_notebook(&self, notebook: &NotebookData) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(notebook)?;
        fs::write(&self.notebook_file, json)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_notes_dir(&self) -> PathBuf {
        self.data_dir.join("notes")
    }

    #[allow(dead_code)]
    pub fn export_note_to_file(&self, note_id: &str, content: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let notes_dir = self.get_notes_dir();
        fs::create_dir_all(&notes_dir)?;
        
        let file_path = notes_dir.join(format!("{}.md", note_id));
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    #[allow(dead_code)]
    pub fn import_note_from_file(&self, file_path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        Ok(content)
    }

    #[allow(dead_code)]
    pub fn backup_data(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let backup_dir = self.data_dir.join("backups");
        fs::create_dir_all(&backup_dir)?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_file = backup_dir.join(format!("notebook_backup_{}.json", timestamp));
        
        if self.notebook_file.exists() {
            fs::copy(&self.notebook_file, &backup_file)?;
        }
        
        Ok(backup_file)
    }

    #[allow(dead_code)]
    pub fn list_backups(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let backup_dir = self.data_dir.join("backups");
        
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(&backup_dir)? {
            let entry = entry?;
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
    pub fn restore_from_backup(&self, backup_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if backup_file.exists() {
            fs::copy(backup_file, &self.notebook_file)?;
        }
        Ok(())
    }
}

impl NotebookStorage for Storage {
    fn load_notebook(&self) -> Result<NotebookData, Box<dyn std::error::Error>> {
        self.load_notebook()
    }
    
    fn save_notebook(&self, notebook: &NotebookData) -> Result<(), Box<dyn std::error::Error>> {
        self.save_notebook(notebook)
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize storage")
    }
}
