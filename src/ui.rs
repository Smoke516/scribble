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

    if app.current_note.is_none() {
        // Landing page takes the whole width. The sidebar and the landing page
        // are both "notes you could open", and showing them side by side puts a
        // worse-ordered list next to a better one. Press `e` for the tree as an
        // overlay; opening a note restores the split.
        draw_welcome_screen(
            f,
            app,
            chunks[1],
            Block::default()
                .borders(Borders::ALL)
                .border_style(TokyoNightTheme::border_inactive())
                .style(TokyoNightTheme::normal()),
        );
    } else if app.config.ui.show_sidebar {
        // Draw folder tree with recent files if enabled
        draw_folder_tree_with_recent(f, app, main_chunks[0]);

        // Draw editor
        draw_editor(f, app, main_chunks[1]);
    } else {
        // Sidebar off (the default): the editor gets the full width and the tree
        // is a Ctrl+E overlay. Set ui.show_sidebar = true to pin it back.
        draw_editor(f, app, chunks[1]);
    }
    
    
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
        AppMode::Explorer => draw_explorer_dialog(f, app),
        AppMode::Move => draw_move_dialog(f, app),
        AppMode::DeleteConfirm => draw_delete_confirm_dialog(f, app),
        AppMode::QuickJump => draw_quick_jump_dialog(f, app),
        AppMode::RecentFiles => draw_recent_files_dialog(f, app),
        AppMode::VaultSwitcher => draw_vault_switcher_dialog(f, app),
        AppMode::TagBrowser => draw_tag_browser_dialog(f, app),
        AppMode::TagInput => draw_tag_input_dialog(f, app),
        AppMode::ThemeBrowser => draw_theme_browser_dialog(f, app),
        AppMode::Rename => draw_rename_dialog(f, app),
        AppMode::Backlinks => draw_backlinks_dialog(f, app),
        AppMode::Outline => draw_outline_dialog(f, app),
        AppMode::Palette => draw_palette_dialog(f, app),
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

    if app.current_note.is_some() {
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

        // Lay the note out once, up front: the gutter, the scroll offset and the
        // cursor all read from this same layout, which is what stops them
        // disagreeing about where the wrapped lines fall.
        let gutter_w: u16 = if app.config.ui.show_line_numbers {
            if is_split_view { 4 } else { 6 }
        } else {
            0
        };
        let wrap_width = inner_rect.width.saturating_sub(gutter_w).max(1) as usize;
        let (screen_rows, line_start) = layout_note(content, wrap_width);

        // editor_scroll is a logical line; the viewport scrolls in screen rows.
        let top_logical = (app.editor_scroll as usize).min(line_start.len().saturating_sub(1));
        let mut screen_scroll = line_start.get(top_logical).copied().unwrap_or(0);

        // Keep the cursor on screen. The renderer is the only place that knows
        // how far the note actually wrapped, so the correction belongs here.
        let cursor_screen_row = screen_row_of(
            &screen_rows,
            &line_start,
            app.editor_cursor.0 as usize,
            app.editor_cursor.1 as usize,
        );
        let view_h = inner_rect.height.max(1) as usize;
        if cursor_screen_row < screen_scroll {
            screen_scroll = cursor_screen_row;
        } else if cursor_screen_row >= screen_scroll + view_h {
            screen_scroll = cursor_screen_row + 1 - view_h;
        }
        if let Some(r) = screen_rows.get(screen_scroll) {
            app.editor_scroll = r.logical as u16;
        }
        let screen_scroll_u16 = screen_scroll.min(u16::MAX as usize) as u16;

        // Optionally render line numbers; returns the rect for the content area
        let content_rect = if app.config.ui.show_line_numbers {
            let line_number_width = gutter_w;
            let editor_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(line_number_width),
                    Constraint::Min(1),
                ])
                .split(inner_rect);

            let cursor_row = (app.editor_cursor.0 + 1) as usize;
            let rel = app.config.ui.relative_line_numbers;
            // One entry per SCREEN row. Continuation rows are blank, so a number
            // always sits beside the line it belongs to.
            let line_numbers: Vec<Line> = screen_rows
                .iter()
                .map(|r| {
                    if r.start != 0 {
                        return Line::from(Span::raw(" ".repeat(line_number_width as usize)));
                    }
                    let i = r.logical + 1;
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
                .scroll((screen_scroll_u16, 0));
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
        if app.spell.enabled && !app.spell.errors.is_empty() {
            // Group errors by row
            use std::collections::HashMap;
            let mut row_errors: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
            for &(row, col, len) in &app.spell.errors {
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
        if app.note_search.active && !app.note_search.matches.is_empty() {
            let query_len = app.note_search.query.len().max(1);
            for (match_idx, &(row, col)) in app.note_search.matches.iter().enumerate() {
                let is_current = match_idx == app.note_search.selected;
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

        // Every highlight above works in logical lines. Wrap only now, by slicing
        // those styled lines into the screen rows computed up front — so the text,
        // the gutter and the scroll all derive from one layout. Rendered with
        // ratatui's wrap OFF, since the wrapping has already happened.
        let mut wrapped: Vec<Line> = screen_rows
            .iter()
            .map(|r| match styled_content.lines.get(r.logical) {
                Some(line) => slice_line(line, r.start, r.end),
                None => Line::from(String::new()),
            })
            .collect();

        // Block cursor for the non-insert modes, painted AFTER wrapping and into
        // the screen row it actually occupies. Painting it before would let a
        // cursor resting past end-of-line pad a logical line beyond the slice it
        // belongs to, and the cursor would simply vanish.
        if app.mode != AppMode::Insert && is_focused && app.current_note.is_some() {
            if let Some(line) = wrapped.get_mut(cursor_screen_row) {
                let col = screen_rows
                    .get(cursor_screen_row)
                    .map(|r| (app.editor_cursor.1 as usize).saturating_sub(r.start))
                    .unwrap_or(0);
                paint_cursor_in_line(
                    line,
                    col,
                    Style::default()
                        .fg(TokyoNightTheme::BG)
                        .bg(TokyoNightTheme::CYAN),
                );
            }
        }

        let paragraph = Paragraph::new(Text::from(wrapped))
            .style(TokyoNightTheme::normal())
            .scroll((screen_scroll_u16, 0));

        f.render_widget(paragraph, content_rect);

        // In-note search bar overlay (bottom of content area)
        if app.mode == AppMode::NoteSearch {
            let bar_y = (content_rect.y + content_rect.height).saturating_sub(1);
            let bar_rect = Rect::new(content_rect.x, bar_y, content_rect.width, 1);
            let match_info = if app.note_search.matches.is_empty() {
                " [no matches]".to_string()
            } else {
                format!(" [{}/{}]", app.note_search.selected + 1, app.note_search.matches.len())
            };
            let bar_line = Line::from(vec![
                Span::styled("/ ", Style::default().fg(TokyoNightTheme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(app.note_search.query.as_str(), Style::default().fg(TokyoNightTheme::FG)),
                Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::SLOW_BLINK)),
                Span::styled(match_info, Style::default().fg(TokyoNightTheme::COMMENT)),
            ]);
            f.render_widget(Clear, bar_rect);
            f.render_widget(Paragraph::new(bar_line)
                .style(Style::default().fg(TokyoNightTheme::FG).bg(TokyoNightTheme::BG_DARK)), bar_rect);
        } else if app.note_search.active && !app.note_search.matches.is_empty() {
            // Still show a subtle hint when active but not typing
            let bar_y = (content_rect.y + content_rect.height).saturating_sub(1);
            let bar_rect = Rect::new(content_rect.x, bar_y, content_rect.width, 1);
            let hint = format!(" /{} [{}/{}]  n/N: next/prev  Esc to clear",
                app.note_search.query, app.note_search.selected + 1, app.note_search.matches.len());
            f.render_widget(Clear, bar_rect);
            f.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(TokyoNightTheme::COMMENT)))
                .style(Style::default().bg(TokyoNightTheme::BG_DARK)), bar_rect);
        }

        // Show cursor if in insert mode
        if app.mode == AppMode::Insert && is_focused {
            // Same layout as everything else: the terminal cursor has to land on
            // the wrapped row, not on `line - scroll`.
            let row = cursor_screen_row.saturating_sub(screen_scroll);
            let col = screen_rows
                .get(cursor_screen_row)
                .map(|r| (app.editor_cursor.1 as usize).saturating_sub(r.start))
                .unwrap_or(0);
            if row < content_rect.height as usize && col < content_rect.width as usize {
                f.set_cursor_position((
                    content_rect.x + col as u16,
                    content_rect.y + row as u16,
                ));
            }
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
        // Render the markdown preview. Decorations are sized to the pane's inner
        // width (area minus the two border columns) so they fill without wrapping.
        let inner_width = area.width.saturating_sub(2) as usize;
        let preview_content = if app.editor_content.is_empty() {
            crate::preview::generate_preview_sample(&app.theme_manager, inner_width)
        } else {
            crate::preview::render_markdown_preview(&app.editor_content, &app.theme_manager, inner_width)
        };
        
        let paragraph = Paragraph::new(preview_content)
            .block(block)
            .style(TokyoNightTheme::normal())
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0)); // Preview scrolls independently of the editor
        
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
                "Press F2 to toggle preview mode.",
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

/// Block-letter wordmark, five rows tall. One entry per letter of "scribble".
///
/// Hand-drawn rather than pulled from a figlet font so the glyphs share a
/// consistent 5x5 cell and the whole mark stays a predictable width — a font
/// with variable-width glyphs would make centring drift between letters.
const WORDMARK: [[&str; 5]; 8] = [
    // s
    ["█████", "█    ", "█████", "    █", "█████"],
    // c
    ["█████", "█    ", "█    ", "█    ", "█████"],
    // r
    ["█████", "█   █", "█████", "█  █ ", "█   █"],
    // i
    ["█████", "  █  ", "  █  ", "  █  ", "█████"],
    // b
    ["█    ", "█    ", "█████", "█   █", "█████"],
    // b
    ["█    ", "█    ", "█████", "█   █", "█████"],
    // l
    ["█    ", "█    ", "█    ", "█    ", "█████"],
    // e
    ["█████", "█    ", "█████", "█    ", "█████"],
];

const WORDMARK_ROWS: usize = 5;
/// 8 glyphs of 5 columns, single-column gutter between them.
const WORDMARK_WIDTH: usize = 8 * 5 + 7;

/// The landing page.
///
/// Answers "what was I doing, and what should I do next" — recency, today's
/// note, outstanding work — rather than listing what the app can do. The feature
/// tour that used to live here duplicated `?`, and a landing page that is never
/// quiet is one you stop reading.
///
/// Laid out as a single centred column: the wordmark is centred over it, and
/// every row below shares a left edge with its key right-aligned against the
/// same right edge, so the eye can run straight down either column.
fn draw_welcome_screen(f: &mut Frame, app: &App, area: Rect, block: Block) {
    let d = app.dashboard();

    // Inner width, minus the block's borders.
    let inner = area.width.saturating_sub(2) as usize;
    // Wide enough for label + detail + key without either being clipped, but
    // never wider than the pane. Falls back gracefully in a narrow split.
    let col = inner.saturating_sub(8).clamp(20, 76).min(inner.saturating_sub(2));
    let left = (inner.saturating_sub(col)) / 2;

    let dim = Style::default().fg(TokyoNightTheme::COMMENT);
    let body = Style::default().fg(TokyoNightTheme::FG_DARK);
    let keycap = Style::default()
        .fg(TokyoNightTheme::CYAN)
        .add_modifier(Modifier::BOLD);
    let mark = Style::default().fg(TokyoNightTheme::BLUE);

    let mut lines: Vec<Line> = Vec::new();
    let pad = |n: usize| Span::raw(" ".repeat(n));
    let blank = |v: &mut Vec<Line>| v.push(Line::from(""));

    // ── Wordmark ────────────────────────────────────────────────────────────
    blank(&mut lines);
    blank(&mut lines);
    // Double the mark when the pane is wide and tall enough to carry it. At full
    // screen the 1x mark is lost in the width; at 1x it still fits a split pane.
    let scale: usize = if inner >= WORDMARK_WIDTH * 2 + 8 && area.height >= 34 { 2 } else { 1 };
    let mark_width = WORDMARK_WIDTH * scale;

    if inner >= mark_width + 2 {
        // Centred on the pane, not on the menu column: the mark is allowed to be
        // wider than the menu without dragging the key column out to the margin.
        let logo_left = (inner.saturating_sub(mark_width)) / 2;
        for row in 0..WORDMARK_ROWS {
            let mut text = String::new();
            for (i, glyph) in WORDMARK.iter().enumerate() {
                if i > 0 {
                    text.push_str(&" ".repeat(scale));
                }
                for ch in glyph[row].chars() {
                    for _ in 0..scale {
                        text.push(ch);
                    }
                }
            }
            // Repeating the row scales the stroke vertically to match.
            for _ in 0..scale {
                lines.push(Line::from(vec![
                    pad(logo_left),
                    Span::styled(text.clone(), mark),
                ]));
            }
        }
    } else {
        // Too narrow for the block letters; fall back rather than wrap them.
        let text = "s c r i b b l e";
        let l = left + (col.saturating_sub(text.len())) / 2;
        lines.push(Line::from(vec![
            pad(l),
            Span::styled(text, mark.add_modifier(Modifier::BOLD)),
        ]));
    }

    // Version, right-aligned under the mark like the reference layout.
    let ver = format!("v{}", VERSION);
    lines.push(Line::from(vec![
        pad(left + col.saturating_sub(ver.len())),
        Span::styled(ver, dim),
    ]));
    blank(&mut lines);
    blank(&mut lines);

    // A menu row: label on the left, dim detail after it, key hard right.
    let row = |lines: &mut Vec<Line>, label: &str, detail: &str, key: &str, selected: bool| {
        const INDENT: usize = 4;
        const GUTTER: usize = 2;
        let keylen = key.chars().count();

        // Label takes a fixed share; detail takes what is left after the key,
        // so the key column stays flush right whatever the labels do.
        let label_w = 28.min(col.saturating_sub(keylen + INDENT + GUTTER + 4));
        // A detail column too narrow to hold anything readable is worse than
        // no column at all — give the space back to the label instead.
        let mut detail_w = col
            .saturating_sub(INDENT + label_w + GUTTER + keylen + 2)
            .min(24);
        if detail_w < 10 {
            detail_w = 0;
        }
        let label_w = if detail_w == 0 {
            col.saturating_sub(INDENT + keylen + GUTTER + 2)
        } else {
            label_w
        };

        let clip = |t: &str, w: usize| -> String {
            if w == 0 {
                String::new()
            } else if t.chars().count() > w {
                format!("{}…", t.chars().take(w - 1).collect::<String>())
            } else {
                t.to_string()
            }
        };

        let gap = col.saturating_sub(INDENT + label_w + GUTTER + detail_w + keylen);
        // The caret is the only selection marker: a full-width highlight bar on a
        // page this sparse reads as an error state rather than a cursor.
        let (marker, label_style) = if selected {
            ("▸ ", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))
        } else {
            ("  ", body)
        };
        lines.push(Line::from(vec![
            pad(left + INDENT - 2),
            Span::styled(marker, Style::default().fg(TokyoNightTheme::CYAN)),
            Span::styled(format!("{:<w$}", clip(label, label_w), w = label_w), label_style),
            pad(GUTTER),
            Span::styled(format!("{:<w$}", clip(detail, detail_w), w = detail_w), dim),
            pad(gap),
            Span::styled(key.to_string(), keycap),
        ]));
    };

    // ── Menu ────────────────────────────────────────────────────────────────
    // Recents and actions are one list so j/k runs the whole thing, with a gap
    // marking where "what I was doing" ends and "what I could do" begins.
    if d.menu.is_empty() {
        row(&mut lines, "Write your first note", "", "n", false);
        row(&mut lines, "Start today's daily note", "", "F4", false);
        row(&mut lines, "Help", "", "?", false);
    } else {
        for (i, item) in d.menu.iter().enumerate() {
            if i == d.recent_count && d.recent_count > 0 {
                blank(&mut lines);
            }
            row(
                &mut lines,
                &item.label,
                &item.detail,
                &item.key,
                i == app.welcome_selected,
            );
        }
    }

    blank(&mut lines);
    blank(&mut lines);

    // ── Footer ──────────────────────────────────────────────────────────────
    // Centred and dim, in the spirit of the reference's plugin-count line.
    // Outstanding work gets its own accent line — it is the one thing here that
    // is a prompt rather than a statistic. Hidden entirely when there is none.
    if d.open_tasks > 0 {
        let text = format!(
            "{} open task{} across {} note{}",
            d.open_tasks,
            if d.open_tasks == 1 { "" } else { "s" },
            d.notes_with_tasks,
            if d.notes_with_tasks == 1 { "" } else { "s" }
        );
        let len = text.chars().count() + 2;
        lines.push(Line::from(vec![
            pad(left + (col.saturating_sub(len)) / 2),
            Span::styled("▸ ", Style::default().fg(TokyoNightTheme::ORANGE)),
            Span::styled(text, Style::default().fg(TokyoNightTheme::ORANGE)),
        ]));
        blank(&mut lines);
    }

    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("{} notes", d.note_count));
    parts.push(format!("{} folders", d.folder_count));
    if d.tag_count > 0 {
        parts.push(format!("{} tags", d.tag_count));
    }
    if let Some(v) = &d.vault_label {
        parts.push(v.clone());
    }
    let footer = parts.join(" · ");
    let flen = footer.chars().count();
    lines.push(Line::from(vec![
        pad(left + (col.saturating_sub(flen)) / 2),
        Span::styled(footer, dim),
    ]));

    f.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
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
        AppMode::TagInput => "TAG",
        AppMode::ThemeBrowser => "THEMES",
        AppMode::Rename => "RENAME",
        AppMode::NoteSearch => "NOTE SEARCH",
        AppMode::Backlinks => "BACKLINKS",
        AppMode::Visual => "VISUAL",
        AppMode::TemplatePicker => "TEMPLATE",
        AppMode::SpellSuggest => "SPELL",
        AppMode::Outline => "OUTLINE",
        AppMode::Palette => "PALETTE",
        AppMode::Explorer => "EXPLORER",
    };

    let pane_text = match app.focused_pane {
        FocusedPane::Folders => format!("{} FOLDERS", Icons::EXPLORER),
        FocusedPane::Editor => format!("{} EDITOR", Icons::EDITOR),
        FocusedPane::Preview => format!("{} PREVIEW", Icons::PREVIEW),
    };

    let mode_style = match app.mode {
        AppMode::Explorer => app.theme_manager.mode_command(),
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
        AppMode::TagInput => app.theme_manager.mode_input(),
        AppMode::ThemeBrowser => app.theme_manager.mode_command(), // Use command style for theme browser
        AppMode::Rename => app.theme_manager.mode_input(),
        AppMode::NoteSearch => app.theme_manager.mode_search(),
        AppMode::Backlinks => app.theme_manager.mode_command(),
        AppMode::Visual => Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::ORANGE).add_modifier(Modifier::BOLD),
        AppMode::TemplatePicker => app.theme_manager.mode_input(),
        AppMode::SpellSuggest => Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::RED).add_modifier(Modifier::BOLD),
        AppMode::Outline => app.theme_manager.mode_command(),
        AppMode::Palette => app.theme_manager.mode_command(),
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


