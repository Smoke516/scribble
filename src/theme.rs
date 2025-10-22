use ratatui::style::{Color, Style, Modifier};

/// Theme trait for consistent styling across all themes
pub trait Theme {
    // Background colors
    fn bg() -> Color;
    fn bg_dark() -> Color;
    fn bg_highlight() -> Color;
    fn bg_popup() -> Color;
    
    // Foreground colors
    fn fg() -> Color;
    fn fg_dark() -> Color;
    fn fg_gutter() -> Color;
    
    // Accent colors
    fn blue() -> Color;
    fn cyan() -> Color;
    fn green() -> Color;
    fn yellow() -> Color;
    fn orange() -> Color;
    fn red() -> Color;
    fn purple() -> Color;
    fn magenta() -> Color;
    
    // Special colors
    fn comment() -> Color;
    fn border() -> Color;
    fn border_highlight() -> Color;
}

/// Available themes enum
#[derive(Debug, Clone)]
pub enum ThemeType {
    TokyoNight,
    GitHubDark,
    CatppuccinMocha,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    Dracula,
    Nord,
    OneDark,
    Gruvbox,
}

impl ThemeType {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tokyo-night" | "tokyonight" => Self::TokyoNight,
            "github-dark" | "githubdark" => Self::GitHubDark,
            "catppuccin-mocha" | "catppuccin_mocha" | "mocha" => Self::CatppuccinMocha,
            "catppuccin-latte" | "catppuccin_latte" | "latte" => Self::CatppuccinLatte,
            "catppuccin-frappe" | "catppuccin_frappe" | "frappe" => Self::CatppuccinFrappe,
            "catppuccin-macchiato" | "catppuccin_macchiato" | "macchiato" => Self::CatppuccinMacchiato,
            "dracula" => Self::Dracula,
            "nord" => Self::Nord,
            "one-dark" | "onedark" => Self::OneDark,
            "gruvbox" => Self::Gruvbox,
            _ => Self::TokyoNight, // Default fallback
        }
    }
    
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::TokyoNight => "tokyo-night",
            Self::GitHubDark => "github-dark",
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::CatppuccinLatte => "catppuccin-latte",
            Self::CatppuccinFrappe => "catppuccin-frappe",
            Self::CatppuccinMacchiato => "catppuccin-macchiato",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::OneDark => "one-dark",
            Self::Gruvbox => "gruvbox",
        }
    }
    
    pub fn available_themes() -> Vec<&'static str> {
        vec![
            "tokyo-night",
            "github-dark",
            "catppuccin-mocha",
            "catppuccin-latte",
            "catppuccin-frappe",
            "catppuccin-macchiato",
            "dracula",
            "nord",
            "one-dark",
            "gruvbox",
        ]
    }
}

/// Tokyo Night Theme
pub struct TokyoNightTheme;

impl Theme for TokyoNightTheme {
    fn bg() -> Color { Color::Rgb(26, 27, 38) }           // #1a1b26
    fn bg_dark() -> Color { Color::Rgb(22, 22, 30) }     // #16161e
    fn bg_highlight() -> Color { Color::Rgb(41, 46, 66) } // #292e42
    fn bg_popup() -> Color { Color::Rgb(22, 22, 30) }    // #16161e
    
    fn fg() -> Color { Color::Rgb(192, 202, 245) }       // #c0caf5
    fn fg_dark() -> Color { Color::Rgb(169, 177, 214) }  // #a9b1d6
    fn fg_gutter() -> Color { Color::Rgb(54, 65, 77) }   // #363a4d
    
    fn blue() -> Color { Color::Rgb(125, 207, 255) }     // #7dcfff
    fn cyan() -> Color { Color::Rgb(125, 207, 255) }     // #7dcfff
    fn green() -> Color { Color::Rgb(154, 230, 180) }    // #9ece6a
    fn yellow() -> Color { Color::Rgb(224, 175, 104) }   // #e0af68
    fn orange() -> Color { Color::Rgb(255, 158, 100) }   // #ff9e64
    fn red() -> Color { Color::Rgb(247, 118, 142) }      // #f7768e
    fn purple() -> Color { Color::Rgb(187, 154, 247) }   // #bb9af7
    fn magenta() -> Color { Color::Rgb(255, 117, 181) }  // #ff75b5
    
    fn comment() -> Color { Color::Rgb(86, 95, 137) }    // #565f89
    fn border() -> Color { Color::Rgb(41, 46, 66) }      // #292e42
    fn border_highlight() -> Color { Color::Rgb(125, 207, 255) } // #7dcfff
}

