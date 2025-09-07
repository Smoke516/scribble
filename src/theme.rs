use ratatui::style::{Color, Style, Modifier};

/// Tokyo Night color palette
pub struct TokyoNightTheme;

impl TokyoNightTheme {
    // Background colors
    pub const BG: Color = Color::Rgb(26, 27, 38);           // #1a1b26 - main background
    pub const BG_DARK: Color = Color::Rgb(22, 22, 30);     // #16161e - darker background
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66); // #292e42 - selection/highlight
    #[allow(dead_code)]
    pub const BG_FLOAT: Color = Color::Rgb(22, 22, 30);    // #16161e - floating panels
    pub const BG_POPUP: Color = Color::Rgb(22, 22, 30);    // #16161e - popups/dialogs
    
    // Foreground colors
    pub const FG: Color = Color::Rgb(192, 202, 245);       // #c0caf5 - main text
    pub const FG_DARK: Color = Color::Rgb(169, 177, 214);  // #a9b1d6 - secondary text
    pub const FG_GUTTER: Color = Color::Rgb(54, 65, 77);   // #363a4d - line numbers, etc.
    
    // Accent colors
    pub const BLUE: Color = Color::Rgb(125, 207, 255);     // #7dcfff - info, links
    pub const CYAN: Color = Color::Rgb(125, 207, 255);     // #7dcfff - cyan accent
    pub const GREEN: Color = Color::Rgb(154, 230, 180);    // #9ece6a - success, strings
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);   // #e0af68 - warnings
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);   // #ff9e64 - keywords
    pub const RED: Color = Color::Rgb(247, 118, 142);      // #f7768e - errors, deletion
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);   // #bb9af7 - constants, purple
    #[allow(dead_code)]
    pub const MAGENTA: Color = Color::Rgb(255, 117, 181);  // #ff75b5 - magenta accent
    
    // Special colors
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);    // #565f89 - comments
    #[allow(dead_code)]
    pub const SELECTION: Color = Color::Rgb(54, 65, 77);   // #363a4d - selections
    pub const BORDER: Color = Color::Rgb(41, 46, 66);      // #292e42 - borders
    pub const BORDER_HIGHLIGHT: Color = Color::Rgb(125, 207, 255); // #7dcfff - active borders
    
    // Git colors
    #[allow(dead_code)]
    pub const GIT_ADD: Color = Color::Rgb(154, 230, 180);  // #9ece6a - additions
    #[allow(dead_code)]
    pub const GIT_CHANGE: Color = Color::Rgb(224, 175, 104); // #e0af68 - changes
    #[allow(dead_code)]
    pub const GIT_DELETE: Color = Color::Rgb(247, 118, 142); // #f7768e - deletions
}

impl TokyoNightTheme {
    /// Get style for normal text with transparent background
    pub fn normal() -> Style {
        Style::default().fg(Self::FG) // No background = transparent
    }
    
    /// Get style for normal text with solid background (for opaque mode)
    #[allow(dead_code)]
    pub fn normal_opaque() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG)
    }

    /// Get style for selected items
    pub fn selected() -> Style {
        Style::default()
            .fg(Self::FG)
            .bg(Self::BG_HIGHLIGHT)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for focused borders
    pub fn border_focused() -> Style {
        Style::default().fg(Self::BORDER_HIGHLIGHT)
    }

    /// Get style for inactive borders
    pub fn border_inactive() -> Style {
        Style::default().fg(Self::BORDER)
    }

    /// Get style for popup/dialog backgrounds
    pub fn popup() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_POPUP) // Keep solid background for readability
    }

    /// Get style for status bar
    pub fn status_bar() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_DARK)
    }

    /// Get style for mode indicators
    pub fn mode_normal() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::BLUE)
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_insert() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::GREEN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_search() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::YELLOW)
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_command() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::PURPLE)
            .add_modifier(Modifier::BOLD)
    }

    pub fn mode_input() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::CYAN)
            .add_modifier(Modifier::BOLD)
    }

    /// Markdown syntax highlighting styles
    pub fn markdown_h1() -> Style {
        Style::default()
            .fg(Self::CYAN)
            .add_modifier(Modifier::BOLD)
    }

    pub fn markdown_h2() -> Style {
        Style::default()
            .fg(Self::BLUE)
            .add_modifier(Modifier::BOLD)
    }

    pub fn markdown_h3() -> Style {
        Style::default()
            .fg(Self::PURPLE)
            .add_modifier(Modifier::BOLD)
    }

    pub fn markdown_list() -> Style {
        Style::default().fg(Self::GREEN)
    }

    pub fn markdown_quote() -> Style {
        Style::default()
            .fg(Self::COMMENT)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn markdown_code() -> Style {
        Style::default()
            .fg(Self::ORANGE)
            .bg(Self::BG_DARK)
    }

    pub fn markdown_code_block() -> Style {
        Style::default()
            .fg(Self::FG)
            .bg(Self::BG_DARK)
    }

    pub fn markdown_bold() -> Style {
        Style::default()
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn markdown_italic() -> Style {
        Style::default()
            .fg(Self::FG_DARK)
            .add_modifier(Modifier::ITALIC)
    }

    #[allow(dead_code)]
    pub fn markdown_link() -> Style {
        Style::default()
            .fg(Self::BLUE)
            .add_modifier(Modifier::UNDERLINED)
    }

    /// File tree icons and styling
    pub fn folder_icon() -> Style {
        Style::default().fg(Self::BLUE)
    }

    pub fn folder_expanded_icon() -> Style {
        Style::default().fg(Self::CYAN)
    }

    pub fn note_icon() -> Style {
        Style::default().fg(Self::GREEN)
    }

    /// Helper text and placeholders
    pub fn placeholder() -> Style {
        Style::default().fg(Self::COMMENT)
    }

    pub fn help_text() -> Style {
        Style::default().fg(Self::FG_DARK)
    }

    /// Success and error messages
    #[allow(dead_code)]
    pub fn success() -> Style {
        Style::default().fg(Self::GREEN)
    }

    #[allow(dead_code)]
    pub fn warning() -> Style {
        Style::default().fg(Self::YELLOW)
    }

    #[allow(dead_code)]
    pub fn error() -> Style {
        Style::default().fg(Self::RED)
    }

    /// Tree indentation guide
    #[allow(dead_code)]
    pub fn tree_guide() -> Style {
        Style::default().fg(Self::FG_GUTTER)
    }

    /// Welcome screen accent
    pub fn welcome_accent() -> Style {
        Style::default()
            .fg(Self::CYAN)
            .add_modifier(Modifier::BOLD)
    }

    /// Scrollbar
    #[allow(dead_code)]
    pub fn scrollbar() -> Style {
        Style::default().fg(Self::FG_GUTTER).bg(Self::BG_DARK)
    }

    /// Search results highlight
    #[allow(dead_code)]
    pub fn search_match() -> Style {
        Style::default()
            .fg(Self::BG)
            .bg(Self::YELLOW)
            .add_modifier(Modifier::BOLD)
    }
}