fn draw_tag_input_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(55, 45, f.area());
    f.render_widget(Clear, area);

    let note_title = app.current_note.as_ref().map(|n| n.title.as_str()).unwrap_or("?");
    let (vault_tags, _) = app.get_tag_stats();
    let block = Block::default()
        .title(format!("🏷️  Tags — {}  ({} in vault)", note_title, vault_tags))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let mut content: Vec<Line> = Vec::new();

    // Current tags (chips)
    let tags = app.current_note_tags();
    content.push(Line::from(Span::styled(
        "Current tags:",
        Style::default().fg(TokyoNightTheme::COMMENT),
    )));
    if tags.is_empty() {
        content.push(Line::from(Span::styled(
            "  (none yet)",
            Style::default().fg(TokyoNightTheme::FG_DARK).add_modifier(Modifier::ITALIC),
        )));
    } else {
        let mut spans = vec![Span::raw("  ")];
        for tag in &tags {
            spans.push(Span::styled(
                format!(" #{} ", tag),
                Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::CYAN),
            ));
            spans.push(Span::raw(" "));
        }
        content.push(Line::from(spans));
    }
    content.push(Line::from(""));

    // Input line
    content.push(Line::from(vec![
        Span::styled("Add tag: ", Style::default().fg(TokyoNightTheme::COMMENT)),
        Span::styled(
            &app.input_buffer,
            Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::SLOW_BLINK)),
    ]));

    // Live suggestions
    if !app.input_buffer.is_empty() {
        let suggestions = app.get_tag_suggestions(&app.input_buffer);
        if !suggestions.is_empty() {
            content.push(Line::from(Span::styled(
                format!("  ↹ {}", suggestions.join("  ")),
                Style::default().fg(TokyoNightTheme::FG_DARK),
            )));
        }
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "Enter: add · Tab: complete · Backspace: del last · Esc: done",
        Style::default().fg(TokyoNightTheme::COMMENT).add_modifier(Modifier::ITALIC),
    )));

    let para = Paragraph::new(content).block(block).alignment(Alignment::Left);
    f.render_widget(para, area);
}

