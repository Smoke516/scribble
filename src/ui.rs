use crate::app::{App, AppMode, FocusedPane, TreeItemType};
use crate::syntax::simple_markdown_highlight;
use crate::theme::{Icons, TokyoNightTheme};
use crate::{VERSION, PKG_NAME};
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

    // Main layout with folders and editor
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),  // Left pane (folders/notes)
            Constraint::Percentage(70),  // Right pane (editor)
        ])
        .split(chunks[1]);

    // Draw breadcrumb
    draw_breadcrumb(f, app, chunks[0]);
    
    // Draw folder tree with recent files if enabled
    draw_folder_tree_with_recent(f, app, main_chunks[0]);
    
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
        AppMode::QuickJump => draw_quick_jump_dialog(f, app),
        AppMode::RecentFiles => draw_recent_files_dialog(f, app),
        AppMode::VaultSwitcher => draw_vault_switcher_dialog(f, app),
        AppMode::TagBrowser => draw_tag_browser_dialog(f, app),
        AppMode::ThemeBrowser => draw_theme_browser_dialog(f, app),
        AppMode::Rename => draw_rename_dialog(f, app),
        AppMode::Backlinks => draw_backlinks_dialog(f, app),
        AppMode::TemplatePicker => draw_template_picker_dialog(f, app),
        AppMode::SpellSuggest => draw_spell_suggest_dialog(f, app),
        _ => {}
    }
}

fn draw_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    let breadcrumb_content = if let Some(ref note) = app.current_note {
        // Show current note path
        let folder_path = if let Some(folder_id) = note.folder_id {
            if let Some(folder) = app.notebook.folders.get(&folder_id) {
                format!("{} {} {} ", Icons::FOLDER_CLOSED, folder.name, Icons::BREADCRUMB_SEPARATOR)
            } else {
                String::new()
            }
        } else {
            format!("{} Root {} ", Icons::FOLDER_CLOSED, Icons::BREADCRUMB_SEPARATOR)
        };
        
        format!("{}{} {}", folder_path, Icons::NOTE, note.title)
    } else {
        "Scribble • Select a note to start editing".to_string()
    };
    
    let breadcrumb = Paragraph::new(breadcrumb_content)
        .style(app.theme_manager.status_bar());
    
    f.render_widget(breadcrumb, area);
}

fn draw_folder_tree_with_recent(f: &mut Frame, app: &mut App, area: Rect) {
    if app.show_recent_files {
        draw_recent_files_panel(f, app, area);
    } else {
        draw_folder_tree(f, app, area);
    }
}

fn draw_folder_tree(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_pane == FocusedPane::Folders;
    
    let border_style = if is_focused {
        app.theme_manager.border_focused()
    } else {
        app.theme_manager.border_inactive()
    };

    // Count notes and folders for title
    let note_count = app.notebook.notes.len();
    let folder_count = app.notebook.folders.len();
    let title = format!("{} Explorer ({} notes, {} folders)", Icons::EXPLORER, note_count, folder_count);

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
                        (Icons::FOLDER_OPEN, app.theme_manager.folder_expanded_icon())
                    } else {
                        (Icons::FOLDER_CLOSED, app.theme_manager.folder_icon())
                    }
                }
                TreeItemType::Note => (Icons::NOTE, app.theme_manager.note_icon()),
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
        .style(app.theme_manager.normal());

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

