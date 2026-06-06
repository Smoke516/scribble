mod app;
mod autocomplete;
mod config;
mod events;
mod models;
mod preview;
mod search;
mod spell;
mod storage;
mod syntax;
mod tags;
mod theme;
mod ui;
mod watcher;

use app::App;
use std::env;
use std::path::PathBuf;

// Version information from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, DisableBracketedPaste, EnableBracketedPaste, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::stdout,
    time::{Duration, Instant},
};

// Auto-detect if current directory is an Obsidian vault
fn detect_obsidian_vault() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    
    // Check current directory and up to 3 parent directories
    let mut check_dir = current_dir.clone();
    for _ in 0..4 {
        let obsidian_dir = check_dir.join(".obsidian");
        if obsidian_dir.exists() && obsidian_dir.is_dir() {
            return Some(check_dir);
        }
        
        // Move to parent directory
        if let Some(parent) = check_dir.parent() {
            check_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let mut config = config::Config::load();
    
    // Handle command line arguments
    let args: Vec<String> = env::args().collect();
    let mut vault_path: Option<PathBuf> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-v" => {
                println!("{} {}", PKG_NAME, VERSION);
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--vault" => {
                if i + 1 < args.len() {
                    vault_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1; // Skip next argument as it's the vault path
                } else {
                    eprintln!("--vault requires a path argument");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
        i += 1;
    }
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load data
    let mut app = App::new(&config);
    
    // Initialize available vaults for vault switcher
    app.initialize_available_vaults(&config);
    
    // Determine vault path with priority: CLI arg > auto-detect > config default
    let final_vault_path = vault_path
        .or_else(|| if config.vaults.auto_detect { detect_obsidian_vault() } else { None })
        .or_else(|| config.vaults.default.clone());
    
    // Initialize storage based on mode
    let storage: Box<dyn storage::NotebookStorage> = if let Some(vault_path) = final_vault_path.clone() {
        // Initialize file watcher for vault mode
        app.initialize_file_watcher(vault_path.clone());
        
        // Update recent vaults in config
        config.add_recent_vault(vault_path.clone());
        if let Err(e) = config.save() {
            eprintln!("Warning: Failed to save config: {}", e);
        }
        
        Box::new(storage::VaultStorage::new(vault_path)?)
    } else {
        // Regular JSON storage
        Box::new(storage::Storage::new()?)
    };
    
    // Load existing notebook data
    match storage.load_notebook() {
        Ok(notebook) => {
            app.notebook = notebook;
            app.initialize_tag_manager();
            app.refresh_tree_view();
            // Start with no note selected - show welcome page instead
            app.set_welcome_message();
        }
        Err(e) => {
            app.set_message(format!("Failed to load notebook: {}. Starting fresh.", e));
        }
    }

    // Main loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    
    let result = loop {
        // Handle returning from external editor
        if app.just_returned_from_editor {
            app.just_returned_from_editor = false;
            // Force a complete redraw
            terminal.clear()?;
        }
        
        // Draw UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle events
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if let Err(e) = events::handle_event(&mut app, Event::Key(key)) {
                        break Err(e.into());
                    }
                }
                Event::Paste(text) => {
                    if let Err(e) = events::handle_paste(&mut app, &text) {
                        break Err(e.into());
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_visual_feedback();
            // Poll for file changes if file watcher is enabled
            app.poll_file_changes();
            last_tick = Instant::now();
        }

        // Persist pending changes to disk as they happen (autosave, explicit
        // save, structural edits) so work survives a crash — not only on exit.
        // Writes only the changed notes (and removes deleted files); folder-tree
        // changes fall back to a full save.
        if app.pending_disk_save {
            let result = if app.force_full_save {
                storage.save_notebook(&app.notebook).map(|_| Vec::new())
            } else {
                let dirty: Vec<_> = app.dirty_note_ids.iter().copied().collect();
                storage.save_incremental(&app.notebook, &dirty, &app.deleted_note_paths)
            };
            match result {
                Ok(assigned) => {
                    // Store back the path chosen for any newly-written note so it
                    // writes to the same file next time.
                    for (id, path) in assigned {
                        if let Some(n) = app.notebook.notes.get_mut(&id) {
                            n.file_path = Some(path.clone());
                        }
                        if app.current_note.as_ref().map(|n| n.id) == Some(id) {
                            if let Some(cn) = app.current_note.as_mut() {
                                cn.file_path = Some(path);
                            }
                        }
                    }
                    app.dirty_note_ids.clear();
                    app.deleted_note_paths.clear();
                    app.force_full_save = false;
                    app.pending_disk_save = false;
                    app.mark_disk_saved();
                }
                Err(e) => app.report_save_failure(e.to_string()),
            }
        }

        if app.should_quit {
            break Ok(());
        }
    };

    // Save notebook data before exiting
    if let Err(e) = storage.save_notebook(&app.notebook) {
        eprintln!("Failed to save notebook data: {}", e);
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    result
}

fn print_help() {
    println!("{}  v{}\n", PKG_NAME, VERSION);
    println!("A powerful terminal-based note-taking app with folder organization,");
    println!("markdown support, and syntax highlighting.\n");
    println!("USAGE:");
    println!("    {}                    Start the application", PKG_NAME);
    println!("    {} --vault <path>      Start with Obsidian vault at <path>", PKG_NAME);
    println!("    {} --version           Show version information", PKG_NAME);
    println!("    {} --help              Show this help message\n", PKG_NAME);
    println!("FEATURES:");
    println!("  • 📝 Rich markdown editing with live preview");
    println!("  • 🗂️  Hierarchical folder organization");
    println!("  • 🔍 Fuzzy search and quick jump navigation");
    println!("  • 🔗 Wiki-style [[note linking]]");
    println!("  • ⚡ Auto-save and intelligent autocompletion");
    println!("  • 🎨 Beautiful Tokyo Night theme");
    println!("  • 🚀 Vim-inspired keybindings\n");
    println!("Once started, press '?' for in-app help.");
}
