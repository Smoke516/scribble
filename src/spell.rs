//! Spell checking via the system `aspell` binary.
//!
//! Uses `aspell list` for batch checking and `aspell -a` for suggestions.
//! Fenced code blocks and inline code spans are skipped automatically.

use std::collections::HashSet;
use std::io::Write;
use std::process::{Command, Stdio};

/// Returns `true` if `aspell` is found and executable.
pub fn check_available() -> bool {
    Command::new("aspell")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|mut c| {
            let _ = c.kill();
            true
        })
        .unwrap_or(false)
}

/// Check spelling in `content`. Returns `(row, col, word_len)` for each error.
///
/// Fenced code blocks (``` fences) and inline backtick spans are skipped.
/// Words shorter than 3 chars are ignored.
pub fn check_content(content: &str) -> Vec<(usize, usize, usize)> {
    // Build per-line cleaned text and a skip flag
    let mut skipped: Vec<bool> = Vec::new();
    let mut cleaned: Vec<String> = Vec::new();
    let mut in_fence = false;

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            skipped.push(true);
            cleaned.push(String::new());
            continue;
        }
        if in_fence {
            skipped.push(true);
            cleaned.push(String::new());
            continue;
        }
        skipped.push(false);
        cleaned.push(strip_inline_code(line));
    }

    let plain_text: String = cleaned.join("\n");
    let misspelled = run_aspell_list(&plain_text);
    if misspelled.is_empty() {
        return Vec::new();
    }

    // Map misspelled words back to positions in the original content
    let mut errors = Vec::new();
    for (row, (line, &skip)) in content.lines().zip(skipped.iter()).enumerate() {
        if skip {
            continue;
        }
        for (col, word) in extract_words(line) {
            if misspelled.contains(&word.to_lowercase()) {
                errors.push((row, col, word.len()));
            }
        }
    }
    errors
}

/// Fetch up to 10 spelling suggestions for a single word using `aspell -a`.
pub fn get_suggestions(word: &str) -> Vec<String> {
    let mut child = match Command::new("aspell")
        .arg("-a")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(word.as_bytes());
        let _ = stdin.write_all(b"\n");
    }

    match child.wait_with_output() {
        Ok(out) => parse_aspell_a(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => Vec::new(),
    }
}

/// Pipe `text` to `aspell list`; returns the set of misspelled words (lowercased).
fn run_aspell_list(text: &str) -> HashSet<String> {
    let mut child = match Command::new("aspell")
        .arg("list")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return HashSet::new(),
    };

    // Write content then close stdin so aspell gets EOF
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }

    match child.wait_with_output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_lowercase())
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Replace inline backtick code spans with spaces so aspell ignores them.
fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for ch in line.chars() {
        match ch {
            '`' => {
                in_code = !in_code;
                out.push(' ');
            }
            _ if in_code => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

/// Parse `aspell -a` pipe output.
///
/// Lines starting with `&` have suggestions:
/// `& word count offset: suggestion1, suggestion2, ...`
fn parse_aspell_a(output: &str) -> Vec<String> {
    for line in output.lines() {
        if line.starts_with('&') {
            if let Some(pos) = line.find(": ") {
                return line[pos + 2..]
                    .split(", ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .take(10)
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Extract `(col_byte_offset, word_str)` pairs from a line.
///
/// Only ASCII-alphabetic words of length ≥ 3 are returned.
/// Apostrophes inside words (contractions) are included.
pub fn extract_words(line: &str) -> Vec<(usize, &str)> {
    let bytes = line.as_bytes();
    let len = line.len();
    let mut result = Vec::new();
    let mut i = 0;

    while i < len {
        if !bytes[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        while i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'\'') {
            i += 1;
        }
        // Trim trailing apostrophe (e.g. "don'" → "don")
        let end = if i > start && bytes[i - 1] == b'\'' {
            i - 1
        } else {
            i
        };
        if end - start >= 3 {
            result.push((start, &line[start..end]));
        }
    }
    result
}