fn draw_editor_with_preview(f: &mut Frame, app: &mut App, area: Rect) {
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

fn draw_editor_only(f: &mut Frame, app: &mut App, area: Rect) {
    draw_editor_pane(f, app, area, false);
}

fn draw_editor_pane(f: &mut Frame, app: &mut App, area: Rect, is_split_view: bool) {
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
            crate::app::SaveStatus::Saved => Icons::SAVED,
            crate::app::SaveStatus::Modified => Icons::MODIFIED,
            crate::app::SaveStatus::Saving => Icons::SAVING,
            crate::app::SaveStatus::Error => Icons::ERROR,
        };
        
        let preview_indicator = if app.preview_enabled { format!(" {}", Icons::PREVIEW) } else { String::new() };
        
        format!("{} {} {} {} | {} lines, {} words, {} chars{}", 
            save_indicator, Icons::EDITOR, note.title, mode_status, line_count, word_count, char_count, preview_indicator)
    } else {
        let preview_indicator = if app.preview_enabled { format!(" {}", Icons::PREVIEW) } else { String::new() };
        format!("{} Editor{}", Icons::EDITOR, preview_indicator)
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

        // Inner content area (inside the border)
        let inner_rect = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // Draw border
        f.render_widget(block, area);

        // Tell app how tall the editor viewport is (used for scroll clamping)
        app.editor_viewport_height = inner_rect.height;

        // Optionally render line numbers; returns the rect for the content area
        let content_rect = if app.config.ui.show_line_numbers {
            let line_number_width = if is_split_view { 4 } else { 6 };
            let editor_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(line_number_width),
                    Constraint::Min(1),
                ])
                .split(inner_rect);

            let line_count = content.lines().count().max(1);
            let cursor_row = (app.editor_cursor.0 + 1) as usize;
            let rel = app.config.ui.relative_line_numbers;
            let line_numbers: Vec<Line> = (1..=line_count)
                .map(|i| {
                    let is_current = i == cursor_row && is_focused;
                    let style = if is_current {
                        Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(TokyoNightTheme::COMMENT)
                    };
                    let display_num = if rel && !is_current {
                        (i as isize - cursor_row as isize).unsigned_abs()
                    } else {
                        i
                    };
                    let number_str = if is_split_view {
                        format!("{:3} ", display_num)
                    } else {
                        format!("{:4} ", display_num)
                    };
                    Line::from(Span::styled(number_str, style))
                })
                .collect();

            let line_numbers_widget = Paragraph::new(line_numbers)
                .style(TokyoNightTheme::normal())
                .scroll((app.editor_scroll, 0));
            f.render_widget(line_numbers_widget, editor_chunks[0]);
            editor_chunks[1]
        } else {
            inner_rect
        };

        // Apply syntax highlighting
        let mut styled_content = simple_markdown_highlight(content);

        // Highlight current line in Normal/NoteSearch/Visual mode
        if app.mode != AppMode::Insert && is_focused {
            let row = app.editor_cursor.0 as usize;
            if let Some(line) = styled_content.lines.get_mut(row) {
                for span in &mut line.spans {
                    span.style = span.style.bg(TokyoNightTheme::BG_HIGHLIGHT);
                }
            }
        }

        // Highlight visual selection
        if app.mode == AppMode::Visual && is_focused {
            let (sel_start, sel_end) = app.get_visual_selection();
            for row in sel_start.0..=sel_end.0 {
                if let Some(line) = styled_content.lines.get_mut(row as usize) {
                    let plain: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    let from_col = if row == sel_start.0 { sel_start.1 as usize } else { 0 };
                    let to_col   = if row == sel_end.0   { (sel_end.1 as usize + 1).min(plain.len()) } else { plain.len() };
                    let from = from_col.min(plain.len());
                    let to   = to_col.min(plain.len());
                    let mut new_spans = vec![];
                    if from > 0 { new_spans.push(Span::raw(plain[..from].to_string())); }
                    if from < to {
                        new_spans.push(Span::styled(
                            plain[from..to].to_string(),
                            Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::BLUE).add_modifier(Modifier::BOLD),
                        ));
                    }
                    if to < plain.len() { new_spans.push(Span::raw(plain[to..].to_string())); }
                    *line = Line::from(new_spans);
                }
            }
        }

        // Highlight spell errors (red + underline)
        if app.spell_check_enabled && !app.spell_errors.is_empty() {
            // Group errors by row
            use std::collections::HashMap;
            let mut row_errors: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
            for &(row, col, len) in &app.spell_errors {
                row_errors.entry(row).or_default().push((col, len));
            }
            for (row, mut errs) in row_errors {
                if let Some(line) = styled_content.lines.get_mut(row) {
                    let plain: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    errs.sort_by_key(|&(col, _)| col);
                    let mut spans: Vec<Span> = Vec::new();
                    let mut pos = 0usize;
                    for (col, len) in errs {
                        let start = col.min(plain.len());
                        let end = (col + len).min(plain.len());
                        if start > pos {
                            spans.push(Span::raw(plain[pos..start].to_string()));
                        }
                        if start < end {
                            spans.push(Span::styled(
                                plain[start..end].to_string(),
                                Style::default()
                                    .fg(TokyoNightTheme::RED)
                                    .add_modifier(Modifier::UNDERLINED),
                            ));
                        }
                        pos = end;
                    }
                    if pos < plain.len() {
                        spans.push(Span::raw(plain[pos..].to_string()));
                    }
                    *line = Line::from(spans);
                }
            }
        }

        // Highlight in-note search matches
        if app.note_search_active && !app.note_search_matches.is_empty() {
            let query_len = app.note_search_query.len().max(1);
            for (match_idx, &(row, col)) in app.note_search_matches.iter().enumerate() {
                let is_current = match_idx == app.note_search_selected;
                let hl_bg = if is_current { TokyoNightTheme::ORANGE } else { TokyoNightTheme::YELLOW };
                if let Some(line) = styled_content.lines.get_mut(row as usize) {
                    // Rebuild line: collect plain text, re-split with highlight at match range
                    let plain: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    let start = (col as usize).min(plain.len());
                    let end = (start + query_len).min(plain.len());
                    *line = Line::from(vec![
                        Span::raw(plain[..start].to_string()),
                        Span::styled(
                            plain[start..end].to_string(),
                            Style::default().fg(TokyoNightTheme::BG).bg(hl_bg).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(plain[end..].to_string()),
                    ]);
                }
            }
        }

        let paragraph = Paragraph::new(styled_content)
            .style(TokyoNightTheme::normal())
            .wrap(Wrap { trim: false })
            .scroll((app.editor_scroll, 0));

        f.render_widget(paragraph, content_rect);

        // In-note search bar overlay (bottom of content area)
        if app.mode == AppMode::NoteSearch {
            let bar_y = (content_rect.y + content_rect.height).saturating_sub(1);
            let bar_rect = Rect::new(content_rect.x, bar_y, content_rect.width, 1);
            let match_info = if app.note_search_matches.is_empty() {
                " [no matches]".to_string()
            } else {
                format!(" [{}/{}]", app.note_search_selected + 1, app.note_search_matches.len())
            };
            let bar_line = Line::from(vec![
                Span::styled("/ ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(app.note_search_query.as_str(), Style::default().fg(TokyoNightTheme::FG)),
                Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::SLOW_BLINK)),
                Span::styled(match_info, Style::default().fg(TokyoNightTheme::COMMENT)),
            ]);
            f.render_widget(Clear, bar_rect);
            f.render_widget(Paragraph::new(bar_line)
                .style(Style::default().fg(TokyoNightTheme::FG).bg(TokyoNightTheme::BG_DARK)), bar_rect);
        } else if app.note_search_active && !app.note_search_matches.is_empty() {
            // Still show a subtle hint when active but not typing
            let bar_y = (content_rect.y + content_rect.height).saturating_sub(1);
            let bar_rect = Rect::new(content_rect.x, bar_y, content_rect.width, 1);
            let hint = format!(" /{} [{}/{}]  n/N: next/prev  Esc to clear",
                app.note_search_query, app.note_search_selected + 1, app.note_search_matches.len());
            f.render_widget(Clear, bar_rect);
            f.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(TokyoNightTheme::COMMENT)))
                .style(Style::default().bg(TokyoNightTheme::BG_DARK)), bar_rect);
        }

        // Normal/Visual mode block cursor overlay
        if app.mode != AppMode::Insert && is_focused && app.current_note.is_some() {
            let cursor_row = app.editor_cursor.0.saturating_sub(app.editor_scroll);
            let cursor_col = app.editor_cursor.1;
            if cursor_row < content_rect.height {
                let cx = content_rect.x + cursor_col;
                let cy = content_rect.y + cursor_row;
                if cx < content_rect.x + content_rect.width {
                    let lines: Vec<&str> = content.lines().collect();
                    let cursor_char = lines.get(app.editor_cursor.0 as usize)
                        .and_then(|l| l.chars().nth(app.editor_cursor.1 as usize))
                        .unwrap_or(' ');
                    f.render_widget(
                        Paragraph::new(Span::styled(
                            cursor_char.to_string(),
                            Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::CYAN),
                        )),
                        Rect::new(cx, cy, 1, 1),
                    );
                }
            }
        }

        // Show cursor if in insert mode
        if app.mode == AppMode::Insert && is_focused {
            let cursor_area = Rect::new(
                content_rect.x + app.editor_cursor.1,
                content_rect.y + app.editor_cursor.0 - app.editor_scroll,
                1,
                1,
            );
            f.set_cursor_position((cursor_area.x, cursor_area.y));
        }

        // Draw autocompletion popup if active
        if app.autocomplete_state.active && app.mode == AppMode::Insert && is_focused {
            draw_autocomplete_popup(f, app, content_rect);
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
    
    let title = format!("{} Live Preview", Icons::PREVIEW);
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style);
    
    if app.current_note.is_some() {
        // Render the markdown preview
        let preview_content = if app.editor_content.is_empty() {
            crate::preview::generate_preview_sample(&app.theme_manager)
        } else {
            crate::preview::render_markdown_preview(&app.editor_content, &app.theme_manager)
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
                Span::styled(format!("{} ", Icons::PREVIEW), Style::default().fg(TokyoNightTheme::CYAN)),
                Span::styled("Markdown Preview", TokyoNightTheme::markdown_h2()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Select a note to see the live preview here.",
                TokyoNightTheme::help_text()
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press Ctrl+P or F2 to toggle preview mode.",
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
    let content_width = 64u16;
    let left_padding = if area.width > content_width { (area.width - content_width) / 2 } else { 0 };
    let p = left_padding as usize;

    let editor_status = if let Some(ref editor) = app.external_editor {
        format!("External editor: {}", editor)
    } else {
        "$EDITOR not set — using built-in editor".to_string()
    };

    // Helper closures for repeated patterns
    macro_rules! pad { () => { Span::raw(" ".repeat(p)) }; }
    macro_rules! pad2 { () => { Span::raw(" ".repeat(p + 2)) }; }
    macro_rules! pad4 { () => { Span::raw(" ".repeat(p + 4)) }; }
    macro_rules! key {
        ($k:expr) => {
            Span::styled(format!("{:<9}", $k),
                Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))
        };
    }
    macro_rules! desc {
        ($d:expr) => { Span::styled($d, TokyoNightTheme::help_text()) };
    }
    macro_rules! section {
        ($label:expr, $color:expr, $rule:expr) => {
            vec![
                Line::from(""),
                Line::from(vec![
                    pad!(),
                    Span::styled($label, Style::default().fg($color).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    pad!(),
                    Span::styled($rule, Style::default().fg($color)),
                ]),
                Line::from(""),
            ]
        };
    }

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(""),
        // ── Title ──────────────────────────────────────────────────────────
        Line::from(vec![
            pad!(),
            Span::styled("SCRIBBLE", Style::default()
                .fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            pad!(),
            Span::styled("────────", Style::default().fg(TokyoNightTheme::CYAN)),
        ]),
        Line::from(vec![
            pad!(),
            Span::styled("Terminal Note-Taking, Vim-Powered", Style::default()
                .fg(TokyoNightTheme::FG_DARK).add_modifier(Modifier::ITALIC)),
        ]),
        Line::from(vec![
            pad!(),
            Span::styled(format!("Version {}", VERSION), Style::default()
                .fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC)),
        ]),
    ];

    // ── Features ────────────────────────────────────────────────────────────
    lines.extend(section!("FEATURES", TokyoNightTheme::PURPLE, "────────"));

    let features: &[(&str, ratatui::style::Color, &str)] = &[
        (Icons::FOLDER_CLOSED, TokyoNightTheme::BLUE,   "Hierarchical folders — Obsidian vault compatible"),
        (Icons::NOTE,          TokyoNightTheme::GREEN,  "Vim modal editing: Normal / Insert / Visual modes"),
        (Icons::PREVIEW,       TokyoNightTheme::CYAN,   "Live split-pane Markdown preview (tables, code blocks)"),
        (Icons::SEARCH,        TokyoNightTheme::PURPLE, "Full-text search (tree) · in-note search (editor)"),
        (Icons::EDITOR,        TokyoNightTheme::ORANGE, "Wiki [[links]] autocomplete + backlinks panel"),
        (Icons::NOTE,          TokyoNightTheme::YELLOW, "Note templates: Blank · Daily Note · Meeting · Project"),
        (Icons::FOLDER_CLOSED, TokyoNightTheme::BLUE,   "Undo/redo, per-note cursor memory, relative line nums"),
        (Icons::PREVIEW,       TokyoNightTheme::GREEN,  "HTML export · tag browser · auto-save"),
    ];
    for (icon, color, text) in features {
        lines.push(Line::from(vec![
            pad2!(),
            Span::styled(format!("{} ", icon), Style::default().fg(*color)),
            Span::styled(*text, TokyoNightTheme::help_text()),
        ]));
    }

    // ── Quick Start ─────────────────────────────────────────────────────────
    lines.extend(section!("QUICK START", TokyoNightTheme::YELLOW, "───────────"));

    // Creating
    lines.push(Line::from(vec![
        pad2!(),
        Span::styled("Creating", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD)),
    ]));
    let creating: &[(&str, &str)] = &[
        ("n",   "New note"),
        ("N",   "New note from template"),
        ("f",   "New folder"),
        ("i",   "Enter Insert mode (start editing)"),
    ];
    for &(k, d) in creating { lines.push(Line::from(vec![pad4!(), key!(k), desc!(d)])); }

    // Navigation
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        pad2!(),
        Span::styled("Navigation", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD)),
    ]));
    let nav: &[(&str, &str)] = &[
        ("Tab",    "Switch panes"),
        ("j / k",  "Down / up"),
        ("h / l",  "Left / right (editor)"),
        ("g / G",  "Top / bottom of note"),
        (":N",     "Jump to line N"),
        ("Enter",  "Open note or folder"),
    ];
    for &(k, d) in nav { lines.push(Line::from(vec![pad4!(), key!(k), desc!(d)])); }

    // Visual mode
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        pad2!(),
        Span::styled("Visual Mode", Style::default().fg(TokyoNightTheme::BLUE).add_modifier(Modifier::BOLD)),
    ]));
    let visual: &[(&str, &str)] = &[
        ("v",  "Enter Visual select"),
        ("y",  "Yank (copy) selection"),
        ("d",  "Delete selection"),
        ("c",  "Change (delete + Insert)"),
        ("Esc","Cancel selection"),
    ];
    for &(k, d) in visual { lines.push(Line::from(vec![pad4!(), key!(k), desc!(d)])); }

    // Tools
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        pad2!(),
        Span::styled("Tools", Style::default().fg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD)),
    ]));
    let tools: &[(&str, &str)] = &[
        ("/",        "Search notes (tree) · in-note search (editor)"),
        ("n / N",    "Next / prev match (in-note search)"),
        ("[[",       "Wiki-link autocomplete"),
        ("Ctrl+B",   "Backlinks panel"),
        ("Ctrl+J",   "Quick jump to note"),
        ("Ctrl+P",   "Toggle live preview"),
        ("?",        "Full help"),
        (":export html", "Export all notes to HTML"),
    ];
    for &(k, d) in tools { lines.push(Line::from(vec![pad4!(), key!(k), desc!(d)])); }

    // ── Status ───────────────────────────────────────────────────────────────
    lines.extend(section!("STATUS", TokyoNightTheme::GREEN, "──────"));
    lines.push(Line::from(vec![
        pad2!(),
        Span::styled("Folders: ", Style::default().fg(TokyoNightTheme::COMMENT)),
        Span::styled(format!("{}", app.notebook.folders.len()), Style::default().fg(TokyoNightTheme::FG)),
        Span::raw("   "),
        Span::styled("Notes: ", Style::default().fg(TokyoNightTheme::COMMENT)),
        Span::styled(format!("{}", app.notebook.notes.len()), Style::default().fg(TokyoNightTheme::FG)),
    ]));
    lines.push(Line::from(vec![
        pad2!(),
        Span::styled("Editor: ", Style::default().fg(TokyoNightTheme::COMMENT)),
        Span::styled(editor_status, Style::default().fg(TokyoNightTheme::FG_DARK)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        pad!(),
        Span::styled("Select a note from the sidebar to begin", Style::default()
            .fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC)),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
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
        AppMode::QuickJump => "QUICK JUMP",
        AppMode::RecentFiles => "RECENT",
        AppMode::VaultSwitcher => "VAULT",
        AppMode::TagBrowser => "TAGS",
        AppMode::ThemeBrowser => "THEMES",
        AppMode::Rename => "RENAME",
        AppMode::NoteSearch => "NOTE SEARCH",
        AppMode::Backlinks => "BACKLINKS",
        AppMode::Visual => "VISUAL",
        AppMode::TemplatePicker => "TEMPLATE",
        AppMode::SpellSuggest => "SPELL",
    };

    let pane_text = match app.focused_pane {
        FocusedPane::Folders => format!("{} FOLDERS", Icons::EXPLORER),
        FocusedPane::Editor => format!("{} EDITOR", Icons::EDITOR),
        FocusedPane::Preview => format!("{} PREVIEW", Icons::PREVIEW),
    };

    let mode_style = match app.mode {
        AppMode::Normal => app.theme_manager.mode_normal(),
        AppMode::Insert => app.theme_manager.mode_insert(),
        AppMode::Search | AppMode::SearchAdvanced | AppMode::SearchReplace => app.theme_manager.mode_search(),
        AppMode::Command => app.theme_manager.mode_command(),
        AppMode::InputNote | AppMode::InputFolder => app.theme_manager.mode_input(),
        AppMode::Move => app.theme_manager.mode_command(), // Use command style for move mode
        AppMode::Help => app.theme_manager.mode_search(), // Use search style for help mode
        AppMode::DeleteConfirm => app.theme_manager.error().add_modifier(Modifier::BOLD),
        AppMode::QuickJump => app.theme_manager.mode_search(), // Use search style for quick jump
        AppMode::RecentFiles => app.theme_manager.mode_command(), // Use command style for recent files
        AppMode::VaultSwitcher => app.theme_manager.mode_command(), // Use command style for vault switcher
        AppMode::TagBrowser => app.theme_manager.mode_command(), // Use command style for tag browser
        AppMode::ThemeBrowser => app.theme_manager.mode_command(), // Use command style for theme browser
        AppMode::Rename => app.theme_manager.mode_input(),
        AppMode::NoteSearch => app.theme_manager.mode_search(),
        AppMode::Backlinks => app.theme_manager.mode_command(),
        AppMode::Visual => Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD),
        AppMode::TemplatePicker => app.theme_manager.mode_input(),
        AppMode::SpellSuggest => Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::RED).add_modifier(Modifier::BOLD),
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
        vec![Span::styled(&app.status_message, app.theme_manager.normal())]
    };
    
    let mut left_spans = vec![
        Span::styled(" ", Style::default().fg(TokyoNightTheme::FG)),
        Span::styled(mode_text, mode_style),
        Span::styled(" | ", Style::default().fg(TokyoNightTheme::FG_DARK)),
        Span::styled(pane_text, Style::default().fg(TokyoNightTheme::CYAN)),
        Span::styled(" | ", Style::default().fg(TokyoNightTheme::FG_DARK)),
    ];
    
    // Add external changes indicator if present
    if app.has_external_changes {
        left_spans.push(Span::styled("🔄 ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)));
        left_spans.push(Span::styled("SYNC ", Style::default().fg(TokyoNightTheme::YELLOW)));
        left_spans.push(Span::styled("| ", Style::default().fg(TokyoNightTheme::FG_DARK)));
    }
    
    left_spans.extend(message_spans);

    // Add sync status if file watcher is active
    let sync_info = if !app.sync_status.is_empty() {
        format!(" | {}", app.sync_status)
    } else {
        String::new()
    };
    
    let right_text = if let Some(ref note) = app.current_note {
        let cursor_info = format!(" | {}:{}", app.editor_cursor.0 + 1, app.editor_cursor.1 + 1);
        
        format!("Modified: {}{} | {} {} notes{}",
                note.modified_at.format("%m/%d %H:%M"),
                cursor_info,
                Icons::NOTE,
                app.notebook.notes.len(),
                sync_info)
    } else {
        format!("{} {} folders | {} {} notes | {} {} search results{}", 
                Icons::FOLDER_CLOSED, app.notebook.folders.len(), 
                Icons::NOTE, app.notebook.notes.len(),
                Icons::SEARCH, app.enhanced_search_results.len(),
                sync_info)
    };

    // Split the area for left and right aligned text
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_text.len() as u16 + 2)])
        .split(area);

    let left_paragraph = Paragraph::new(Line::from(left_spans))
        .block(Block::default().borders(Borders::TOP).border_style(app.theme_manager.border_inactive()))
        .style(app.theme_manager.status_bar());

    let right_paragraph = Paragraph::new(Span::styled(right_text, app.theme_manager.help_text()))
        .block(Block::default().borders(Borders::TOP).border_style(app.theme_manager.border_inactive()))
        .style(app.theme_manager.status_bar())
        .alignment(Alignment::Right);

    f.render_widget(left_paragraph, status_chunks[0]);
    f.render_widget(right_paragraph, status_chunks[1]);
}