// Backward compatibility constants for existing code
impl TokyoNightTheme {
    pub const BG: Color = Color::Rgb(26, 27, 38);
    pub const BG_DARK: Color = Color::Rgb(22, 22, 30);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66);
    pub const BG_POPUP: Color = Color::Rgb(22, 22, 30);
    pub const FG: Color = Color::Rgb(192, 202, 245);
    pub const FG_DARK: Color = Color::Rgb(169, 177, 214);
    pub const FG_GUTTER: Color = Color::Rgb(54, 65, 77);
    pub const BLUE: Color = Color::Rgb(125, 207, 255);
    pub const CYAN: Color = Color::Rgb(125, 207, 255);
    pub const GREEN: Color = Color::Rgb(154, 230, 180);
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);
    pub const RED: Color = Color::Rgb(247, 118, 142);
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);
    pub const MAGENTA: Color = Color::Rgb(255, 117, 181);
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);
    pub const BORDER: Color = Color::Rgb(41, 46, 66);
    pub const BORDER_HIGHLIGHT: Color = Color::Rgb(125, 207, 255);
    
    // Backward compatibility methods for existing code
    pub fn normal() -> Style {
        Style::default().fg(Self::FG)
    }
    
    pub fn normal_opaque() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG)
    }
    
    pub fn selected() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_HIGHLIGHT).add_modifier(Modifier::BOLD)
    }
    
    pub fn border_focused() -> Style {
        Style::default().fg(Self::BORDER_HIGHLIGHT)
    }
    
    pub fn border_inactive() -> Style {
        Style::default().fg(Self::BORDER)
    }
    
    pub fn popup() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_POPUP)
    }
    
    pub fn status_bar() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_DARK)
    }
    
    pub fn mode_normal() -> Style {
        Style::default().fg(Self::BG).bg(Self::BLUE).add_modifier(Modifier::BOLD)
    }
    
    pub fn mode_insert() -> Style {
        Style::default().fg(Self::BG).bg(Self::GREEN).add_modifier(Modifier::BOLD)
    }
    
    pub fn mode_search() -> Style {
        Style::default().fg(Self::BG).bg(Self::YELLOW).add_modifier(Modifier::BOLD)
    }
    
    pub fn mode_command() -> Style {
        Style::default().fg(Self::BG).bg(Self::PURPLE).add_modifier(Modifier::BOLD)
    }
    
    pub fn mode_input() -> Style {
        Style::default().fg(Self::BG).bg(Self::CYAN).add_modifier(Modifier::BOLD)
    }
    
    pub fn markdown_h1() -> Style {
        Style::default().fg(Self::CYAN).add_modifier(Modifier::BOLD)
    }
    
    pub fn markdown_h2() -> Style {
        Style::default().fg(Self::BLUE).add_modifier(Modifier::BOLD)
    }
    
    pub fn markdown_h3() -> Style {
        Style::default().fg(Self::PURPLE).add_modifier(Modifier::BOLD)
    }
    
    pub fn markdown_list() -> Style {
        Style::default().fg(Self::GREEN)
    }
    
    pub fn markdown_quote() -> Style {
        Style::default().fg(Self::COMMENT).add_modifier(Modifier::ITALIC)
    }
    
    pub fn markdown_code() -> Style {
        Style::default().fg(Self::ORANGE).bg(Self::BG_DARK)
    }
    
    pub fn markdown_code_block() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_DARK)
    }
    
    pub fn markdown_bold() -> Style {
        Style::default().fg(Self::FG).add_modifier(Modifier::BOLD)
    }
    
    pub fn markdown_italic() -> Style {
        Style::default().fg(Self::FG_DARK).add_modifier(Modifier::ITALIC)
    }
    
    pub fn markdown_link() -> Style {
        Style::default().fg(Self::BLUE).add_modifier(Modifier::UNDERLINED)
    }
    
    pub fn folder_icon() -> Style {
        Style::default().fg(Self::BLUE)
    }
    
    pub fn folder_expanded_icon() -> Style {
        Style::default().fg(Self::CYAN)
    }
    
    pub fn note_icon() -> Style {
        Style::default().fg(Self::GREEN)
    }
    
    pub fn placeholder() -> Style {
        Style::default().fg(Self::COMMENT)
    }
    
    pub fn help_text() -> Style {
        Style::default().fg(Self::FG_DARK)
    }
    
    pub fn success() -> Style {
        Style::default().fg(Self::GREEN)
    }
    
    pub fn warning() -> Style {
        Style::default().fg(Self::YELLOW)
    }
    
    pub fn error() -> Style {
        Style::default().fg(Self::RED)
    }
    
    pub fn tree_guide() -> Style {
        Style::default().fg(Self::FG_GUTTER)
    }
    
    pub fn welcome_accent() -> Style {
        Style::default().fg(Self::CYAN).add_modifier(Modifier::BOLD)
    }
    
    pub fn scrollbar() -> Style {
        Style::default().fg(Self::FG_GUTTER).bg(Self::BG_DARK)
    }
    
    pub fn search_match() -> Style {
        Style::default().fg(Self::BG).bg(Self::YELLOW).add_modifier(Modifier::BOLD)
    }
}

/// GitHub Dark Theme
pub struct GitHubDarkTheme;

