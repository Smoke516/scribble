use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, HeadingLevel};
use ratatui::{
    text::{Line, Span, Text},
    style::{Style, Modifier},
};
use crate::theme::TokyoNightTheme;

/// Render markdown content to styled ratatui Text
pub fn render_markdown_preview(content: &str) -> Text<'static> {
    let parser = Parser::new_ext(content, pulldown_cmark::Options::all());
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut in_heading = false;
    let mut heading_level = 1;
    let mut in_list = false;
    let mut list_item_depth = 0usize;
    let mut in_emphasis = false;
    let mut in_strong = false;
    let mut in_code = false;
    // Table state
    let mut in_table = false;
    let mut in_table_head = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    
    for event in parser {
        match event {
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
                        // Draw language label + top border
                        let lang_label = if code_block_lang.is_empty() {
                            " code ".to_string()
                        } else {
                            format!(" {} ", code_block_lang)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("╭─{}─", "─".repeat(lang_label.len())),
                                Style::default().fg(TokyoNightTheme::COMMENT),
                            ),
                            Span::styled(lang_label, Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD)),
                        ]));
                    }
                    Tag::List(_) => {
                        list_item_depth += 1;
                        in_list = list_item_depth > 0;
                    }
                    Tag::Emphasis => { in_emphasis = true; }
                    Tag::Strong   => { in_strong = true; }
                    Tag::BlockQuote => {
                        current_line.push(Span::styled("▌ ", Style::default().fg(TokyoNightTheme::COMMENT)));
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
            Event::End(tag_end) => {
                match tag_end {
                    TagEnd::Heading(_) => {
                        in_heading = false;
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                        lines.push(Line::from(""));
                    }
                    TagEnd::CodeBlock => {
                        // Bottom border
                        lines.push(Line::from(Span::styled(
                            "╰─────────────────────────────────────────".to_string(),
                            Style::default().fg(TokyoNightTheme::COMMENT),
                        )));
                        in_code_block = false;
                        code_block_lang.clear();
                    }
                    TagEnd::List(_) => {
                        if list_item_depth > 0 { list_item_depth -= 1; }
                        in_list = list_item_depth > 0;
                    }
                    TagEnd::Emphasis => { in_emphasis = false; }
                    TagEnd::Strong   => { in_strong = false; }
                    TagEnd::Paragraph => {
                        if !in_table {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            lines.push(Line::from(""));
                        }
                    }
                    TagEnd::Item => {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    TagEnd::TableCell => {
                        current_row.push(current_cell.clone());
                        current_cell.clear();
                    }
                    TagEnd::TableHead => {
                        // Render header row with separator
                        let header_spans: Vec<Span> = current_row.iter().enumerate().map(|(i, cell)| {
                            let sep = if i > 0 { " │ " } else { "" };
                            let mut s = sep.to_string();
                            s.push_str(cell);
                            Span::styled(s, Style::default().fg(TokyoNightTheme::CYAN).add_modifier(Modifier::BOLD))
                        }).collect();
                        lines.push(Line::from(header_spans));
                        // Separator line
                        let sep_width: usize = current_row.iter().map(|c| c.len() + 3).sum::<usize>().saturating_sub(1);
                        lines.push(Line::from(Span::styled(
                            "─".repeat(sep_width.max(4)),
                            Style::default().fg(TokyoNightTheme::FG_DARK),
                        )));
                        table_rows.push(current_row.clone());
                        current_row.clear();
                        in_table_head = false;
                    }
                    TagEnd::TableRow => {
                        if !in_table_head {
                            let row_spans: Vec<Span> = current_row.iter().enumerate().map(|(i, cell)| {
                                let sep = if i > 0 { " │ " } else { "" };
                                Span::styled(format!("{}{}", sep, cell), Style::default().fg(TokyoNightTheme::FG))
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
            Event::Text(text) => {
                if in_table {
                    current_cell.push_str(&text);
                } else if in_code_block {
                    // Render each line of code block with left border
                    for (i, code_line) in text.lines().enumerate() {
                        if i > 0 { lines.push(Line::from(current_line.clone())); current_line.clear(); }
                        current_line.push(Span::styled("│ ", Style::default().fg(TokyoNightTheme::COMMENT)));
                        current_line.push(Span::styled(code_line.to_string(), TokyoNightTheme::markdown_code_block()));
                    }
                    if !text.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                } else {
                    let style = if in_code {
                        TokyoNightTheme::markdown_code()
                    } else if in_heading {
                        match heading_level {
                            1 => TokyoNightTheme::markdown_h1(),
                            2 => TokyoNightTheme::markdown_h2(),
                            3 => TokyoNightTheme::markdown_h3(),
                            _ => TokyoNightTheme::markdown_h3(),
                        }
                    } else {
                        let mut base_style = Style::default().fg(TokyoNightTheme::FG);
                        if in_strong   { base_style = base_style.add_modifier(Modifier::BOLD); }
                        if in_emphasis { base_style = base_style.add_modifier(Modifier::ITALIC); }
                        base_style
                    };

                    if in_list && current_line.is_empty() {
                        let indent = "  ".repeat(list_item_depth.saturating_sub(1));
                        current_line.push(Span::styled(
                            format!("{}• ", indent),
                            TokyoNightTheme::markdown_list(),
                        ));
                    }

                    if in_heading && current_line.is_empty() {
                        let prefix = match heading_level { 1=>"# ", 2=>"## ", 3=>"### ", _=>"#### " };
                        current_line.push(Span::styled(prefix, style));
                    }

                    current_line.push(Span::styled(text.to_string(), style));
                }
            }
            Event::Code(text) => {
                current_line.push(Span::styled(
                    format!(" {} ", text),
                    TokyoNightTheme::markdown_code()
                ));
            }
            Event::Html(html) => {
                // Basic HTML support - just render as text with different styling
                current_line.push(Span::styled(
                    html.to_string(),
                    Style::default().fg(TokyoNightTheme::ORANGE)
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(current_line.clone()));
                current_line.clear();
            }
            _ => {}
        }
    }
    
    // Add any remaining content
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }
    
    // If we have no content, show a placeholder
    if lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("📝 ", Style::default().fg(TokyoNightTheme::CYAN)),
            Span::styled("Live Preview", TokyoNightTheme::markdown_h2()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Start typing in the editor to see the preview here...",
            TokyoNightTheme::help_text()
        )));
    }
    
    Text::from(lines)
}

/// Generate a simple preview for display purposes
pub fn generate_preview_sample() -> Text<'static> {
    let sample_markdown = r#"# Welcome to Live Preview! 

This is a **live markdown preview** that updates as you type.

## Features Supported:

- **Bold text** and *italic text*
- `inline code` with highlighting
- Lists with bullet points
- Headers of different sizes

### Code Blocks:
```rust
fn hello_world() {
    println!("Hello from Scribble!");
}
```

> This is a blockquote with beautiful styling

Start editing your note to see the magic happen! ✨"#;

    render_markdown_preview(sample_markdown)
}