fn draw_search_dialog(f: &mut Frame, app: &App) {
    let has_results = !app.search_dialog_note_ids.is_empty();

    let area = if has_results {
        centered_rect(75, 60, f.area())
    } else {
        centered_rect(70, 25, f.area())
    };
    f.render_widget(Clear, area);

    let mode_label = if app.is_fuzzy_search { "~ Fuzzy" } else { "/ Regular" };
    let title = format!("{} Quick Search [{}]", Icons::SEARCH, mode_label);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    // Build input line with blinking cursor
    let mut input_content = vec![];
    if app.input_buffer.is_empty() {
        input_content.push(Span::styled("Search notes...", TokyoNightTheme::placeholder()));
    } else {
        input_content.push(Span::styled(&app.input_buffer, Style::default().fg(TokyoNightTheme::FG)));
    }
    input_content.push(Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::SLOW_BLINK)));

    if has_results {
        let inner = block.inner(area);
        f.render_widget(block, area);

        // Split inner area: 3 lines for input/help, rest for results
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(inner);

        let help_text = "↑↓: Navigate | Tab: Toggle fuzzy | Enter: Open | Esc: Close";
        let input_para = Paragraph::new(vec![
            Line::from(Span::styled(help_text, TokyoNightTheme::help_text())),
            Line::from(""),
            Line::from(input_content),
        ]);
        f.render_widget(input_para, chunks[0]);

        // Results list
        let items: Vec<ListItem> = app.search_dialog_note_ids
            .iter()
            .map(|id| {
                let title = app.notebook.notes.get(id)
                    .map(|n| n.title.as_str())
                    .unwrap_or("(unknown)");
                ListItem::new(Line::from(Span::styled(title, Style::default().fg(TokyoNightTheme::FG))))
            })
            .collect();

        let results_list = List::new(items)
            .highlight_style(TokyoNightTheme::selected())
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        list_state.select(Some(app.search_dialog_selected));

        f.render_stateful_widget(results_list, chunks[1], &mut list_state);
    } else {
        // Compact form when no results yet
        let help_text = "Enter: Search | Tab: Toggle fuzzy/regular | Esc: Cancel";
        let content = vec![
            Line::from(Span::styled(help_text, TokyoNightTheme::help_text())),
            Line::from(""),
            Line::from(input_content),
            Line::from(""),
            Line::from(Span::styled(
                "Tip: Tab toggles fuzzy/regular | Ctrl+F opens fuzzy from Normal mode",
                Style::default().fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC),
            )),
        ];
        let input = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left);
        f.render_widget(input, area);
    }
}

