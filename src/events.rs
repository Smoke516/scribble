use crate::app::{App, AppMode, FocusedPane, TreeItemType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, Event};

pub fn handle_event(app: &mut App, event: Event) -> Result<(), Box<dyn std::error::Error>> {
    if let Event::Key(key) = event {
        match app.mode {
            AppMode::Normal => handle_normal_mode(app, key),
            AppMode::Insert => handle_insert_mode(app, key),
            AppMode::Search => handle_search_mode(app, key),
            AppMode::SearchAdvanced => handle_advanced_search_mode(app, key),
            AppMode::SearchReplace => handle_replace_mode(app, key),
            AppMode::Command => handle_command_mode(app, key),
            AppMode::InputNote => handle_input_note_mode(app, key),
            AppMode::InputFolder => handle_input_folder_mode(app, key),
            AppMode::Move => handle_move_mode(app, key),
            AppMode::Help => handle_help_mode(app, key),
            AppMode::DeleteConfirm => handle_delete_confirm_mode(app, key),
        }
    }
    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_down();
            } else {
                app.navigate_down();
            }
        },
        KeyCode::Char('k') | KeyCode::Up => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_up();
            } else {
                app.navigate_up();
            }
        },
        KeyCode::Char('g') => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_to_top();
            } else {
                app.navigate_to_top();
            }
        },
        KeyCode::Char('G') => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_to_bottom();
            } else {
                app.navigate_to_bottom();
            }
        },
        
        // Pane switching
        KeyCode::Tab => {
            app.focused_pane = if app.preview_enabled {
                match app.focused_pane {
                    FocusedPane::Folders => FocusedPane::Editor,
                    FocusedPane::Editor => FocusedPane::Preview,
                    FocusedPane::Preview => FocusedPane::Folders,
                }
            } else {
                match app.focused_pane {
                    FocusedPane::Folders => FocusedPane::Editor,
                    FocusedPane::Editor => FocusedPane::Folders,
                    FocusedPane::Preview => FocusedPane::Editor, // Fallback if preview gets disabled
                }
            };
        }
        
        // Actions
        KeyCode::Enter => {
            if let Some(item) = app.get_selected_item() {
                match item.item_type {
                    TreeItemType::Note => {
                        app.select_note(item.id);
                    }
                    TreeItemType::Folder => {
                        app.toggle_folder_expansion();
                    }
                }
            }
        }
        
        // Create new items
        KeyCode::Char('n') => {
            let folder_id = if let Some(item) = app.get_selected_item() {
                match item.item_type {
                    TreeItemType::Folder => Some(item.id),
                    TreeItemType::Note => None, // Create in root
                }
            } else {
                None
            };
            
            app.start_new_note_input(folder_id);
        }
        
        KeyCode::Char('f') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Default behavior: create folder at root level
            // Use Shift+F to create subfolder in selected folder
            let parent_id = None; // Always create at root by default
            app.start_new_folder_input(parent_id);
        }
        
        // Create subfolder (Shift+F)
        KeyCode::Char('F') => {
            let parent_id = if let Some(item) = app.get_selected_item() {
                match item.item_type {
                    TreeItemType::Folder => Some(item.id),
                    TreeItemType::Note => {
                        // Find the parent folder of the selected note
                        if let Some(note) = app.notebook.notes.get(&item.id) {
                            note.folder_id
                        } else {
                            None
                        }
                    },
                }
            } else {
                None
            };
            
            app.start_new_folder_input(parent_id);
        }
        
        // Edit mode
        KeyCode::Char('i') => {
            if app.current_note.is_some() {
                app.mode = AppMode::Insert;
                app.focused_pane = FocusedPane::Editor;
            } else {
                app.set_message("No note selected".to_string());
            }
        }
        
        // External editor
        KeyCode::Char('e') => {
            if let Err(e) = app.open_in_external_editor() {
                app.set_message(e);
            }
        }
        
        // Delete (only if not Ctrl+D)
        KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Err(e) = app.start_delete_confirmation() {
                app.set_message(e);
            }
        }
        
        // Move (only if not Ctrl+M)
        KeyCode::Char('m') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.start_move_item();
        }
        
        // Search
        KeyCode::Char('/') => {
            app.mode = AppMode::Search;
            app.input_buffer.clear();
        }
        
        // Advanced search (Ctrl+F)
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.mode = AppMode::SearchAdvanced;
            app.input_buffer.clear();
        }
        
        // Search and replace (Ctrl+R)
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.current_note.is_some() {
                app.mode = AppMode::SearchReplace;
                app.input_buffer.clear();
            } else {
                app.set_message("No note selected for replace".to_string());
            }
        }
        
        // Commands
        KeyCode::Char(':') => {
            app.mode = AppMode::Command;
            app.command_buffer.clear();
        }
        
        // Save (Ctrl+S)
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        
        // Toggle markdown preview (Ctrl+M)
        KeyCode::Char('m') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_preview();
        }
        
        // Scrolling controls (Ctrl+U for half page up, Ctrl+D for half page down)
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_half_page_up();
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_half_page_down();
            }
        }
        
        // Page Up/Down for scrolling
        KeyCode::PageUp => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_page_up();
            }
        }
        KeyCode::PageDown => {
            if (app.focused_pane == FocusedPane::Editor || app.focused_pane == FocusedPane::Preview) && app.current_note.is_some() {
                app.scroll_page_down();
            }
        }
        
        // Quit
        KeyCode::Char('q') => app.quit(),
        
        // Help
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        
        _ => {}
    }
}

