
pub(crate) fn detect_external_editor() -> Option<String> {
    // Check environment variables first
    if let Ok(editor) = std::env::var("EDITOR") {
        return Some(editor);
    }
    
    // Try to find helix first (preferred)
    if command_exists("hx") {
        return Some("hx".to_string());
    }
    
    if command_exists("helix") {
        return Some("helix".to_string());
    }
    
    // Fallback to other popular editors
    let editors = ["nvim", "vim", "nano", "emacs"];
    for editor in &editors {
        if command_exists(editor) {
            return Some(editor.to_string());
        }
    }
    
    None
}

pub(crate) fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}


pub fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub(crate) fn parse_datetime(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{DateTime, Utc, NaiveDateTime};
    
    // Try parsing the format used by our export: "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
    }
    
    // Try parsing ISO 8601 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }
    
    // Try parsing other common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S UTC",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d",
        "%m/%d/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ];
    
    for format in &formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
            return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
    }
    
    None
}

/// Hand the terminal to `editor` for the duration, then take it back.
///
/// Scribble lives on the alternate screen with raw mode, mouse capture and
/// bracketed paste on. All of that has to come off before another full-screen
/// program runs, or the editor draws over scribble's screen and receives mouse
/// and paste sequences it never asked for. Leaving the alternate screen also puts
/// the editor session where the user expects it — on the normal screen, with
/// their scrollback.
///
/// Every step is best-effort on the way back: if one fails, the rest must still
/// run, or the terminal is left in a worse state than the error being reported.
pub(crate) fn run_external_editor(editor: &str, file_path: &std::path::Path) -> Result<(), String> {
    use crossterm::{
        execute,
        event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::io::{stdout, Write};

    let mut out = stdout();
    let _ = disable_raw_mode();
    let _ = execute!(out, DisableMouseCapture, DisableBracketedPaste, LeaveAlternateScreen);
    let _ = out.flush();

    let status = std::process::Command::new(editor)
        .arg(file_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    let _ = enable_raw_mode();
    let _ = execute!(out, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste);
    let _ = out.flush();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{} exited with code {:?}", editor, s.code())),
        Err(e) => Err(format!("Failed to start {}: {}", editor, e)),
    }
}