fn draw_command_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("💻 Command Mode")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    // Create the input line with cursor
    let mut input_content = vec![
        Span::styled(":", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
    ];
    
    if !app.command_buffer.is_empty() {
        input_content.push(Span::styled(&app.command_buffer, Style::default().fg(TokyoNightTheme::FG)));
    }
    
    // Add cursor (blinking effect)
    input_content.push(Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::SLOW_BLINK)));
    
    let content = vec![
        Line::from(vec![
            Span::styled("Available Commands:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("File: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(":w :q :wq", Style::default().fg(TokyoNightTheme::GREEN)),
            Span::styled(" • Export: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(":export :backup :backups", Style::default().fg(TokyoNightTheme::GREEN)),
        ]),
        Line::from(vec![
            Span::styled("Theme: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(":theme list|<name>|current", Style::default().fg(TokyoNightTheme::GREEN)),
            Span::styled(" • Vault: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(":vault", Style::default().fg(TokyoNightTheme::GREEN)),
        ]),
        Line::from(vec![
            Span::styled("Import: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(":import <dir>", Style::default().fg(TokyoNightTheme::GREEN)),
            Span::styled(" • Help: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(":help", Style::default().fg(TokyoNightTheme::GREEN)),
        ]),
        Line::from(""),
        Line::from(input_content),
        Line::from(""),
        Line::from(Span::styled("Press Enter to execute, Esc to cancel", 
            Style::default().fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC))),
    ];

    let input = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(input, area);
}

fn draw_input_note_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("{} New Note Name", Icons::NOTE))
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

fn draw_rename_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 25, f.area());
    f.render_widget(Clear, area);

    let item_type_str = if let Some(ref item_type) = app.rename_item_type {
        match item_type {
            TreeItemType::Note => "Note",
            TreeItemType::Folder => "Folder",
        }
    } else {
        "Item"
    };

    let block = Block::default()
        .title(format!("✏️ Rename {}", item_type_str))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let old_name = &app.rename_item_name;
    
    let content = vec![
        Line::from(vec![
            Span::styled("Current name: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(old_name, Style::default().fg(TokyoNightTheme::FG).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("New name: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(&app.input_buffer, Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::SLOW_BLINK)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press Enter to confirm, Esc to cancel", 
            Style::default().fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC))),
    ];

    let input = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

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
    let area = centered_rect(80, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("❓ Help - {} v{}", PKG_NAME, VERSION))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let editor_info = if let Some(ref editor) = app.external_editor {
        format!("External editor: {}", editor)
    } else {
        "No external editor found (set $EDITOR)".to_string()
    };

    let spell_status = if !app.aspell_available {
        "aspell not installed"
    } else if app.spell_check_enabled {
        "ON"
    } else {
        "OFF"
    };

    let help_text = Text::from(vec![
        // Header
        Line::from(vec![
            Span::styled("Scribble ", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(format!("v{}  ", VERSION), Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled("Complete Keybinding Reference", Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
        Line::from(vec![
            Span::styled("Spell check: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(spell_status, Style::default().fg(
                if app.spell_check_enabled { TokyoNightTheme::GREEN } else { TokyoNightTheme::COMMENT }
            )),
            Span::styled("   Editor: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(&editor_info, Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
        Line::from(""),

        // ── Notes & Tree ──────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Notes & Tree", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  n          New note (in selected folder)   N    New note from template"),
        Line::from("  f          New folder (root)               F    New subfolder"),
        Line::from("  r          Rename selected item            m    Move selected item"),
        Line::from("  d          Delete (confirm prompt)         u    Undo last delete"),
        Line::from("  Enter      Open note / expand folder"),
        Line::from(""),

        // ── Normal Mode ───────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Normal Mode  ", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("(editor focused)", Style::default().fg(TokyoNightTheme::COMMENT)),
        ]),
        Line::from(""),
        Line::from("  h / l      Cursor left / right            0 / $   Line start / end"),
        Line::from("  j / k      Cursor down / up              w / b   Word forward / back"),
        Line::from("  g / G      First / last line             :N      Jump to line N"),
        Line::from(""),
        Line::from("  i          Insert at cursor              A       Append at line end"),
        Line::from("  o / O      New line below / above"),
        Line::from(""),
        Line::from("  x          Delete char at cursor         dd      Delete line"),
        Line::from("  yy         Yank (copy) line              p       Paste below"),
        Line::from("  Ctrl+Z     Undo                          Ctrl+Y  Redo"),
        Line::from(""),
        Line::from("  v          Enter Visual select mode"),
        Line::from("  z=         Suggest spelling fix for word at cursor"),
        Line::from("  e          Open note in external editor"),
        Line::from(""),

        // ── Visual Mode ───────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Visual Mode  ", Style::default().fg(TokyoNightTheme::BLUE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("(enter with v)", Style::default().fg(TokyoNightTheme::COMMENT)),
        ]),
        Line::from(""),
        Line::from("  h/j/k/l    Extend selection              0 / $   To line start/end"),
        Line::from("  w / b      Extend word forward/back      g / G   To top / bottom"),
        Line::from("  y          Yank (copy) selection         d       Delete selection"),
        Line::from("  c          Change (delete + Insert)      Esc/v   Cancel"),
        Line::from(""),

        // ── Insert Mode ───────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Insert Mode", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  [[         Wiki-link autocomplete (note titles)"),
        Line::from("  Tab        Accept autocomplete suggestion  ↑/↓   Navigate suggestions"),
        Line::from("  Ctrl+Z     Undo                           Ctrl+Y  Redo"),
        Line::from("  Ctrl+S     Save                           Ctrl+P  Toggle preview"),
        Line::from("  Esc        Exit Insert mode (auto-saves + spell check)"),
        Line::from(""),

        // ── Search ────────────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Search", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  / (tree)   Search across all notes        Ctrl+F  Fuzzy note search"),
        Line::from("  / (editor) In-note search (highlighted)   n/N    Next / prev match"),
        Line::from("  Esc        Clear in-note highlights        Ctrl+R  Search & replace"),
        Line::from("  Ctrl+J     Quick Jump (fuzzy)              Ctrl+O  Recent files"),
        Line::from("  Ctrl+L     Follow [[wiki link]] at cursor   Ctrl+B  Backlinks panel"),
        Line::from(""),

        // ── Spell Check ───────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Spell Check  ", Style::default().fg(TokyoNightTheme::RED).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("(requires aspell)", Style::default().fg(TokyoNightTheme::COMMENT)),
        ]),
        Line::from(""),
        Line::from("  :spell      Enable spell check            :nospell  Disable"),
        Line::from("  z=          Suggestions for word at cursor"),
        Line::from("  j/k         Navigate suggestions          Enter    Apply suggestion"),
        Line::from("  1-9         Quick-pick suggestion          Esc     Cancel"),
        Line::from("  Errors shown as red underline. Check runs on Esc from Insert mode."),
        Line::from(""),

        // ── Tags ──────────────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Tags", Style::default().fg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  Ctrl+T     Open tag browser               s      Toggle sort order"),
        Line::from("  Enter      Filter notes by tag            c      Clear filters"),
        Line::from("  1-9        Quick tag select               Backsp  Remove last filter"),
        Line::from("  YAML: tags: [work, ideas]   Inline: #hashtag"),
        Line::from(""),

        // ── Preview & Display ─────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Preview & Display", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  Ctrl+P / F2  Toggle live Markdown preview  Tab     Cycle panes"),
        Line::from("  Ctrl+U/D     Half-page scroll              PgUp/Dn Page scroll"),
        Line::from("  Line nums: absolute by default; set relative_line_numbers = true in config"),
        Line::from("  Current line highlighted in Normal mode; cursor shown as block"),
        Line::from(""),

        // ── Vault / Obsidian ──────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Vault / Obsidian", Style::default().fg(TokyoNightTheme::BLUE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  Ctrl+V     Vault switcher                  :vault  Command mode"),
        Line::from("  [[link]]   Wiki-style note links           Ctrl+L  Follow link"),
        Line::from("  Ctrl+B     Show backlinks for current note"),
        Line::from("  Live file-watch sync, YAML frontmatter, #tag support"),
        Line::from(""),

        // ── Themes ────────────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Themes", Style::default().fg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  F3               Theme browser             :theme list    Browse"),
        Line::from("  :theme <name>    Apply theme               :theme current Show active"),
        Line::from(""),

        // ── Command Mode ──────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Command Mode  ", Style::default().fg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("(press : in Normal mode)", Style::default().fg(TokyoNightTheme::COMMENT)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  File:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))]),
        Line::from("    :w  :write       Save note               :q  :quit     Quit"),
        Line::from("    :wq              Save & quit             :N            Jump to line"),
        Line::from(""),
        Line::from(vec![Span::styled("  Export / Import:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))]),
        Line::from("    :export html          Export all notes as HTML (~/Documents/scribble_export)"),
        Line::from("    :export html <path>   Export HTML to custom path"),
        Line::from("    :export [path]        Export as markdown files"),
        Line::from("    :import <dir>         Import markdown files"),
        Line::from("    :backup              Create backup          :backups  List backups"),
        Line::from(""),
        Line::from(vec![Span::styled("  Spell Check:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))]),
        Line::from("    :spell  :spellon      Enable spell check"),
        Line::from("    :nospell  :spelloff   Disable spell check"),
        Line::from(""),
        Line::from(vec![Span::styled("  Themes:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))]),
        Line::from("    :theme list          Browse themes          :theme <name>  Apply"),
        Line::from("    :theme current       Show active theme"),
        Line::from(""),
        Line::from(vec![Span::styled("  Other:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))]),
        Line::from("    :vault              Open vault switcher     :h  :help   This dialog"),
        Line::from(""),

        // ── Tips ──────────────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Tips", Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  • Type [[ in Insert mode to get wiki-link autocomplete for note titles"),
        Line::from("  • Cursor position is remembered per-note when you switch between notes"),
        Line::from("  • :spell on + z= gives Vim-style spell correction with aspell suggestions"),
        Line::from("  • N (tree focused) picks a template; applies Blank/Daily/Meeting/Project"),
        Line::from("  • Use Ctrl+J for instant fuzzy-jump to any note by title"),
        Line::from("  • Ctrl+V to switch vaults; works transparently with Obsidian folders"),
        Line::from(""),
        
        // System Info
        Line::from(Span::styled("─".repeat(72), Style::default().fg(TokyoNightTheme::FG_GUTTER))),
        Line::from(""),
        Line::from(vec![
            Span::styled("📋 Version: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(format!("{} v{}", PKG_NAME, VERSION), Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled("  |  ", Style::default().fg(TokyoNightTheme::FG_GUTTER)),
            Span::styled("🔧 Editor: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(&editor_info, Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
        Line::from(""),
        
        // Exit instructions
        Line::from(vec![
            Span::styled("💡 ", Style::default().fg(TokyoNightTheme::YELLOW)),
            Span::styled("Navigate: ", Style::default().fg(TokyoNightTheme::FG_DARK)),
            Span::styled("j/k", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" line  ", Style::default().fg(TokyoNightTheme::FG_DARK)),
            Span::styled("d/u", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" half-page  ", Style::default().fg(TokyoNightTheme::FG_DARK)),
            Span::styled("g/G", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" top/bottom  ", Style::default().fg(TokyoNightTheme::FG_DARK)),
            Span::styled("q/Esc", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled(" close", Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
    ]);

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .style(TokyoNightTheme::normal())
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left)
        .scroll((app.help_scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_autocomplete_popup(f: &mut Frame, app: &App, editor_area: Rect) {
    if !app.autocomplete_state.active || app.autocomplete_state.suggestions.is_empty() {
        return;
    }

    // Calculate popup position based on cursor
    let cursor_x = app.editor_cursor.1;
    let cursor_y = app.editor_cursor.0.saturating_sub(app.editor_scroll);

    // Position popup below cursor, but adjust if it would go off screen
    let popup_height = (app.autocomplete_state.suggestions.len() as u16 + 2).min(8); // Max 6 suggestions + border
    let popup_width = 40;
    
    let popup_x = editor_area.x + cursor_x.min(editor_area.width.saturating_sub(popup_width));
    let popup_y = if cursor_y + popup_height < editor_area.height {
        editor_area.y + cursor_y + 1 // Below cursor
    } else {
        editor_area.y + cursor_y.saturating_sub(popup_height) // Above cursor
    };

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    // Create list items for suggestions
    let items: Vec<ListItem> = app.autocomplete_state.suggestions
        .iter()
        .enumerate()
        .map(|(i, suggestion)| {
            let is_selected = i == app.autocomplete_state.selected_index;
            
            let icon = if suggestion.trigger.starts_with("#") {
                "#"
            } else if suggestion.trigger == "-" || suggestion.trigger == "*" {
                "•"
            } else if suggestion.trigger.contains("`") {
                "`"
            } else if suggestion.trigger == "[" || suggestion.trigger == "![" {
                "🔗"
            } else if suggestion.trigger == "**" || suggestion.trigger == "*" {
                "*"
            } else if suggestion.trigger == "|" {
                "📋"
            } else {
                "📝"
            };
            
            let style = if is_selected {
                Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::BLUE)
            } else {
                Style::default().fg(TokyoNightTheme::FG)
            };
            
            let line = Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(TokyoNightTheme::CYAN)),
                Span::styled(&suggestion.description, style),
            ]);
            
            ListItem::new(line).style(style)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("💡 Autocomplete")
        .border_style(Style::default().fg(TokyoNightTheme::BLUE))
        .style(Style::default().bg(TokyoNightTheme::BG_POPUP));

    let list = List::new(items).block(block);
    f.render_widget(list, popup_area);
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

// New UI functions for the latest features

fn draw_recent_files_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{} Recent Files", Icons::CLOCK))
        .border_style(TokyoNightTheme::border_focused());

    let recent_files = app.get_recent_files_display();
    
    let items: Vec<ListItem> = recent_files
        .iter()
        .enumerate()
        .map(|(i, (_note_id, title, last_accessed))| {
            let is_selected = i == app.recent_files_selected;
            
            let style = if is_selected {
                TokyoNightTheme::selected()
            } else {
                Style::default().fg(TokyoNightTheme::FG)
            };
            
            let line = Line::from(vec![
                Span::styled(format!("{}  ", i + 1), Style::default().fg(TokyoNightTheme::COMMENT)),
                Span::styled(format!("{} ", Icons::NOTE), TokyoNightTheme::note_icon()),
                Span::styled(title, Style::default().fg(TokyoNightTheme::FG)),
                Span::styled(format!("  ({})", last_accessed), Style::default().fg(TokyoNightTheme::COMMENT)),
            ]);
            
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(TokyoNightTheme::normal());

    f.render_widget(list, area);
}

fn draw_quick_jump_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 60, f.area());
    f.render_widget(Clear, area);

    let _block = Block::default()
        .title(format!("{} Quick Jump", Icons::SEARCH))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    // Split area for input and results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input box
            Constraint::Min(5),     // Results
            Constraint::Length(2),  // Help text
        ])
        .split(area);

    // Draw input box
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Search")
        .border_style(TokyoNightTheme::border_focused());

    let input_text = if app.quick_jump_query.is_empty() {
        "Type to search notes..."
    } else {
        &app.quick_jump_query
    };

    let input_style = if app.quick_jump_query.is_empty() {
        Style::default().fg(TokyoNightTheme::COMMENT)
    } else {
        Style::default().fg(TokyoNightTheme::FG)
    };

    let input_paragraph = Paragraph::new(input_text)
        .block(input_block)
        .style(input_style);

    f.render_widget(input_paragraph, chunks[0]);

    // Draw results
    let results_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Results ({})", app.quick_jump_results.len()))
        .border_style(TokyoNightTheme::border_inactive());

    let results = app.get_quick_jump_results_display();
    let items: Vec<ListItem> = results
        .iter()
        .enumerate()
        .map(|(i, (_note_id, title, folder))| {
            let is_selected = i == app.quick_jump_selected;
            
            let style = if is_selected {
                Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::BLUE)
            } else {
                Style::default().fg(TokyoNightTheme::FG)
            };
            
            let line = Line::from(vec![
                Span::styled(format!("{} ", Icons::NOTE), TokyoNightTheme::note_icon()),
                Span::styled(title, Style::default().fg(TokyoNightTheme::FG)),
                Span::styled(format!("  in {}", folder), Style::default().fg(TokyoNightTheme::COMMENT)),
            ]);
            
            ListItem::new(line).style(style)
        })
        .collect();

    let results_list = List::new(items)
        .block(results_block)
        .style(TokyoNightTheme::normal());

    f.render_widget(results_list, chunks[1]);

    // Draw help text
    let help_text = "↑↓: Navigate • Enter: Open • Esc: Cancel";
    let help_paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(TokyoNightTheme::COMMENT))
        .alignment(Alignment::Center);

    f.render_widget(help_paragraph, chunks[2]);
}

fn draw_recent_files_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("{} Recent Files", Icons::CLOCK))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let recent_files = app.get_recent_files_display();
    
    if recent_files.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No recent files yet.", Style::default().fg(TokyoNightTheme::COMMENT)),
            ]),
            Line::from(vec![
                Span::styled("Open some notes to see them here!", Style::default().fg(TokyoNightTheme::COMMENT)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press Esc to close", Style::default().fg(TokyoNightTheme::COMMENT)),
            ]),
        ];
        
        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Center);
            
        f.render_widget(paragraph, area);
    } else {
        // Split area for list and help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),     // File list
                Constraint::Length(2),  // Help text
            ])
            .split(area);

        let items: Vec<ListItem> = recent_files
            .iter()
            .enumerate()
            .map(|(i, (_note_id, title, last_accessed))| {
                let is_selected = i == app.recent_files_selected;
                
                let style = if is_selected {
                    Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::BLUE)
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                };
                
                let line = Line::from(vec![
                    Span::styled(format!("{}  ", i + 1), Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} ", Icons::NOTE), TokyoNightTheme::note_icon()),
                    Span::styled(title, Style::default().fg(TokyoNightTheme::FG)),
                    Span::styled(format!("  ({})", last_accessed), Style::default().fg(TokyoNightTheme::COMMENT)),
                ]);
                
                ListItem::new(line).style(style)
            })
            .collect();

        let files_block = Block::default()
            .borders(Borders::ALL)
            .title("Select a file")
            .border_style(TokyoNightTheme::border_inactive());

        let list = List::new(items)
            .block(files_block)
            .style(TokyoNightTheme::normal());

        f.render_widget(list, chunks[0]);

        // Help text
        let help_text = "↑↓: Navigate • Enter: Open • 1-9: Quick select • Esc: Cancel";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, chunks[1]);
    }
}

fn draw_vault_switcher_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("📁 Vault Switcher"))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let vault_info = app.get_vault_display_info();
    
    if vault_info.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No Obsidian vaults found.", Style::default().fg(TokyoNightTheme::COMMENT)),
            ]),
            Line::from(vec![
                Span::styled("Create a vault in Obsidian or run scribble --vault <path>", Style::default().fg(TokyoNightTheme::COMMENT)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press Esc to close", Style::default().fg(TokyoNightTheme::COMMENT)),
            ]),
        ];
        
        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Center);
            
        f.render_widget(paragraph, area);
    } else {
        // Split area for list and help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),     // Vault list
                Constraint::Length(2),  // Help text
            ])
            .split(area);

        let items: Vec<ListItem> = vault_info
            .iter()
            .enumerate()
            .map(|(i, (name, path))| {
                let is_selected = i == app.vault_switcher_selected;
                
                let style = if is_selected {
                    Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::BLUE)
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                };
                
                let line = vec![
                    Line::from(vec![
                        Span::styled(format!("{}  ", i + 1), Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{} ", Icons::FOLDER_CLOSED), TokyoNightTheme::folder_icon()),
                        Span::styled(name, Style::default().fg(TokyoNightTheme::FG)),
                    ]),
                    Line::from(vec![
                        Span::styled(format!("  {}", path), Style::default().fg(TokyoNightTheme::COMMENT)),
                    ]),
                ];
                
                ListItem::new(line).style(style)
            })
            .collect();

        let vaults_block = Block::default()
            .borders(Borders::ALL)
            .title("Select a vault")
            .border_style(TokyoNightTheme::border_inactive());

        let list = List::new(items)
            .block(vaults_block)
            .style(TokyoNightTheme::normal());

        f.render_widget(list, chunks[0]);

        // Help text
        let help_text = "↑↓: Navigate • Enter: Switch • 1-9: Quick select • Esc: Cancel";
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center);

        f.render_widget(help_paragraph, chunks[1]);
    }
}