fn handle_insert_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            // Auto-save on exit insert mode
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    's' => {
                        if let Err(e) = app.save_current_note() {
                            app.set_message(e);
                        }
                    }
                    'm' => {
                        app.toggle_preview();
                    }
                    'u' => {
                        app.scroll_half_page_up();
                    }
                    'd' => {
                        app.scroll_half_page_down();
                    }
                    _ => {}
                }
            } else {
                app.editor_content.push(c);
                app.mark_modified();
                update_cursor_position(app);
            }
        }
        
        KeyCode::Enter => {
            app.editor_content.push('\n');
            app.mark_modified();
            app.editor_cursor.0 += 1;
            app.editor_cursor.1 = 0;
            app.adjust_scroll_to_cursor();
        }
        
        KeyCode::Backspace => {
            if !app.editor_content.is_empty() {
                app.editor_content.pop();
                app.mark_modified();
                update_cursor_position(app);
            }
        }
        
        KeyCode::Tab => {
            app.editor_content.push_str("    "); // 4 spaces
            app.mark_modified();
            update_cursor_position(app);
        }
        
        KeyCode::Up => {
            if app.editor_cursor.0 > 0 {
                app.editor_cursor.0 -= 1;
                app.adjust_scroll_to_cursor();
            }
        }
        
        KeyCode::Down => {
            let lines = app.editor_content.lines().count() as u16;
            if app.editor_cursor.0 < lines.saturating_sub(1) {
                app.editor_cursor.0 += 1;
                app.adjust_scroll_to_cursor();
            }
        }
        
        // Page Up/Down scrolling in insert mode
        KeyCode::PageUp => {
            app.scroll_page_up();
        }
        KeyCode::PageDown => {
            app.scroll_page_down();
        }
        
        KeyCode::Left => {
            if app.editor_cursor.1 > 0 {
                app.editor_cursor.1 -= 1;
            }
        }
        
        KeyCode::Right => {
            let current_line = app.editor_content
                .lines()
                .nth(app.editor_cursor.0 as usize)
                .unwrap_or("");
            if (app.editor_cursor.1 as usize) < current_line.len() {
                app.editor_cursor.1 += 1;
            }
        }
        
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        
        KeyCode::Enter => {
            if !app.input_buffer.is_empty() {
                app.search_notes(app.input_buffer.clone());
            }
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.command_buffer.clear();
        }
        
        KeyCode::Enter => {
            execute_command(app, &app.command_buffer.clone());
            app.mode = AppMode::Normal;
            app.command_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.command_buffer.pop();
        }
        
        _ => {}
    }
}

fn execute_command(app: &mut App, command: &str) {
    let command = command.trim();
    match command {
        "q" | "quit" => app.quit(),
        "w" | "write" => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            }
        }
        "wq" => {
            if let Err(e) = app.save_current_note() {
                app.set_message(e);
            } else {
                app.quit();
            }
        }
        "export" => {
            match app.export_all_notes() {
                Ok(_) => app.set_operation_success("All notes exported successfully".to_string(), Some("📦".to_string())),
                Err(e) => app.set_operation_error(format!("Export failed: {}", e), Some("🚨".to_string())),
            }
        },
        "backup" => {
            match app.create_backup() {
                Ok(_) => app.set_operation_success("Backup created successfully".to_string(), Some("💾".to_string())),
                Err(e) => app.set_operation_error(format!("Backup failed: {}", e), Some("🚨".to_string())),
            }
        },
        _ => {
            if command.starts_with("export ") {
                let path = command.strip_prefix("export ").unwrap_or("").trim();
                match app.export_notes_to_directory(path) {
                    Ok(_) => app.set_operation_success(format!("Notes exported to '{}'", path), Some("📦".to_string())),
                    Err(e) => app.set_operation_error(format!("Export failed: {}", e), Some("🚨".to_string())),
                }
            } else if command.starts_with("import ") {
                let path = command.strip_prefix("import ").unwrap_or("").trim();
                match app.import_notes_from_directory(path) {
                    Ok(_) => app.set_operation_success(format!("Notes imported from '{}'", path), Some("📦".to_string())),
                    Err(e) => app.set_operation_error(format!("Import failed: {}", e), Some("🚨".to_string())),
                }
            } else {
                app.set_message(format!("Unknown command: {}", command));
            }
        }
    }
}

