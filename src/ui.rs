use crate::app::{App, AppMode, FocusedPane, TreeItemType};
use crate::syntax::simple_markdown_highlight;
use crate::theme::TokyoNightTheme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Create main layout with breadcrumb (transparent background)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // Breadcrumb bar
            Constraint::Min(1),      // Main content
            Constraint::Length(3),   // Status bar
        ])
        .split(size);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),  // Left pane (folders/notes)
            Constraint::Percentage(70),  // Right pane (editor)
        ])
        .split(chunks[1]);

    // Draw breadcrumb
    draw_breadcrumb(f, app, chunks[0]);
    
    // Draw folder tree
    draw_folder_tree(f, app, main_chunks[0]);
    
    // Draw editor
    draw_editor(f, app, main_chunks[1]);
    
    // Draw status bar
    draw_status_bar(f, app, chunks[2]);

    // Draw modal dialogs if in special modes
    match app.mode {
        AppMode::Search => draw_search_dialog(f, app),
        AppMode::SearchAdvanced => draw_advanced_search_dialog(f, app),
        AppMode::SearchReplace => draw_replace_dialog(f, app),
        AppMode::Command => draw_command_dialog(f, app),
        AppMode::InputNote => draw_input_note_dialog(f, app),
        AppMode::InputFolder => draw_input_folder_dialog(f, app),
        AppMode::Help => draw_help_dialog(f, app),
        AppMode::DeleteConfirm => draw_delete_confirm_dialog(f, app),
        _ => {},
    }
}

fn draw_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let breadcrumb_content = if let Some(ref note) = app.current_note {
        // Show current note path
        let folder_path = if let Some(folder_id) = note.folder_id {
            if let Some(folder) = app.notebook.folders.get(&folder_id) {
                format!("📁 {} > ", folder.name)
            } else {
                String::new()
            }
        } else {
            "📁 Root > ".to_string()
        };
        
        format!("{}📝 {}", folder_path, note.title)
    } else {
        "Scribble • Select a note to start editing".to_string()
    };
    
    let breadcrumb = Paragraph::new(breadcrumb_content)
        .style(Style::default()
            .fg(TokyoNightTheme::FG_DARK)
            .bg(TokyoNightTheme::BG_DARK));
    
    f.render_widget(breadcrumb, area);
}

fn draw_folder_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Folders;
    
    let border_style = if is_focused {
        TokyoNightTheme::border_focused()
    } else {
        TokyoNightTheme::border_inactive()
    };

    // Count notes and folders for title
    let note_count = app.notebook.notes.len();
    let folder_count = app.notebook.folders.len();
    let title = format!("📁 Explorer ({} notes, {} folders)", note_count, folder_count);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    let items: Vec<ListItem> = app.folder_tree_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            // Create tree guides
            let mut tree_guide = String::new();
            
            // Add vertical lines for each depth level
            for d in 0..item.depth {
                if d == item.depth - 1 {
                    // Last level - use branch character
                    tree_guide.push_str("├─ ");
                } else {
                    // Not last level - use continuation
                    tree_guide.push_str("│  ");
                }
            }
            
            let (icon, icon_style) = match item.item_type {
                TreeItemType::Folder => {
                    if item.expanded {
                        ("📂", TokyoNightTheme::folder_expanded_icon())
                    } else {
                        ("📁", TokyoNightTheme::folder_icon())
                    }
                }
                TreeItemType::Note => ("📝", TokyoNightTheme::note_icon()),
            };
            
            let style = if app.mode == AppMode::Move {
                // Special styling for move mode
                if let Some(move_id) = app.move_item_id {
                    if item.id == move_id {
                        // Highlight the item being moved
                        Style::default().fg(TokyoNightTheme::YELLOW).bg(TokyoNightTheme::BG_HIGHLIGHT).add_modifier(Modifier::BOLD)
                    } else if i == app.selected_folder_index {
                        // Highlight the current destination
                        match item.item_type {
                            TreeItemType::Folder => {
                                Style::default().fg(TokyoNightTheme::GREEN).bg(TokyoNightTheme::BG_HIGHLIGHT).add_modifier(Modifier::BOLD)
                            }
                            TreeItemType::Note => {
                                // Show parent folder as destination for notes
                                Style::default().fg(TokyoNightTheme::CYAN).bg(TokyoNightTheme::BG_HIGHLIGHT)
                            }
                        }
                    } else {
                        // Dim other items
                        Style::default().fg(TokyoNightTheme::COMMENT)
                    }
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                }
            } else if i == app.selected_folder_index && is_focused {
                TokyoNightTheme::selected()
            } else if i == app.selected_folder_index {
                Style::default().fg(TokyoNightTheme::FG).bg(TokyoNightTheme::BG_HIGHLIGHT)
            } else {
                Style::default().fg(TokyoNightTheme::FG)
            };

            // Create rich content with tree guides and icons
            let tree_part = if item.depth > 0 {
                Span::styled(tree_guide, Style::default().fg(TokyoNightTheme::COMMENT))
            } else {
                Span::raw("")
            };
            
            let icon_span = Span::styled(format!("{} ", icon), icon_style);
            let name_span = Span::styled(&item.name, Style::default().fg(TokyoNightTheme::FG));
            
            let line = Line::from(vec![tree_part, icon_span, name_span]);
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(TokyoNightTheme::normal());

    // Create list state for proper scrolling
    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_folder_index));

    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_editor(f: &mut Frame, app: &mut App, area: Rect) {
    if app.preview_enabled {
        draw_editor_with_preview(f, app, area);
    } else {
        draw_editor_only(f, app, area);
    }
}