fn draw_tag_browser_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("🏷️ Tag Browser ({} tags, {} tagged notes)", 
            app.tag_manager.get_tag_count(),
            app.tag_manager.get_tagged_note_count()))
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Split area for tag list and help/filter info
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),     // Tag list
            Constraint::Length(4),   // Help text and active filters
        ])
        .split(inner_area);

    // Get tag items based on current sort
    let tag_items = app.get_tag_browser_items();
    
    if tag_items.is_empty() {
        // No tags found
        let no_tags_text = "No tags found\n\nTip: Add tags to your notes using:\n  • YAML frontmatter: tags: [example, test]\n  • Inline hashtags: #example #test";
        let no_tags_paragraph = Paragraph::new(no_tags_text)
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        
        f.render_widget(no_tags_paragraph, chunks[0]);
    } else {
        // Create list items for tags
        let items: Vec<ListItem> = tag_items
            .iter()
            .enumerate()
            .map(|(i, (tag_name, count))| {
                let style = if i == app.tag_browser_selected {
                    TokyoNightTheme::selected()
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                };
                
                // Check if this tag is in active filters
                let filter_indicator = if app.tag_filter_active.contains(&tag_name.to_string()) {
                    " ✓"
                } else {
                    ""
                };
                
                let tag_line = Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(TokyoNightTheme::FG_DARK)),
                    Span::styled("🏷️ ", Style::default().fg(TokyoNightTheme::YELLOW)),
                    Span::styled(format!("#{}", tag_name), Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" ({})", count), Style::default().fg(TokyoNightTheme::COMMENT)),
                    Span::styled(filter_indicator, Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD)),
                ]);
                
                ListItem::new(tag_line).style(style)
            })
            .collect();

        let sort_indicator = if app.tag_browser_sort_by_frequency { "frequency" } else { "alphabetical" };
        let tags_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Tags (sorted by {})", sort_indicator))
            .border_style(TokyoNightTheme::border_inactive());

        let list = List::new(items)
            .block(tags_block)
            .style(TokyoNightTheme::normal());

        f.render_widget(list, chunks[0]);
    }
    
    // Active filters and help text
    let mut help_lines = vec![];
    
    // Show active filters
    if !app.tag_filter_active.is_empty() {
        let filter_text = format!("Active filters: {}", 
            app.tag_filter_active.iter()
                .map(|tag| format!("#{}", tag))
                .collect::<Vec<_>>()
                .join(", "));
        help_lines.push(Line::from(Span::styled(filter_text, Style::default().fg(TokyoNightTheme::GREEN))));
        help_lines.push(Line::from(""));
    }
    
    // Help text
    help_lines.extend(vec![
        Line::from("↑↓: Navigate • Enter: Filter by tag • s: Toggle sort • c: Clear filters"),
        Line::from("1-9: Quick filter • Backspace: Remove last filter • Esc: Cancel"),
    ]);
    
    let help_paragraph = Paragraph::new(help_lines)
        .style(Style::default().fg(TokyoNightTheme::COMMENT))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    
    f.render_widget(help_paragraph, chunks[1]);
}

