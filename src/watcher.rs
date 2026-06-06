use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    #[allow(dead_code)]  // TODO: rename detection not implemented
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
                                path.extension().is_some_and(|ext| ext == "md") &&
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
                                relevant_paths.first().map(|path| FileChangeEvent::Created(path.clone()))
                            },
                            EventKind::Modify(_) => {
                                relevant_paths.first().map(|path| FileChangeEvent::Modified(path.clone()))
                            },
                            EventKind::Remove(_) => {
                                relevant_paths.first().map(|path| FileChangeEvent::Deleted(path.clone()))
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
    
}