fn draw_editor_with_preview(f: &mut Frame, app: &App, area: Rect) {
    // Split the area horizontally for editor and preview
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // Editor
            Constraint::Percentage(50),  // Preview
        ])
        .split(area);
    
    draw_editor_pane(f, app, chunks[0], true);
    draw_preview_pane(f, app, chunks[1]);
}

fn draw_editor_only(f: &mut Frame, app: &App, area: Rect) {
    draw_editor_pane(f, app, area, false);
}

fn draw_editor_pane(f: &mut Frame, app: &App, area: Rect, is_split_view: bool) {
    let is_focused = app.focused_pane == FocusedPane::Editor;
    
    let border_style = if is_focused {
        TokyoNightTheme::border_focused()
    } else {
        TokyoNightTheme::border_inactive()
    };

    let title = if let Some(ref note) = app.current_note {
        let word_count = app.editor_content.split_whitespace().count();
        let char_count = app.editor_content.chars().count();
        let line_count = app.editor_content.lines().count().max(1);
        
        let mode_status = if app.mode == AppMode::Insert { "(EDIT)" } else { "" };
        let save_indicator = match app.save_status {
            crate::app::SaveStatus::Saved => "✅",
            crate::app::SaveStatus::Modified => "🟡",
            crate::app::SaveStatus::Saving => "⏳",
            crate::app::SaveStatus::Error => "❌",
        };
        
        let preview_indicator = if app.preview_enabled { " 👁️" } else { "" };
        
        format!("{} ✏️  {} {} | {} lines, {} words, {} chars{}", 
            save_indicator, note.title, mode_status, line_count, word_count, char_count, preview_indicator)
    } else {
        let preview_indicator = if app.preview_enabled { " 👁️" } else { "" };
        format!("✏️  Editor{}", preview_indicator)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);

    if let Some(_) = app.current_note {
        let content = if app.editor_content.is_empty() {
            "# Start writing your note here...\n\nPress 'i' to enter insert mode\nPress 'Esc' to return to normal mode"
        } else {
            &app.editor_content
        };

        // Create editor layout with line numbers (adjust for split view)
        let line_number_width = if is_split_view { 4 } else { 6 };
        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(line_number_width),  // Line numbers
                Constraint::Min(1),     // Content
            ])
            .split(Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            });

        // Draw border
        f.render_widget(block, area);

        // Draw line numbers
        let line_count = content.lines().count().max(1);
        let line_numbers: Vec<Line> = (1..=line_count)
            .map(|i| {
                let style = if i == (app.editor_cursor.0 + 1) as usize && is_focused {
                    Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TokyoNightTheme::COMMENT)
                };
                let number_str = if is_split_view { 
                    format!("{:3} ", i)
                } else {
                    format!("{:4} ", i)
                };
                Line::from(Span::styled(number_str, style))
            })
            .collect();
        
        let line_numbers_widget = Paragraph::new(line_numbers)
            .style(TokyoNightTheme::normal())
            .scroll((app.editor_scroll, 0));
        
        f.render_widget(line_numbers_widget, editor_chunks[0]);

        // Apply enhanced syntax highlighting to content
        let styled_content = simple_markdown_highlight(content);
        
        let paragraph = Paragraph::new(styled_content)
            .style(TokyoNightTheme::normal())
            .wrap(Wrap { trim: false })
            .scroll((app.editor_scroll, 0));

        f.render_widget(paragraph, editor_chunks[1]);

        // Show cursor if in insert mode (account for line numbers)
        if app.mode == AppMode::Insert && is_focused {
            let cursor_area = Rect::new(
                editor_chunks[1].x + app.editor_cursor.1,
                editor_chunks[1].y + app.editor_cursor.0 - app.editor_scroll,
                1,
                1,
            );
            f.set_cursor_position((cursor_area.x, cursor_area.y));
        }
    } else {
        draw_welcome_screen(f, app, area, block);
    }
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Preview;
    
    let border_style = if is_focused {
        TokyoNightTheme::border_focused()
    } else {
        TokyoNightTheme::border_inactive()
    };
    
    let title = "👁️ Live Preview";
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);
    
    if app.current_note.is_some() {
        // Render the markdown preview
        let preview_content = if app.editor_content.is_empty() {
            crate::preview::generate_preview_sample()
        } else {
            crate::preview::render_markdown_preview(&app.editor_content)
        };
        
        let paragraph = Paragraph::new(preview_content)
            .block(block)
            .style(TokyoNightTheme::normal())
            .wrap(Wrap { trim: false })
            .scroll((app.editor_scroll, 0)); // Sync scroll with editor
        
        f.render_widget(paragraph, area);
    } else {
        // Show preview placeholder when no note is selected
        let placeholder_content = Text::from(vec![
            Line::from(vec![
                Span::styled("👁️ ", Style::default().fg(TokyoNightTheme::CYAN)),
                Span::styled("Markdown Preview", TokyoNightTheme::markdown_h2()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Select a note to see the live preview here.",
                TokyoNightTheme::help_text()
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press Ctrl+M to toggle preview mode.",
                TokyoNightTheme::help_text()
            )),
        ]);
        
        let paragraph = Paragraph::new(placeholder_content)
            .block(block)
            .style(TokyoNightTheme::normal())
            .alignment(Alignment::Center);
        
        f.render_widget(paragraph, area);
    }
}

