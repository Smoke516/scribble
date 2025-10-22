# Scribble Themes

Scribble now supports multiple beautiful themes! Here are all the available themes and how to use them.

## Available Themes

### Dark Themes
- **tokyo-night** (default) - A clean, modern dark theme inspired by Tokyo Night
- **github-dark** - GitHub's dark theme with familiar colors
- **catppuccin-mocha** - The popular Catppuccin theme in mocha flavor
- **catppuccin-frappe** - Catppuccin frappe variant
- **catppuccin-macchiato** - Catppuccin macchiato variant
- **dracula** - The classic Dracula theme
- **nord** - Arctic-inspired clean theme
- **one-dark** - Atom's One Dark theme
- **gruvbox** - Retro groove color scheme

### Light Themes
- **catppuccin-latte** - Light variant of Catppuccin

## How to Change Themes

### Via Configuration File
Edit your config file at `~/.config/scribble/config.toml`:

```toml
[ui]
theme = "github-dark"
```

### Via Command Mode (Runtime)
1. Press `:` to enter command mode
2. Type `theme <theme-name>` (e.g., `theme dracula`)
3. Press Enter

### Available Commands
- `theme <name>` - Switch to a specific theme
- `theme list` - Show all available themes
- `theme current` - Show current theme name

## Theme Preview

Each theme includes:
- Consistent color palette for syntax highlighting
- Properly themed UI elements (borders, selections, status bar)
- Markdown formatting (headers, lists, code blocks, etc.)
- Mode indicators with appropriate colors
- File tree with themed icons

## Creating Custom Themes

To add your own theme:

1. Add your theme to the `ThemeType` enum in `src/theme.rs`
2. Create a new theme struct implementing the `Theme` trait
3. Add the theme to the `ThemeManager` match statements
4. Add the theme name to `available_themes()`

Example:

```rust
pub struct MyCustomTheme;

impl Theme for MyCustomTheme {
    fn bg() -> Color { Color::Rgb(30, 30, 30) }
    fn fg() -> Color { Color::Rgb(255, 255, 255) }
    // ... implement all required methods
}
```

## Tips

- Light themes work best in bright environments
- Dark themes are easier on the eyes in low-light conditions
- The theme is saved automatically when changed
- Themes affect all UI elements consistently