impl Theme for GitHubDarkTheme {
    fn bg() -> Color { Color::Rgb(13, 17, 23) }           // #0d1117
    fn bg_dark() -> Color { Color::Rgb(22, 27, 34) }     // #161b22
    fn bg_highlight() -> Color { Color::Rgb(33, 38, 45) } // #21262d
    fn bg_popup() -> Color { Color::Rgb(22, 27, 34) }    // #161b22
    
    fn fg() -> Color { Color::Rgb(230, 237, 243) }       // #e6edf3
    fn fg_dark() -> Color { Color::Rgb(125, 133, 144) }  // #7d8590
    fn fg_gutter() -> Color { Color::Rgb(72, 78, 88) }   // #484f58
    
    fn blue() -> Color { Color::Rgb(88, 166, 255) }      // #58a6ff
    fn cyan() -> Color { Color::Rgb(57, 186, 230) }      // #39bae6
    fn green() -> Color { Color::Rgb(63, 185, 80) }      // #3fb950
    fn yellow() -> Color { Color::Rgb(219, 154, 47) }    // #db9a2f
    fn orange() -> Color { Color::Rgb(255, 135, 67) }    // #ff8743
    fn red() -> Color { Color::Rgb(255, 123, 114) }      // #ff7b72
    fn purple() -> Color { Color::Rgb(218, 120, 255) }   // #da78ff
    fn magenta() -> Color { Color::Rgb(242, 105, 201) }  // #f269c9
    
    fn comment() -> Color { Color::Rgb(125, 133, 144) }  // #7d8590
    fn border() -> Color { Color::Rgb(48, 54, 61) }      // #30363d
    fn border_highlight() -> Color { Color::Rgb(88, 166, 255) } // #58a6ff
}

/// Catppuccin Mocha Theme
pub struct CatppuccinMochaTheme;

impl Theme for CatppuccinMochaTheme {
    fn bg() -> Color { Color::Rgb(30, 30, 46) }           // #1e1e2e
    fn bg_dark() -> Color { Color::Rgb(24, 24, 37) }     // #181825
    fn bg_highlight() -> Color { Color::Rgb(49, 50, 68) } // #313244
    fn bg_popup() -> Color { Color::Rgb(24, 24, 37) }    // #181825
    
    fn fg() -> Color { Color::Rgb(205, 214, 244) }       // #cdd6f4
    fn fg_dark() -> Color { Color::Rgb(166, 173, 200) }  // #a6adc8
    fn fg_gutter() -> Color { Color::Rgb(108, 112, 134) } // #6c7086
    
    fn blue() -> Color { Color::Rgb(137, 180, 250) }     // #89b4fa
    fn cyan() -> Color { Color::Rgb(137, 220, 235) }     // #89dceb
    fn green() -> Color { Color::Rgb(166, 227, 161) }    // #a6e3a1
    fn yellow() -> Color { Color::Rgb(249, 226, 175) }   // #f9e2af
    fn orange() -> Color { Color::Rgb(250, 179, 135) }   // #fab387
    fn red() -> Color { Color::Rgb(243, 139, 168) }      // #f38ba8
    fn purple() -> Color { Color::Rgb(203, 166, 247) }   // #cba6f7
    fn magenta() -> Color { Color::Rgb(245, 194, 231) }  // #f5c2e7
    
    fn comment() -> Color { Color::Rgb(108, 112, 134) }  // #6c7086
    fn border() -> Color { Color::Rgb(49, 50, 68) }      // #313244
    fn border_highlight() -> Color { Color::Rgb(137, 180, 250) } // #89b4fa
}

/// Catppuccin Latte Theme (Light)
pub struct CatppuccinLatteTheme;

impl Theme for CatppuccinLatteTheme {
    fn bg() -> Color { Color::Rgb(239, 241, 245) }       // #eff1f5
    fn bg_dark() -> Color { Color::Rgb(230, 233, 239) } // #e6e9ef
    fn bg_highlight() -> Color { Color::Rgb(204, 208, 218) } // #ccd0da
    fn bg_popup() -> Color { Color::Rgb(230, 233, 239) } // #e6e9ef
    
    fn fg() -> Color { Color::Rgb(76, 79, 105) }         // #4c4f69
    fn fg_dark() -> Color { Color::Rgb(108, 111, 133) }  // #6c6f85
    fn fg_gutter() -> Color { Color::Rgb(156, 160, 176) } // #9ca0b0
    
    fn blue() -> Color { Color::Rgb(30, 102, 245) }      // #1e66f5
    fn cyan() -> Color { Color::Rgb(4, 165, 229) }       // #04a5e5
    fn green() -> Color { Color::Rgb(64, 160, 43) }      // #40a02b
    fn yellow() -> Color { Color::Rgb(223, 142, 29) }    // #df8e1d
    fn orange() -> Color { Color::Rgb(254, 100, 11) }    // #fe640b
    fn red() -> Color { Color::Rgb(210, 15, 57) }        // #d20f39
    fn purple() -> Color { Color::Rgb(136, 57, 239) }    // #8839ef
    fn magenta() -> Color { Color::Rgb(234, 118, 203) }  // #ea76cb
    
