use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, HeadingLevel};
use ratatui::{
    text::{Line, Span, Text},
    style::{Style, Modifier},
};
use crate::theme::TokyoNightTheme;

/// Render markdown content to styled ratatui Text
pub fn render_markdown_preview(content: &str) -> Text<'static> {
    let parser = Parser::new(content);
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut in_code_block = false;
    let mut in_heading = false;
    let mut heading_level = 1;
    let mut in_list = false;
    let mut in_emphasis = false;
    let mut in_strong = false;
    let mut in_code = false;
    
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
                        // Add some spacing before headings
                        if !lines.is_empty() {
                            lines.push(Line::from(""));
                        }
                    }
                    Tag::CodeBlock(CodeBlockKind::Fenced(_)) => {
                        in_code_block = true;
                        // Add the current line and start a new one
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                    }
                    // Tag::Code is handled in Event::Code now
                    Tag::List(_) => {
                        in_list = true;
                    }
                    Tag::Emphasis => {
                        in_emphasis = true;
                    }
                    Tag::Strong => {
                        in_strong = true;
                    }
                    Tag::BlockQuote => {
                        current_line.push(Span::styled("▌ ", Style::default().fg(TokyoNightTheme::COMMENT)));
                    }
                    _ => {}
                }
            }
            Event::End(tag_end) => {
                match tag_end {
                    TagEnd::Heading(_) => {
                        in_heading = false;
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                        lines.push(Line::from("")); // Add spacing after headings
                    }
                    TagEnd::CodeBlock => {
                        in_code_block = false;
                    }
                    TagEnd::List(_) => {
                        in_list = false;
                    }
                    TagEnd::Emphasis => {
                        in_emphasis = false;
                    }
                    TagEnd::Strong => {
                        in_strong = false;
                    }
                    TagEnd::Paragraph => {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                        lines.push(Line::from("")); // Add spacing after paragraphs
                    }
                    TagEnd::Item => {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let style = if in_code_block {
                    TokyoNightTheme::markdown_code_block()
                } else if in_code {
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
                    if in_strong {
                        base_style = base_style.add_modifier(Modifier::BOLD);
                    }
                    if in_emphasis {
                        base_style = base_style.add_modifier(Modifier::ITALIC);
                    }
                    base_style
                };
                
                if in_list {
                    if current_line.is_empty() {
                        current_line.push(Span::styled("• ", TokyoNightTheme::markdown_list()));
                    }
                }
                
                if in_heading && current_line.is_empty() {
                    let prefix = match heading_level {
                        1 => "# ",
                        2 => "## ",
                        3 => "### ",
                        _ => "#### ",
                    };
                    current_line.push(Span::styled(prefix, style));
                }
                
                current_line.push(Span::styled(text.to_string(), style));
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
