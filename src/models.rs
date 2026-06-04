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
pub struct DeletedItem {
    pub item: DeletedItemType,
    pub deleted_at: DateTime<Utc>,
    pub original_parent: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeletedItemType {
    Note(Note),
    Folder(Folder),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashBin {
    pub items: Vec<DeletedItem>,
    pub max_items: usize,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLink {
    pub source_note_id: Uuid,
    pub target_note_title: String,
    pub target_note_id: Option<Uuid>, // None if target doesn't exist yet
    pub position: usize, // Position in source note content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub note_id: Uuid,
    pub last_accessed: DateTime<Utc>,
    pub access_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookData {
    pub folders: HashMap<Uuid, Folder>,
    pub notes: HashMap<Uuid, Note>,
    pub root_folder_ids: Vec<Uuid>,
    pub trash_bin: TrashBin,
    pub links: Vec<NoteLink>,
    pub recent_files: Vec<RecentFile>,
}

impl TrashBin {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            max_items: 100,
            retention_days: 30,
        }
    }

    pub fn add_item(&mut self, item: DeletedItemType, original_parent: Option<Uuid>) {
        let deleted_item = DeletedItem {
            item,
            deleted_at: Utc::now(),
            original_parent,
        };
        
        self.items.push(deleted_item);
        
        // Limit the number of items in trash
        if self.items.len() > self.max_items {
            self.items.remove(0);
        }
    }

    pub fn restore_latest(&mut self) -> Option<DeletedItem> {
        self.items.pop()
    }

    #[allow(dead_code)]
    pub fn cleanup_old_items(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::days(self.retention_days as i64);
        self.items.retain(|item| item.deleted_at > cutoff);
    }
}

impl NotebookData {
    pub fn new() -> Self {
        Self {
            folders: HashMap::new(),
            notes: HashMap::new(),
            root_folder_ids: Vec::new(),
            trash_bin: TrashBin::new(),
            links: Vec::new(),
            recent_files: Vec::new(),
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

        if let Some(folder) = self.folders.remove(&folder_id) {
            // Remove from root folders if needed
            self.root_folder_ids.retain(|&id| id != folder_id);
            
            // Add to trash bin
            self.trash_bin.add_item(DeletedItemType::Folder(folder.clone()), folder.parent_id);
        }
        
        Ok(())
    }

    pub fn remove_note(&mut self, note_id: Uuid) {
        if let Some(note) = self.notes.remove(&note_id) {
            // Add to trash bin
            self.trash_bin.add_item(DeletedItemType::Note(note.clone()), note.folder_id);
            
            // Remove any links involving this note
            self.links.retain(|link| {
                link.source_note_id != note_id && 
                link.target_note_id != Some(note_id)
            });
        }
    }

    pub fn undo_last_delete(&mut self) -> Result<String, String> {
        if let Some(deleted_item) = self.trash_bin.restore_latest() {
            match deleted_item.item {
                DeletedItemType::Note(note) => {
                    let title = note.title.clone();
                    self.notes.insert(note.id, note);
                    self.rebuild_links(); // Rebuild links after restoration
                    Ok(format!("Restored note: {}", title))
                }
                DeletedItemType::Folder(folder) => {
                    let name = folder.name.clone();
                    let folder_id = folder.id;
                    
                    // Re-add to root folders if it was a root folder
                    if folder.parent_id.is_none() {
                        self.root_folder_ids.push(folder_id);
                    }
                    
                    self.folders.insert(folder_id, folder);
                    Ok(format!("Restored folder: {}", name))
                }
            }
        } else {
            Err("Nothing to undo".to_string())
        }
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

    pub fn fuzzy_search_notes(&self, query: &str) -> Vec<(&Note, i64)> {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;
        
        let matcher = SkimMatcherV2::default();
        let mut results: Vec<(&Note, i64)> = Vec::new();
        
        for note in self.notes.values() {
            let mut best_score = 0i64;
            
            // Check title match (higher priority)
            if let Some(score) = matcher.fuzzy_match(&note.title, query) {
                best_score = std::cmp::max(best_score, score * 2); // Title matches get 2x score
            }
            
            // Check content match
            if let Some(score) = matcher.fuzzy_match(&note.content, query) {
                best_score = std::cmp::max(best_score, score);
            }
            
            // Check tag matches
            for tag in &note.tags {
                if let Some(score) = matcher.fuzzy_match(tag, query) {
                    best_score = std::cmp::max(best_score, score + 10); // Small bonus for tag matches
                }
            }
            
            if best_score > 0 {
                results.push((note, best_score));
            }
        }
        
        // Sort by score (highest first)
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

    pub fn parse_links_in_note(&mut self, note_id: Uuid) {
        // Remove existing links for this note
        self.links.retain(|link| link.source_note_id != note_id);
        
        if let Some(note) = self.notes.get(&note_id) {
            let link_regex = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
            
            for (_pos, cap) in link_regex.captures_iter(&note.content).enumerate() {
                if let Some(title_match) = cap.get(1) {
                    let target_title = title_match.as_str().to_string();
                    let target_id = self.find_note_by_title(&target_title);
                    
                    let link = NoteLink {
                        source_note_id: note_id,
                        target_note_title: target_title,
                        target_note_id: target_id,
                        position: title_match.start(),
                    };
                    
                    self.links.push(link);
                }
            }
        }
    }

    pub fn find_note_by_title(&self, title: &str) -> Option<Uuid> {
        self.notes.values()
            .find(|note| note.title.eq_ignore_ascii_case(title))
            .map(|note| note.id)
    }

    pub fn get_backlinks(&self, note_id: Uuid) -> Vec<&NoteLink> {
        self.links.iter()
            .filter(|link| link.target_note_id == Some(note_id))
            .collect()
    }

    #[allow(dead_code)]  // TODO: backlink graph: outgoing links unused
    pub fn get_outgoing_links(&self, note_id: Uuid) -> Vec<&NoteLink> {
        self.links.iter()
            .filter(|link| link.source_note_id == note_id)
            .collect()
    }

    pub fn rebuild_links(&mut self) {
        let note_ids: Vec<Uuid> = self.notes.keys().cloned().collect();
        for note_id in note_ids {
            self.parse_links_in_note(note_id);
        }
    }

    pub fn add_recent_file(&mut self, note_id: Uuid) {
        let now = Utc::now();
        
        // Remove existing entry if present
        self.recent_files.retain(|rf| rf.note_id != note_id);
        
        // Add to front of list
        self.recent_files.insert(0, RecentFile {
            note_id,
            last_accessed: now,
            access_count: 1,
        });
        
        // Keep only last 15 items
        if self.recent_files.len() > 15 {
            self.recent_files.truncate(15);
        }
    }

    pub fn get_recent_files(&self) -> Vec<&RecentFile> {
        self.recent_files.iter().collect()
    }

}

impl Default for NotebookData {
    fn default() -> Self {
        Self::new()
    }
}