fn handle_input_note_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_input();
        }
        
        KeyCode::Enter => {
            app.finish_new_note_input();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

fn handle_input_folder_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_input();
        }
        
        KeyCode::Enter => {
            app.finish_new_folder_input();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

fn update_cursor_position(app: &mut App) {
    let lines: Vec<&str> = app.editor_content.lines().collect();
    let current_line_index = app.editor_cursor.0 as usize;
    
    if current_line_index < lines.len() {
        let current_line = lines[current_line_index];
        app.editor_cursor.1 = current_line.len() as u16;
    } else {
        // We're at the end, position cursor after the last line
        app.editor_cursor.0 = lines.len() as u16;
        app.editor_cursor.1 = 0;
    }
}

fn handle_advanced_search_mode(app: &mut App, key: KeyEvent) {
    use crate::search::SearchQuery;
    
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        
        KeyCode::Enter => {
            if !app.input_buffer.is_empty() {
                let mut query = SearchQuery::new(app.input_buffer.clone());
                
                // Check for special modifiers in the query
                if app.input_buffer.starts_with("regex:") {
                    let pattern = app.input_buffer.strip_prefix("regex:").unwrap_or("").trim();
                    query = SearchQuery::new(pattern.to_string()).with_regex();
                } else if app.input_buffer.starts_with("case:") {
                    let pattern = app.input_buffer.strip_prefix("case:").unwrap_or("").trim();
                    query = SearchQuery::new(pattern.to_string()).case_sensitive();
                }
                
                // TODO: Add folder filtering support
                // if app.input_buffer.starts_with("folder:") { ... }
                
                app.enhanced_search_notes(query);
            }
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        // Up/Down to navigate search history
        KeyCode::Up => {
            let history = app.get_search_history();
            if !history.is_empty() {
                app.input_buffer = history[0].clone();
            }
        }
        
        _ => {}
    }
}

fn handle_replace_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
            app.command_buffer.clear();
        }
        
        KeyCode::Enter => {
            // Parse input as "find|replace"
            if let Some(pos) = app.input_buffer.find('|') {
                let find = app.input_buffer[..pos].to_string();
                let replace = app.input_buffer[pos + 1..].to_string();
                
                if let Some(ref mut note) = app.current_note {
                    let is_regex = app.command_buffer.contains("regex");
                    let case_sensitive = app.command_buffer.contains("case");
                    
                    match app.enhanced_search.replace_in_note(note, &find, &replace, is_regex, case_sensitive) {
                        Ok(count) => {
                            if count > 0 {
                                app.editor_content = note.content.clone();
                                // Update the note in the notebook
                                app.notebook.notes.insert(note.id, note.clone());
                                app.set_message(format!("Replaced {} occurrences", count));
                            } else {
                                app.set_message("No matches found to replace".to_string());
                            }
                        }
                        Err(e) => {
                            app.set_message(format!("Replace error: {}", e));
                        }
                    }
                } else {
                    app.set_message("No note selected".to_string());
                }
            } else {
                app.set_message("Format: find_text|replace_text".to_string());
            }
            
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
            app.command_buffer.clear();
        }
        
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'r' => app.command_buffer.push_str("regex "),
                    'c' => app.command_buffer.push_str("case "),
                    _ => {}
                }
            } else {
                app.input_buffer.push(c);
            }
        }
        
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        
        _ => {}
    }
}

fn handle_move_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.cancel_move();
        }
        
        // Navigation in move mode
        KeyCode::Char('j') | KeyCode::Down => app.navigate_down(),
        KeyCode::Char('k') | KeyCode::Up => app.navigate_up(),
        KeyCode::Char('g') => app.navigate_to_top(),
        KeyCode::Char('G') => app.navigate_to_bottom(),
        
        // Execute move
        KeyCode::Enter => {
            if let Err(e) = app.execute_move() {
                app.set_message(e);
                app.cancel_move();
            }
        }
        
        // Help in move mode
        KeyCode::Char('?') => {
            app.set_message("Move mode: j/k=navigate, Enter=move to selected location, Esc=cancel".to_string());
        }
        
        _ => {}
    }
}

fn handle_help_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_delete_confirm_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        // Confirm deletion with 'y' or Enter
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Err(e) = app.confirm_delete() {
                app.set_message(e);
            }
        }
        
        // Cancel deletion with 'n', Esc, or any other key
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_delete();
        }
        
        // Any other key cancels the operation
        _ => {
            app.cancel_delete();
        }
    }
}
