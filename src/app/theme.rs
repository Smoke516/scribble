use super::*;

impl App {
    pub fn change_theme(&mut self, theme_name: &str) {
        self.theme_manager.set_theme(theme_name);
        
        // Update and save config
        self.config.ui.theme = theme_name.to_string();
        if let Err(e) = self.config.save() {
            self.set_operation_error(
                format!("Theme changed but failed to save config: {}", e),
                Some("⚠️".to_string())
            );
        } else {
            self.set_operation_info(
                format!("Theme changed to: {}", theme_name),
                Some("🎨".to_string())
            );
        }
    }
    
    pub fn get_available_themes() -> Vec<&'static str> {
        use crate::theme::ThemeType;
        ThemeType::available_themes()
    }
    
    pub fn current_theme_name(&self) -> &'static str {
        self.theme_manager.current_theme().to_string()
    }
    
    pub fn show_theme_browser(&mut self) {
        self.mode = AppMode::ThemeBrowser;
        self.theme_browser_selected = 0;
        // Find current theme in list
        let current = self.current_theme_name();
        let themes = Self::get_available_themes();
        if let Some(pos) = themes.iter().position(|&t| t == current) {
            self.theme_browser_selected = pos;
        }
        self.set_message("Opened theme browser - use arrow keys to navigate, Enter to select".to_string());
    }
    
    pub fn navigate_theme_browser(&mut self, direction: i32) {
        let themes = Self::get_available_themes();
        let max_index = themes.len().saturating_sub(1);
        
        if direction < 0 && self.theme_browser_selected > 0 {
            self.theme_browser_selected -= 1;
        } else if direction > 0 && self.theme_browser_selected < max_index {
            self.theme_browser_selected += 1;
        }
    }
    
    pub fn select_theme_from_browser(&mut self) {
        let themes = Self::get_available_themes();
        if let Some(&theme_name) = themes.get(self.theme_browser_selected) {
            self.change_theme(theme_name);
            self.mode = AppMode::Normal;
        }
    }
    
    pub fn cancel_theme_browser(&mut self) {
        self.mode = AppMode::Normal;
    }

}