fn draw_advanced_search_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 30, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("🔍 Advanced Search")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());

    let help_text = "Prefixes: regex: case: folder: | History: ↑/↓ | Enter: Search | Esc: Cancel";
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



/// One screen row of the wrapped note: which logical line it came from, and the
/// half-open character range of that line it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScreenRow {
    logical: usize,
    start: usize,
    end: usize,
}

/// Lay the note out exactly as it will be drawn.
///
/// ratatui's own `Wrap` is deliberately not used here. It wraps at render time,
/// so the gutter, the scroll offset and the cursor each had to guess where the
/// breaks would fall — and each guessed differently: the gutter numbered screen
/// rows instead of lines, and the scroll counted lines while ratatui counted
/// rows. Wrapping here and rendering with wrap off makes this the single
/// authority, so there is no second algorithm left to disagree with.
///
/// Rows tile their line with no gaps, so every character — including the space a
/// break lands on — belongs to exactly one row and the cursor always maps.
fn layout_note(content: &str, width: usize) -> (Vec<ScreenRow>, Vec<usize>) {
    let width = width.max(1);
    let mut rows: Vec<ScreenRow> = Vec::new();
    let mut line_start: Vec<usize> = Vec::new();

    for (logical, line) in content.lines().enumerate() {
        line_start.push(rows.len());
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            rows.push(ScreenRow { logical, start: 0, end: 0 });
            continue;
        }
        let mut pos = 0usize;
        while pos < chars.len() {
            let hard = (pos + width).min(chars.len());
            let mut brk = hard;
            if hard < chars.len() {
                // Prefer the last space that fits; a word longer than the pane
                // still has to be broken mid-word.
                let mut b = hard;
                while b > pos && !chars[b - 1].is_whitespace() {
                    b -= 1;
                }
                if b > pos {
                    brk = b;
                }
            }
            rows.push(ScreenRow { logical, start: pos, end: brk });
            pos = brk;
        }
    }

    if rows.is_empty() {
        line_start.push(0);
        rows.push(ScreenRow { logical: 0, start: 0, end: 0 });
    }
    (rows, line_start)
}

