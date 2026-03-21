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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub default: Option<String>,
    pub helix_integration: bool,
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
    pub preview_width: u16,
    pub show_line_numbers: bool,
    pub relative_line_numbers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    pub auto_save: bool,
    pub follow_links_in_new_pane: bool,
    pub backup_on_import: bool,
    pub file_watching: bool,
    pub spell_check: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            default: None,
            helix_integration: true,
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
            follow_links_in_new_pane: false,
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
        } else if let Some(home_dir) = dirs::home_dir() {
            Some(home_dir.join(".config").join("scribble").join("config.toml"))
        } else {
            None
        }
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