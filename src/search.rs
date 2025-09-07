use crate::models::{Note, NotebookData};
use regex::Regex;
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub is_regex: bool,
    pub folder_id: Option<Uuid>, // None = search all, Some = search in specific folder
    pub case_sensitive: bool,
}

impl SearchQuery {
    pub fn new(text: String) -> Self {
        Self {
            text,
            is_regex: false,
            folder_id: None,
            case_sensitive: false,
        }
    }
    
    pub fn with_regex(mut self) -> Self {
        self.is_regex = true;
        self
    }
    
    pub fn in_folder(mut self, folder_id: Option<Uuid>) -> Self {
        self.folder_id = folder_id;
        self
    }
    
    pub fn case_sensitive(mut self) -> Self {
        self.case_sensitive = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note: Note,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_text: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    Title,
    Content,
    Tag,
}

pub struct SearchHistory {
    queries: VecDeque<String>,
    max_size: usize,
}

impl SearchHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            queries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }
    
    pub fn add(&mut self, query: String) {
        if !query.trim().is_empty() {
            // Remove if already exists to avoid duplicates
            self.queries.retain(|q| q != &query);
            
            // Add to front
            self.queries.push_front(query);
            
            // Trim to max size
            while self.queries.len() > self.max_size {
                self.queries.pop_back();
            }
        }
    }
    
    pub fn get_history(&self) -> Vec<&String> {
        self.queries.iter().collect()
    }
    
    pub fn clear(&mut self) {
        self.queries.clear();
    }
}

pub struct EnhancedSearch {
    history: SearchHistory,
}

impl EnhancedSearch {
    pub fn new() -> Self {
        Self {
            history: SearchHistory::new(50),
        }
    }
    
    pub fn search(&mut self, notebook: &NotebookData, query: SearchQuery) -> Result<Vec<SearchResult>, String> {
        // Add to history if not empty
        if !query.text.trim().is_empty() {
            self.history.add(query.text.clone());
        }
        
        let notes_to_search: Vec<&Note> = if let Some(folder_id) = query.folder_id {
            // Search only in specific folder
            notebook.notes.values()
                .filter(|note| note.folder_id == Some(folder_id))
                .collect()
        } else {
            // Search all notes
            notebook.notes.values().collect()
        };
        
        let mut results = Vec::new();
        
        for note in notes_to_search {
            if let Some(search_result) = self.search_note(note, &query)? {
                results.push(search_result);
            }
        }
        
        // Sort results by relevance (title matches first, then by number of matches)
        results.sort_by(|a, b| {
            let a_title_matches = a.matches.iter().filter(|m| m.match_type == MatchType::Title).count();
            let b_title_matches = b.matches.iter().filter(|m| m.match_type == MatchType::Title).count();
            
            b_title_matches.cmp(&a_title_matches)
                .then(b.matches.len().cmp(&a.matches.len()))
                .then(a.note.title.cmp(&b.note.title))
        });
        
        Ok(results)
    }
    
    fn search_note(&self, note: &Note, query: &SearchQuery) -> Result<Option<SearchResult>, String> {
        let mut matches = Vec::new();
        
        // Search in title
        if let Some(title_matches) = self.find_matches(&note.title, query, MatchType::Title)? {
            matches.extend(title_matches);
        }
        
        // Search in content
        for (line_num, line) in note.content.lines().enumerate() {
            if let Some(content_matches) = self.find_matches_in_line(line, line_num, query)? {
                matches.extend(content_matches);
            }
        }
        
        // Search in tags
        for tag in &note.tags {
            if let Some(tag_matches) = self.find_matches(tag, query, MatchType::Tag)? {
                matches.extend(tag_matches);
            }
        }
        
        if matches.is_empty() {
            Ok(None)
        } else {
            Ok(Some(SearchResult {
                note: note.clone(),
                matches,
            }))
        }
    }
    
    fn find_matches(&self, text: &str, query: &SearchQuery, match_type: MatchType) -> Result<Option<Vec<SearchMatch>>, String> {
        if query.is_regex {
            self.find_regex_matches(text, query, match_type, 0)
        } else {
            self.find_text_matches(text, query, match_type, 0)
        }
    }
    
