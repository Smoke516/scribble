
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

pub(crate) fn create_temp_file(title: &str, content: &str) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    
    let temp_dir = std::env::temp_dir();
    let sanitized_title = title.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();
    
    let temp_file = temp_dir.join(format!("scribble_{}_{}.md", 
        sanitized_title, 
        std::process::id()));
    
    let mut file = std::fs::File::create(&temp_file)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    
    Ok(temp_file)
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

pub(crate) fn run_external_editor(editor: &str, file_path: &std::path::PathBuf) -> Result<(), String> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
        cursor::Show
    };
    use std::io::{stdout, Write};
    
    // Fully reset terminal to normal mode
    let mut stdout = stdout();
    
    // Disable raw mode first
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;
    
    // Clear screen and show cursor
    execute!(stdout, Clear(ClearType::All), Show)
        .map_err(|e| format!("Failed to clear screen: {}", e))?;
    
    // Flush to ensure terminal is ready
    stdout.flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;
    
    // Run the external editor with proper stdio inheritance
    let status = std::process::Command::new(editor)
        .arg(file_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to start {}: {}", editor, e))?;
    
    // Give terminal a moment to settle after editor exits
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Re-enable raw mode for TUI
    enable_raw_mode().map_err(|e| format!("Failed to re-enable raw mode: {}", e))?;
    
    // Clear and reset for our TUI
    execute!(stdout, Clear(ClearType::All))
        .map_err(|e| format!("Failed to clear screen for TUI: {}", e))?;
    
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with code {:?}", editor, status.code()))
    }
}
