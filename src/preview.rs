use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

use crate::theme::{ThemeColors, ThemeManager};

/// Callout type with display info
struct CalloutInfo {
    icon: &'static str,
    label: String,
    color: Color,
}

/// Parse an Obsidian-style callout marker from text like "[!note] Optional title".
///
/// The text handed here is a whole rendered line rather than a single parser
/// event: `[!note]` is a link reference as far as CommonMark is concerned, so the
/// parser hands it back in pieces (`[`, `!note`, `]`) and matching on the first
/// piece alone never fires.
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

/// Content with a leading YAML frontmatter block removed.
///
/// Done here rather than by the parser: `ENABLE_YAML_STYLE_METADATA_BLOCKS` treats
/// *any* pair of `---` lines as a metadata block, so a horizontal rule halfway
/// down a note opened one and swallowed everything up to the next rule.
/// Frontmatter is only frontmatter at the very top, and only when it closes.
fn strip_frontmatter(content: &str) -> &str {
    let Some(rest) = content.strip_prefix("---\n") else {
        return content;
    };
    // A note may legitimately open with a horizontal rule. What tells the two
    // apart is what comes next: frontmatter starts with a key, immediately.
    if !rest.lines().next().is_some_and(is_yaml_key) {
        return content;
    }
    let mut offset = "---\n".len();
    for line in rest.lines() {
        let line_len = line.len() + 1; // the line and its newline
        if line.trim_end() == "---" {
            return content.get(offset + line_len..).unwrap_or("");
        }
        offset += line_len;
    }
    // Never closed, so it was not frontmatter. Leave the note alone.
    content
}