/// Take the characters `[start, end)` of a styled line, keeping each span's own
/// style so wrapping does not flatten the markdown colouring.
fn slice_line(line: &Line<'_>, start: usize, end: usize) -> Line<'static> {
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut seen = 0usize;
    for span in &line.spans {
        let chars: Vec<char> = span.content.chars().collect();
        let n = chars.len();
        let from = seen.max(start);
        let to = (seen + n).min(end);
        if from < to {
            let text: String = chars[(from - seen)..(to - seen)].iter().collect();
            out.push(Span::styled(text, span.style));
        }
        seen += n;
        if seen >= end {
            break;
        }
    }
    if out.is_empty() {
        out.push(Span::raw(String::new()));
    }
    Line::from(out)
}

/// Which screen row holds a given cursor position.
fn screen_row_of(rows: &[ScreenRow], line_start: &[usize], logical: usize, col: usize) -> usize {
    let first = line_start.get(logical).copied().unwrap_or(0);
    let mut last = first;
    for (i, r) in rows.iter().enumerate().skip(first) {
        if r.logical != logical {
            break;
        }
        last = i;
        if col >= r.start && col < r.end {
            return i;
        }
    }
    // Resting past the final character of the line.
    last
}

/// Paint the block cursor into the text itself instead of overlaying it at a
/// computed screen position.
///
/// The editor soft-wraps, so one logical line can occupy several screen rows and
/// `cursor_row - scroll` is simply not where the character is: the cursor drifted
/// upward by one row for every wrapped row above it, landing off the very line
/// the highlight had marked. Styling the character in place hands the positioning
/// to ratatui's own wrapping — which is why the current-line highlight, done the
/// same way, was always correct.
///
/// Only the span containing the cursor is split, so markdown colouring either
/// side of it survives.
fn paint_cursor_in_line(line: &mut Line<'_>, col: usize, cursor_style: Style) {
    let mut out: Vec<Span> = Vec::new();
    let mut seen = 0usize;
    let mut placed = false;

    for span in std::mem::take(&mut line.spans) {
        let text = span.content.to_string();
        let len = text.chars().count();
        if !placed && col >= seen && col < seen + len {
            let k = col - seen;
            let before: String = text.chars().take(k).collect();
            let at: String = text.chars().skip(k).take(1).collect();
            let after: String = text.chars().skip(k + 1).collect();
            if !before.is_empty() {
                out.push(Span::styled(before, span.style));
            }
            out.push(Span::styled(at, cursor_style));
            if !after.is_empty() {
                out.push(Span::styled(after, span.style));
            }
            placed = true;
        } else {
            out.push(Span::styled(text, span.style));
        }
        seen += len;
    }

    if !placed {
        // Past the end of the line: an empty line, or resting on the newline in
        // Normal mode. Show the cursor on a trailing space.
        let gap = col.saturating_sub(seen);
        if gap > 0 {
            out.push(Span::raw(" ".repeat(gap)));
        }
        out.push(Span::styled(" ".to_string(), cursor_style));
    }

    line.spans = out;
}


/// The destination picker for a move.
///
/// `execute_move` reads the tree selection, so moving has always required the
/// tree to be on screen to aim at. With the sidebar hidden by default there was
/// nothing to aim at: the status bar said "select destination folder" and no
/// selection was visible anywhere. Same tree as the explorer, with a banner
/// naming what is in flight.
fn draw_move_dialog(f: &mut Frame, app: &mut App) {
    let area = centered_rect(56, 76, f.area());
    f.render_widget(Clear, area);
    // Opaque, like every other dialog — see draw_explorer_dialog.
    f.render_widget(Block::default().style(TokyoNightTheme::popup()), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    let what = app
        .move_item_id
        .and_then(|id| {
            app.folder_tree_items
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.name.clone())
        })
        .unwrap_or_else(|| "item".to_string());

    // Trim the name rather than the key hints: the hints are the part you need
    // when you have never used this dialog before.
    let hints = "Enter here · ~ root · h/l fold · Esc";
    let room = (chunks[0].width as usize).saturating_sub(hints.len() + 10);
    let what = if what.chars().count() > room && room > 1 {
        format!("{}…", what.chars().take(room - 1).collect::<String>())
    } else {
        what
    };

    let banner = Line::from(vec![
        Span::styled(
            format!(" Move \"{}\" to  ", what),
            Style::default()
                .fg(TokyoNightTheme::ORANGE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(hints, Style::default().fg(TokyoNightTheme::COMMENT)),
    ]);
    f.render_widget(
        Paragraph::new(banner).style(Style::default().bg(TokyoNightTheme::BG_DARK)),
        chunks[0],
    );

    draw_folder_tree(f, app, chunks[1]);
}

/// The folder tree, floating. Deliberately the same renderer as the sidebar
/// rather than a second tree widget: one tree, one set of behaviours, and it
/// keeps working when the sidebar is not on screen.
fn draw_explorer_dialog(f: &mut Frame, app: &mut App) {
    let area = centered_rect(52, 74, f.area());
    f.render_widget(Clear, area);
    // draw_folder_tree styles itself for the sidebar, which sits on the app
    // background and so sets no background of its own. As an overlay that leaves
    // the note behind it showing through — and, on a transparent terminal, the
    // desktop. Lay the popup background down first; the tree paints over it with
    // foreground colours only, so it survives.
    f.render_widget(Block::default().style(TokyoNightTheme::popup()), area);
    draw_folder_tree(f, app, area);
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

    let spell_status = if !app.spell.aspell_available {
        "aspell not installed"
    } else if app.spell.enabled {
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
                if app.spell.enabled { TokyoNightTheme::GREEN } else { TokyoNightTheme::COMMENT }
            )),
            Span::styled("   Editor: ", Style::default().fg(TokyoNightTheme::COMMENT)),
            Span::styled(&editor_info, Style::default().fg(TokyoNightTheme::FG_DARK)),
        ]),
        Line::from(""),

        // ── Landing Page ──────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Landing Page  ", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled("(shown when no note is open)", Style::default().fg(TokyoNightTheme::COMMENT)),
        ]),
        Line::from(""),
        Line::from("  j / k      Move down / up                 Enter   Open the highlighted row"),
        Line::from("  1 - 8      Jump straight to that recent note"),
        Line::from("  F4         Today's daily note             Ctrl+E  Browse the vault"),
        Line::from("  q          Quit                           ?       This help"),
        Line::from("  Shows your eight most recent notes, today's note, and any open tasks."),
        Line::from(""),

        // ── Notes & Explorer ──────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Notes & Explorer", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  Ctrl+E     Explorer: the folder tree as an overlay (sidebar is off by default)"),
        Line::from("  In Explorer: j/k move · Enter open · h collapse · Esc / e / q close"),
        Line::from("               n new note · f new folder · F folder at root"),
        Line::from("               r rename · m move · d delete (returns to the tree)"),
        Line::from(""),
        Line::from("  n          New note                        N    New note from template"),
        Line::from("  r          Rename current note             m    Move current note"),
        Line::from("  u          Undo last delete                dd   Delete line (editor)"),
        Line::from("  Enter      Open note / expand folder"),
        Line::from("  q          Close note → landing page       Q       Quit immediately"),
        Line::from("  m          Move: j/k pick a folder · ~ vault root · Enter drop · Esc cancel"),
        Line::from("  Pin the sidebar back with  show_sidebar = true  under [ui] in config.toml"),
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
        Line::from("  x          Delete char at cursor         p / P   Paste yank / system clipboard"),
        Line::from("  Ctrl+Z     Undo                          Ctrl+Y  Redo"),
        Line::from(""),
        Line::from("  Operators: d delete, c change, y yank. Combine with a motion:"),
        Line::from("  dw / cw    Word            d$ / D    To line end     dd / cc / yy  Whole line"),
        Line::from("  db / de    Word back/end   d0 / d^   To line start    dG / dgg      To end / start"),
        Line::from("  diw / daw  Inner / a word  dj / dk   Line below/above 3dd / d3w     With a count"),
        Line::from("  D / C / Y  Shorthand for d$ / c$ / yy"),
        Line::from("  (e is a motion only after an operator: plain e opens the external editor)"),
        Line::from(""),
        Line::from("  v          Enter Visual select mode"),
        Line::from("  Space      Toggle task checkbox [ ] <-> [x] on current line"),
        Line::from("  Ctrl+P     Go to: notes, tags, headings, commands (one door for all of them)"),
        Line::from("             > commands   # tags   ? full text   @ headings"),
        Line::from("  Ctrl+G     Outline: jump to a heading    F4      Open today's daily note"),
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
        Line::from("  Ctrl+S     Save                           F2      Toggle preview"),
        Line::from("  Ctrl+V     Paste system clipboard         Ctrl+L  Follow [[wiki link]] at cursor"),
        Line::from("  Ctrl+U/D   Half-page scroll up / down"),
        Line::from("  Esc        Exit Insert mode (auto-saves + spell check)"),
        Line::from(""),

        // ── Search ────────────────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Search", Style::default().fg(TokyoNightTheme::PURPLE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  / (tree)   Search across all notes        Ctrl+F  Fuzzy note search"),
        Line::from("  Ctrl+A     Advanced search (regex: case: folder:)  Ctrl+R  Replace"),
        Line::from("  / (editor) In-note search (highlighted)   n/N    Next / prev match"),
        Line::from("  Esc        Clear in-note highlights"),
        Line::from("  Ctrl+J     Quick Jump (fuzzy)              Ctrl+O  Recent files"),
        Line::from("  Ctrl+L     Follow [[wiki link]] at cursor   Ctrl+B  Links panel (in + out)"),
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
        Line::from("  t          Edit current note's tags       Ctrl+T  Open tag browser"),
        Line::from("  Enter      Filter notes by tag            s      Toggle sort order"),
        Line::from("  1-9        Quick tag select               c      Clear filters"),
        Line::from("  In tag editor: type+Enter add · Tab complete · Backspace removes last"),
        Line::from("  YAML: tags: [work, ideas]   Inline: #hashtag"),
        Line::from(""),

        // ── Preview & Display ─────────────────────────────────────────────────
        Line::from(vec![
            Span::styled("Preview & Display", Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  F2         Toggle live Markdown preview    Tab     Cycle panes"),
        Line::from("  Preview pane: j/k scroll · g/G top/bottom (independent of editor)"),
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
        Line::from("  Ctrl+B     Links panel: incoming + outgoing (Tab switches)"),
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
        Line::from("  • N (outside the editor) picks a template: Blank/Daily/Meeting/Project"),
        Line::from("  • Use Ctrl+J for instant fuzzy-jump to any note by title"),
        Line::from("  • Ctrl+V to switch vaults; works transparently with Obsidian folders"),
        Line::from("  • Prefer a permanent sidebar? show_sidebar = true under [ui] in config.toml"),
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
        .title("📁 Vault Switcher".to_string())
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
    let area = centered_rect(60, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("🔗 Links — '{}'", note_title))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Reserve space for the outgoing section only when there is one.
    let out_h = if app.links.outgoing.is_empty() {
        0
    } else {
        (app.links.outgoing.len() as u16 + 2).min(8)
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(out_h), Constraint::Length(1)])
        .split(inner);

    let incoming_focused = app.links.focus == crate::app::BacklinkFocus::Incoming;
    let outgoing_focused = !incoming_focused;
    let border_for = |focused: bool| if focused {
        TokyoNightTheme::border_focused()
    } else {
        TokyoNightTheme::border_inactive()
    };

    // ── Incoming: notes that link to this one ──
    let in_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Linked from ({})", app.links.incoming.len()))
        .border_style(border_for(incoming_focused));
    if app.links.incoming.is_empty() {
        f.render_widget(
            Paragraph::new("No notes link here yet.")
                .block(in_block)
                .style(Style::default().fg(TokyoNightTheme::COMMENT))
                .alignment(Alignment::Center),
            chunks[0],
        );
    } else {
        let items: Vec<ListItem> = app.links.incoming
            .iter()
            .map(|(_, title)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", Icons::NOTE), TokyoNightTheme::note_icon()),
                    Span::styled(title.as_str(), Style::default().fg(TokyoNightTheme::FG)),
                ]))
            })
            .collect();
        let list = List::new(items)
            .block(in_block)
            .highlight_style(TokyoNightTheme::selected())
            .highlight_symbol("▶ ");
        let mut list_state = ListState::default();
        // Only show a highlighted row in the section that has focus.
        list_state.select(incoming_focused.then_some(app.links.incoming_selected));
        f.render_stateful_widget(list, chunks[0], &mut list_state);
    }

    // ── Outgoing: notes this one links to (broken links flagged) ──
    if !app.links.outgoing.is_empty() {
        let items: Vec<ListItem> = app.links.outgoing
            .iter()
            .map(|(target_id, title)| {
                if target_id.is_some() {
                    ListItem::new(Line::from(vec![
                        Span::styled("→ ", Style::default().fg(TokyoNightTheme::COMMENT)),
                        Span::styled(title.as_str(), Style::default().fg(TokyoNightTheme::FG)),
                    ]))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled("→ ", Style::default().fg(TokyoNightTheme::COMMENT)),
                        Span::styled(title.as_str(), Style::default().fg(TokyoNightTheme::RED)),
                        Span::styled("  (missing — Enter to create)", Style::default().fg(TokyoNightTheme::COMMENT)),
                    ]))
                }
            })
            .collect();
        let out_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Links to ({})", app.links.outgoing.len()))
            .border_style(border_for(outgoing_focused));
        let list = List::new(items)
            .block(out_block)
            .highlight_style(TokyoNightTheme::selected())
            .highlight_symbol("▶ ");
        let mut list_state = ListState::default();
        list_state.select(outgoing_focused.then_some(app.links.outgoing_selected));
        f.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    let hint = if app.links.outgoing.is_empty() || app.links.incoming.is_empty() {
        "↑↓ / j/k: Navigate  Enter: Open  Esc/q: Close"
    } else {
        "↑↓ / j/k: Navigate  Tab: Switch section  Enter: Open  Esc/q: Close"
    };
    f.render_widget(
        Paragraph::new(hint)
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_outline_dialog(f: &mut Frame, app: &App) {
    let note_title = app.current_note.as_ref().map(|n| n.title.as_str()).unwrap_or("?");
    let area = centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("🗂  Outline — '{}'", note_title))
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = app.outline_headings
        .iter()
        .map(|(_, level, text)| {
            // Indent by heading level so the document structure is visible.
            let indent = "  ".repeat((*level as usize).saturating_sub(1));
            ListItem::new(Line::from(vec![
                Span::styled(indent, Style::default()),
                Span::styled(format!("{} ", "#".repeat(*level as usize)), Style::default().fg(TokyoNightTheme::COMMENT)),
                Span::styled(text.as_str(), Style::default().fg(TokyoNightTheme::FG)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(TokyoNightTheme::selected())
        .highlight_symbol("▶ ");
    let mut list_state = ListState::default();
    list_state.select(Some(app.outline_selected));
    f.render_stateful_widget(list, chunks[0], &mut list_state);

    f.render_widget(
        Paragraph::new("↑↓ / j/k: Navigate  Enter: Jump  Esc/q: Close")
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center),
        chunks[1],
    );
}

/// The palette: a text field, a hint, and one ranked list of everything.
///
/// The kind badge is what lets four pools share one list — without it the rows
/// read as four lists stapled together rather than as an ordering by relevance.
fn draw_palette_dialog(f: &mut Frame, app: &App) {
    use crate::palette::Mode;

    let (mode, _) = Mode::split(&app.palette_query);
    let area = centered_rect(70, 70, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("  Go to")
        .borders(Borders::ALL)
        .border_style(TokyoNightTheme::border_focused())
        .style(TokyoNightTheme::popup());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // query
            Constraint::Length(1), // hint
            Constraint::Min(3),    // results
            Constraint::Length(1), // keys
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(TokyoNightTheme::GREEN)),
            Span::styled(
                app.palette_query.as_str(),
                Style::default().fg(TokyoNightTheme::FG),
            ),
            Span::styled("█", Style::default().fg(TokyoNightTheme::CYAN)),
        ])),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(mode.hint()).style(Style::default().fg(TokyoNightTheme::COMMENT)),
        chunks[1],
    );

    let items: Vec<ListItem> = app
        .palette_items
        .iter()
        .map(|item| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<5}", item.kind.badge()),
                    Style::default().fg(match item.kind {
                        crate::palette::Kind::Note => TokyoNightTheme::BLUE,
                        crate::palette::Kind::Tag => TokyoNightTheme::MAGENTA,
                        crate::palette::Kind::Heading => TokyoNightTheme::CYAN,
                        crate::palette::Kind::Command => TokyoNightTheme::YELLOW,
                    }),
                ),
                Span::styled(item.label.as_str(), Style::default().fg(TokyoNightTheme::FG)),
                Span::styled(
                    if item.detail.is_empty() {
                        String::new()
                    } else {
                        format!("   {}", item.detail)
                    },
                    Style::default().fg(TokyoNightTheme::COMMENT),
                ),
            ]))
        })
        .collect();

    if items.is_empty() {
        f.render_widget(
            Paragraph::new("No matches")
                .style(Style::default().fg(TokyoNightTheme::COMMENT))
                .alignment(Alignment::Center),
            chunks[2],
        );
    } else {
        let list = List::new(items)
            .highlight_style(TokyoNightTheme::selected())
            .highlight_symbol("▶ ");
        let mut list_state = ListState::default();
        list_state.select(Some(app.palette_selected));
        f.render_stateful_widget(list, chunks[2], &mut list_state);
    }

    f.render_widget(
        Paragraph::new("↑↓ / Ctrl+N/P: Navigate  Enter: Go  Esc: Close")
            .style(Style::default().fg(TokyoNightTheme::COMMENT))
            .alignment(Alignment::Center),
        chunks[3],
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
    let (row, col, wlen) = app.spell.word_range;
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

    if app.spell.suggestions.is_empty() {
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
        for (i, sug) in app.spell.suggestions.iter().enumerate() {
            let is_sel = i == app.spell.suggestions_selected;
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

#[cfg(test)]
mod preview_tests {
    use super::*;
    use crate::models::Note;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Render the whole UI to a string grid at the given size.
    fn render(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..h {
            for x in 0..w {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn app_with_preview(content: &str) -> App {
        let mut app = App::default();
        let mut note = Note::new("Probe".to_string(), None);
        note.content = content.to_string();
        app.notebook.add_note(note.clone());
        app.current_note = Some(note);
        app.editor_content = content.to_string();
        app.preview_enabled = true;
        app
    }

    #[test]
    fn preview_pane_renders_markdown() {
        let mut app = app_with_preview("# Hello\n\nWorld text\n- [ ] todo\n- [x] done");
        let screen = render(&mut app, 100, 30);
        assert!(screen.contains("Live Preview"), "preview pane title missing");
        assert!(screen.contains("Hello"), "heading not rendered");
        assert!(screen.contains("todo") && screen.contains("done"), "tasks not rendered");
        assert!(screen.contains('☐') && screen.contains('☑'), "task checkboxes not rendered");
    }

    #[test]
    fn decorations_never_exceed_pane_width() {
        // Heading underlines, code-block borders and horizontal rules must fit
        // within the pane width, or they wrap into stray stub lines. Render the
        // unit directly at a narrow width and assert every decoration line fits.
        let theme = crate::theme::ThemeManager::new("tokyo-night");
        let md = "# Heading\n\nText\n\n---\n\n```rust\nfn x() {}\n```\n";
        let width = 30usize;
        let text = crate::preview::render_markdown_preview(md, &theme, width);
        for line in &text.lines {
            let w: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            assert!(w <= width, "preview line {:?} is {} cols, exceeds pane width {}",
                line.spans.iter().map(|s| s.content.as_ref()).collect::<String>(), w, width);
        }
    }
}

#[cfg(test)]
mod cursor_paint_tests {
    use super::*;

    fn plain(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn cursor_char(line: &Line) -> Option<String> {
        line.spans
            .iter()
            .find(|s| s.style.bg == Some(TokyoNightTheme::CYAN))
            .map(|s| s.content.to_string())
    }

    fn cur() -> Style {
        Style::default().fg(TokyoNightTheme::BG).bg(TokyoNightTheme::CYAN)
    }

    #[test]
    fn marks_the_character_under_the_cursor_without_changing_the_text() {
        let mut line = Line::from("hello world");
        paint_cursor_in_line(&mut line, 6, cur());
        assert_eq!(plain(&line), "hello world", "text must be untouched");
        assert_eq!(cursor_char(&line).as_deref(), Some("w"));
    }

    /// Splitting must not flatten the markdown colouring either side of it.
    #[test]
    fn surrounding_styles_survive_the_split() {
        let red = Style::default().fg(TokyoNightTheme::RED);
        let mut line = Line::from(vec![Span::styled("abcdef", red)]);
        paint_cursor_in_line(&mut line, 3, cur());

        assert_eq!(plain(&line), "abcdef");
        assert_eq!(cursor_char(&line).as_deref(), Some("d"));
        let kept: Vec<&str> = line
            .spans
            .iter()
            .filter(|s| s.style.fg == Some(TokyoNightTheme::RED))
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(kept, vec!["abc", "ef"], "colour preserved on both sides");
    }

    /// Normal mode rests the cursor past the last character on empty lines.
    #[test]
    fn empty_line_still_shows_a_cursor() {
        let mut line = Line::from("");
        paint_cursor_in_line(&mut line, 0, cur());
        assert_eq!(cursor_char(&line).as_deref(), Some(" "));
    }

    #[test]
    fn cursor_past_end_of_line_pads_out_to_it() {
        let mut line = Line::from("ab");
        paint_cursor_in_line(&mut line, 5, cur());
        assert_eq!(plain(&line), "ab    ", "padded to the cursor column");
        assert_eq!(cursor_char(&line).as_deref(), Some(" "));
    }

    #[test]
    fn cursor_on_the_first_character_works() {
        let mut line = Line::from("xyz");
        paint_cursor_in_line(&mut line, 0, cur());
        assert_eq!(plain(&line), "xyz");
        assert_eq!(cursor_char(&line).as_deref(), Some("x"));
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    fn text_of(content: &str, rows: &[ScreenRow]) -> Vec<String> {
        let lines: Vec<&str> = content.lines().collect();
        rows.iter()
            .map(|r| {
                lines
                    .get(r.logical)
                    .map(|l| l.chars().skip(r.start).take(r.end - r.start).collect())
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn short_lines_are_one_row_each() {
        let (rows, starts) = layout_note("one\ntwo\nthree", 40);
        assert_eq!(rows.len(), 3);
        assert_eq!(starts, vec![0, 1, 2]);
        assert!(rows.iter().all(|r| r.start == 0));
    }

    #[test]
    fn a_long_line_breaks_on_a_space() {
        let (rows, starts) = layout_note("aaa bbb ccc ddd", 8);
        assert_eq!(starts, vec![0], "still one logical line");
        assert!(rows.len() > 1, "but several screen rows");
        let shown = text_of("aaa bbb ccc ddd", &rows);
        assert!(
            shown.iter().all(|r| r.chars().count() <= 8),
            "no row exceeds the width: {:?}",
            shown
        );
        assert_eq!(shown.concat(), "aaa bbb ccc ddd", "no character is lost");
    }

    /// A word longer than the pane has to be broken mid-word rather than
    /// overflowing or being dropped.
    #[test]
    fn an_over_long_word_is_hard_broken() {
        let content = "supercalifragilistic";
        let (rows, _) = layout_note(content, 6);
        let shown = text_of(content, &rows);
        assert!(shown.iter().all(|r| r.chars().count() <= 6), "{:?}", shown);
        assert_eq!(shown.concat(), content);
    }

    /// Rows must tile their line with no gaps, or a cursor resting on the space a
    /// break landed on would belong to no row at all.
    #[test]
    fn rows_tile_their_line_without_gaps() {
        let content = "the quick brown fox jumps over the lazy dog";
        let (rows, _) = layout_note(content, 11);
        let mut expected = 0usize;
        for r in &rows {
            assert_eq!(r.start, expected, "gap or overlap at {:?}", r);
            expected = r.end;
        }
        assert_eq!(expected, content.chars().count());
    }

    #[test]
    fn empty_lines_still_occupy_a_row() {
        let (rows, starts) = layout_note("a\n\nb", 10);
        assert_eq!(rows.len(), 3);
        assert_eq!(starts, vec![0, 1, 2]);
        assert_eq!(rows[1].start, rows[1].end, "the blank line is a zero-width row");
    }

    #[test]
    fn empty_content_still_yields_one_row() {
        let (rows, starts) = layout_note("", 10);
        assert_eq!(rows.len(), 1);
        assert_eq!(starts, vec![0]);
    }

    #[test]
    fn cursor_maps_to_the_row_holding_its_column() {
        let content = "aaa bbb ccc ddd";
        let (rows, starts) = layout_note(content, 8);
        // column 0 is on the first row; a column past the first break is not
        assert_eq!(screen_row_of(&rows, &starts, 0, 0), 0);
        let later = screen_row_of(&rows, &starts, 0, 12);
        assert!(later > 0, "a column past the wrap is on a continuation row");
        assert!(rows[later].start <= 12 && 12 < rows[later].end);
    }

    /// Resting past the last character (Normal mode at end of line) still lands
    /// on that line's final row rather than falling through to the next line.
    #[test]
    fn cursor_past_end_of_line_stays_on_that_line() {
        let content = "short\nnext";
        let (rows, starts) = layout_note(content, 20);
        let r = screen_row_of(&rows, &starts, 0, 99);
        assert_eq!(rows[r].logical, 0);
    }

    #[test]
    fn slicing_preserves_span_styles() {
        let red = Style::default().fg(TokyoNightTheme::RED);
        let line = Line::from(vec![
            Span::styled("abcd", red),
            Span::raw("efgh"),
        ]);
        let cut = slice_line(&line, 2, 6);
        let plain: String = cut.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(plain, "cdef");
        assert_eq!(cut.spans[0].style.fg, Some(TokyoNightTheme::RED));
        assert_eq!(cut.spans[1].style.fg, None, "the unstyled half stays unstyled");
    }
}