    fn comment() -> Color { Color::Rgb(156, 160, 176) }  // #9ca0b0
    fn border() -> Color { Color::Rgb(204, 208, 218) }   // #ccd0da
    fn border_highlight() -> Color { Color::Rgb(30, 102, 245) } // #1e66f5
}

/// Catppuccin Frappe Theme
pub struct CatppuccinFrappeTheme;

impl Theme for CatppuccinFrappeTheme {
    fn bg() -> Color { Color::Rgb(48, 52, 70) }           // #303446
    fn bg_dark() -> Color { Color::Rgb(41, 44, 60) }     // #292c3c
    fn bg_highlight() -> Color { Color::Rgb(65, 69, 89) } // #414559
    fn bg_popup() -> Color { Color::Rgb(41, 44, 60) }    // #292c3c
    
    fn fg() -> Color { Color::Rgb(198, 208, 245) }       // #c6d0f5
    fn fg_dark() -> Color { Color::Rgb(162, 173, 206) }  // #a2b3ce
    fn fg_gutter() -> Color { Color::Rgb(115, 121, 148) } // #737994
    
    fn blue() -> Color { Color::Rgb(140, 170, 238) }     // #8caaee
    fn cyan() -> Color { Color::Rgb(153, 209, 219) }     // #99d1db
    fn green() -> Color { Color::Rgb(166, 209, 137) }    // #a6d189
    fn yellow() -> Color { Color::Rgb(229, 200, 144) }   // #e5c890
    fn orange() -> Color { Color::Rgb(239, 159, 118) }   // #ef9f76
    fn red() -> Color { Color::Rgb(234, 153, 156) }      // #ea999c
    fn purple() -> Color { Color::Rgb(202, 158, 230) }   // #ca9ee6
    fn magenta() -> Color { Color::Rgb(244, 184, 228) }  // #f4b8e4
    
    fn comment() -> Color { Color::Rgb(115, 121, 148) }  // #737994
    fn border() -> Color { Color::Rgb(65, 69, 89) }      // #414559
    fn border_highlight() -> Color { Color::Rgb(140, 170, 238) } // #8caaee
}

/// Catppuccin Macchiato Theme
pub struct CatppuccinMacchiatoTheme;

impl Theme for CatppuccinMacchiatoTheme {
    fn bg() -> Color { Color::Rgb(36, 39, 58) }           // #24273a
    fn bg_dark() -> Color { Color::Rgb(30, 32, 48) }     // #1e2030
    fn bg_highlight() -> Color { Color::Rgb(54, 58, 79) } // #363a4f
    fn bg_popup() -> Color { Color::Rgb(30, 32, 48) }    // #1e2030
    
    fn fg() -> Color { Color::Rgb(202, 211, 245) }       // #cad3f5
    fn fg_dark() -> Color { Color::Rgb(165, 173, 203) }  // #a5adcb
    fn fg_gutter() -> Color { Color::Rgb(110, 115, 141) } // #6e738d
    
    fn blue() -> Color { Color::Rgb(138, 173, 244) }     // #8aadf4
    fn cyan() -> Color { Color::Rgb(145, 215, 227) }     // #91d7e3
    fn green() -> Color { Color::Rgb(166, 218, 149) }    // #a6da95
    fn yellow() -> Color { Color::Rgb(238, 212, 159) }   // #eed49f
    fn orange() -> Color { Color::Rgb(245, 169, 127) }   // #f5a97f
    fn red() -> Color { Color::Rgb(237, 135, 150) }      // #ed8796
    fn purple() -> Color { Color::Rgb(198, 160, 246) }   // #c6a0f6
    fn magenta() -> Color { Color::Rgb(245, 189, 230) }  // #f5bde6
    
    fn comment() -> Color { Color::Rgb(110, 115, 141) }  // #6e738d
    fn border() -> Color { Color::Rgb(54, 58, 79) }      // #363a4f
    fn border_highlight() -> Color { Color::Rgb(138, 173, 244) } // #8aadf4
}

/// Dracula Theme
pub struct DraculaTheme;

impl Theme for DraculaTheme {
    fn bg() -> Color { Color::Rgb(40, 42, 54) }           // #282a36
    fn bg_dark() -> Color { Color::Rgb(33, 34, 44) }     // #21222c
    fn bg_highlight() -> Color { Color::Rgb(68, 71, 90) } // #44475a
    fn bg_popup() -> Color { Color::Rgb(33, 34, 44) }    // #21222c
    
    fn fg() -> Color { Color::Rgb(248, 248, 242) }       // #f8f8f2
    fn fg_dark() -> Color { Color::Rgb(98, 114, 164) }   // #6272a4
    fn fg_gutter() -> Color { Color::Rgb(98, 114, 164) } // #6272a4
    
