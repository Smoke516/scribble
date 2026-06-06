use super::*;

impl App {
    // -------------------------------------------------------------------------
    // Text undo
    // -------------------------------------------------------------------------

    /// Save a snapshot of the current editor content to the undo stack.
    /// Clears the redo stack because a new edit invalidates redo history.
    pub fn push_undo_snapshot(&mut self) {
        if let Some((last_content, _)) = self.undo_stack.last() {
            if last_content == &self.editor_content {
                return; // nothing changed
            }
        }
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push((self.editor_content.clone(), self.editor_cursor));
        self.redo_stack.clear();
    }

    /// Restore the previous editor snapshot. Pushes current state to redo stack.
    pub fn undo_text(&mut self) -> bool {
        if let Some((content, cursor)) = self.undo_stack.pop() {
            // Save current state to redo stack before replacing it
            if self.redo_stack.len() >= 50 {
                self.redo_stack.remove(0);
            }
            self.redo_stack.push((self.editor_content.clone(), self.editor_cursor));
            self.editor_content = content;
            self.editor_cursor = cursor;
            self.mark_modified();
            self.update_preview_content();
            true
        } else {
            false
        }
    }

    /// Re-apply the last undone change. Returns true if a redo snapshot was available.
    pub fn redo_text(&mut self) -> bool {
        if let Some((content, cursor)) = self.redo_stack.pop() {
            // Save current state to undo stack
            if self.undo_stack.len() >= 50 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push((self.editor_content.clone(), self.editor_cursor));
            self.editor_content = content;
            self.editor_cursor = cursor;
            self.mark_modified();
            self.update_preview_content();
            true
        } else {
            false
        }
    }

    // -------------------------------------------------------------------------
    // Vim-style cursor movement (Normal mode, editor focused)
    // -------------------------------------------------------------------------

