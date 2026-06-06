use super::*;
use crate::search::{SearchQuery};

impl App {
    pub fn search_notes(&mut self, query: String) {
        self.search_query = query.clone();
        
        // Use basic search for backward compatibility
        self.search_results = self.notebook.search_notes(&query).into_iter().cloned().collect();
        
        // Also perform enhanced search
        let search_query = SearchQuery::new(query.clone());
        match self.enhanced_search.search(&self.notebook, search_query) {
            Ok(results) => {
                self.enhanced_search_results = results;
                let total_matches: usize = self.enhanced_search_results.iter()
                    .map(|r| r.matches.len())
                    .sum();
                    
                if !self.enhanced_search_results.is_empty() {
                    // Extract needed data first to avoid borrowing issues
                    let first_note_id = self.enhanced_search_results[0].note.id;
                    let first_note_title = self.enhanced_search_results[0].note.title.clone();
                    let results_count = self.enhanced_search_results.len();
                    
                    // Automatically navigate to and open the first search result
                    self.open_note_by_id(first_note_id);
                    self.set_message(format!("Found {} notes with {} matches for '{}' - Opened first result: '{}'", 
                        results_count, total_matches, query, first_note_title));
                } else {
                    self.set_message(format!("No matches found for '{}'", query));
                }
            }
            Err(e) => {
                self.set_message(format!("Search error: {}", e));
            }
        }
    }
    
    pub fn enhanced_search_notes(&mut self, query: SearchQuery) {
        match self.enhanced_search.search(&self.notebook, query) {
            Ok(results) => {
                self.enhanced_search_results = results;
                let total_matches: usize = self.enhanced_search_results.iter()
                    .map(|r| r.matches.len())
                    .sum();
                    
                if !self.enhanced_search_results.is_empty() {
                    // Extract needed data first to avoid borrowing issues
                    let first_note_id = self.enhanced_search_results[0].note.id;
                    let first_note_title = self.enhanced_search_results[0].note.title.clone();
                    let results_count = self.enhanced_search_results.len();
                    
                    // Automatically navigate to and open the first search result
                    self.open_note_by_id(first_note_id);
                    self.set_message(format!("Enhanced search found {} notes with {} matches - Opened first result: '{}'", 
                        results_count, total_matches, first_note_title));
                } else {
                    self.set_message("No matches found".to_string());
                }
            }
            Err(e) => {
                self.set_message(format!("Search error: {}", e));
            }
        }
    }
    
    pub fn get_search_history(&self) -> Vec<&String> {
        self.enhanced_search.get_search_history()
    }
    
    #[allow(dead_code)]  // TODO: clear-search-history not key-bound
    pub fn clear_search_history(&mut self) {
        self.enhanced_search.clear_history();
        self.set_message("Search history cleared".to_string());
    }

    // New fuzzy search method
    pub fn fuzzy_search_notes(&mut self, query: String) {
        let results = self.notebook.fuzzy_search_notes(&query);
        
        if !results.is_empty() {
            // Convert to search results format for compatibility
            self.search_results = results.iter().map(|(note, _score)| (*note).clone()).collect();
            
            // Extract needed data before mutable operations
            let first_note_id = results[0].0.id;
            let first_note_title = results[0].0.title.clone();
            let first_score = results[0].1;
            let results_count = results.len();
            
            // Automatically navigate to and open the first search result
            self.open_note_by_id(first_note_id);
            self.set_operation_success(
                format!("Found {} notes for '{}' - Opened: '{}' (score: {})", 
                    results_count, query, first_note_title, first_score),
                Some("🔍".to_string())
            );
        } else {
            self.search_results.clear();
            self.set_operation_error(
                format!("No fuzzy matches found for '{}'", query),
                Some("❓".to_string())
            );
        }
    }

    // Undo delete functionality
    pub fn undo_last_delete(&mut self) -> Result<(), String> {
        match self.notebook.undo_last_delete() {
            Ok(message) => {
                self.refresh_tree_view();
                self.set_operation_success(message, Some("↩️".to_string()));
                Ok(())
            }
            Err(message) => {
                self.set_operation_error(message.clone(), Some("❌".to_string()));
                Err(message)
            }
        }
    }

    // Note linking functionality
    pub fn parse_current_note_links(&mut self) {
        if let Some(ref note) = self.current_note {
            let note_id = note.id;
            self.notebook.parse_links_in_note(note_id);
        }
    }

    pub fn follow_link_at_cursor(&mut self) -> Result<(), String> {
        if let Some(ref _note) = self.current_note {
            // Calculate the actual byte position of the cursor
            let cursor_pos = self.calculate_cursor_byte_position();
            
            if let Some(link_title) = self.extract_link_at_position(cursor_pos) {
                if let Some(target_id) = self.notebook.find_note_by_title(&link_title) {
                    self.open_note_by_id(target_id);
                    self.set_operation_success(
                        format!("Followed link to: {}", link_title),
                        Some("🔗".to_string())
                    );
                    Ok(())
                } else {
                    // Offer to create the note
                    self.set_operation_error(
                        format!("Note '{}' not found. Press n to create a new note with this title.", link_title),
                        Some("🔍".to_string())
                    );
                    Err(format!("Note '{}' not found", link_title))
                }
            } else {
                Err("No [[wiki link]] found at cursor position".to_string())
            }
        } else {
            Err("No note currently open".to_string())
        }
    }

    pub(crate) fn calculate_cursor_byte_position(&self) -> usize {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let mut byte_position = 0;
        
        // Add bytes from all lines before the cursor line
        for i in 0..(self.editor_cursor.0 as usize) {
            if let Some(line) = lines.get(i) {
                byte_position += line.len() + 1; // +1 for newline character
            }
        }
        
        // Add bytes from the cursor column position within the current line
        byte_position += self.editor_cursor.1 as usize;
        
        // Make sure we don't exceed content length
        byte_position.min(self.editor_content.len())
    }
    
    pub(crate) fn extract_link_at_position(&self, pos: usize) -> Option<String> {
        use regex::Regex;
        
        let link_regex = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        
        // First, try to find a link that contains the cursor position
        for captures in link_regex.captures_iter(&self.editor_content) {
            if let Some(full_match) = captures.get(0) {
                // Check if cursor is within the link
                if pos >= full_match.start() && pos <= full_match.end() {
                    if let Some(title) = captures.get(1) {
                        return Some(title.as_str().trim().to_string());
                    }
                }
            }
        }
        
        // If no direct match, try to find the closest link within the current line
        let lines: Vec<&str> = self.editor_content.lines().collect();
        let current_line_index = self.editor_cursor.0 as usize;
        
        if let Some(current_line) = lines.get(current_line_index) {
            // Look for links in the current line
            for captures in link_regex.captures_iter(current_line) {
                if let Some(title) = captures.get(1) {
                    return Some(title.as_str().trim().to_string());
                }
            }
        }
        
        None
    }

    pub fn get_backlinks_for_current_note(&self) -> Vec<String> {
        if let Some(ref note) = self.current_note {
            self.notebook.get_backlinks(note.id)
                .iter()
                .filter_map(|link| {
                    self.notebook.notes.get(&link.source_note_id)
                        .map(|source_note| source_note.title.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Enter advanced search (regex:/case: prefixes over note content).
    pub fn start_advanced_search(&mut self) {
        self.mode = AppMode::SearchAdvanced;
        self.input_buffer.clear();
    }

    pub fn get_outgoing_links_for_current_note(&self) -> Vec<String> {
        if let Some(ref note) = self.current_note {
            self.notebook.get_outgoing_links(note.id)
                .iter()
                .map(|link| link.target_note_title.clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}