fn draw_backlinks_dialog(f: &mut Frame, app: &App) {
    let note_title = app.current_note.as_ref().map(|n| n.title.as_str()).unwrap_or("?");
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("🔗 Backlinks — notes linking to '{}'", note_title))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    if app.backlinks_cache.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No notes link to this note.",
                Style::default().fg(TokyoNightTheme::COMMENT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Create [[wiki links]] in other notes to see them here.",
                Style::default().fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC),
            )),
        ];
        let para = Paragraph::new(content).block(block).alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = app.backlinks_cache
        .iter()
        .enumerate()
        .map(|(i, (_, title))| {
            let style = if i == app.backlinks_selected {
                TokyoNightTheme::selected()
            } else {
                Style::default().fg(TokyoNightTheme::FG)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", Icons::NOTE), TokyoNightTheme::note_icon()),
                Span::styled(title.as_str(), Style::default().fg(TokyoNightTheme::FG)),
            ])).style(style)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(TokyoNightTheme::selected())
        .highlight_symbol("▶ ");
    let mut list_state = ListState::default();
    list_state.select(Some(app.backlinks_selected));
    f.render_stateful_widget(list, chunks[0], &mut list_state);

    f.render_widget(
        Paragraph::new("↑↓ / j/k: Navigate  Enter: Open  Esc/q: Close")
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center),
        chunks[1],
    );
}