    fn blue() -> Color { Color::Rgb(139, 233, 253) }     // #8be9fd
    fn cyan() -> Color { Color::Rgb(139, 233, 253) }     // #8be9fd
    fn green() -> Color { Color::Rgb(80, 250, 123) }     // #50fa7b
    fn yellow() -> Color { Color::Rgb(241, 250, 140) }   // #f1fa8c
    fn orange() -> Color { Color::Rgb(255, 184, 108) }   // #ffb86c
    fn red() -> Color { Color::Rgb(255, 85, 85) }        // #ff5555
    fn purple() -> Color { Color::Rgb(189, 147, 249) }   // #bd93f9
    fn magenta() -> Color { Color::Rgb(255, 121, 198) }  // #ff79c6
    
    fn comment() -> Color { Color::Rgb(98, 114, 164) }   // #6272a4
    fn border() -> Color { Color::Rgb(68, 71, 90) }      // #44475a
    fn border_highlight() -> Color { Color::Rgb(139, 233, 253) } // #8be9fd
}

/// Nord Theme
pub struct NordTheme;

impl Theme for NordTheme {
    fn bg() -> Color { Color::Rgb(46, 52, 64) }           // #2e3440
    fn bg_dark() -> Color { Color::Rgb(59, 66, 82) }     // #3b4252
    fn bg_highlight() -> Color { Color::Rgb(67, 76, 94) } // #434c5e
    fn bg_popup() -> Color { Color::Rgb(59, 66, 82) }    // #3b4252
    
    fn fg() -> Color { Color::Rgb(236, 239, 244) }       // #eceff4
    fn fg_dark() -> Color { Color::Rgb(229, 233, 240) }  // #e5e9f0
    fn fg_gutter() -> Color { Color::Rgb(76, 86, 106) }  // #4c566a
    
    fn blue() -> Color { Color::Rgb(129, 161, 193) }     // #81a1c1
    fn cyan() -> Color { Color::Rgb(136, 192, 208) }     // #88c0d0
    fn green() -> Color { Color::Rgb(163, 190, 140) }    // #a3be8c
    fn yellow() -> Color { Color::Rgb(235, 203, 139) }   // #ebcb8b
    fn orange() -> Color { Color::Rgb(208, 135, 112) }   // #d08770
    fn red() -> Color { Color::Rgb(191, 97, 106) }       // #bf616a
    fn purple() -> Color { Color::Rgb(180, 142, 173) }   // #b48ead
    fn magenta() -> Color { Color::Rgb(180, 142, 173) }  // #b48ead
    
    fn comment() -> Color { Color::Rgb(76, 86, 106) }    // #4c566a
    fn border() -> Color { Color::Rgb(67, 76, 94) }      // #434c5e
    fn border_highlight() -> Color { Color::Rgb(129, 161, 193) } // #81a1c1
}

/// One Dark Theme
pub struct OneDarkTheme;

impl Theme for OneDarkTheme {
    fn bg() -> Color { Color::Rgb(40, 44, 52) }           // #282c34
    fn bg_dark() -> Color { Color::Rgb(33, 37, 43) }     // #21252b
    fn bg_highlight() -> Color { Color::Rgb(57, 63, 74) } // #393f4a
    fn bg_popup() -> Color { Color::Rgb(33, 37, 43) }    // #21252b
    
    fn fg() -> Color { Color::Rgb(171, 178, 191) }       // #abb2bf
    fn fg_dark() -> Color { Color::Rgb(92, 99, 112) }    // #5c6370
    fn fg_gutter() -> Color { Color::Rgb(73, 80, 96) }   // #495460
    
    fn blue() -> Color { Color::Rgb(97, 175, 239) }      // #61afef
    fn cyan() -> Color { Color::Rgb(86, 182, 194) }      // #56b6c2
    fn green() -> Color { Color::Rgb(152, 195, 121) }    // #98c379
    fn yellow() -> Color { Color::Rgb(229, 192, 123) }   // #e5c07b
    fn orange() -> Color { Color::Rgb(209, 154, 102) }   // #d19a66
    fn red() -> Color { Color::Rgb(224, 108, 117) }      // #e06c75
    fn purple() -> Color { Color::Rgb(198, 120, 221) }   // #c678dd
    fn magenta() -> Color { Color::Rgb(198, 120, 221) }  // #c678dd
    
    fn comment() -> Color { Color::Rgb(92, 99, 112) }    // #5c6370
    fn border() -> Color { Color::Rgb(57, 63, 74) }      // #393f4a
    fn border_highlight() -> Color { Color::Rgb(97, 175, 239) } // #61afef
}

/// Gruvbox Theme
pub struct GruvboxTheme;

impl Theme for GruvboxTheme {
    fn bg() -> Color { Color::Rgb(40, 40, 40) }           // #282828
    fn bg_dark() -> Color { Color::Rgb(29, 32, 33) }     // #1d2021
    fn bg_highlight() -> Color { Color::Rgb(60, 56, 54) } // #3c3836
    fn bg_popup() -> Color { Color::Rgb(29, 32, 33) }    // #1d2021
    