fn draw_welcome_screen(f: &mut Frame, app: &App, area: Rect, block: Block) {
    let editor_info = if let Some(ref editor) = app.external_editor {
        format!("External editor detected: {}", editor)
    } else {
        "No external editor found (set $EDITOR or install helix/nvim/vim)".to_string()
    };
    
    let welcome_text = Text::from(vec![
        // ASCII Art Title
        Line::from(""),
        Line::from(vec![
            Span::styled("  ███████╗", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("  ██████╗██╗██████╗  ██╗██████╗ ██████╗ ██╗     ███████╗", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ██╔════╝", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("██╔════╝██║██╔══██╗ ██║██╔══██╗██╔══██╗██║     ██╔════╝", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ███████╗", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("██║     ██║██████╔╝ ██║██████╔╝██████╔╝██║     █████╗  ", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ╚════██║", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("██║     ██║██╔══██╗ ██║██╔══██╗██╔══██╗██║     ██╔══╝  ", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ███████║", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("╚██████╗██║██║  ██║ ██║██████╔╝██████╔╝███████╗███████╗", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ╚══════╝", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(" ╚═════╝╚═╝╚═╝  ╚═╝ ╚═╝╚═════╝ ╚═════╝ ╚══════╝╚══════╝", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("                      ✨ ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Your Terminal Note-Taking Companion", Style::default().fg(TokyoNightTheme::FG_DARK).add_modifier(Modifier::ITALIC)),
            Span::styled(" ✨", Style::default().fg(TokyoNightTheme::YELLOW)),
        ]),
        Line::from(""),
        
        // Separator
        Line::from(Span::styled("─".repeat(80), Style::default().fg(TokyoNightTheme::FG_GUTTER))),
        Line::from(""),
        
        // Live Preview Feature Highlight
        Line::from(vec![
            Span::styled("👁️ ", Style::default().fg(TokyoNightTheme::CYAN)),
            Span::styled("NEW: Live Markdown Preview!", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Press ", TokyoNightTheme::help_text()),
            Span::styled("Ctrl+M", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to toggle split-view markdown preview", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // System Info (more subtle)
        Line::from(vec![
            Span::styled("🔧 System: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(&editor_info, Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
        Line::from(""),
        
        // Quick Actions Section
        Line::from(vec![
            Span::styled("🚀 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Quick Actions", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📝 ", Style::default().fg(TokyoNightTheme::GREEN)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'n'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to create a new note", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("  📂 ", Style::default().fg(TokyoNightTheme::BLUE)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'f'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to create a folder, ", TokyoNightTheme::help_text()),
            Span::styled("'F'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" for subfolder", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("  🔍 ", Style::default().fg(TokyoNightTheme::PURPLE)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'/'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to search through your notes", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("  📁 ", Style::default().fg(TokyoNightTheme::ORANGE)),
            Span::styled("Select a note from the sidebar to start editing", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // Footer
        Line::from(Span::styled("─".repeat(80), Style::default().fg(TokyoNightTheme::FG_GUTTER))),
        Line::from(""),
        Line::from(vec![
            Span::styled("💡 Tip: ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'?'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" anytime for help  •  ", TokyoNightTheme::help_text()),
            Span::styled("Happy note-taking! 📝✨", Style::default().fg(TokyoNightTheme::FG_DARK).add_modifier(Modifier::ITALIC)),
        ]),
    ]);

    let paragraph = Paragraph::new(welcome_text)
        .block(block)
        .style(TokyoNightTheme::normal())
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Insert => "INSERT",
        AppMode::Search => "SEARCH",
        AppMode::SearchAdvanced => "ADV SEARCH",
        AppMode::SearchReplace => "REPLACE",
        AppMode::Command => "COMMAND",
        AppMode::InputNote => "NEW NOTE",
        AppMode::InputFolder => "NEW FOLDER",
        AppMode::Move => "MOVE",
        AppMode::Help => "HELP",
        AppMode::DeleteConfirm => "DELETE?",
    };

    let pane_text = match app.focused_pane {
        FocusedPane::Folders => "📁 FOLDERS",
        FocusedPane::Editor => "📝 EDITOR",
        FocusedPane::Preview => "👁️ PREVIEW",
    };

    let mode_style = match app.mode {
        AppMode::Normal => TokyoNightTheme::mode_normal(),
        AppMode::Insert => TokyoNightTheme::mode_insert(),
        AppMode::Search | AppMode::SearchAdvanced | AppMode::SearchReplace => TokyoNightTheme::mode_search(),
        AppMode::Command => TokyoNightTheme::mode_command(),
        AppMode::InputNote | AppMode::InputFolder => TokyoNightTheme::mode_input(),
        AppMode::Move => TokyoNightTheme::mode_command(), // Use command style for move mode
        AppMode::Help => TokyoNightTheme::mode_search(), // Use search style for help mode
        AppMode::DeleteConfirm => Style::default().fg(TokyoNightTheme::RED).bg(TokyoNightTheme::BG_HIGHLIGHT).add_modifier(Modifier::BOLD),
    };
    
    // Create enhanced message display with operation result feedback
    let message_spans = if let Some(ref result) = app.operation_result {
        match result {
            crate::app::OperationResult::Success { message, icon } => {
                vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(TokyoNightTheme::GREEN)),
                    Span::styled(message, Style::default().fg(TokyoNightTheme::GREEN)),
                ]
            }
            crate::app::OperationResult::Error { message, icon } => {
                vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(TokyoNightTheme::RED)),
                    Span::styled(message, Style::default().fg(TokyoNightTheme::RED)),
                ]
            }
            crate::app::OperationResult::Info { message, icon } => {
                vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(TokyoNightTheme::CYAN)),
                    Span::styled(message, Style::default().fg(TokyoNightTheme::CYAN)),
                ]
            }
        }
    } else {
        vec![Span::styled(&app.status_message, Style::default().fg(TokyoNightTheme::FG))]
    };
    
    let mut left_spans = vec![
        Span::styled(" ", Style::default().fg(TokyoNightTheme::FG)),
        Span::styled(mode_text, mode_style),
        Span::styled(" | ", Style::default().fg(TokyoNightTheme::FG_DARK)),
        Span::styled(pane_text, Style::default().fg(TokyoNightTheme::CYAN)),
        Span::styled(" | ", Style::default().fg(TokyoNightTheme::FG_DARK)),
    ];
    left_spans.extend(message_spans);

    let right_text = if let Some(ref note) = app.current_note {
        let cursor_info = if app.mode == AppMode::Insert {
            format!(" | {}:{}", app.editor_cursor.0 + 1, app.editor_cursor.1 + 1)
        } else {
            String::new()
        };
        
        format!("Modified: {}{} | 📄 {} notes", 
                note.modified_at.format("%m/%d %H:%M"),
                cursor_info,
                app.notebook.notes.len())
    } else {
        format!("📁 {} folders | 📄 {} notes | 🔍 {} search results", 
                app.notebook.folders.len(), 
                app.notebook.notes.len(),
                app.enhanced_search_results.len())
    };

    // Split the area for left and right aligned text
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_text.len() as u16 + 2)])
        .split(area);

    let left_paragraph = Paragraph::new(Line::from(left_spans))
        .block(Block::default().borders(Borders::TOP).border_style(TokyoNightTheme::border_inactive()))
        .style(TokyoNightTheme::status_bar());

    let right_paragraph = Paragraph::new(Span::styled(right_text, Style::default().fg(TokyoNightTheme::FG_DARK)))
        .block(Block::default().borders(Borders::TOP).border_style(TokyoNightTheme::border_inactive()))
        .style(TokyoNightTheme::status_bar())
        .alignment(Alignment::Right);

    f.render_widget(left_paragraph, status_chunks[0]);
    f.render_widget(right_paragraph, status_chunks[1]);
}

fn draw_search_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("🔍 Quick Search")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let help_text = "Enter search terms | Esc: Cancel | Enter: Search";
    let input_text = if app.input_buffer.is_empty() {
        Span::styled("Search notes and content...", TokyoNightTheme::placeholder())
    } else {
        Span::styled(app.input_buffer.as_str(), Style::default().fg(TokyoNightTheme::FG))
    };

    let content = vec![
        Line::from(Span::styled(help_text, TokyoNightTheme::help_text())),
        Line::from(""),
        Line::from(input_text),
        Line::from(""),
        Line::from(Span::styled("Tip: Use Ctrl+F for advanced search with regex support", 
            Style::default().fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC))),
    ];

    let input = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(input, area);
}

fn draw_command_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("⌨️  Command")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let input_text = if app.command_buffer.is_empty() {
        Span::styled("Commands: :w :q :wq :export :backup :import <dir>...", TokyoNightTheme::placeholder())
    } else {
        Span::styled(app.command_buffer.as_str(), Style::default().fg(TokyoNightTheme::FG))
    };

    let input = Paragraph::new(input_text)
        .block(block);

    f.render_widget(input, area);
}

fn draw_input_note_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("📝 New Note Name")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let input_text = if app.input_buffer.is_empty() {
        Span::styled("Enter note name (or press Enter for 'Untitled Note')", TokyoNightTheme::placeholder())
    } else {
        Span::styled(app.input_buffer.as_str(), Style::default().fg(TokyoNightTheme::FG))
    };

    let input = Paragraph::new(input_text)
        .block(block);

    f.render_widget(input, area);
}

fn draw_input_folder_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("📁 New Folder Name")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let input_text = if app.input_buffer.is_empty() {
        Span::styled("Enter folder name (or press Enter for 'New Folder')", TokyoNightTheme::placeholder())
    } else {
        Span::styled(app.input_buffer.as_str(), Style::default().fg(TokyoNightTheme::FG))
    };

    let input = Paragraph::new(input_text)
        .block(block);

    f.render_widget(input, area);
}


fn draw_advanced_search_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("🔍 Advanced Search")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let help_text = "Prefixes: regex: case: | History: ↑/↓ | Enter: Search | Esc: Cancel";
    let input_text = if app.input_buffer.is_empty() {
        Span::styled("Enter search pattern...", TokyoNightTheme::placeholder())
    } else {
        Span::styled(app.input_buffer.as_str(), Style::default().fg(TokyoNightTheme::FG))
    };

    let content = vec![
        Line::from(Span::styled(help_text, TokyoNightTheme::help_text())),
        Line::from(""),
        Line::from(input_text),
    ];

    let input = Paragraph::new(content)
        .block(block);

    f.render_widget(input, area);
}

fn draw_replace_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 35, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("🔄 Find & Replace")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let help_text = "Format: find_text|replace_text | Modifiers: Ctrl+R (regex) Ctrl+C (case)";
    let modifiers_text = if app.command_buffer.is_empty() {
        "No modifiers active"
    } else {
        &app.command_buffer
    };
    
    let input_text = if app.input_buffer.is_empty() {
        Span::styled("old_text|new_text", TokyoNightTheme::placeholder())
    } else {
        Span::styled(app.input_buffer.as_str(), Style::default().fg(TokyoNightTheme::FG))
    };

    let content = vec![
        Line::from(Span::styled(help_text, TokyoNightTheme::help_text())),
        Line::from(Span::styled(format!("Modifiers: {}", modifiers_text), TokyoNightTheme::help_text())),
        Line::from(""),
        Line::from(input_text),
    ];

    let input = Paragraph::new(content)
        .block(block);

    f.render_widget(input, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_help_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(85, 90, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("❓ Help - Scribble")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let editor_info = if let Some(ref editor) = app.external_editor {
        format!("External editor: {}", editor)
    } else {
        "No external editor found (set $EDITOR or install helix/nvim/vim)".to_string()
    };

    let help_text = Text::from(vec![
        // Header
        Line::from(vec![
            Span::styled("🚀 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Quick Actions", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📝 ", Style::default().fg(TokyoNightTheme::GREEN)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'n'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to create a new note", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("  📂 ", Style::default().fg(TokyoNightTheme::BLUE)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'f'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to create a folder, ", TokyoNightTheme::help_text()),
            Span::styled("'F'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" for subfolder", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("  🔍 ", Style::default().fg(TokyoNightTheme::PURPLE)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'/'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to search through your notes", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // Navigation
        Line::from(vec![
            Span::styled("⌨️  ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Navigation & Selection", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("j/k ↑/↓", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Move up/down in lists", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("g      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Jump to top", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("G      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Jump to bottom", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Tab    ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Switch between sidebar and editor", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Enter  ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Open note or toggle folder", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // Editing
        Line::from(vec![
            Span::styled("✏️  ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Editing & Writing", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("i      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Enter insert mode (start editing)", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("e      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Open in external editor", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Esc    ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Exit insert mode (auto-saves)", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Ctrl+S ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Manual save (in any mode)", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Ctrl+M ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Toggle live markdown preview", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // File Operations
        Line::from(vec![
            Span::styled("🗂️  ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("File Operations", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("d      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Delete selected note or folder (with confirmation)", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("m      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Move note/folder to different location", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // Search
        Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Search & Replace", Style::default().fg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("/      ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Basic search in notes", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Ctrl+F ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Advanced search with regex & case options", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled("Ctrl+R ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Find and replace in current note", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // Commands
        Line::from(vec![
            Span::styled("💻 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Commands", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(":w     ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Save current note", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(":export", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Export all notes to files", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(":backup", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Create backup of all data", TokyoNightTheme::help_text()),
        ]),
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(":q     ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("   Quit application", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        
        // System Info
        Line::from(Span::styled("─".repeat(70), Style::default().fg(TokyoNightTheme::FG_GUTTER))),
        Line::from(""),
        Line::from(vec![
            Span::styled("🔧 System: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(&editor_info, Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
        Line::from(""),
        
        // Footer
        Line::from(vec![
            Span::styled("💡 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("Esc", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(", ", TokyoNightTheme::help_text()),
            Span::styled("'q'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(", or ", TokyoNightTheme::help_text()),
            Span::styled("'?'", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" to close this help dialog", TokyoNightTheme::help_text()),
        ]),
    ]);

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .style(TokyoNightTheme::normal())
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_delete_confirm_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("⚠️  Confirm Deletion")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TokyoNightTheme::RED))
        .style(TokyoNightTheme::popup());

    let item_type = if let Some(ref item_type) = app.delete_item_type {
        match item_type {
            TreeItemType::Note => "note",
            TreeItemType::Folder => "folder",
        }
    } else {
        "item"
    };

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Are you sure you want to delete this ", TokyoNightTheme::help_text()),
            Span::styled(item_type, Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("?", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  📝 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled(&app.delete_item_name, Style::default().fg(TokyoNightTheme::FG).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", TokyoNightTheme::help_text()),
            Span::styled("'y'", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", TokyoNightTheme::help_text()),
            Span::styled("Enter", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" to confirm, ", TokyoNightTheme::help_text()),
            Span::styled("'n'", Style::default().fg(TokyoNightTheme::RED).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", TokyoNightTheme::help_text()),
            Span::styled("Esc", Style::default().fg(TokyoNightTheme::RED).add_modifier(Modifier::BOLD)),
            Span::styled(" to cancel", TokyoNightTheme::help_text()),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