fn draw_template_picker_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("📄 New Note From Template")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let templates = crate::app::App::get_templates();
    let mut content = vec![
        Line::from(Span::styled(
            "Choose a template:",
            Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, (name, _)) in templates.iter().enumerate() {
        let is_selected = i == app.template_picker_selected;
        let prefix = if is_selected { "> " } else { "  " };
        let style = if is_selected {
            Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TokyoNightTheme::FG)
        };
        content.push(Line::from(Span::styled(
            format!("{}{}. {}", prefix, i + 1, name),
            style,
        )));
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "↑↓ / j/k: Navigate  Enter: Apply  1-4: Quick pick  Esc: Cancel",
        Style::default().fg(TokyoNightTheme::COMMENT),
    )));

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_spell_suggest_dialog(f: &mut Frame, app: &App) {
    let (row, col, wlen) = app.spell_word_range;
    let word = app.editor_content
        .lines()
        .nth(row)
        .map(|l| &l[col.min(l.len())..(col + wlen).min(l.len())])
        .unwrap_or("");

    let area = centered_rect(44, 60, f.area());
    f.render_widget(Clear, area);

    let title = format!(" Spell: \"{}\" ", word);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TokyoNightTheme::RED))
        .style(TokyoNightTheme::popup());

    let mut content: Vec<Line> = Vec::new();

    if app.spell_suggestions.is_empty() {
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            "  No suggestions found.",
            Style::default().fg(TokyoNightTheme::COMMENT),
        )));
    } else {
        content.push(Line::from(Span::styled(
            "  Suggestions:",
            Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD),
        )));
        content.push(Line::from(""));
        for (i, sug) in app.spell_suggestions.iter().enumerate() {
            let is_sel = i == app.spell_suggestions_selected;
            let prefix = if is_sel { "> " } else { "  " };
            let style = if is_sel {
                Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TokyoNightTheme::FG)
            };
            content.push(Line::from(Span::styled(
                format!("{}{}.  {}", prefix, i + 1, sug),
                style,
            )));
        }
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "  ↑↓/j/k: Navigate  Enter: Apply  1-9: Pick  Esc: Cancel",
        Style::default().fg(TokyoNightTheme::COMMENT),
    )));

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn draw_theme_browser_dialog(f: &mut Frame, app: &App) {
    
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("🎨 Theme Browser")
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    // Get available themes
    let themes = crate::app::App::get_available_themes();
    let current_theme = app.current_theme_name();
    
    // Create simple list of themes
    let mut content = vec![
        Line::from(Span::styled("Available Themes:", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];
    
    for (i, &theme_name) in themes.iter().enumerate() {
        let is_selected = i == app.theme_browser_selected;
        let is_current = theme_name == current_theme;
        
        let prefix = if is_selected { "> " } else { "  " };
        let current_indicator = if is_current { " (current)" } else { "" };
        
        let style = if is_selected {
            Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default().fg(TokyoNightTheme::GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TokyoNightTheme::FG)
        };
        
        content.push(Line::from(Span::styled(
            format!("{}{}.  {}{}", prefix, i + 1, theme_name, current_indicator), 
            style
        )));
    }
    
    content.push(Line::from(""));
    content.push(Line::from(Span::styled("↑↓: Navigate • Enter: Apply • q: Cancel", Style::default().fg(TokyoNightTheme::COMMENT))));
    
    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);
    
    f.render_widget(paragraph, area);
}
