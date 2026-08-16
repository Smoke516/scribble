//! Operator-pending editing: `d`, `c` and `y` composed with motions and text
//! objects, the way vim actually does it.
//!
//! Before this, `dd` and `yy` were hard-coded two-key sequences — the only thing
//! either operator could be combined with was itself. An operator-pending model
//! costs about the same code and gives every operator every motion, so `dw`,
//! `d$`, `c3w`, `ciw`, `yG` and the rest all fall out of one table rather than
//! needing a key binding each.
//!
//! The resolution half is deliberately pure: it maps `(content, cursor, count,
//! target)` to a span of text and touches nothing. That is what makes the awkward
//! parts — counts, line crossing, vim's end-of-line exception for `dw` — testable
//! without standing up an editor.

/// What to do with the span a motion selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Delete,
    Change,
    Yank,
}

/// Where a motion lands, relative to the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    WordForward,
    WordBackward,
    WordEnd,
    LineStart,
    FirstNonBlank,
    LineEnd,
    Down,
    Up,
    FileStart,
    FileEnd,
    /// The doubled form — `dd`, `cc`, `yy`.
    WholeLine,
}

/// A region defined by what it is rather than by where the cursor travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObject {
    /// `iw` — the word under the cursor.
    InnerWord,
    /// `aw` — the word under the cursor plus the whitespace after it.
    AWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Motion(Motion),
    Object(TextObject),
}

/// The text an operator will act on.
///
/// The distinction is not cosmetic: `dd` takes the line's newline with it and
/// leaves no empty line behind, while `dw` never does, and `p` pastes a linewise
/// yank onto its own line but a charwise yank inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    /// Half-open, in (row, column) char coordinates.
    Chars { start: (usize, usize), end: (usize, usize) },
    /// Inclusive range of whole lines.
    Lines { first: usize, last: usize },
}

/// vim's three character classes. Word and punctuation are separate classes, which
/// is why `w` stops between `foo` and `(` rather than skipping over both.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Class {
    Blank,
    Word,
    Punct,
}

