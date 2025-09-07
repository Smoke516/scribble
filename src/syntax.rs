use crate::theme::TokyoNightTheme;
use ratatui::{
    style::{Color, Style, Modifier},
    text::{Line, Span, Text},
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, FontStyle};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct SyntaxHighlighter {
    #[allow(dead_code)]
    syntax_set: SyntaxSet,
    #[allow(dead_code)]
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        
        Self {
            syntax_set,
            theme_set,
        }
    }

    #[allow(dead_code)]
    pub fn highlight_markdown<'a>(&self, content: &'a str) -> Text<'a> {
        let mut lines = Vec::new();
        
        // Try to find markdown syntax
        let syntax = self.syntax_set
            .find_syntax_by_extension("md")
            .or_else(|| self.syntax_set.find_syntax_by_name("Markdown"))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use a dark theme that works well in terminals
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        for line in LinesWithEndings::from(content) {
            let highlighted = highlighter.highlight_line(line, &self.syntax_set).unwrap_or_default();
            
            let mut spans = Vec::new();
            for (style, text) in highlighted {
                let fg_color = convert_color(style.foreground);
                let mut ratatui_style = Style::default().fg(fg_color);
                
                if style.font_style.contains(FontStyle::BOLD) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                }
                if style.font_style.contains(FontStyle::ITALIC) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                }
                if style.font_style.contains(FontStyle::UNDERLINE) {
                    ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                }
                
                spans.push(Span::styled(text, ratatui_style));
            }
            
            lines.push(Line::from(spans));
        }
        
        Text::from(lines)
    }
}

#[allow(dead_code)]
fn convert_color(syntect_color: syntect::highlighting::Color) -> Color {
    Color::Rgb(syntect_color.r, syntect_color.g, syntect_color.b)
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

// Fallback simple markdown highlighting for cases where syntect fails
pub fn simple_markdown_highlight(content: &str) -> Text<'_> {
    let mut lines = Vec::new();
    
    for line in content.lines() {
        if line.starts_with("# ") {
            // H1 - Tokyo Night cyan
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_h1()
            )));
        } else if line.starts_with("## ") {
            // H2 - Tokyo Night blue
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_h2()
            )));
        } else if line.starts_with("### ") {
            // H3 - Tokyo Night purple
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_h3()
            )));
        } else if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
            // List items - Tokyo Night green
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_list()
            )));
        } else if line.starts_with("> ") {
            // Blockquotes - Tokyo Night comment style
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_quote()
            )));
        } else if line.starts_with("```") {
            // Code blocks - Tokyo Night code block style
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_code_block()
            )));
        } else if line.trim().starts_with("```") && !line.trim_start_matches("```").trim().is_empty() {
            // Code block language specification
            lines.push(Line::from(Span::styled(
                line,
                TokyoNightTheme::markdown_code_block()
            )));
        } else if line.contains("**") && count_occurrences(line, "**") >= 2 {
            // Bold text (simple detection)
            lines.push(parse_bold_text(line));
        } else if line.contains("*") && !line.starts_with("*") && count_occurrences(line, "*") >= 2 {
            // Italic text (simple detection)
            lines.push(parse_italic_text(line));
        } else if line.contains("`") {
            // Inline code
            lines.push(parse_inline_code(line));
        } else {
            // Regular text - use normal theme style
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(TokyoNightTheme::FG)
            )));
        }
    }
    
    Text::from(lines)
}

fn count_occurrences(text: &str, pattern: &str) -> usize {
    text.matches(pattern).count()
}

fn parse_bold_text(line: &str) -> Line<'_> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_bold = false;
    let mut chars = line.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume the second *
            
            if !current.is_empty() {
                let style = if in_bold {
                    TokyoNightTheme::markdown_bold()
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                };
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            in_bold = !in_bold;
        } else {
            current.push(ch);
        }
    }
    
    if !current.is_empty() {
        let style = if in_bold {
            TokyoNightTheme::markdown_bold()
        } else {
            Style::default().fg(TokyoNightTheme::FG)
        };
        spans.push(Span::styled(current, style));
    }
    
    Line::from(spans)
}

fn parse_italic_text(line: &str) -> Line<'_> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_italic = false;
    
    for ch in line.chars() {
        if ch == '*' {
            if !current.is_empty() {
                let style = if in_italic {
                    TokyoNightTheme::markdown_italic()
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                };
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            in_italic = !in_italic;
        } else {
            current.push(ch);
        }
    }
    
    if !current.is_empty() {
        let style = if in_italic {
            TokyoNightTheme::markdown_italic()
        } else {
            Style::default().fg(TokyoNightTheme::FG)
        };
        spans.push(Span::styled(current, style));
    }
    
    Line::from(spans)
}

fn parse_inline_code(line: &str) -> Line<'_> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    
    for ch in line.chars() {
        if ch == '`' {
            if !current.is_empty() {
                let style = if in_code {
                    TokyoNightTheme::markdown_code()
                } else {
                    Style::default().fg(TokyoNightTheme::FG)
                };
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            in_code = !in_code;
        } else {
            current.push(ch);
        }
    }
    
    if !current.is_empty() {
        let style = if in_code {
            TokyoNightTheme::markdown_code()
        } else {
            Style::default().fg(TokyoNightTheme::FG)
        };
        spans.push(Span::styled(current, style));
    }
    
    Line::from(spans)
}
