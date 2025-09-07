use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub folder_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub file_path: Option<PathBuf>,
}

impl Note {
    pub fn new(title: String, folder_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            content: String::new(),
            folder_id,
            created_at: now,
            modified_at: now,
            tags: Vec::new(),
            file_path: None,
        }
    }

    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.modified_at = Utc::now();
    }

    #[allow(dead_code)]
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.modified_at = Utc::now();
        }
    }

    #[allow(dead_code)]
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.modified_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expanded: bool,
}

impl Folder {
    pub fn new(name: String, parent_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            parent_id,
            created_at: Utc::now(),
            expanded: true,
        }
    }

    #[allow(dead_code)]
    pub fn rename(&mut self, new_name: String) {
        self.name = new_name;
    }
}

#[derive(Debug, Clone)]
pub struct FolderTreeNode {
    pub folder: Folder,
    pub children: Vec<FolderTreeNode>,
    pub notes: Vec<Note>,
    pub depth: usize,
}

impl FolderTreeNode {
    pub fn new(folder: Folder, depth: usize) -> Self {
        Self {
            folder,
            children: Vec::new(),
            notes: Vec::new(),
            depth,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookData {
    pub folders: HashMap<Uuid, Folder>,
    pub notes: HashMap<Uuid, Note>,
    pub root_folder_ids: Vec<Uuid>,
}

impl NotebookData {
    pub fn new() -> Self {
        Self {
            folders: HashMap::new(),
            notes: HashMap::new(),
            root_folder_ids: Vec::new(),
        }
    }

    pub fn add_folder(&mut self, folder: Folder) {
        if folder.parent_id.is_none() {
            self.root_folder_ids.push(folder.id);
        }
        self.folders.insert(folder.id, folder);
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.insert(note.id, note);
    }

    pub fn remove_folder(&mut self, folder_id: Uuid) -> Result<(), String> {
        // Check if folder has children
        let has_children = self.folders.values()
            .any(|f| f.parent_id == Some(folder_id));
        
        if has_children {
            return Err("Cannot delete folder with subfolders".to_string());
        }

        // Check if folder has notes
        let has_notes = self.notes.values()
            .any(|n| n.folder_id == Some(folder_id));
        
        if has_notes {
            return Err("Cannot delete folder with notes".to_string());
        }

        // Remove from root folders if needed
        self.root_folder_ids.retain(|&id| id != folder_id);
        
        // Remove the folder
        self.folders.remove(&folder_id);
        
        Ok(())
    }

    pub fn remove_note(&mut self, note_id: Uuid) {
        self.notes.remove(&note_id);
    }

    pub fn get_folder_notes(&self, folder_id: Option<Uuid>) -> Vec<&Note> {
        self.notes.values()
            .filter(|note| note.folder_id == folder_id)
            .collect()
    }

    pub fn build_folder_tree(&self) -> Vec<FolderTreeNode> {
        let mut tree = Vec::new();
        
        for &root_id in &self.root_folder_ids {
            if let Some(folder) = self.folders.get(&root_id) {
                let node = self.build_tree_node(folder.clone(), 0);
                tree.push(node);
            }
        }
        
        tree
    }

    fn build_tree_node(&self, folder: Folder, depth: usize) -> FolderTreeNode {
        let mut node = FolderTreeNode::new(folder.clone(), depth);
        
        // Add notes for this folder
        node.notes = self.get_folder_notes(Some(folder.id))
            .into_iter()
            .cloned()
            .collect();
        
        // Add child folders
        let children: Vec<_> = self.folders.values()
            .filter(|f| f.parent_id == Some(folder.id))
            .cloned()
            .collect();
        
        for child_folder in children {
            let child_node = self.build_tree_node(child_folder, depth + 1);
            node.children.push(child_node);
        }
        
        node
    }

    pub fn search_notes(&self, query: &str) -> Vec<&Note> {
        let query_lower = query.to_lowercase();
        self.notes.values()
            .filter(|note| {
                note.title.to_lowercase().contains(&query_lower) ||
                note.content.to_lowercase().contains(&query_lower) ||
                note.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

impl Default for NotebookData {
    fn default() -> Self {
        Self::new()
    }
}
