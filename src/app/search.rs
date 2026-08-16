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



    


    /// Enter advanced search (regex:/case: prefixes over note content).
    pub fn start_advanced_search(&mut self) {
        self.mode = AppMode::SearchAdvanced;
        self.input_buffer.clear();
    }

}