    fn fg() -> Color { Color::Rgb(235, 219, 178) }       // #ebdbb2
    fn fg_dark() -> Color { Color::Rgb(168, 153, 132) }  // #a89984
    fn fg_gutter() -> Color { Color::Rgb(146, 131, 116) } // #928374
    
    fn blue() -> Color { Color::Rgb(131, 165, 152) }     // #83a598
    fn cyan() -> Color { Color::Rgb(142, 192, 124) }     // #8ec07c
    fn green() -> Color { Color::Rgb(184, 187, 38) }     // #b8bb26
    fn yellow() -> Color { Color::Rgb(250, 189, 47) }    // #fabd2f
    fn orange() -> Color { Color::Rgb(254, 128, 25) }    // #fe8019
    fn red() -> Color { Color::Rgb(251, 73, 52) }        // #fb4934
    fn purple() -> Color { Color::Rgb(211, 134, 155) }   // #d3869b
    fn magenta() -> Color { Color::Rgb(211, 134, 155) }  // #d3869b
    
    fn comment() -> Color { Color::Rgb(146, 131, 116) }  // #928374
    fn border() -> Color { Color::Rgb(60, 56, 54) }      // #3c3836
    fn border_highlight() -> Color { Color::Rgb(131, 165, 152) } // #83a598
}

/// Main theme manager
pub struct ThemeManager {
    current_theme: ThemeType,
}

impl ThemeManager {
    pub fn new(theme_name: &str) -> Self {
        Self {
            current_theme: ThemeType::from_string(theme_name),
        }
    }
    
    pub fn set_theme(&mut self, theme_name: &str) {
        self.current_theme = ThemeType::from_string(theme_name);
    }
    
