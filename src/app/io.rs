use super::*;
use crate::models::Note;

impl App {
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
            
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
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
    
    pub(crate) fn import_note_from_file(&mut self, file_path: &std::path::Path, result: &mut ImportResult) -> Result<(), String> {
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
    
    pub(crate) fn parse_imported_note_content(&self, content: &str, fallback_title: &str) -> Result<ParsedNote, String> {
        
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
    
    pub(crate) fn resolve_title_conflict(&self, original_title: &str, _result: &mut ImportResult) -> String {
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
}