    /// Move cursor down one line, adjusting scroll so it stays visible.
    pub fn cursor_down_normal(&mut self) {
        let line_count = self.editor_content.lines().count() as u16;
        if self.editor_cursor.0 < line_count.saturating_sub(1) {
            self.editor_cursor.0 += 1;
            let lines: Vec<&str> = self.editor_content.lines().collect();
            if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
                self.editor_cursor.1 = self.editor_cursor.1.min(line.len() as u16);
            }
            self.adjust_scroll_to_cursor();
        }
    }

    /// Move cursor up one line, adjusting scroll so it stays visible.
    pub fn cursor_up_normal(&mut self) {
        if self.editor_cursor.0 > 0 {
            self.editor_cursor.0 -= 1;
            let lines: Vec<&str> = self.editor_content.lines().collect();
            if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
                self.editor_cursor.1 = self.editor_cursor.1.min(line.len() as u16);
            }
            self.adjust_scroll_to_cursor();
        }
    }

    /// Move cursor to the start of the current line (0).
    pub fn cursor_to_line_start(&mut self) {
        self.editor_cursor.1 = 0;
    }

    /// Move cursor to the end of the current line ($).
    pub fn cursor_to_line_end(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
            self.editor_cursor.1 = line.len() as u16;
        }
    }

    /// Move cursor forward one word (w).
    pub fn cursor_word_forward(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.editor_cursor.1 as usize;
            // Skip current non-whitespace
            while col < chars.len() && !chars[col].is_whitespace() { col += 1; }
            // Skip whitespace
            while col < chars.len() && chars[col].is_whitespace() { col += 1; }
            self.editor_cursor.1 = col as u16;
        }
    }

    /// Move cursor backward one word (b).
    pub fn cursor_word_backward(&mut self) {
        let lines: Vec<&str> = self.editor_content.lines().collect();
        if let Some(line) = lines.get(self.editor_cursor.0 as usize) {
            let chars: Vec<char> = line.chars().collect();
            let mut col = self.editor_cursor.1 as usize;
            // Skip whitespace backward
            while col > 0 && chars[col - 1].is_whitespace() { col -= 1; }
            // Skip word chars backward
            while col > 0 && !chars[col - 1].is_whitespace() { col -= 1; }
            self.editor_cursor.1 = col as u16;
        }
    }

    // -------------------------------------------------------------------------
    // Vim-style editing operations (Normal mode)
    // -------------------------------------------------------------------------

    /// Delete the character under the cursor (x).
    pub fn delete_char_at_cursor(&mut self) {
        let row = self.editor_cursor.0 as usize;
        let col = self.editor_cursor.1 as usize;
        // Collect what we need and drop the borrow before mutating
        let char_exists = self.editor_content.lines()
            .nth(row)
            .map(|l| col < l.len())
            .unwrap_or(false);
        if char_exists {
            let abs_pos = self.get_line_start_position(row) + col;
            self.editor_content.remove(abs_pos);
            // Clamp cursor to new line length
            let new_len = self.editor_content
                .lines()
                .nth(row)
                .map(|l| l.len())
                .unwrap_or(0);
            if self.editor_cursor.1 as usize > new_len && new_len > 0 {
                self.editor_cursor.1 = new_len as u16;
            }
            self.mark_modified();
            self.update_preview_content();
        }
    }

    /// Delete the current line and store it in the yank buffer (dd).
    pub fn delete_current_line(&mut self) {
        let row = self.editor_cursor.0 as usize;
        // Collect owned data before any mutable operations
        let line_info: Option<(usize, String, usize)> = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            lines.get(row).map(|l| (lines.len(), l.to_string(), l.len()))
        };
        let (line_count, line_content, line_len) = match line_info {
            Some(v) => v,
            None => return,
        };

        self.push_undo_snapshot();
        self.yank_buffer = line_content;

        let start = self.get_line_start_position(row);
        if row + 1 < line_count {
            // Remove line including its trailing newline
            let end = start + line_len + 1;
            self.editor_content.drain(start..end);
        } else if start > 0 {
            // Last line: also remove the preceding newline
            self.editor_content.truncate(start.saturating_sub(1));
            self.editor_cursor.0 = self.editor_cursor.0.saturating_sub(1);
        } else {
            self.editor_content.clear();
            self.editor_cursor = (0, 0);
            self.mark_modified();
            self.update_preview_content();
            return;
        }

        let new_line_count = self.editor_content.lines().count() as u16;
        if self.editor_cursor.0 >= new_line_count && new_line_count > 0 {
            self.editor_cursor.0 = new_line_count - 1;
        }
        self.editor_cursor.1 = 0;
        self.mark_modified();
        self.update_preview_content();
    }

    /// Copy the current line into the yank buffer (yy).
    pub fn yank_current_line(&mut self) {
        let row = self.editor_cursor.0 as usize;
        // Collect to owned String before any mutable operations
        let yanked: Option<String> = self.editor_content.lines().nth(row).map(|l| l.to_string());
        if let Some(line) = yanked {
            self.yank_buffer = line;
            let preview: String = self.yank_buffer.chars().take(40).collect();
            self.set_operation_info(format!("Yanked: \"{}\"", preview), Some("📋".to_string()));
        }
    }

    /// Read text from the system clipboard.
    pub fn read_system_clipboard(&self) -> Option<String> {
        // Try wl-paste (Wayland), then xclip, then xsel (X11)
        let commands: &[(&str, &[&str])] = &[
            ("wl-paste", &["--no-newline"]),
            ("xclip", &["-selection", "clipboard", "-o"]),
            ("xsel", &["--clipboard", "--output"]),
        ];
        for (cmd, args) in commands {
            if let Ok(output) = std::process::Command::new(cmd)
                .args(*args)
                .output()
            {
                if output.status.success() {
                    return String::from_utf8(output.stdout).ok();
                }
            }
        }
        None
    }

    /// Paste system clipboard on a new line below the cursor (P).
    pub fn paste_clipboard_below(&mut self) {
        let text = match self.read_system_clipboard() {
            Some(t) if !t.is_empty() => t,
            _ => {
                self.set_message("System clipboard is empty (requires xclip, xsel, or wl-paste)".to_string());
                return;
            }
        };
        self.push_undo_snapshot();
        let insert_pos = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let row = self.editor_cursor.0 as usize;
            if row < lines.len() {
                self.get_line_start_position(row) + lines[row].len()
            } else {
                self.editor_content.len()
            }
        };
        let to_insert = format!("\n{}", text);
        self.editor_content.insert_str(insert_pos, &to_insert);
        self.editor_cursor.0 += 1;
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
        self.update_preview_content();
        let preview: String = text.chars().take(40).collect();
        self.set_operation_info(format!("Pasted: \"{}\"", preview), Some("📋".to_string()));
    }

    /// Paste system clipboard at the cursor position (for Insert mode).
    pub fn paste_clipboard_at_cursor(&mut self, cursor_byte_index: usize) {
        let text = match self.read_system_clipboard() {
            Some(t) if !t.is_empty() => t,
            _ => {
                self.set_message("System clipboard is empty (requires xclip, xsel, or wl-paste)".to_string());
                return;
            }
        };
        self.push_undo_snapshot();
        self.editor_content.insert_str(cursor_byte_index, &text);
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        if newline_count > 0 {
            self.editor_cursor.0 += newline_count as u16;
            let last_line_len = text.rsplit('\n').next().map(|l| l.len()).unwrap_or(0);
            self.editor_cursor.1 = last_line_len as u16;
        } else {
            self.editor_cursor.1 += text.len() as u16;
        }
        self.adjust_scroll_to_cursor();
        self.mark_modified();
        self.update_preview_content();
    }

    /// Paste yank buffer on a new line below the cursor (p).
    pub fn paste_below(&mut self) {
        if self.yank_buffer.is_empty() {
            self.set_message("Nothing in yank buffer".to_string());
            return;
        }
        self.push_undo_snapshot();
        // Compute insert position and drop borrow before mutating
        let insert_pos = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let row = self.editor_cursor.0 as usize;
            if row < lines.len() {
                self.get_line_start_position(row) + lines[row].len()
            } else {
                self.editor_content.len()
            }
        };
        let to_insert = format!("\n{}", self.yank_buffer);
        self.editor_content.insert_str(insert_pos, &to_insert);
        self.editor_cursor.0 += 1;
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
        self.update_preview_content();
    }

    /// Open a new blank line below the cursor and prepare for insert (o).
    pub fn open_line_below(&mut self) {
        self.push_undo_snapshot();
        // Compute insert position and drop borrow before mutating
        let insert_pos = {
            let lines: Vec<&str> = self.editor_content.lines().collect();
            let row = self.editor_cursor.0 as usize;
            if row < lines.len() {
                self.get_line_start_position(row) + lines[row].len()
            } else {
                self.editor_content.len()
            }
        };
        self.editor_content.insert(insert_pos, '\n');
        self.editor_cursor.0 += 1;
        self.editor_cursor.1 = 0;
        self.adjust_scroll_to_cursor();
        self.mark_modified();
    }

}
