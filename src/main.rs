mod app;
mod events;
mod models;
mod preview;
mod search;
mod storage;
mod syntax;
mod theme;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::stdout,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load data
    let mut app = App::new();
    let storage = storage::Storage::new()?;
    
    // Load existing notebook data
    match storage.load_notebook() {
        Ok(notebook) => {
            app.notebook = notebook;
            app.refresh_tree_view();
            app.set_message(format!("Loaded {} notes across {} folders", 
                app.notebook.notes.len(), app.notebook.folders.len()));
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
            if let Event::Key(key) = event::read()? {
                if let Err(e) = events::handle_event(&mut app, Event::Key(key)) {
                    break Err(e.into());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_visual_feedback();
            last_tick = Instant::now();
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
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