    fn find_matches_in_line(&self, line: &str, line_num: usize, query: &SearchQuery) -> Result<Option<Vec<SearchMatch>>, String> {
        if query.is_regex {
            self.find_regex_matches(line, query, MatchType::Content, line_num)
        } else {
            self.find_text_matches(line, query, MatchType::Content, line_num)
        }
    }
    
    fn find_regex_matches(&self, text: &str, query: &SearchQuery, match_type: MatchType, line_num: usize) -> Result<Option<Vec<SearchMatch>>, String> {
        let regex = if query.case_sensitive {
            Regex::new(&query.text)
        } else {
            Regex::new(&format!("(?i){}", query.text))
        };
        
        let regex = regex.map_err(|e| format!("Invalid regex: {}", e))?;
        
        let matches: Vec<SearchMatch> = regex.find_iter(text)
            .map(|m| SearchMatch {
                line_number: line_num,
                line_text: text.to_string(),
                start_offset: m.start(),
                end_offset: m.end(),
                match_type: match_type.clone(),
            })
            .collect();
        
        if matches.is_empty() {
            Ok(None)
        } else {
            Ok(Some(matches))
        }
    }
    
    fn find_text_matches(&self, text: &str, query: &SearchQuery, match_type: MatchType, line_num: usize) -> Result<Option<Vec<SearchMatch>>, String> {
        let (search_text, search_query) = if query.case_sensitive {
            (text, query.text.as_str())
        } else {
            (text, query.text.as_str())
        };
        
        let mut matches = Vec::new();
        let mut start_pos = 0;
        
        loop {
            let match_pos = if query.case_sensitive {
                search_text[start_pos..].find(search_query)
            } else {
                search_text[start_pos..].to_lowercase().find(&search_query.to_lowercase())
            };
            
            if let Some(pos) = match_pos {
                let absolute_pos = start_pos + pos;
                matches.push(SearchMatch {
                    line_number: line_num,
                    line_text: text.to_string(),
                    start_offset: absolute_pos,
                    end_offset: absolute_pos + query.text.len(),
                    match_type: match_type.clone(),
                });
                start_pos = absolute_pos + query.text.len();
            } else {
                break;
            }
        }
        
        if matches.is_empty() {
            Ok(None)
        } else {
            Ok(Some(matches))
        }
    }
    
    pub fn get_search_history(&self) -> Vec<&String> {
        self.history.get_history()
    }
    
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
    
    /// Basic find and replace functionality
    pub fn replace_in_note(&self, note: &mut Note, find: &str, replace: &str, is_regex: bool, case_sensitive: bool) -> Result<usize, String> {
        let mut replacements = 0;
        
        // Replace in title
        let title_result = self.replace_in_text(&note.title, find, replace, is_regex, case_sensitive)?;
        note.title = title_result.0;
        replacements += title_result.1;
        
        // Replace in content
        let content_result = self.replace_in_text(&note.content, find, replace, is_regex, case_sensitive)?;
        note.content = content_result.0;
        replacements += content_result.1;
        
        // Update modification time if replacements were made
        if replacements > 0 {
            note.modified_at = chrono::Utc::now();
        }
        
        Ok(replacements)
    }
    
    fn replace_in_text(&self, text: &str, find: &str, replace: &str, is_regex: bool, case_sensitive: bool) -> Result<(String, usize), String> {
        if is_regex {
            let regex = if case_sensitive {
                Regex::new(find)
            } else {
                Regex::new(&format!("(?i){}", find))
            };
            
            let regex = regex.map_err(|e| format!("Invalid regex: {}", e))?;
            let matches_count = regex.find_iter(text).count();
            let result = regex.replace_all(text, replace).into_owned();
            Ok((result, matches_count))
        } else {
            let mut result = text.to_string();
            let mut count = 0;
            
            if case_sensitive {
                while let Some(pos) = result.find(find) {
                    result.replace_range(pos..pos + find.len(), replace);
                    count += 1;
                }
            } else {
                let find_lower = find.to_lowercase();
                let mut start = 0;
                
                while let Some(pos) = result[start..].to_lowercase().find(&find_lower) {
                    let absolute_pos = start + pos;
                    result.replace_range(absolute_pos..absolute_pos + find.len(), replace);
                    count += 1;
                    start = absolute_pos + replace.len();
                }
            }
            
            Ok((result, count))
        }
    }
}

impl Default for EnhancedSearch {
    fn default() -> Self {
        Self::new()
    }
}
