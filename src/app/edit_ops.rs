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
    }

    /// Paste yank buffer on a new line below the cursor (p).
    pub fn paste_below(&mut self) {
        if self.yank_buffer.is_empty() {
            self.set_message("Nothing in yank buffer".to_string());
            return;
        }
        self.push_undo_snapshot();

        // A charwise yank — anything from `dw`, `yiw`, `d$` — goes back in inline,
        // just after the cursor. Pasting it onto a line of its own, the way a
        // linewise yank goes, would mangle the sentence it came out of.
        if !self.yank_linewise {
            let insert_pos = {
                let lines: Vec<&str> = self.editor_content.lines().collect();
                let row = self.editor_cursor.0 as usize;
                let col = self.editor_cursor.1 as usize;
                let line_start = self.get_line_start_position(row);
                let byte_in_line = lines
                    .get(row)
                    .map(|l| {
                        l.char_indices()
                            .nth(col + 1)
                            .map(|(b, _)| b)
                            .unwrap_or(l.len())
                    })
                    .unwrap_or(0);
                (line_start + byte_in_line).min(self.editor_content.len())
            };
            let yanked = self.yank_buffer.clone();
            self.editor_content.insert_str(insert_pos, &yanked);
            self.editor_cursor.1 += yanked.chars().count() as u16;
            self.clamp_cursor_to_content();
            self.mark_modified();
            return;
        }

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
    }

    /// Run an operator over the span a motion or text object selects.
    ///
    /// A motion with nowhere to go resolves to nothing and this does nothing at all
    /// — no undo snapshot, no modified flag. `dw` at the very end of a note should
    /// not cost the user an undo step for a keystroke that did not change anything.
    pub fn apply_operator(
        &mut self,
        operator: crate::vim::Operator,
        target: crate::vim::Target,
        count: usize,
    ) {
        use crate::vim::{cut, resolve, Operator, Span};

        let cursor = (self.editor_cursor.0 as usize, self.editor_cursor.1 as usize);
        let Some(span) = resolve(&self.editor_content, cursor, count, target, operator) else {
            return;
        };
        let (removed, remaining, new_cursor) = cut(&self.editor_content, span);
        if removed.is_empty() {
            return;
        }

        let linewise = matches!(span, Span::Lines { .. });
        // A linewise yank is stored without its trailing newline, because that is
        // the shape `paste_below` has always expected.
        self.yank_buffer = if linewise {
            removed.trim_end_matches('\n').to_string()
        } else {
            removed.clone()
        };
        self.yank_linewise = linewise;

        if operator == Operator::Yank {
            let preview: String = self.yank_buffer.chars().take(40).collect();
            self.set_operation_info(format!("Yanked: \"{}\"", preview), Some("📋".to_string()));
            return;
        }

        self.push_undo_snapshot();
        self.editor_content = remaining;
        self.editor_cursor = (new_cursor.0 as u16, new_cursor.1 as u16);

        if operator == Operator::Change {
            // `cc` empties the line rather than removing it: vim leaves you an empty
            // line to type on, it does not pull the next line up under the cursor.
            if linewise {
                let at = self.get_line_start_position(self.editor_cursor.0 as usize);
                self.editor_content.insert(at, '\n');
                self.editor_cursor.1 = 0;
            }
            self.mode = AppMode::Insert;
        }

        self.clamp_cursor_to_content();
        self.mark_modified();
    }

    /// Forget any half-typed operator sequence.
    pub fn clear_pending_operator(&mut self) {
        self.pending_op = None;
        self.pending_count = None;
        self.pending_op_prefix = None;
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