    pub fn current_theme(&self) -> &ThemeType {
        &self.current_theme
    }
    
    
    pub fn normal(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()),
        }
    }
    
    pub fn normal_opaque(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()).bg(TokyoNightTheme::bg()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()).bg(GitHubDarkTheme::bg()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()).bg(CatppuccinMochaTheme::bg()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()).bg(CatppuccinLatteTheme::bg()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()).bg(CatppuccinFrappeTheme::bg()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()).bg(CatppuccinMacchiatoTheme::bg()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()).bg(DraculaTheme::bg()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()).bg(NordTheme::bg()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()).bg(OneDarkTheme::bg()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()).bg(GruvboxTheme::bg()),
        }
    }

    pub fn selected(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()).bg(TokyoNightTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()).bg(GitHubDarkTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()).bg(CatppuccinMochaTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()).bg(CatppuccinLatteTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()).bg(CatppuccinFrappeTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()).bg(CatppuccinMacchiatoTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()).bg(DraculaTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()).bg(NordTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()).bg(OneDarkTheme::bg_highlight()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()).bg(GruvboxTheme::bg_highlight()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn border_focused(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::border_highlight()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::border_highlight()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::border_highlight()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::border_highlight()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::border_highlight()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::border_highlight()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::border_highlight()),
            ThemeType::Nord => Style::default().fg(NordTheme::border_highlight()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::border_highlight()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::border_highlight()),
        }
    }

    pub fn border_inactive(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::border()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::border()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::border()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::border()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::border()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::border()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::border()),
            ThemeType::Nord => Style::default().fg(NordTheme::border()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::border()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::border()),
        }
    }

    pub fn popup(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()).bg(TokyoNightTheme::bg_popup()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()).bg(GitHubDarkTheme::bg_popup()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()).bg(CatppuccinMochaTheme::bg_popup()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()).bg(CatppuccinLatteTheme::bg_popup()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()).bg(CatppuccinFrappeTheme::bg_popup()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()).bg(CatppuccinMacchiatoTheme::bg_popup()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()).bg(DraculaTheme::bg_popup()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()).bg(NordTheme::bg_popup()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()).bg(OneDarkTheme::bg_popup()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()).bg(GruvboxTheme::bg_popup()),
        }
    }

    pub fn status_bar(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()).bg(TokyoNightTheme::bg_dark()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()).bg(GitHubDarkTheme::bg_dark()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()).bg(CatppuccinMochaTheme::bg_dark()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()).bg(CatppuccinLatteTheme::bg_dark()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()).bg(CatppuccinFrappeTheme::bg_dark()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()).bg(CatppuccinMacchiatoTheme::bg_dark()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()).bg(DraculaTheme::bg_dark()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()).bg(NordTheme::bg_dark()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()).bg(OneDarkTheme::bg_dark()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()).bg(GruvboxTheme::bg_dark()),
        }
    }

    // Mode styles
    pub fn mode_normal(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::bg()).bg(TokyoNightTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::bg()).bg(GitHubDarkTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::bg()).bg(CatppuccinMochaTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::bg()).bg(CatppuccinLatteTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::bg()).bg(CatppuccinFrappeTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::bg()).bg(CatppuccinMacchiatoTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::bg()).bg(DraculaTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::bg()).bg(NordTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::bg()).bg(OneDarkTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::bg()).bg(GruvboxTheme::blue()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn mode_insert(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::bg()).bg(TokyoNightTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::bg()).bg(GitHubDarkTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::bg()).bg(CatppuccinMochaTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::bg()).bg(CatppuccinLatteTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::bg()).bg(CatppuccinFrappeTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::bg()).bg(CatppuccinMacchiatoTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::bg()).bg(DraculaTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::bg()).bg(NordTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::bg()).bg(OneDarkTheme::green()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::bg()).bg(GruvboxTheme::green()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn mode_search(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::bg()).bg(TokyoNightTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::bg()).bg(GitHubDarkTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::bg()).bg(CatppuccinMochaTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::bg()).bg(CatppuccinLatteTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::bg()).bg(CatppuccinFrappeTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::bg()).bg(CatppuccinMacchiatoTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::bg()).bg(DraculaTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::bg()).bg(NordTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::bg()).bg(OneDarkTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::bg()).bg(GruvboxTheme::yellow()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn mode_command(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::bg()).bg(TokyoNightTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::bg()).bg(GitHubDarkTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::bg()).bg(CatppuccinMochaTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::bg()).bg(CatppuccinLatteTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::bg()).bg(CatppuccinFrappeTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::bg()).bg(CatppuccinMacchiatoTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::bg()).bg(DraculaTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::bg()).bg(NordTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::bg()).bg(OneDarkTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::bg()).bg(GruvboxTheme::purple()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn mode_input(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::bg()).bg(TokyoNightTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::bg()).bg(GitHubDarkTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::bg()).bg(CatppuccinMochaTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::bg()).bg(CatppuccinLatteTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::bg()).bg(CatppuccinFrappeTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::bg()).bg(CatppuccinMacchiatoTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::bg()).bg(DraculaTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::bg()).bg(NordTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::bg()).bg(OneDarkTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::bg()).bg(GruvboxTheme::cyan()).add_modifier(Modifier::BOLD),
        }
    }

    // Markdown styles
    pub fn markdown_h1(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::cyan()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn markdown_h2(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::blue()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::blue()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn markdown_h3(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::purple()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::purple()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn markdown_list(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::green()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::green()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::green()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::green()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::green()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::green()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::green()),
            ThemeType::Nord => Style::default().fg(NordTheme::green()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::green()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::green()),
        }
    }

    pub fn markdown_quote(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::Nord => Style::default().fg(NordTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::comment()).add_modifier(Modifier::ITALIC),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::comment()).add_modifier(Modifier::ITALIC),
        }
    }

    pub fn markdown_code(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::orange()).bg(TokyoNightTheme::bg_dark()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::orange()).bg(GitHubDarkTheme::bg_dark()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::orange()).bg(CatppuccinMochaTheme::bg_dark()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::orange()).bg(CatppuccinLatteTheme::bg_dark()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::orange()).bg(CatppuccinFrappeTheme::bg_dark()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::orange()).bg(CatppuccinMacchiatoTheme::bg_dark()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::orange()).bg(DraculaTheme::bg_dark()),
            ThemeType::Nord => Style::default().fg(NordTheme::orange()).bg(NordTheme::bg_dark()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::orange()).bg(OneDarkTheme::bg_dark()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::orange()).bg(GruvboxTheme::bg_dark()),
        }
    }

    pub fn markdown_code_block(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()).bg(TokyoNightTheme::bg_dark()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()).bg(GitHubDarkTheme::bg_dark()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()).bg(CatppuccinMochaTheme::bg_dark()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()).bg(CatppuccinLatteTheme::bg_dark()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()).bg(CatppuccinFrappeTheme::bg_dark()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()).bg(CatppuccinMacchiatoTheme::bg_dark()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()).bg(DraculaTheme::bg_dark()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()).bg(NordTheme::bg_dark()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()).bg(OneDarkTheme::bg_dark()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()).bg(GruvboxTheme::bg_dark()),
        }
    }

    pub fn markdown_bold(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn markdown_italic(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::Nord => Style::default().fg(NordTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg_dark()).add_modifier(Modifier::ITALIC),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg_dark()).add_modifier(Modifier::ITALIC),
        }
    }

    pub fn markdown_link(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::Nord => Style::default().fg(NordTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::blue()).add_modifier(Modifier::UNDERLINED),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::blue()).add_modifier(Modifier::UNDERLINED),
        }
    }

    // File tree icons
    pub fn folder_icon(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::blue()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::blue()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::blue()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::blue()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::blue()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::blue()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::blue()),
            ThemeType::Nord => Style::default().fg(NordTheme::blue()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::blue()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::blue()),
        }
    }

    pub fn folder_expanded_icon(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::cyan()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::cyan()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::cyan()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::cyan()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::cyan()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::cyan()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::cyan()),
            ThemeType::Nord => Style::default().fg(NordTheme::cyan()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::cyan()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::cyan()),
        }
    }

    pub fn note_icon(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::green()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::green()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::green()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::green()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::green()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::green()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::green()),
            ThemeType::Nord => Style::default().fg(NordTheme::green()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::green()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::green()),
        }
    }

    // Helper methods
    pub fn placeholder(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::comment()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::comment()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::comment()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::comment()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::comment()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::comment()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::comment()),
            ThemeType::Nord => Style::default().fg(NordTheme::comment()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::comment()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::comment()),
        }
    }

    pub fn help_text(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg_dark()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg_dark()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg_dark()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg_dark()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg_dark()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg_dark()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg_dark()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg_dark()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg_dark()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg_dark()),
        }
    }

    pub fn success(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::green()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::green()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::green()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::green()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::green()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::green()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::green()),
            ThemeType::Nord => Style::default().fg(NordTheme::green()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::green()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::green()),
        }
    }

    pub fn warning(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::yellow()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::yellow()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::yellow()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::yellow()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::yellow()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::yellow()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::yellow()),
            ThemeType::Nord => Style::default().fg(NordTheme::yellow()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::yellow()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::yellow()),
        }
    }

    pub fn error(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::red()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::red()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::red()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::red()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::red()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::red()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::red()),
            ThemeType::Nord => Style::default().fg(NordTheme::red()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::red()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::red()),
        }
    }

    pub fn tree_guide(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg_gutter()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg_gutter()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg_gutter()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg_gutter()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg_gutter()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg_gutter()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg_gutter()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg_gutter()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg_gutter()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg_gutter()),
        }
    }

    pub fn welcome_accent(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::cyan()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::cyan()).add_modifier(Modifier::BOLD),
        }
    }

    pub fn scrollbar(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::fg_gutter()).bg(TokyoNightTheme::bg_dark()),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::fg_gutter()).bg(GitHubDarkTheme::bg_dark()),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::fg_gutter()).bg(CatppuccinMochaTheme::bg_dark()),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::fg_gutter()).bg(CatppuccinLatteTheme::bg_dark()),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::fg_gutter()).bg(CatppuccinFrappeTheme::bg_dark()),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::fg_gutter()).bg(CatppuccinMacchiatoTheme::bg_dark()),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::fg_gutter()).bg(DraculaTheme::bg_dark()),
            ThemeType::Nord => Style::default().fg(NordTheme::fg_gutter()).bg(NordTheme::bg_dark()),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::fg_gutter()).bg(OneDarkTheme::bg_dark()),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::fg_gutter()).bg(GruvboxTheme::bg_dark()),
        }
    }

    pub fn search_match(&self) -> Style {
        match self.current_theme {
            ThemeType::TokyoNight => Style::default().fg(TokyoNightTheme::bg()).bg(TokyoNightTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::GitHubDark => Style::default().fg(GitHubDarkTheme::bg()).bg(GitHubDarkTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMocha => Style::default().fg(CatppuccinMochaTheme::bg()).bg(CatppuccinMochaTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinLatte => Style::default().fg(CatppuccinLatteTheme::bg()).bg(CatppuccinLatteTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinFrappe => Style::default().fg(CatppuccinFrappeTheme::bg()).bg(CatppuccinFrappeTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::CatppuccinMacchiato => Style::default().fg(CatppuccinMacchiatoTheme::bg()).bg(CatppuccinMacchiatoTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::Dracula => Style::default().fg(DraculaTheme::bg()).bg(DraculaTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::Nord => Style::default().fg(NordTheme::bg()).bg(NordTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::OneDark => Style::default().fg(OneDarkTheme::bg()).bg(OneDarkTheme::yellow()).add_modifier(Modifier::BOLD),
            ThemeType::Gruvbox => Style::default().fg(GruvboxTheme::bg()).bg(GruvboxTheme::yellow()).add_modifier(Modifier::BOLD),
        }
    }
}

/// Monochrome icon constants for consistent theming
pub struct Icons;

impl Icons {
    // File tree icons
    pub const FOLDER_CLOSED: &'static str = "▶";
    pub const FOLDER_OPEN: &'static str = "▼";
    pub const NOTE: &'static str = "●";
    pub const ROOT: &'static str = "~";
    
    // Status indicators
    pub const SAVED: &'static str = "●";
    pub const MODIFIED: &'static str = "○";
    pub const SAVING: &'static str = "◐";
    pub const ERROR: &'static str = "✗";
    
    // Mode indicators
    pub const EDITOR: &'static str = "▣";
    pub const PREVIEW: &'static str = "◈";
    pub const SEARCH: &'static str = "◉";
    pub const CLOCK: &'static str = "◷";
    
    // Navigation
    pub const BREADCRUMB_SEPARATOR: &'static str = "▸";
    pub const EXPLORER: &'static str = "≡";
    
    // Alternative icon sets (you can switch between these)
    pub const FOLDER_CLOSED_ALT: &'static str = "►";
    pub const FOLDER_OPEN_ALT: &'static str = "▽";
    pub const NOTE_ALT: &'static str = "◦";
    pub const FOLDER_CLOSED_SIMPLE: &'static str = "+";
    pub const FOLDER_OPEN_SIMPLE: &'static str = "-";
    pub const NOTE_SIMPLE: &'static str = "•";
}