fn class_of(c: char) -> Class {
    if c.is_whitespace() {
        Class::Blank
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}

/// The buffer as lines of chars, which is the shape every motion here wants.
fn grid(content: &str) -> Vec<Vec<char>> {
    let mut lines: Vec<Vec<char>> = content.lines().map(|l| l.chars().collect()).collect();
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

/// Step one character forward, moving to the next line at the end of this one.
/// The position one past the last character of a line is a real position (that is
/// where the newline sits), so a caller can distinguish "end of line" from "start
/// of the next".
fn next_pos(lines: &[Vec<char>], (row, col): (usize, usize)) -> Option<(usize, usize)> {
    if col < lines[row].len() {
        Some((row, col + 1))
    } else if row + 1 < lines.len() {
        Some((row + 1, 0))
    } else {
        None
    }
}

fn prev_pos(lines: &[Vec<char>], (row, col): (usize, usize)) -> Option<(usize, usize)> {
    if col > 0 {
        Some((row, col - 1))
    } else if row > 0 {
        Some((row - 1, lines[row - 1].len()))
    } else {
        None
    }
}

fn char_at(lines: &[Vec<char>], (row, col): (usize, usize)) -> Option<char> {
    lines.get(row).and_then(|l| l.get(col)).copied()
}

/// Where `w` lands: the start of the next word, counting a run of punctuation as a
/// word in its own right. Crosses lines, and an empty line counts as a word — which
/// is why `w` stops on blank lines in a paragraph.
fn word_forward(lines: &[Vec<char>], from: (usize, usize)) -> (usize, usize) {
    let mut pos = from;
    let start_class = char_at(lines, pos).map(class_of);

    // Step off the current word.
    if let Some(cls) = start_class {
        if cls != Class::Blank {
            while let Some(next) = next_pos(lines, pos) {
                pos = next;
                match char_at(lines, pos) {
                    Some(c) if class_of(c) == cls => continue,
                    _ => break,
                }
            }
        }
    }

    // Then over any blanks, stopping on an empty line the way vim does.
    loop {
        match char_at(lines, pos) {
            Some(c) if class_of(c) == Class::Blank => {}
            Some(_) => break,
            None => {
                if pos.1 == 0 && lines[pos.0].is_empty() && pos != from {
                    break;
                }
            }
        }
        match next_pos(lines, pos) {
            Some(next) => pos = next,
            None => break,
        }
    }
    pos
}

/// Where `b` lands: the start of the word before the cursor.
fn word_backward(lines: &[Vec<char>], from: (usize, usize)) -> (usize, usize) {
    let mut pos = from;

    // Back over blanks (and over the line break) to land inside a word.
    loop {
        let Some(prev) = prev_pos(lines, pos) else { return pos };
        pos = prev;
        match char_at(lines, pos) {
            Some(c) if class_of(c) != Class::Blank => break,
            None if lines[pos.0].is_empty() => return pos,
            _ => continue,
        }
    }

    // Then to the front of whatever class we landed in.
    let cls = char_at(lines, pos).map(class_of).unwrap_or(Class::Blank);
    while let Some(prev) = prev_pos(lines, pos) {
        match char_at(lines, prev) {
            Some(c) if class_of(c) == cls => pos = prev,
            _ => break,
        }
    }
    pos
}

/// Where `e` lands: the last character of the current or next word. Unlike `w`,
/// this is inclusive of the character it lands on, so operators add one.
fn word_end(lines: &[Vec<char>], from: (usize, usize)) -> (usize, usize) {
    let mut pos = match next_pos(lines, from) {
        Some(p) => p,
        None => return from,
    };

    // Forward to the next non-blank.
    while char_at(lines, pos).map(class_of) != Some(Class::Word)
        && char_at(lines, pos).map(class_of) != Some(Class::Punct)
    {
        match next_pos(lines, pos) {
            Some(next) => pos = next,
            None => return pos,
        }
    }

    // Then to the end of its run.
    let cls = class_of(char_at(lines, pos).expect("landed on a character"));
    while let Some(next) = next_pos(lines, pos) {
        match char_at(lines, next) {
            Some(c) if class_of(c) == cls => pos = next,
            _ => break,
        }
    }
    pos
}

/// The bounds of the word under the cursor, as a half-open column range.
fn word_under(line: &[char], col: usize) -> (usize, usize) {
    if line.is_empty() {
        return (0, 0);
    }
    let col = col.min(line.len().saturating_sub(1));
    let cls = class_of(line[col]);

    let mut start = col;
    while start > 0 && class_of(line[start - 1]) == cls {
        start -= 1;
    }
    let mut end = col + 1;
    while end < line.len() && class_of(line[end]) == cls {
        end += 1;
    }
    (start, end)
}

/// Resolve a target into the span of text an operator should act on.
///
/// `cursor` is `(row, column)` in characters. Returns `None` when the motion has
/// nowhere to go, which the caller should treat as "do nothing" rather than as an
/// empty edit — `dw` at the very end of the buffer must not count as an edit.
pub fn resolve(
    content: &str,
    cursor: (usize, usize),
    count: usize,
    target: Target,
    operator: Operator,
) -> Option<Span> {
    let lines = grid(content);
    let count = count.max(1);
    let row = cursor.0.min(lines.len().saturating_sub(1));
    let col = cursor.1.min(lines[row].len());

    match target {
        Target::Motion(Motion::WholeLine) => {
            let last = (row + count - 1).min(lines.len().saturating_sub(1));
            Some(Span::Lines { first: row, last })
        }
        Target::Motion(Motion::Down) => {
            let last = (row + count).min(lines.len().saturating_sub(1));
            Some(Span::Lines { first: row, last })
        }
        Target::Motion(Motion::Up) => {
            let first = row.saturating_sub(count);
            Some(Span::Lines { first, last: row })
        }
        Target::Motion(Motion::FileStart) => Some(Span::Lines { first: 0, last: row }),
        Target::Motion(Motion::FileEnd) => Some(Span::Lines {
            first: row,
            last: lines.len().saturating_sub(1),
        }),
        Target::Motion(Motion::LineEnd) => {
            let end = lines[row].len();
            if col >= end {
                return None;
            }
            Some(Span::Chars { start: (row, col), end: (row, end) })
        }
        Target::Motion(Motion::LineStart) => {
            if col == 0 {
                return None;
            }
            Some(Span::Chars { start: (row, 0), end: (row, col) })
        }
        Target::Motion(Motion::FirstNonBlank) => {
            let first = lines[row]
                .iter()
                .position(|c| !c.is_whitespace())
                .unwrap_or(0);
            if col <= first {
                return None;
            }
            Some(Span::Chars { start: (row, first), end: (row, col) })
        }
        Target::Motion(Motion::WordForward) => {
            let mut pos = (row, col);
            for _ in 0..count {
                let next = word_forward(&lines, pos);
                if next == pos {
                    break;
                }
                pos = next;
            }
            if pos == (row, col) {
                return None;
            }
            // vim's exception: `dw` on the last word of a line stops at the end of
            // the line rather than dragging the next line up. Without this, deleting
            // the final word of a paragraph silently joins it to the next one.
            if operator != Operator::Yank && pos.0 > row && !lines[row][col..].is_empty() {
                let end = lines[row].len();
                if col < end {
                    return Some(Span::Chars { start: (row, col), end: (row, end) });
                }
            }
            Some(Span::Chars { start: (row, col), end: pos })
        }
        Target::Motion(Motion::WordBackward) => {
            let mut pos = (row, col);
            for _ in 0..count {
                let next = word_backward(&lines, pos);
                if next == pos {
                    break;
                }
                pos = next;
            }
            if pos == (row, col) {
                return None;
            }
            Some(Span::Chars { start: pos, end: (row, col) })
        }
        Target::Motion(Motion::WordEnd) => {
            let mut pos = (row, col);
            for _ in 0..count {
                let next = word_end(&lines, pos);
                if next == pos {
                    break;
                }
                pos = next;
            }
            if pos == (row, col) {
                return None;
            }
            // `e` is inclusive of the character it lands on.
            let end = next_pos(&lines, pos).unwrap_or(pos);
            Some(Span::Chars { start: (row, col), end })
        }
        Target::Object(object) => {
            let line = &lines[row];
            if line.is_empty() {
                return None;
            }
            let (start, mut end) = word_under(line, col);
            if object == TextObject::AWord {
                // `aw` takes the trailing whitespace, or the leading whitespace when
                // there is none after — so `daw` on the last word of a line does not
                // leave a dangling space behind it.
                let after = end;
                while end < line.len() && line[end].is_whitespace() {
                    end += 1;
                }
                if end == after {
                    let mut back = start;
                    while back > 0 && line[back - 1].is_whitespace() {
                        back -= 1;
                    }
                    return Some(Span::Chars { start: (row, back), end: (row, end) });
                }
            }
            Some(Span::Chars { start: (row, start), end: (row, end) })
        }
    }
}

/// The text a span covers, and the buffer with that text removed.
///
/// Returns `(removed, remaining, cursor)`. A linewise span takes its trailing
/// newline with it, which is what stops `dd` leaving a blank line behind.
pub fn cut(content: &str, span: Span) -> (String, String, (usize, usize)) {
    let lines: Vec<&str> = content.lines().collect();

    match span {
        Span::Lines { first, last } => {
            // `"".lines()` yields nothing at all, so an empty buffer has no line 0
            // to slice — even though `dd` on it is a perfectly ordinary keystroke.
            if lines.is_empty() || first >= lines.len() {
                return (String::new(), content.to_string(), (0, 0));
            }
            let last = last.min(lines.len() - 1);
            let removed: String = lines[first..=last]
                .iter()
                .map(|l| format!("{}\n", l))
                .collect();
            let mut kept: Vec<&str> = Vec::new();
            kept.extend_from_slice(&lines[..first]);
            if last + 1 < lines.len() {
                kept.extend_from_slice(&lines[last + 1..]);
            }
            let remaining = if kept.is_empty() {
                String::new()
            } else {
                format!("{}\n", kept.join("\n"))
            };
            // Land on the line that took the deleted one's place, or the last line
            // if the deletion ran off the end.
            let row = first.min(kept.len().saturating_sub(1));
            (removed, remaining, (row, 0))
        }
        Span::Chars { start, end } => {
            let start_off = offset_of(content, start);
            let end_off = offset_of(content, end);
            let (start_off, end_off) = (start_off.min(end_off), start_off.max(end_off));
            let removed = content[start_off..end_off].to_string();
            let mut remaining = String::with_capacity(content.len() - removed.len());
            remaining.push_str(&content[..start_off]);
            remaining.push_str(&content[end_off..]);
            (removed, remaining, start)
        }
    }
}

/// Byte offset of a (row, column) char position.
fn offset_of(content: &str, (row, col): (usize, usize)) -> usize {
    let mut offset = 0;
    for (i, line) in content.lines().enumerate() {
        if i == row {
            return offset
                + line
                    .char_indices()
                    .nth(col)
                    .map(|(b, _)| b)
                    .unwrap_or(line.len());
        }
        offset += line.len() + 1;
    }
    content.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut_with(content: &str, cursor: (usize, usize), count: usize, target: Target, op: Operator) -> (String, String) {
        let span = resolve(content, cursor, count, target, op).expect("motion resolved");
        let (removed, remaining, _) = cut(content, span);
        (removed, remaining)
    }

    fn dw(content: &str, cursor: (usize, usize), count: usize) -> (String, String) {
        cut_with(content, cursor, count, Target::Motion(Motion::WordForward), Operator::Delete)
    }

    #[test]
    fn dw_deletes_a_word_and_the_space_after_it() {
        let (removed, left) = dw("the quick brown fox\n", (0, 4), 1);
        assert_eq!(removed, "quick ");
        assert_eq!(left, "the brown fox\n");
    }

    #[test]
    fn a_count_multiplies_the_motion() {
        let (removed, left) = dw("the quick brown fox\n", (0, 4), 2);
        assert_eq!(removed, "quick brown ");
        assert_eq!(left, "the fox\n");
    }

    /// Punctuation is its own word class, which is why `dw` on `foo` stops at the
    /// bracket rather than swallowing it.
    #[test]
    fn punctuation_is_a_word_of_its_own() {
        let (removed, left) = dw("call(arg)\n", (0, 0), 1);
        assert_eq!(removed, "call");
        assert_eq!(left, "(arg)\n");
    }

    /// vim's end-of-line exception. Without it, deleting the last word of a line
    /// silently drags the following line up, which is never what anyone meant.
    #[test]
    fn dw_on_the_last_word_of_a_line_does_not_join_the_next_one() {
        let (removed, left) = dw("first line\nsecond line\n", (0, 6), 1);
        assert_eq!(removed, "line");
        assert_eq!(left, "first \nsecond line\n");
    }

    /// Yanking has no such exception — `yw` really does reach into the next line,
    /// because nothing is being destroyed.
    #[test]
    fn yw_crosses_the_line_break_that_dw_stops_at() {
        let (removed, _) = cut_with(
            "first line\nsecond line\n",
            (0, 6),
            1,
            Target::Motion(Motion::WordForward),
            Operator::Yank,
        );
        assert_eq!(removed, "line\n");
    }

    #[test]
    fn db_deletes_backwards_to_the_start_of_the_previous_word() {
        let (removed, left) = cut_with(
            "the quick brown\n",
            (0, 10),
            1,
            Target::Motion(Motion::WordBackward),
            Operator::Delete,
        );
        assert_eq!(removed, "quick ");
        assert_eq!(left, "the brown\n");
    }

    /// `e` is inclusive of the character it lands on, unlike every other motion here.
    #[test]
    fn de_takes_the_character_it_lands_on() {
        let (removed, left) = cut_with(
            "the quick brown\n",
            (0, 4),
            1,
            Target::Motion(Motion::WordEnd),
            Operator::Delete,
        );
        assert_eq!(removed, "quick");
        assert_eq!(left, "the  brown\n");
    }

    #[test]
    fn d_dollar_deletes_to_the_end_of_the_line() {
        let (removed, left) = cut_with(
            "keep this remove this\nnext\n",
            (0, 10),
            1,
            Target::Motion(Motion::LineEnd),
            Operator::Delete,
        );
        assert_eq!(removed, "remove this");
        assert_eq!(left, "keep this \nnext\n");
    }

    #[test]
    fn d_zero_deletes_back_to_the_start_of_the_line() {
        let (removed, left) = cut_with(
            "remove this keep this\n",
            (0, 12),
            1,
            Target::Motion(Motion::LineStart),
            Operator::Delete,
        );
        assert_eq!(removed, "remove this ");
        assert_eq!(left, "keep this\n");
    }

    /// `dd` takes the newline with it, so no blank line is left behind.
    #[test]
    fn dd_removes_the_whole_line_including_its_newline() {
        let (removed, left) = cut_with(
            "one\ntwo\nthree\n",
            (1, 1),
            1,
            Target::Motion(Motion::WholeLine),
            Operator::Delete,
        );
        assert_eq!(removed, "two\n");
        assert_eq!(left, "one\nthree\n");
    }

    #[test]
    fn a_count_on_dd_takes_that_many_lines() {
        let (removed, left) = cut_with(
            "one\ntwo\nthree\nfour\n",
            (0, 0),
            3,
            Target::Motion(Motion::WholeLine),
            Operator::Delete,
        );
        assert_eq!(removed, "one\ntwo\nthree\n");
        assert_eq!(left, "four\n");
    }

    /// A count that runs past the end of the buffer clamps rather than panicking.
    #[test]
    fn a_count_past_the_end_of_the_buffer_is_clamped() {
        let (removed, left) = cut_with(
            "one\ntwo\n",
            (0, 0),
            99,
            Target::Motion(Motion::WholeLine),
            Operator::Delete,
        );
        assert_eq!(removed, "one\ntwo\n");
        assert_eq!(left, "");
    }

    #[test]
    fn dj_is_linewise_and_takes_both_lines() {
        let (removed, left) = cut_with(
            "one\ntwo\nthree\n",
            (0, 2),
            1,
            Target::Motion(Motion::Down),
            Operator::Delete,
        );
        assert_eq!(removed, "one\ntwo\n");
        assert_eq!(left, "three\n");
    }

    #[test]
    fn diw_takes_the_word_under_the_cursor_and_nothing_else() {
        let (removed, left) = cut_with(
            "the quick brown\n",
            (0, 6),
            1,
            Target::Object(TextObject::InnerWord),
            Operator::Delete,
        );
        assert_eq!(removed, "quick");
        assert_eq!(left, "the  brown\n");
    }

    #[test]
    fn daw_takes_the_trailing_space_too() {
        let (removed, left) = cut_with(
            "the quick brown\n",
            (0, 6),
            1,
            Target::Object(TextObject::AWord),
            Operator::Delete,
        );
        assert_eq!(removed, "quick ");
        assert_eq!(left, "the brown\n");
    }

    /// On the last word of a line there is no trailing whitespace to take, so `aw`
    /// takes the leading whitespace instead rather than leaving a dangling space.
    #[test]
    fn daw_on_the_last_word_takes_the_space_before_it() {
        let (removed, left) = cut_with(
            "the quick brown\n",
            (0, 12),
            1,
            Target::Object(TextObject::AWord),
            Operator::Delete,
        );
        assert_eq!(removed, " brown");
        assert_eq!(left, "the quick\n");
    }

    /// A motion with nowhere to go must not register as an edit — otherwise `dw` at
    /// the end of the buffer pushes an undo snapshot for having done nothing.
    #[test]
    fn a_motion_with_nowhere_to_go_resolves_to_nothing() {
        assert!(resolve("word", (0, 4), 1, Target::Motion(Motion::WordForward), Operator::Delete).is_none());
        assert!(resolve("word", (0, 0), 1, Target::Motion(Motion::LineStart), Operator::Delete).is_none());
        assert!(resolve("word", (0, 4), 1, Target::Motion(Motion::LineEnd), Operator::Delete).is_none());
    }

    /// Multi-byte characters are why every position here is a char index and every
    /// splice goes through offset_of. Slicing by column directly would panic.
    #[test]
    fn multi_byte_characters_do_not_break_the_splice() {
        let (removed, left) = dw("héllo wörld again\n", (0, 6), 1);
        assert_eq!(removed, "wörld ");
        assert_eq!(left, "héllo again\n");
    }

    #[test]
    fn an_empty_buffer_is_not_a_panic() {
        assert!(resolve("", (0, 0), 1, Target::Motion(Motion::WordForward), Operator::Delete).is_none());
        assert!(resolve("", (0, 0), 1, Target::Object(TextObject::InnerWord), Operator::Delete).is_none());
        let span = resolve("", (0, 0), 1, Target::Motion(Motion::WholeLine), Operator::Delete).unwrap();
        let (_, left, _) = cut("", span);
        assert_eq!(left, "");
    }

    /// The cursor has to end up somewhere valid, or the next keystroke indexes off
    /// the end of the buffer.
    #[test]
    fn deleting_the_last_line_leaves_the_cursor_on_a_real_line() {
        let span = resolve("one\ntwo\n", (1, 0), 1, Target::Motion(Motion::WholeLine), Operator::Delete).unwrap();
        let (_, left, cursor) = cut("one\ntwo\n", span);
        assert_eq!(left, "one\n");
        assert_eq!(cursor.0, 0, "cursor left pointing at a line that no longer exists");
    }

    #[test]
    fn dG_deletes_to_the_end_of_the_buffer() {
        let (removed, left) = cut_with(
            "one\ntwo\nthree\n",
            (1, 0),
            1,
            Target::Motion(Motion::FileEnd),
            Operator::Delete,
        );
        assert_eq!(removed, "two\nthree\n");
        assert_eq!(left, "one\n");
    }

    #[test]
    fn dgg_deletes_back_to_the_start_of_the_buffer() {
        let (removed, left) = cut_with(
            "one\ntwo\nthree\n",
            (1, 0),
            1,
            Target::Motion(Motion::FileStart),
            Operator::Delete,
        );
        assert_eq!(removed, "one\ntwo\n");
        assert_eq!(left, "three\n");
    }
}
