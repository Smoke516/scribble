use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed(PathBuf, PathBuf), // (from, to)
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<FileChangeEvent>,
}

impl FileWatcher {
    pub fn new(vault_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Filter for markdown files and ignore .obsidian directory changes
                        let relevant_paths: Vec<_> = event.paths.iter()
                            .filter(|path| {
                                // Only watch .md files
                                path.extension().map_or(false, |ext| ext == "md") &&
                                // Ignore .obsidian directory
                                !path.components().any(|comp| comp.as_os_str() == ".obsidian")
                            })
                            .cloned()
                            .collect();
                        
                        if relevant_paths.is_empty() {
                            return;
                        }
                        
                        let change_event = match event.kind {
                            EventKind::Create(_) => {
                                if let Some(path) = relevant_paths.first() {
                                    Some(FileChangeEvent::Created(path.clone()))
                                } else {
                                    None
                                }
                            },
                            EventKind::Modify(_) => {
                                if let Some(path) = relevant_paths.first() {
                                    Some(FileChangeEvent::Modified(path.clone()))
                                } else {
                                    None
                                }
                            },
                            EventKind::Remove(_) => {
                                if let Some(path) = relevant_paths.first() {
                                    Some(FileChangeEvent::Deleted(path.clone()))
                                } else {
                                    None
                                }
                            },
                            _ => None,
                        };
                        
                        if let Some(change) = change_event {
                            let _ = tx.send(change);
                        }
                    },
                    Err(e) => eprintln!("File watcher error: {:?}", e),
                }
            },
            Config::default(),
        )?;
        
        // Watch the vault directory
        watcher.watch(&vault_path, RecursiveMode::Recursive)?;
        
        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }
    
    pub fn poll_changes(&self) -> Vec<FileChangeEvent> {
        let mut changes = Vec::new();
        
        // Collect all available changes without blocking
        while let Ok(change) = self.receiver.try_recv() {
            changes.push(change);
        }
        
        changes
    }
    
    pub fn has_changes(&self) -> bool {
        match self.receiver.try_recv() {
            Ok(change) => {
                // Put the change back (we just wanted to check if there were any)
                // Note: This is a simplified approach. In a real implementation,
                // you might want to use a more sophisticated buffering system.
                true
            },
            Err(_) => false,
        }
    }
}

// Helper function to debounce file changes (common in file watchers)
pub struct ChangeDebouncer {
    pending_changes: std::collections::HashMap<PathBuf, (FileChangeEvent, std::time::Instant)>,
    debounce_duration: Duration,
}

impl ChangeDebouncer {
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            pending_changes: std::collections::HashMap::new(),
            debounce_duration: Duration::from_millis(debounce_ms),
        }
    }
    
    pub fn add_change(&mut self, event: FileChangeEvent) {
        let path = match &event {
            FileChangeEvent::Created(p) => p,
            FileChangeEvent::Modified(p) => p,
            FileChangeEvent::Deleted(p) => p,
            FileChangeEvent::Renamed(_, to) => to,
        };
        
        self.pending_changes.insert(path.clone(), (event, std::time::Instant::now()));
    }
    
    pub fn get_debounced_changes(&mut self) -> Vec<FileChangeEvent> {
        let now = std::time::Instant::now();
        let mut ready_changes = Vec::new();
        
        self.pending_changes.retain(|_path, (event, timestamp)| {
            if now.duration_since(*timestamp) >= self.debounce_duration {
                ready_changes.push(event.clone());
                false // remove from pending
            } else {
                true // keep in pending
            }
        });
        
        ready_changes
    }
}