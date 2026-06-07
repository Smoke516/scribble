use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, HeadingLevel};
use ratatui::{
    text::{Line, Span, Text},
    style::{Style, Modifier, Color},
};
use crate::theme::{ThemeManager, ThemeColors};

/// Callout type with display info
struct CalloutInfo {
    icon: &'static str,
    label: String,
    color: Color,
}

/// Parse an Obsidian-style callout marker from text like "[!note] Optional title"
fn parse_callout(text: &str, c: &ThemeColors) -> Option<CalloutInfo> {
    let trimmed = text.trim();
    if !trimmed.starts_with("[!") {
        return None;
    }
    let end_bracket = trimmed.find(']')?;
    let callout_type = &trimmed[2..end_bracket];
    let title_rest = trimmed[end_bracket + 1..].trim();
    let lower = callout_type.to_lowercase();

    let (icon, color) = match lower.as_str() {
        "note"                       => ("✎", c.blue),
        "tip" | "hint"               => ("✦", c.cyan),
        "info"                       => ("ℹ", c.blue),
        "warning" | "caution" | "attention" => ("⚠", c.yellow),
        "danger" | "error"           => ("✘", c.red),
        "bug"                        => ("⊙", c.red),
        "example"                    => ("▸", c.purple),
        "quote" | "cite"             => ("❝", c.comment),
        "todo"                       => ("☐", c.cyan),
        "success" | "check" | "done" => ("✔", c.green),
        "question" | "help" | "faq"  => ("?", c.yellow),
        "failure" | "fail" | "missing" => ("✘", c.red),
        "abstract" | "summary" | "tldr" => ("≡", c.cyan),
        "important"                  => ("⚡", c.orange),
        _                            => ("▌", c.comment),
    };

    let label = if title_rest.is_empty() {
        // Capitalize the callout type
        let mut chars = lower.chars();
        match chars.next() {
            Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
            None => lower.clone(),
        }
    } else {
        title_rest.to_string()
    };

    Some(CalloutInfo { icon, label, color })
}

