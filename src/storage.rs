use crate::models::NotebookData;
use dirs;
use serde_json;
use std::fs;
use std::path::PathBuf;

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

impl Default for Storage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize storage")
    }
}