/// A `key:` line, the way a frontmatter block always opens.
fn is_yaml_key(line: &str) -> bool {
    let Some((key, _)) = line.split_once(':') else {
        return false;
    };
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Display width of a string in terminal columns.
fn dwidth(s: &str) -> usize {
    Span::raw(s).width()
}

/// Pad to `width` columns; longer strings are returned untouched.
fn pad_to(s: &str, width: usize) -> String {
    let w = dwidth(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

/// Truncate to `width` columns, marking the cut with an ellipsis.
fn truncate_to(s: &str, width: usize) -> String {
    if dwidth(s) <= width {
        return s.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = dwidth(ch.encode_utf8(&mut [0u8; 4]));
        if w + cw > width - 1 {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

/// One level of list nesting.
struct ListLevel {
    ordered: bool,
    counter: u64,
}

/// Builds the preview one line at a time.
///
/// Every line is `prefix + content`: the prefix carries blockquote bars, the list
/// bullet or checkbox, and the indent that keeps a wrapped or continued line under
/// the text it belongs to. It is laid down once, when the line opens, so nothing
/// can prepend it a second time on the way out.
struct Renderer<'a> {
    theme: &'a ThemeManager,
    c: ThemeColors,
    width: usize,

    lines: Vec<Line<'static>>,
    prefix: Vec<Span<'static>>,
    pending: Vec<Span<'static>>,
    line_open: bool,

    // Inline styling — counted, so nesting closes in the right order.
    emphasis: usize,
    strong: usize,
    strike: usize,
    in_link: bool,
    link_url: String,
    in_heading: bool,
    heading_level: u8,

    // Blockquotes and callouts
    bq_depth: usize,
    bq_first_line: bool,
    callout_color: Option<Color>,

    // Lists
    lists: Vec<ListLevel>,
    /// Marker waiting for the item's first line to open.
    item_marker: Option<Vec<Span<'static>>>,
    /// Column the item's text starts at, so continuation lines line up under it.
    item_indent: usize,
    in_item: bool,
    checked_item: bool,

    // Code blocks
    in_code: bool,
    code_buf: String,

    // Tables
    in_table: bool,
    in_head: bool,
    cell: String,
    row: Vec<String>,
    head: Vec<String>,
    rows: Vec<Vec<String>>,

}

impl<'a> Renderer<'a> {
    fn new(theme: &'a ThemeManager, width: usize) -> Self {
        Self {
            theme,
            c: theme.colors(),
            // Clamp to a sane floor so very narrow panes still render a short rule.
            width: width.clamp(4, 400),
            lines: Vec::new(),
            prefix: Vec::new(),
            pending: Vec::new(),
            line_open: false,
            emphasis: 0,
            strong: 0,
            strike: 0,
            in_link: false,
            link_url: String::new(),
            in_heading: false,
            heading_level: 1,
            bq_depth: 0,
            bq_first_line: false,
            callout_color: None,
            lists: Vec::new(),
            item_marker: None,
            item_indent: 0,
            in_item: false,
            checked_item: false,
            in_code: false,
            code_buf: String::new(),
            in_table: false,
            in_head: false,
            cell: String::new(),
            row: Vec::new(),
            head: Vec::new(),
            rows: Vec::new(),
        }
    }

    // ── Line building ───────────────────────────────────────────

    /// Lay down the prefix for a line that is about to receive content.
    fn open_line(&mut self) {
        if self.line_open {
            return;
        }
        self.line_open = true;
        let mut prefix: Vec<Span<'static>> = Vec::new();
        if self.bq_depth > 0 {
            let clr = self.callout_color.unwrap_or(self.c.comment);
            prefix.push(Span::styled(
                "▌ ".repeat(self.bq_depth),
                Style::default().fg(clr),
            ));
        }
        if let Some(marker) = self.item_marker.take() {
            prefix.extend(marker);
        } else if self.in_item || !self.lists.is_empty() {
            prefix.push(Span::raw(" ".repeat(self.item_indent)));
        }
        self.prefix = prefix;
    }

    /// Drop trailing lines that carry no content — an empty line, or one holding
    /// nothing but blockquote bars.
    fn trim_trailing_blanks(&mut self) {
        while let Some(last) = self.lines.last() {
            let text: String = last.spans.iter().map(|s| s.content.as_ref()).collect();
            if text.chars().all(|ch| ch == '▌' || ch.is_whitespace()) {
                self.lines.pop();
            } else {
                return;
            }
        }
    }

    fn push_span(&mut self, span: Span<'static>) {
        self.open_line();
        self.pending.push(span);
    }

    /// Emit the line under construction, if there is one.
    fn flush(&mut self) {
        if !self.line_open && self.pending.is_empty() {
            return;
        }
        if self.bq_depth > 0 && self.bq_first_line && !self.pending.is_empty() {
            self.bq_first_line = false;
            let text: String = self.pending.iter().map(|s| s.content.as_ref()).collect();
            if let Some(info) = parse_callout(&text, &self.c) {
                // A callout replaces its own first line with a titled header, and
                // recolors the bar for the rest of the quote.
                self.callout_color = Some(info.color);
                self.lines.push(Line::from(vec![
                    Span::styled("▌ ".repeat(self.bq_depth), Style::default().fg(info.color)),
                    Span::styled(format!("{} ", info.icon), Style::default().fg(info.color)),
                    Span::styled(
                        info.label,
                        Style::default().fg(info.color).add_modifier(Modifier::BOLD),
                    ),
                ]));
                self.pending.clear();
                self.prefix.clear();
                self.line_open = false;
                return;
            }
        }
        let mut spans = std::mem::take(&mut self.prefix);
        spans.append(&mut self.pending);
        self.lines.push(Line::from(spans));
        self.line_open = false;
    }

    /// A single blank line between blocks — never two, never one at the top.
    ///
    /// Inside a blockquote the separator keeps the bar, so a quote with two
    /// paragraphs reads as one quote rather than two.
    fn blank(&mut self) {
        self.flush();
        let content = if self.bq_depth > 0 {
            "▌ ".repeat(self.bq_depth).trim_end().to_string()
        } else {
            String::new()
        };
        match self.lines.last() {
            None => return,
            Some(last) => {
                let text: String = last.spans.iter().map(|s| s.content.as_ref()).collect();
                if text.trim_end() == content {
                    return;
                }
            }
        }
        let line = if content.is_empty() {
            Line::from("")
        } else {
            let color = self.callout_color.unwrap_or(self.c.comment);
            Line::from(Span::styled(content, Style::default().fg(color)))
        };
        self.lines.push(line);
    }

    fn rule(&self, ch: char, style: Style) -> Line<'static> {
        Line::from(Span::styled(
            ch.to_string().repeat(self.width),
            style,
        ))
    }

    // ── Styling ─────────────────────────────────────────────────

    fn text_style(&self) -> Style {
        if self.in_heading {
            return match self.heading_level {
                1 => self.theme.markdown_h1(),
                2 => self.theme.markdown_h2(),
                _ => self.theme.markdown_h3(),
            };
        }
        if self.checked_item {
            // Obsidian-style: completed tasks are dimmed and struck through.
            return Style::default()
                .fg(self.c.comment)
                .add_modifier(Modifier::CROSSED_OUT);
        }
        let mut style = if self.in_link {
            self.theme.markdown_link()
        } else if self.bq_depth > 0 {
            Style::default()
                .fg(self.c.fg_dark)
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default().fg(self.c.fg)
        };
        if self.strong > 0 {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.emphasis > 0 {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.strike > 0 {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        style
    }

    // ── Blocks ──────────────────────────────────────────────────

    fn start_item(&mut self) {
        self.flush();
        self.in_item = true;
        self.checked_item = false;
        let indent = "  ".repeat(self.lists.len().saturating_sub(1));
        let marker = match self.lists.last() {
            Some(level) if level.ordered => format!("{}. ", level.counter),
            _ => "• ".to_string(),
        };
        self.item_indent = dwidth(&indent) + dwidth(&marker);
        self.item_marker = Some(vec![Span::styled(
            format!("{}{}", indent, marker),
            self.theme.markdown_list(),
        )]);
    }

    fn task_marker(&mut self, checked: bool) {
        self.checked_item = checked;
        let indent = "  ".repeat(self.lists.len().saturating_sub(1));
        let (icon, color) = if checked {
            ("☑ ", self.c.green)
        } else {
            ("☐ ", self.c.fg_dark)
        };
        self.item_indent = dwidth(&indent) + dwidth(icon);
        self.item_marker = Some(vec![Span::styled(
            format!("{}{}", indent, icon),
            Style::default().fg(color),
        )]);
    }

    fn open_code_block(&mut self, kind: CodeBlockKind) {
        self.flush();
        self.in_code = true;
        self.code_buf.clear();
        let lang = match kind {
            CodeBlockKind::Fenced(lang) => lang.to_string(),
            CodeBlockKind::Indented => String::new(),
        };
        let label = if lang.is_empty() {
            " code ".to_string()
        } else {
            format!(" {} ", lang)
        };
        // ╭─ rust ────────────…  sized to the pane, so it lines up with the foot.
        let used = 2 + dwidth(&label);
        let fill = self.width.saturating_sub(used).max(1);
        self.lines.push(Line::from(vec![
            Span::styled("╭─", Style::default().fg(self.c.comment)),
            Span::styled(
                label,
                Style::default().fg(self.c.cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("─".repeat(fill), Style::default().fg(self.c.comment)),
        ]));
    }

    fn close_code_block(&mut self) {
        let body = std::mem::take(&mut self.code_buf);
        let body = body.strip_suffix('\n').unwrap_or(&body);
        if !body.is_empty() {
            for code_line in body.split('\n') {
                self.lines.push(Line::from(vec![
                    Span::styled("│ ", Style::default().fg(self.c.comment)),
                    Span::styled(code_line.to_string(), self.theme.markdown_code_block()),
                ]));
            }
        }
        let foot = format!("╰{}", "─".repeat(self.width.saturating_sub(1)));
        self.lines.push(Line::from(Span::styled(
            foot,
            Style::default().fg(self.c.comment),
        )));
        self.in_code = false;
        self.blank();
    }

    /// Emit the buffered table with its columns aligned.
    ///
    /// Rows are held until the table ends because a column's width is not known
    /// until every cell in it has been seen.
    fn emit_table(&mut self) {
        let cols = self
            .rows
            .iter()
            .map(|r| r.len())
            .chain(std::iter::once(self.head.len()))
            .max()
            .unwrap_or(0);
        if cols == 0 {
            return;
        }

        let cell = |row: &Vec<String>, i: usize| row.get(i).cloned().unwrap_or_default();
        let mut widths: Vec<usize> = (0..cols)
            .map(|i| {
                let head_w = dwidth(&cell(&self.head, i));
                self.rows
                    .iter()
                    .map(|r| dwidth(&cell(r, i)))
                    .chain(std::iter::once(head_w))
                    .max()
                    .unwrap_or(0)
                    .max(1)
            })
            .collect();

        // Shrink the widest column until the table fits the pane.
        let gaps = 3 * cols.saturating_sub(1);
        while widths.iter().sum::<usize>() + gaps > self.width {
            let (idx, &max) = widths.iter().enumerate().max_by_key(|(_, w)| **w).unwrap();
            if max <= 3 {
                break;
            }
            widths[idx] = max - 1;
        }

        let render_row = |row: &Vec<String>, style: Style| -> Line<'static> {
            let mut spans: Vec<Span<'static>> = Vec::new();
            for (i, w) in widths.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw(" │ "));
                }
                spans.push(Span::styled(pad_to(&truncate_to(&cell(row, i), *w), *w), style));
            }
            Line::from(spans)
        };

        if !self.head.is_empty() {
            let head = self.head.clone();
            self.lines.push(render_row(
                &head,
                Style::default()
                    .fg(self.c.cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            let sep: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
            self.lines.push(Line::from(Span::styled(
                sep.join("─┼─"),
                Style::default().fg(self.c.fg_dark),
            )));
        }
        let body = std::mem::take(&mut self.rows);
        for row in &body {
            self.lines
                .push(render_row(row, Style::default().fg(self.c.fg)));
        }
        self.head.clear();
    }

    // ── Event loop ──────────────────────────────────────────────

    fn run(&mut self, content: &str) {
        // Smart punctuation is deliberately off: the preview should show the
        // quotes and dashes you typed, not prettier ones you did not.
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_HEADING_ATTRIBUTES;

        let content = strip_frontmatter(content);
        for event in Parser::new_ext(content, options) {
            match event {
                Event::Start(tag) => self.start(tag),
                Event::End(tag) => self.end(tag),
                Event::Text(text) => {
                    if self.in_table {
                        self.cell.push_str(&text);
                    } else if self.in_code {
                        self.code_buf.push_str(&text);
                    } else {
                        let style = self.text_style();
                        self.push_span(Span::styled(text.to_string(), style));
                    }
                }
                Event::Code(code) => {
                    if self.in_table {
                        self.cell.push_str(&code);
                    } else {
                        // No padding: the code style carries its own background,
                        // and spaces here double up with the ones around it.
                        let style = self.theme.markdown_code();
                        self.push_span(Span::styled(code.to_string(), style));
                    }
                }
                Event::Html(html) => {
                    self.flush();
                    for html_line in html.trim_end().lines() {
                        let line = Line::from(Span::styled(
                            html_line.to_string(),
                            Style::default().fg(self.c.orange),
                        ));
                        self.lines.push(line);
                    }
                }
                Event::InlineHtml(html) => {
                    if self.in_table {
                        self.cell.push_str(&html);
                    } else {
                        self.push_span(Span::styled(
                            html.to_string(),
                            Style::default().fg(self.c.orange),
                        ));
                    }
                }
                Event::FootnoteReference(name) => {
                    if self.in_table {
                        self.cell.push_str(&format!("[{}]", name));
                    } else {
                        self.push_span(Span::styled(
                            format!("[{}]", name),
                            Style::default().fg(self.c.cyan).add_modifier(Modifier::BOLD),
                        ));
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if self.in_table {
                        self.cell.push(' ');
                    } else {
                        self.flush();
                    }
                }
                Event::Rule => {
                    self.blank();
                    let line = self.rule('━', Style::default().fg(self.c.comment));
                    self.lines.push(line);
                    self.lines.push(Line::from(""));
                }
                Event::TaskListMarker(checked) => self.task_marker(checked),
            }
        }
        self.flush();
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                // A second paragraph in the same list item continues under the
                // marker rather than claiming one of its own.
                if self.in_item && self.item_marker.is_none() {
                    self.blank();
                }
            }
            Tag::Heading { level, .. } => {
                self.blank();
                self.in_heading = true;
                self.heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
            }
            Tag::CodeBlock(kind) => self.open_code_block(kind),
            Tag::List(start) => {
                self.flush();
                self.lists.push(ListLevel {
                    ordered: start.is_some(),
                    counter: start.unwrap_or(1),
                });
            }
            Tag::Item => self.start_item(),
            Tag::Emphasis => self.emphasis += 1,
            Tag::Strong => self.strong += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } => {
                self.in_link = true;
                self.link_url = dest_url.to_string();
            }
            Tag::Image { dest_url, .. } => {
                if !self.in_table {
                    self.push_span(Span::styled("🖼 ", Style::default().fg(self.c.purple)));
                }
                self.in_link = true;
                self.link_url = dest_url.to_string();
            }
            Tag::BlockQuote => {
                self.flush();
                if self.bq_depth == 0 {
                    self.blank();
                }
                self.bq_depth += 1;
                self.bq_first_line = true;
            }
            Tag::FootnoteDefinition(name) => {
                self.blank();
                let marker = format!("[{}]: ", name);
                self.item_indent = dwidth(&marker);
                self.item_marker = Some(vec![Span::styled(
                    marker,
                    Style::default().fg(self.c.cyan).add_modifier(Modifier::BOLD),
                )]);
                self.in_item = true;
            }
            Tag::Table(_) => {
                self.blank();
                self.in_table = true;
                self.head.clear();
                self.rows.clear();
                self.row.clear();
            }
            Tag::TableHead => {
                self.in_head = true;
                self.row.clear();
            }
            Tag::TableRow => self.row.clear(),
            Tag::TableCell => self.cell.clear(),
            Tag::HtmlBlock | Tag::MetadataBlock(_) => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                if !self.in_item {
                    self.blank();
                }
            }
            TagEnd::Heading(_) => {
                self.flush();
                self.in_heading = false;
                if self.heading_level <= 2 {
                    // Obsidian-style: a subtle separator under H1 and H2.
                    let rule = self.rule('─', Style::default().fg(self.c.bg_highlight));
                    self.lines.push(rule);
                }
                self.blank();
            }
            TagEnd::CodeBlock => self.close_code_block(),
            TagEnd::List(_) => {
                self.flush();
                self.lists.pop();
                if self.lists.is_empty() {
                    self.item_indent = 0;
                    self.blank();
                }
            }
            TagEnd::Item => {
                self.flush();
                if let Some(level) = self.lists.last_mut() {
                    level.counter += 1;
                }
                self.in_item = false;
                self.checked_item = false;
                self.item_marker = None;
            }
            TagEnd::Emphasis => self.emphasis = self.emphasis.saturating_sub(1),
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => {
                if !self.link_url.is_empty() && !self.in_table {
                    self.push_span(Span::styled(
                        format!(" ↗ {}", self.link_url),
                        Style::default().fg(self.c.comment),
                    ));
                }
                self.in_link = false;
                self.link_url.clear();
            }
            TagEnd::Image => {
                if !self.link_url.is_empty() && !self.in_table {
                    self.push_span(Span::styled(
                        format!(" ({})", self.link_url),
                        Style::default().fg(self.c.comment),
                    ));
                }
                self.in_link = false;
                self.link_url.clear();
            }
            TagEnd::BlockQuote => {
                self.flush();
                self.bq_depth = self.bq_depth.saturating_sub(1);
                self.trim_trailing_blanks();
                if self.bq_depth == 0 {
                    self.callout_color = None;
                    self.bq_first_line = false;
                    self.blank();
                }
            }
            TagEnd::FootnoteDefinition => {
                self.flush();
                self.in_item = false;
                self.item_marker = None;
                self.item_indent = 0;
            }
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.cell);
                self.row.push(cell.trim().to_string());
            }
            TagEnd::TableHead => {
                self.head = std::mem::take(&mut self.row);
                self.in_head = false;
            }
            TagEnd::TableRow => {
                let row = std::mem::take(&mut self.row);
                if !self.in_head {
                    self.rows.push(row);
                }
            }
            TagEnd::Table => {
                self.emit_table();
                self.in_table = false;
                self.blank();
            }
            TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => {}
        }
    }

    fn finish(mut self) -> Text<'static> {
        // Trailing blank lines are noise at the bottom of the pane.
        self.trim_trailing_blanks();
        Text::from(self.lines)
    }
}

/// Render markdown content to styled ratatui Text (Obsidian-like).
///
/// `width` is the inner width of the preview pane in columns; full-width
/// decorations (heading underlines, code-block borders, horizontal rules) and
/// table columns are sized to it so they fill the pane exactly instead of
/// wrapping into stub lines.
pub fn render_markdown_preview(content: &str, theme: &ThemeManager, width: usize) -> Text<'static> {
    let mut renderer = Renderer::new(theme, width);
    renderer.run(content);
    let text = renderer.finish();

    if text.lines.is_empty() {
        let c = theme.colors();
        return Text::from(vec![
            Line::from(vec![
                Span::styled("📝 ", Style::default().fg(c.cyan)),
                Span::styled("Live Preview", theme.markdown_h2()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Start typing in the editor to see the preview here...",
                theme.help_text(),
            )),
        ]);
    }
    text
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
  - Nested items line up under their parent

### Task Lists

- [x] Completed task
- [ ] Pending task

### Ordered Lists

1. First item
2. Second item
3. Third item

### Tables

| Feature | Chord | Notes |
|---|---|---|
| Preview | F2 | This pane |
| Go to | Ctrl+P | Notes, tags, headings |

---

### Code Blocks

```rust
fn hello_world() {
    println!("Hello from Scribble!");
}
```

> This is a blockquote with beautiful styling

> [!note] Callout Support
> Obsidian-style callouts are rendered here.

> [!tip] Pro Tip
> Use callouts to highlight important information.

Start editing your note to see the magic happen!"#;

    render_markdown_preview(sample_markdown, theme, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeManager;

    fn theme() -> ThemeManager {
        ThemeManager::new("tokyo-night")
    }

    /// The rendered pane as plain text — what the eye actually reads.
    fn render(md: &str, width: usize) -> Vec<String> {
        let text = render_markdown_preview(md, &theme(), width);
        text.lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect()
    }

    /// Column the first vertical divider sits in, measured in display cells.
    fn divider_column(line: &str) -> Option<usize> {
        let mut col = 0;
        for ch in line.chars() {
            if ch == '│' || ch == '┼' {
                return Some(col);
            }
            col += dwidth(ch.encode_utf8(&mut [0u8; 4]));
        }
        None
    }

    fn line_with(md: &str, needle: &str) -> String {
        render(md, 40)
            .into_iter()
            .find(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no rendered line contains {:?}\n{:#?}", needle, render(md, 40)))
    }

    #[test]
    fn blockquote_bar_is_not_doubled() {
        for line in render("> a quoted line\n> second line\n", 40) {
            assert!(
                !line.starts_with("▌ ▌"),
                "blockquote bar rendered twice: {:?}",
                line
            );
        }
    }

    #[test]
    fn nested_blockquote_bar_matches_the_depth() {
        let lines = render("> outer\n> > inner quote\n", 40);
        assert!(lines.contains(&"▌ outer".to_string()), "{:#?}", lines);
        assert!(lines.contains(&"▌ ▌ inner quote".to_string()), "{:#?}", lines);
    }

    #[test]
    fn blockquote_paragraph_break_keeps_the_bar() {
        let lines = render("> first para\n>\n> second para\n", 40);
        assert!(
            lines.iter().all(|l| l.starts_with('▌')),
            "a line inside the quote lost its bar: {:#?}",
            lines
        );
    }

    #[test]
    fn callout_renders_its_icon_and_title() {
        // `[!note]` is a link reference to CommonMark, so the parser hands it back
        // in pieces — the marker has to be matched against the whole line.
        let header = line_with("> [!note] Heads up\n> body\n", "Heads up");
        assert!(header.contains('✎'), "callout icon missing: {:?}", header);
        assert!(!header.contains("[!note]"), "raw marker left in: {:?}", header);
    }

    #[test]
    fn callout_type_alone_is_titled_by_its_type() {
        let header = line_with("> [!warning]\n> mind the gap\n", "⚠");
        assert!(header.contains("Warning"), "{:?}", header);
    }

    #[test]
    fn code_fence_borders_span_the_pane() {
        let lines = render("```rust\nfn main() {}\n```\n", 40);
        let head = &lines[0];
        let foot = lines.iter().find(|l| l.starts_with('╰')).expect("no foot");
        assert!(head.starts_with("╭─ rust "), "fence head malformed: {:?}", head);
        assert_eq!(dwidth(head), 40, "fence head does not fill the pane: {:?}", head);
        assert_eq!(dwidth(foot), 40, "fence foot does not fill the pane: {:?}", foot);
    }

    #[test]
    fn code_block_keeps_its_blank_lines() {
        let lines = render("```\na\n\nb\n```\n", 40);
        let body: Vec<&String> = lines.iter().filter(|l| l.starts_with('│')).collect();
        assert_eq!(body.len(), 3, "code body lines lost: {:#?}", lines);
    }

    #[test]
    fn nested_lists_each_get_their_own_line() {
        let lines = render("- a\n  - b\n    - c\n- d\n", 40);
        assert_eq!(
            lines,
            vec!["• a", "  • b", "    • c", "• d"],
            "nested items were glued together"
        );
    }

    #[test]
    fn ordered_list_numbers_itself_and_nests() {
        let lines = render("1. one\n2. two\n   - inner\n3. three\n", 40);
        assert_eq!(lines, vec!["1. one", "2. two", "  • inner", "3. three"]);
    }

    #[test]
    fn ordered_list_honours_its_start_number() {
        let lines = render("5. five\n6. six\n", 40);
        assert_eq!(lines, vec!["5. five", "6. six"]);
    }

    #[test]
    fn task_markers_replace_the_bullet_at_every_depth() {
        let lines = render("- [ ] top\n  - [x] sub done\n", 40);
        assert_eq!(lines, vec!["☐ top", "  ☑ sub done"]);
    }

    #[test]
    fn item_starting_with_inline_code_keeps_its_bullet() {
        let lines = render("- `cargo build` compiles it\n", 40);
        assert_eq!(lines, vec!["• cargo build compiles it"]);
    }

    #[test]
    fn second_paragraph_of_an_item_lines_up_under_it() {
        let lines = render("- first para\n\n  second para\n- next\n", 40);
        assert_eq!(lines, vec!["• first para", "", "  second para", "• next"]);
    }

    #[test]
    fn table_columns_are_aligned() {
        let lines = render(
            "| Name | Qty |\n|---|---|\n| Widget | 3 |\n| Grommet assembly | 12 |\n",
            40,
        );
        let widths: Vec<usize> = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| divider_column(l).expect("no column divider"))
            .collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "column divider does not line up: {:#?}",
            lines
        );
    }

    #[test]
    fn inline_code_in_a_cell_stays_in_the_cell() {
        let lines = render("| A |\n|---|\n| the `small` one |\n", 40);
        assert!(
            lines.iter().any(|l| l.contains("the small one")),
            "cell content lost: {:#?}",
            lines
        );
        assert_eq!(
            lines.iter().filter(|l| l.contains("small")).count(),
            1,
            "inline code leaked out of the table: {:#?}",
            lines
        );
    }

    #[test]
    fn a_wide_table_is_shrunk_to_the_pane() {
        let md = "| One | Two |\n|---|---|\n| a very long cell that will not fit | another long one |\n";
        for line in render(md, 30) {
            assert!(dwidth(&line) <= 30, "table line overflows the pane: {:?}", line);
        }
    }

    #[test]
    fn headings_are_separated_by_one_blank_line() {
        let lines = render("# H1\n\ntext\n\n## H2\n", 40);
        assert!(
            !lines.windows(2).any(|w| w[0].is_empty() && w[1].is_empty()),
            "double blank line between blocks: {:#?}",
            lines
        );
        assert!(!lines.first().unwrap().is_empty(), "leading blank line");
    }

    #[test]
    fn h1_and_h2_are_underlined_to_the_pane_width() {
        let lines = render("# Title\n", 40);
        assert_eq!(lines[0], "Title");
        assert_eq!(dwidth(&lines[1]), 40, "underline does not fill the pane");
    }

    #[test]
    fn inline_code_does_not_pad_itself_with_spaces() {
        assert_eq!(render("use `x` here\n", 40), vec!["use x here"]);
    }

    #[test]
    fn footnote_definition_is_labelled() {
        let lines = render("text[^1]\n\n[^1]: the body\n", 40);
        assert!(lines.contains(&"text[1]".to_string()), "{:#?}", lines);
        assert!(
            lines.contains(&"[1]: the body".to_string()),
            "footnote definition is indistinguishable from a paragraph: {:#?}",
            lines
        );
    }

    /// Reported from a real note: three `---` separators, and everything between
    /// the first two vanished from the preview.
    ///
    /// `ENABLE_YAML_STYLE_METADATA_BLOCKS` treats any pair of `---` lines as a
    /// metadata block, wherever they are, so the rule opened one and the parser
    /// handed back the whole middle of the note as frontmatter to hide.
    #[test]
    fn a_horizontal_rule_does_not_swallow_the_note() {
        let note = "# Yomi\n\n---\n  - Milk\n- I think that this is fixed\n\n1. No more auto numbering\n2. This is the better way\n\n---\n\nThis is what I want\n\n---\n";
        let lines = render(note, 40);
        for needle in [
            "Yomi",
            "Milk",
            "I think that this is fixed",
            "No more auto numbering",
            "This is the better way",
            "This is what I want",
        ] {
            assert!(
                lines.iter().any(|l| l.contains(needle)),
                "{:?} disappeared from the preview:\n{:#?}",
                needle,
                lines
            );
        }
        assert_eq!(
            lines.iter().filter(|l| l.starts_with('━')).count(),
            3,
            "not every rule was drawn: {:#?}",
            lines
        );
    }

    #[test]
    fn two_rules_in_a_row_keep_what_lies_between_them() {
        let lines = render("---\n\nmiddle\n\n---\n\nend\n", 40);
        assert!(lines.iter().any(|l| l.contains("middle")), "{:#?}", lines);
        assert!(lines.iter().any(|l| l.contains("end")), "{:#?}", lines);
    }

    /// An opening `---` with no closing one is a rule, not the start of
    /// frontmatter that eats the rest of the file.
    #[test]
    fn an_unclosed_leading_marker_is_left_alone() {
        let lines = render("---\n\nstill here\n", 40);
        assert!(
            lines.iter().any(|l| l.contains("still here")),
            "the note was treated as unterminated frontmatter: {:#?}",
            lines
        );
    }

    #[test]
    fn frontmatter_is_only_stripped_from_the_very_top() {
        let lines = render("intro\n\n---\ntitle: not frontmatter\n---\n\ntail\n", 40);
        assert!(lines.iter().any(|l| l.contains("title: not frontmatter")), "{:#?}", lines);
        assert!(lines.iter().any(|l| l.contains("tail")), "{:#?}", lines);
    }

    #[test]
    fn frontmatter_is_not_rendered() {
        let lines = render("---\ntitle: Note\n---\n\nBody text\n", 40);
        assert_eq!(lines, vec!["Body text"], "frontmatter leaked into the preview");
    }

    #[test]
    fn punctuation_is_shown_as_typed() {
        let lines = render("she said \"hello\" -- really\n", 40);
        assert_eq!(lines, vec!["she said \"hello\" -- really"]);
    }

    #[test]
    fn a_checked_task_is_dimmed_and_struck_through() {
        let text = render_markdown_preview("- [x] done\n", &theme(), 40);
        let span = text.lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("done"))
            .expect("task text missing");
        assert!(
            span.style.add_modifier.contains(Modifier::CROSSED_OUT),
            "completed task is not struck through"
        );
    }

    #[test]
    fn empty_content_shows_the_placeholder() {
        let lines = render("", 40);
        assert!(lines.iter().any(|l| l.contains("Live Preview")), "{:#?}", lines);
    }

    #[test]
    fn full_width_decorations_never_overflow_the_pane() {
        let md = "# Heading\n\n---\n\n```sh\necho hi\n```\n";
        for width in [10usize, 24, 40, 80] {
            for line in render(md, width) {
                assert!(
                    dwidth(&line) <= width,
                    "line {:?} exceeds pane width {}",
                    line,
                    width
                );
            }
        }
    }

    #[test]
    fn the_sample_preview_renders_every_feature_it_advertises() {
        let text = generate_preview_sample(&theme(), 60);
        let lines: Vec<String> = text
            .lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect();
        for needle in ["✎ Callout Support", "☑ Completed task", "╭─ rust ", "1. First item"] {
            assert!(
                lines.iter().any(|l| l.contains(needle)),
                "sample preview is missing {:?}",
                needle
            );
        }
    }
}