/// Render markdown content to styled ratatui Text (Obsidian-like).
///
/// `width` is the inner width of the preview pane in columns; full-width
/// decorations (heading underlines, code-block borders, horizontal rules) are
/// sized to it so they fill the pane exactly instead of wrapping into stub lines.
pub fn render_markdown_preview(content: &str, theme: &ThemeManager, width: usize) -> Text<'static> {
    let c = theme.colors();
    // Clamp to a sane floor so very narrow panes still render a short rule.
    let rule_width = width.clamp(4, 400);
    let parser = Parser::new_ext(content, pulldown_cmark::Options::all());
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut in_heading = false;
    let mut heading_level: u8 = 1;
    let mut in_emphasis = false;
    let mut in_strong = false;
    let mut in_strikethrough = false;
    let mut in_link = false;
    let mut link_url = String::new();

    // Blockquote state (depth for nesting)
    let mut blockquote_depth: usize = 0;
    let mut callout_color: Option<Color> = None;
    let mut callout_first_text = false;

    // List state — stack tracks ordered (Some(start)) vs unordered (None)
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut list_counters: Vec<u64> = Vec::new();
    let mut is_task_item = false;
    let mut is_checked_task = false;
    let mut in_list_item = false;

    // Table state
    let mut in_table = false;
    let mut in_table_head = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    // Helper: build blockquote prefix spans for current depth
    let default_bq_color = c.comment;
    let bq_prefix = |depth: usize, color: Option<Color>| -> Vec<Span<'static>> {
        if depth == 0 {
            return vec![];
        }
        let clr = color.unwrap_or(default_bq_color);
        let bar = "▌ ".repeat(depth);
        vec![Span::styled(bar, Style::default().fg(clr))]
    };

    for event in parser {
        match event {
            // ── Tag starts ──────────────────────────────────────────
            Event::Start(tag) => {
                match tag {
                    Tag::Heading { level, .. } => {
                        in_heading = true;
                        heading_level = match level {
                            HeadingLevel::H1 => 1,
                            HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3,
                            HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5,
                            HeadingLevel::H6 => 6,
                        };
                        if !lines.is_empty() { lines.push(Line::from("")); }
                    }
                    Tag::CodeBlock(kind) => {
                        in_code_block = true;
                        code_block_lang = match kind {
                            CodeBlockKind::Fenced(lang) => lang.to_string(),
                            CodeBlockKind::Indented => String::new(),
                        };
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        let lang_label = if code_block_lang.is_empty() {
                            " code ".to_string()
                        } else {
                            format!(" {} ", code_block_lang)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("╭─{}─", "─".repeat(lang_label.len())),
                                Style::default().fg(c.comment),
                            ),
                            Span::styled(
                                lang_label,
                                Style::default().fg(c.cyan).add_modifier(Modifier::BOLD),
                            ),
                        ]));
                    }
                    Tag::List(ordered) => {
                        list_stack.push(ordered);
                        list_counters.push(ordered.unwrap_or(1));
                    }
                    Tag::Item => {
                        is_task_item = false; // will be set by TaskListMarker if present
                        is_checked_task = false;
                        in_list_item = true;
                    }
                    Tag::Emphasis      => { in_emphasis = true; }
                    Tag::Strong        => { in_strong = true; }
                    Tag::Strikethrough => { in_strikethrough = true; }
                    Tag::Link { dest_url, .. } => {
                        in_link = true;
                        link_url = dest_url.to_string();
                    }
                    Tag::Image { dest_url, .. } => {
                        // Show image placeholder with alt text coming via Text events
                        current_line.push(Span::styled(
                            "🖼 ",
                            Style::default().fg(c.purple),
                        ));
                        // Store URL to show after alt text
                        in_link = true;
                        link_url = dest_url.to_string();
                    }
                    Tag::BlockQuote => {
                        blockquote_depth += 1;
                        callout_first_text = true;
                        callout_color = None;
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                    }
                    Tag::Table(_) => {
                        in_table = true;
                        table_rows.clear();
                    }
                    Tag::TableHead => { in_table_head = true; }
                    Tag::TableRow  => { current_row.clear(); }
                    Tag::TableCell => { current_cell.clear(); }
                    _ => {}
                }
            }

            // ── Tag ends ────────────────────────────────────────────
            Event::End(tag_end) => {
                match tag_end {
                    TagEnd::Heading(_) => {
                        in_heading = false;
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                        // Obsidian-style: subtle separator under H1 and H2
                        if heading_level <= 2 {
                            lines.push(Line::from(Span::styled(
                                "─".repeat(rule_width),
                                Style::default().fg(c.bg_highlight),
                            )));
                        }
                        lines.push(Line::from(""));
                    }
                    TagEnd::CodeBlock => {
                        lines.push(Line::from(Span::styled(
                            format!("╰{}", "─".repeat(rule_width.saturating_sub(1).max(3))),
                            Style::default().fg(c.comment),
                        )));
                        in_code_block = false;
                        code_block_lang.clear();
                    }
                    TagEnd::List(_) => {
                        list_stack.pop();
                        list_counters.pop();
                    }
                    TagEnd::Item => {
                        // Only push if End(Paragraph) hasn't already flushed
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        // Increment counter for ordered lists
                        if let Some(n) = list_counters.last_mut() {
                            *n += 1;
                        }
                        is_task_item = false;
                        is_checked_task = false;
                        in_list_item = false;
                    }
                    TagEnd::Emphasis      => { in_emphasis = false; }
                    TagEnd::Strong        => { in_strong = false; }
                    TagEnd::Strikethrough => { in_strikethrough = false; }
                    TagEnd::Link => {
                        // Append URL after link text (dimmed)
                        if !link_url.is_empty() {
                            current_line.push(Span::styled(
                                format!(" ↗ {}", link_url),
                                Style::default().fg(c.comment),
                            ));
                        }
                        in_link = false;
                        link_url.clear();
                    }
                    TagEnd::Image => {
                        // Show image URL
                        if !link_url.is_empty() {
                            current_line.push(Span::styled(
                                format!(" ({})", link_url),
                                Style::default().fg(c.comment),
                            ));
                        }
                        in_link = false;
                        link_url.clear();
                    }
                    TagEnd::Paragraph => {
                        if !in_table {
                            if blockquote_depth > 0 {
                                // Prepend blockquote bar if not already present
                                let mut bq_line = bq_prefix(blockquote_depth, callout_color);
                                bq_line.append(&mut current_line);
                                lines.push(Line::from(bq_line));
                            } else {
                                lines.push(Line::from(current_line.clone()));
                            }
                            current_line.clear();
                            // Don't add blank line inside list items (avoids double spacing)
                            if !in_list_item {
                                lines.push(Line::from(""));
                            }
                        }
                    }
                    TagEnd::BlockQuote => {
                        blockquote_depth = blockquote_depth.saturating_sub(1);
                        if blockquote_depth == 0 {
                            callout_color = None;
                        }
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        lines.push(Line::from(""));
                    }
                    TagEnd::TableCell => {
                        current_row.push(current_cell.clone());
                        current_cell.clear();
                    }
                    TagEnd::TableHead => {
                        let hdr_color = c.cyan;
                        let header_spans: Vec<Span> = current_row.iter().enumerate().map(move |(i, cell)| {
                            let sep = if i > 0 { " │ " } else { "" };
                            let mut s = sep.to_string();
                            s.push_str(cell);
                            Span::styled(s, Style::default().fg(hdr_color).add_modifier(Modifier::BOLD))
                        }).collect();
                        lines.push(Line::from(header_spans));
                        let sep_width: usize = current_row.iter().map(|cl| cl.len() + 3).sum::<usize>().saturating_sub(1);
                        lines.push(Line::from(Span::styled(
                            "─".repeat(sep_width.max(4)),
                            Style::default().fg(c.fg_dark),
                        )));
                        table_rows.push(current_row.clone());
                        current_row.clear();
                        in_table_head = false;
                    }
                    TagEnd::TableRow => {
                        if !in_table_head {
                            let row_fg = c.fg;
                            let row_spans: Vec<Span> = current_row.iter().enumerate().map(move |(i, cell)| {
                                let sep = if i > 0 { " │ " } else { "" };
                                Span::styled(format!("{}{}", sep, cell), Style::default().fg(row_fg))
                            }).collect();
                            lines.push(Line::from(row_spans));
                        }
                        current_row.clear();
                    }
                    TagEnd::Table => {
                        in_table = false;
                        lines.push(Line::from(""));
                    }
                    _ => {}
                }
            }

            // ── Text content ────────────────────────────────────────
            Event::Text(text) => {
                if in_table {
                    current_cell.push_str(&text);
                } else if in_code_block {
                    for (i, code_line) in text.lines().enumerate() {
                        if i > 0 { lines.push(Line::from(current_line.clone())); current_line.clear(); }
                        current_line.push(Span::styled("│ ", Style::default().fg(c.comment)));
                        current_line.push(Span::styled(code_line.to_string(), theme.markdown_code_block()));
                    }
                    if !text.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                } else if blockquote_depth > 0 && callout_first_text {
                    // Check for Obsidian-style callout marker: [!type]
                    callout_first_text = false;
                    if let Some(info) = parse_callout(&text, &c) {
                        callout_color = Some(info.color);
                        // Render callout header line
                        let mut header = bq_prefix(blockquote_depth, Some(info.color));
                        header.push(Span::styled(
                            format!("{} ", info.icon),
                            Style::default().fg(info.color),
                        ));
                        header.push(Span::styled(
                            info.label,
                            Style::default().fg(info.color).add_modifier(Modifier::BOLD),
                        ));
                        lines.push(Line::from(header));
                    } else {
                        // Normal blockquote text — render with bar prefix + italic
                        let style = {
                            let mut s = Style::default().fg(c.fg_dark).add_modifier(Modifier::ITALIC);
                            if in_strong   { s = s.add_modifier(Modifier::BOLD); }
                            s
                        };
                        let mut bq_line = bq_prefix(blockquote_depth, callout_color);
                        bq_line.push(Span::styled(text.to_string(), style));
                        current_line = bq_line;
                    }
                } else if blockquote_depth > 0 {
                    // Subsequent blockquote text
                    if current_line.is_empty() {
                        current_line = bq_prefix(blockquote_depth, callout_color);
                    }
                    let style = {
                        let mut s = Style::default().fg(c.fg_dark).add_modifier(Modifier::ITALIC);
                        if in_strong   { s = s.add_modifier(Modifier::BOLD); }
                        if in_emphasis { s = s.add_modifier(Modifier::ITALIC); }
                        s
                    };
                    current_line.push(Span::styled(text.to_string(), style));
                } else {
                    // Determine text style
                    let style = if in_heading {
                        match heading_level {
                            1 => theme.markdown_h1(),
                            2 => theme.markdown_h2(),
                            3 => theme.markdown_h3(),
                            _ => theme.markdown_h3(),
                        }
                    } else if in_link {
                        theme.markdown_link()
                    } else if is_checked_task {
                        // Obsidian-style: completed tasks get dimmed + strikethrough
                        Style::default().fg(c.comment).add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        let mut base_style = Style::default().fg(c.fg);
                        if in_strong        { base_style = base_style.add_modifier(Modifier::BOLD); }
                        if in_emphasis      { base_style = base_style.add_modifier(Modifier::ITALIC); }
                        if in_strikethrough { base_style = base_style.add_modifier(Modifier::CROSSED_OUT); }
                        base_style
                    };

                    // List bullet / number prefix
                    if !list_stack.is_empty() && current_line.is_empty() && !is_task_item {
                        let depth = list_stack.len();
                        let indent = "  ".repeat(depth.saturating_sub(1));
                        let marker = match list_stack.last() {
                            Some(Some(_)) => {
                                // Ordered list — show current counter
                                let n = list_counters.last().copied().unwrap_or(1);
                                format!("{}{}. ", indent, n)
                            }
                            _ => format!("{}• ", indent),
                        };
                        current_line.push(Span::styled(marker, theme.markdown_list()));
                    }

                    // Heading: Obsidian hides the `#` prefix
                    if in_heading && current_line.is_empty() {
                        // No prefix — just render the heading text directly
                    }

                    current_line.push(Span::styled(text.to_string(), style));
                }
            }

            // ── Inline code ─────────────────────────────────────────
            Event::Code(text) => {
                current_line.push(Span::styled(
                    format!(" {} ", text),
                    theme.markdown_code(),
                ));
            }

            // ── Horizontal rule ─────────────────────────────────────
            Event::Rule => {
                if !current_line.is_empty() {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "━".repeat(rule_width),
                    Style::default().fg(c.comment),
                )));
                lines.push(Line::from(""));
            }

            // ── Task list checkboxes ────────────────────────────────
            Event::TaskListMarker(checked) => {
                is_task_item = true;
                is_checked_task = checked;
                let depth = list_stack.len();
                let indent = "  ".repeat(depth.saturating_sub(1));
                let (icon, color) = if checked {
                    ("☑ ", c.green)
                } else {
                    ("☐ ", c.fg_dark)
                };
                current_line.push(Span::styled(
                    format!("{}{}", indent, icon),
                    Style::default().fg(color),
                ));
            }

            // ── HTML pass-through ───────────────────────────────────
            Event::Html(html) => {
                current_line.push(Span::styled(
                    html.to_string(),
                    Style::default().fg(c.orange),
                ));
            }

            // ── Line breaks ─────────────────────────────────────────
            Event::SoftBreak | Event::HardBreak => {
                if blockquote_depth > 0 {
                    // Push the current line with blockquote prefix
                    if current_line.is_empty() {
                        current_line = bq_prefix(blockquote_depth, callout_color);
                    }
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                } else {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
            }

            // ── Footnote reference ──────────────────────────────────
            Event::FootnoteReference(name) => {
                current_line.push(Span::styled(
                    format!("[{}]", name),
                    Style::default().fg(c.cyan).add_modifier(Modifier::BOLD),
                ));
            }

            _ => {}
        }
    }

    // Add any remaining content
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    // Placeholder when empty
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("📝 ", Style::default().fg(c.cyan)),
            Span::styled("Live Preview", theme.markdown_h2()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Start typing in the editor to see the preview here...",
            theme.help_text(),
        )));
    }

    Text::from(lines)
}

/// Generate a sample preview showcasing all supported features
pub fn generate_preview_sample(theme: &ThemeManager, width: usize) -> Text<'static> {
    let sample_markdown = r#"# Welcome to Live Preview!

This is a **live markdown preview** that updates as you type.

## Features Supported

- **Bold text** and *italic text*
- ~~Strikethrough~~ text
- `inline code` with highlighting
- [Links](https://example.com) with URLs

### Task Lists

- [x] Completed task
- [ ] Pending task
- [x] Another done item

### Ordered Lists

1. First item
2. Second item
3. Third item

---

### Code Blocks

```rust
fn hello_world() {
    println!("Hello from Scribble!");
}
```

> This is a blockquote with beautiful styling

> [!note] Callout Support
> Obsidian-style callouts are now rendered!

> [!tip] Pro Tip
> Use callouts to highlight important information.

Start editing your note to see the magic happen!"#;

    render_markdown_preview(sample_markdown, theme, width)
}
