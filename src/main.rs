mod app;
mod autocomplete;
mod capture;
mod config;
mod error;
mod events;
mod models;
mod palette;
mod preview;
mod search;
mod spell;
mod storage;
mod syntax;
mod tags;
mod tasks;
mod theme;
mod ui;
mod vim;
mod watcher;

use app::App;
use std::env;
use std::io::IsTerminal;
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

/// Put the terminal back the way we found it. Best-effort and idempotent: it runs
/// from the panic hook as well as the normal exit path, and a failure here must
/// not mask whatever error is already unwinding.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let mut config = config::Config::load();
    
    // Handle command line arguments
    let args: Vec<String> = env::args().collect();
    let mut vault_path: Option<PathBuf> = None;
    
    // Capture flags are collected rather than acted on here, because they need the
    // vault, and the vault may be named by a --vault that comes after them.
    let mut new_note = false;
    let mut today = false;
    let mut capture_text: Option<String> = None;

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
            "--new" | "-n" => new_note = true,
            "--today" | "-t" => today = true,
            // The capture text is a plain positional, so it reads the same wherever
            // it lands: `-n "..."`, `-n -t "..."` and `-t "..."` all mean what they
            // look like. Omitting it entirely falls back to stdin, so
            // `git log -1 | scribble -n` composes.
            text if (new_note || today) && capture_text.is_none() && !text.starts_with('-') => {
                capture_text = Some(text.to_string());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Where a capture goes, resolved the same way the TUI resolves its vault.
    let capture_vault = || {
        vault_path
            .clone()
            .or_else(|| if config.vaults.auto_detect { detect_obsidian_vault() } else { None })
            .or_else(|| config.vaults.default.clone())
    };

    // `--today` alone opens the TUI; every other capture form is headless. Deciding
    // here keeps `scribble -t` (open today) and `scribble -t "..."` (append to
    // today) from needing different flags for what is obviously the same note.
    let headless = capture_text.is_some() || (new_note && !std::io::stdin().is_terminal());

    // Headless capture: write the note, print where it went, and never touch the
    // terminal. Capturing a thought should cost less than opening an editor.
    if headless {
        let Some(vault) = capture_vault() else {
            eprintln!("{}", capture::CaptureError::NoVault);
            std::process::exit(1);
        };
        let result = capture::resolve_text(capture_text).and_then(|text| {
            if today {
                capture::today_note(&vault, &config, Some(&text))
            } else {
                capture::new_note(&vault, &text)
            }
        });
        match result {
            Ok(path) => {
                println!("{}", path.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    // `-n` with nothing to capture and nothing piped in is a mistake, not a request
    // to launch the app — say so rather than silently ignoring the flag.
    if new_note {
        eprintln!("{}", capture::CaptureError::NoText);
        std::process::exit(1);
    }

    // `--today` on its own has nothing to capture, so it opens the TUI on today's
    // note instead — created first, so the app finds it during its normal load.
    let mut open_at: Option<PathBuf> = None;
    if today {
        let Some(vault) = capture_vault() else {
            eprintln!("{}", capture::CaptureError::NoVault);
            std::process::exit(1);
        };
        match capture::today_note(&vault, &config, None) {
            Ok(path) => open_at = Some(path),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }


    // A panic must not strand the user in raw mode on the alternate screen with no
    // echo and no line editing. Restore first, then chain to the default hook so
    // the message and backtrace land on a terminal that can actually show them.
    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_panic_hook(info);
    }));

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
    let mut storage: Box<dyn storage::NotebookStorage> = if let Some(vault_path) = final_vault_path.clone() {
        // `behavior.file_watching` was never consulted, so the watcher could not be
        // turned off however the config was written.
        if config.behavior.file_watching {
            app.initialize_file_watcher(vault_path.clone());
        }
        
        // Update recent vaults in config
        config.add_recent_vault(vault_path.clone());
        if let Err(e) = config.save() {
            eprintln!("Warning: Failed to save config: {}", e);
        }
        
        app.vault_path = Some(vault_path.clone());
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
            // `--today` asked for a specific note, so land on it rather than on the
            // welcome page. It was created before the load, so it is already here.
            let today_id = open_at.as_ref().and_then(|path| {
                app.notebook
                    .notes
                    .values()
                    .find(|n| n.file_path.as_deref() == Some(path.as_path()))
                    .map(|n| n.id)
            });
            match today_id {
                Some(id) => app.open_note_by_id(id),
                // Start with no note selected - show welcome page instead
                None => app.set_welcome_message(),
            }
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
            if let Err(e) = terminal.clear() {
                break Err(e.into());
            }
        }

        // Draw UI. Every fallible call in this loop must `break Err(..)` rather
        // than `?`: returning straight out of main would skip the terminal
        // restore below and leave the shell in raw mode.
        if let Err(e) = terminal.draw(|f| ui::draw(f, &mut app)) {
            break Err(e.into());
        }

        // Handle events
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        let event_ready = match crossterm::event::poll(timeout) {
            Ok(ready) => ready,
            Err(e) => break Err(e.into()),
        };

        if event_ready {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if let Err(e) = events::handle_event(&mut app, Event::Key(key)) {
                        break Err(e);
                    }
                }
                Ok(Event::Paste(text)) => {
                    if let Err(e) = events::handle_paste(&mut app, &text) {
                        break Err(e);
                    }
                }
                Ok(_) => {}
                Err(e) => break Err(e.into()),
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_visual_feedback();
            // Poll for file changes if file watcher is enabled
            app.poll_file_changes();
            last_tick = Instant::now();
        }

        // Apply queued folder moves/renames on disk first (rename the directory
        // and its files), remapping the in-memory note paths to the new location.
        if !app.disk.pending_folder_relocations.is_empty() {
            let relocations = std::mem::take(&mut app.disk.pending_folder_relocations);
            for (old_rel, new_rel) in relocations {
                match storage.relocate_folder(&app.notebook, &old_rel, &new_rel) {
                    Ok(updated) => {
                        for (id, new_path) in updated {
                            if let Some(n) = app.notebook.notes.get_mut(&id) {
                                n.file_path = Some(new_path.clone());
                            }
                            if app.current_note.as_ref().map(|n| n.id) == Some(id) {
                                if let Some(cn) = app.current_note.as_mut() {
                                    cn.file_path = Some(new_path);
                                }
                            }
                        }
                        app.mark_disk_saved();
                    }
                    Err(e) => app.report_save_failure(e.to_string()),
                }
            }
        }

        // Persist pending changes to disk as they happen (autosave, explicit
        // save, structural edits) so work survives a crash — not only on exit.
        // Writes only the changed notes (and removes deleted files); folder-tree
        // changes fall back to a full save.
        if app.disk.pending_disk_save {
            let result = if app.disk.force_full_save {
                storage
                    .save_notebook(&app.notebook)
                    .map(|_| storage::SaveReport::default())
            } else {
                let dirty: Vec<_> = app.disk.dirty_note_ids.iter().copied().collect();
                storage.save_incremental(&app.notebook, &dirty, &app.disk.deleted_note_paths)
            };
            match result {
                Ok(report) => {
                    app.apply_save_report(report);
                    app.disk.clear_after_write();
                    app.mark_disk_saved();
                }
                Err(e) => app.report_save_failure(e.to_string()),
            }
        }

        // Hand the note to the external editor. After the save block, so the file
        // it opens is what was on screen rather than the last flushed version.
        if app.disk.pending_external_edit {
            app.disk.pending_external_edit = false;

            let target = app
                .current_note
                .as_ref()
                .and_then(|n| n.file_path.clone().map(|p| (n.id, p)));

            match (target, app.external_editor.clone()) {
                (Some((id, path)), Some(editor)) => {
                    // The real file in the vault, not a copy in /tmp. The moment
                    // you reach for a full editor is the moment you want its file
                    // tree, its git status and its search to see where the note
                    // actually lives — and its frontmatter, so tags are editable
                    // there too.
                    match app::run_external_editor(&editor, &path) {
                        // Reload rather than trusting our buffer: the file may have
                        // gained frontmatter edits, and the note's disk stamp has to
                        // match what is now on disk or the next save would treat our
                        // own handoff as somebody else's write.
                        Ok(()) => app.reload_note_from_disk(id, &path),
                        Err(e) => app.set_message(e),
                    }
                    app.just_returned_from_editor = true;
                }
                (None, _) => app.set_message(
                    "This note has no file yet — save it before editing externally".to_string(),
                ),
                (_, None) => app.set_message("No external editor configured".to_string()),
            }
        }

        // Switch vaults. This runs after the save block on purpose: whatever is
        // still owed goes to the vault we are leaving, not the one we are joining.
        if let Some(target) = app.disk.pending_vault_switch.take() {
            // The save block above only runs when a write is pending, and a
            // force_full_save can leave work behind, so flush explicitly rather
            // than assuming. Losing edits to the old vault would be the worst
            // possible outcome of changing which vault you are looking at.
            if app.disk.has_pending_work() {
                let dirty: Vec<_> = app.disk.dirty_note_ids.iter().copied().collect();
                let flushed = if app.disk.force_full_save {
                    storage
                        .save_notebook(&app.notebook)
                        .map(|_| storage::SaveReport::default())
                } else {
                    storage.save_incremental(&app.notebook, &dirty, &app.disk.deleted_note_paths)
                };
                if let Err(e) = flushed {
                    app.report_save_failure(format!("{} — vault not switched", e));
                    continue;
                }
            }

            // Build the new storage before giving anything up: a vault that has
            // been unmounted or deleted should leave you where you were, with a
            // message, rather than in a half-switched state.
            use storage::NotebookStorage as _;
            match storage::VaultStorage::new(target.clone())
                .and_then(|s| s.load_notebook().map(|nb| (s, nb)))
            {
                Ok((new_storage, notebook)) => {
                    storage = Box::new(new_storage);
                    app.initialize_file_watcher(target.clone());
                    app.adopt_vault(target.clone(), notebook);

                    // Remember it, so the next plain `scribble` opens this vault.
                    // Nothing wrote `vaults.default` before, which is why picking a
                    // vault and restarting appeared to do nothing at all.
                    config.vaults.default = Some(target.clone());
                    config.add_recent_vault(target);
                    app.initialize_available_vaults(&config);
                    if let Err(e) = config.save() {
                        app.set_message(format!("Switched, but could not save config: {}", e));
                    }
                }
                Err(e) => app.set_message(format!("Could not open that vault: {}", e)),
            }
        }

        if app.should_quit {
            break Ok(());
        }
    };

    // Save anything the loop did not manage to flush. The loop persists as you
    // type, so in the normal case there is nothing left to do here — and an
    // unconditional full save would rewrite every note in the vault, bumping
    // every mtime and re-uploading the lot to whatever is syncing it. Only the
    // outstanding work is written, and only when there is some (a save that
    // failed mid-session leaves `pending_disk_save` set for exactly this retry).
    if app.disk.has_pending_work() {
        let dirty: Vec<_> = app.disk.dirty_note_ids.iter().copied().collect();
        let outcome = if app.disk.force_full_save {
            storage
                .save_notebook(&app.notebook)
                .map(|_| storage::SaveReport::default())
        } else {
            storage.save_incremental(&app.notebook, &dirty, &app.disk.deleted_note_paths)
        };
        match outcome {
            // The app is already gone by now, so there is no status line to report a
            // conflict to. Say it on stdout instead: a preserved file the user never
            // hears about is a file they will not think to look for.
            Ok(report) => {
                for conflict in report.conflicts {
                    eprintln!(
                        "'{}' had changed on disk; the version found there was kept as {}",
                        conflict.note_title,
                        conflict.preserved_at.display()
                    );
                }
            }
            Err(e) => eprintln!("Failed to save notebook data: {}", e),
        }
    }

    // Restore terminal. Best-effort for the same reason as the panic hook: if
    // disabling raw mode fails we still want to leave the alternate screen, so
    // none of these steps may short-circuit the ones after it.
    restore_terminal();
    let _ = terminal.show_cursor();

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
    println!("QUICK CAPTURE (no TUI; prints the path it wrote):");
    println!("    {} -n \"buy milk\"       Create a note, titled from its first line", PKG_NAME);
    println!("    {} -t \"buy milk\"       Append an entry to today's daily note", PKG_NAME);
    println!("    {} -t                  Open today's daily note in the app", PKG_NAME);
    println!("    ... | {} -n            Capture piped stdin\n", PKG_NAME);
    println!("FEATURES:");
    println!("  • 📝 Rich markdown editing with live preview");
    println!("  • 🗂️  Hierarchical folder organization");
    println!("  • 🔍 Fuzzy search and quick jump navigation");
    println!("  • ⚡ Auto-save and intelligent autocompletion");
    println!("  • 🎨 Beautiful Tokyo Night theme");
    println!("  • 🚀 Vim-inspired keybindings\n");
    println!("Once started, press '?' for in-app help.");
}
