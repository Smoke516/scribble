#![allow(dead_code)] // palette/API surface; some helpers kept for completeness
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub editor: EditorConfig,
    pub vaults: VaultConfig,
    pub ui: UiConfig,
    pub behavior: BehaviorConfig,
    pub capture: CaptureConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultConfig {
    pub default: Option<PathBuf>,
    pub recent: Vec<PathBuf>,
    pub auto_detect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    /// Show the folder tree beside the editor. Off by default: the tree is
    /// available as an overlay on Ctrl+E, and a permanent sidebar competes with
    /// the editor for attention. Set true to pin it back.
    pub show_sidebar: bool,
    pub preview_width: u16,
    pub show_line_numbers: bool,
    pub relative_line_numbers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    pub auto_save: bool,
    pub backup_on_import: bool,
    pub file_watching: bool,
    pub spell_check: bool,
}

/// Settings for `--new` and `--today`, the command-line capture paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    /// Vault-relative folder for *new* daily notes. Empty — the default — means the
    /// vault root, which is where F4 has always put them. Existing daily notes are
    /// found by title wherever they already live, so changing this never orphans
    /// the ones you already have.
    pub daily_folder: String,
    /// strftime pattern for a daily note's title, resolved against *local* time —
    /// a daily note that rolls over at UTC midnight is the wrong day's note for
    /// most of the world.
    pub daily_format: String,
    /// Prefix each captured entry with the time it was captured. In a daily log,
    /// when an entry was written is usually half of what it means.
    pub timestamp_entries: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            daily_folder: String::new(),
            daily_format: "%Y-%m-%d".to_string(),
            timestamp_entries: true,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            default: None,
        }
    }
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            default: None,
            recent: Vec::new(),
            auto_detect: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "tokyo-night".to_string(),
            show_sidebar: false,
            preview_width: 50,
            show_line_numbers: true,
            relative_line_numbers: false,
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            auto_save: true,
            backup_on_import: true,
            file_watching: true,
            spell_check: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Some(config_path) = Self::config_file_path() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<Config>(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_path) = Self::config_file_path() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(self)?;
            fs::write(config_path, content)?;
        }
        Ok(())
    }

    pub fn config_file_path() -> Option<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            Some(config_dir.join("scribble").join("config.toml"))
        } else { dirs::home_dir().map(|home_dir| home_dir.join(".config").join("scribble").join("config.toml")) }
    }

    pub fn add_recent_vault(&mut self, vault_path: PathBuf) {
        // Remove if already exists
        self.vaults.recent.retain(|p| p != &vault_path);
        
        // Add to front
        self.vaults.recent.insert(0, vault_path);
        
        // Keep only last 10
        if self.vaults.recent.len() > 10 {
            self.vaults.recent.truncate(10);
        }
    }

    pub fn get_editor(&self) -> Option<String> {
        self.editor.default.clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .or_else(|| {
                // Try to detect common editors
                for editor in &["hx", "helix", "nvim", "vim", "nano"] {
                    if which::which(editor).is_ok() {
                        return Some(editor.to_string());
                    }
                }
                None
            })
    }
}

// Simple which implementation to avoid extra dependency
mod which {
    use std::path::PathBuf;
    use std::env;

    pub fn which(program: &str) -> Result<PathBuf, ()> {
        if let Ok(path) = env::var("PATH") {
            for dir in path.split(':') {
                let full_path = PathBuf::from(dir).join(program);
                if full_path.exists() && full_path.is_file() {
                    return Ok(full_path);
                }
            }
        }
        Err(())
    }
}
#[cfg(test)]
mod tests {
    /// Every setting must be read by something.
    ///
    /// Eight of them were not: the config promised control over the editor, the
    /// preview width, autosave, file watching, spell check and import backups, and
    /// changing any of them did nothing at all. A setting that is ignored is worse
    /// than one that is absent, because you believe you turned something off.
    ///
    /// Source-scanning is crude, but it is the only check that fails when someone
    /// adds a field and forgets to wire it up — which is exactly how these got here.
    #[test]
    fn every_config_field_is_read_somewhere() {
        const SOURCES: [(&str, &str); 9] = [
            ("config.rs", include_str!("config.rs")),
            ("main.rs", include_str!("main.rs")),
            ("ui.rs", include_str!("ui.rs")),
            ("events.rs", include_str!("events.rs")),
            ("capture.rs", include_str!("capture.rs")),
            ("app/mod.rs", include_str!("app/mod.rs")),
            ("app/io.rs", include_str!("app/io.rs")),
            ("app/view.rs", include_str!("app/view.rs")),
            ("app/tags.rs", include_str!("app/tags.rs")),
        ];

        // Field names as they appear in a read: `.field`.
        let fields = [
            "default", "auto_detect", "recent", "theme", "show_sidebar", "preview_width",
            "show_line_numbers", "relative_line_numbers", "auto_save", "backup_on_import",
            "file_watching", "spell_check", "daily_folder", "daily_format", "timestamp_entries",
        ];

        // Comments must not count as reads. The first version of this test was
        // satisfied by the doc comment sitting above the code it described, which
        // made it pass while the setting was genuinely unused.
        let code: Vec<String> = SOURCES
            .iter()
            .filter(|(name, _)| *name != "config.rs")
            .map(|(_, src)| {
                src.lines()
                    .map(|l| match l.find("//") {
                        Some(i) => &l[..i],
                        None => l,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect();

        let mut dead = Vec::new();
        for field in fields {
            let needle = format!(".{}", field);
            if !code.iter().any(|src| src.contains(&needle)) {
                dead.push(field);
            }
        }

        assert!(
            dead.is_empty(),
            "config settings that nothing reads, so changing them does nothing: {:?}",
            dead
        );
    }
}
