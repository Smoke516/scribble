# Scribble ✏️

A powerful terminal-based note-taking app with folder organization, markdown support, and syntax highlighting.

![Terminal Interface](https://img.shields.io/badge/interface-terminal-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey)

## Features

### Core Features
- **📁 Folder organization** - Hierarchical folder structure to organize your notes
- **📝 Markdown support** - Write notes in markdown with live syntax highlighting
- **🎨 Syntax highlighting** - Beautiful markdown highlighting for better readability
- **✏️ External editor support** - Edit notes in Helix, Neovim, or your favorite editor
- **💾 Auto-save** - Notes are automatically saved when you exit edit mode
- **🔍 Full-text search** - Search through all your notes by title or content
- **⌨️ Vim-like navigation** - Familiar keyboard shortcuts for efficient navigation

### Interface
- **Tokyo Night theme** - Beautiful dark theme with carefully chosen colors
- **Two-pane layout** - Left pane for folder/note navigation, right pane for editing
- **Clean, responsive UI** - Works beautifully in any terminal size
- **Multiple modes** - Normal, Insert, Search, and Command modes like in Vim
- **Real-time updates** - See your changes reflected immediately

## Installation

### Prerequisites
- **Rust** (1.70+) - Install from [rustup.rs](https://rustup.rs/)

### Build from Source

```bash
# Clone the repository
git clone <your-repo-url>
cd scribble

# Build and install
cargo build --release
cargo install --path .
```

### Run without installing
```bash
cargo run
```

## Quick Start

1. **Launch the app:**
   ```bash
   scribble
   ```

2. **First time setup:**
   - The app starts with some default folders and a welcome note
   - Use `j/k` or arrow keys to navigate the folder tree

3. **Create your first note:**
   - Press `n` to create a new note
   - Enter insert mode automatically and start writing in markdown

4. **Navigate and organize:**
   - Press `f` to create a new folder
   - Use `Enter` to open notes or expand/collapse folders
   - Press `Tab` to switch between panes

5. **Use external editor:**
   - Press `e` to open the current note in your external editor
   - Supports Helix (`hx`), Neovim, Vim, and more
   - Changes are automatically saved back to Scribble

## Usage

### Basic Operations
| Key | Action |
|-----|--------|
| `j/k` or `↓/↑` | Navigate up/down in folder tree |
| `g/G` | Go to top/bottom |
| `Enter` | Open note or toggle folder expansion |
| `Tab` | Switch between folder pane and editor |
| `n` | Create new note |
| `f` | Create new folder |
| `d` | Delete selected item |
| `i` | Enter insert mode (edit note) |
| `e` | Open note in external editor |
| `Esc` | Return to normal mode |

### Search and Navigation
| Key | Action |
|-----|--------|
| `/` | Search notes by content or title |
| `?` | Show help message |

### File Operations
| Key | Action |
|-----|--------|
| `:w` or `Ctrl+S` | Save current note |
| `:q` | Quit application |
| `:wq` | Save and quit |

### Note Format
Write notes in standard markdown:

```markdown
# This is a heading

## Subheading

- List item 1
- List item 2

> This is a blockquote

**Bold text** and *italic text*

`inline code` and:

```code
code blocks
```
```

### Visual Indicators
- 📁 Collapsed folder (Tokyo Night blue) | 📂 Expanded folder (Tokyo Night cyan)
- 📄 Note file (Tokyo Night green)
- Tokyo Night themed markdown elements:
  - **Cyan** - H1 headers (#7dcfff)
  - **Blue** - H2 headers (#7aa2f7)  
  - **Purple** - H3 headers (#bb9af7)
  - **Green** - List items (#9ece6a)
  - **Gray italic** - Blockquotes (#565f89)
  - **Orange on dark** - Code blocks (#ff9e64)
- Mode indicators with distinct Tokyo Night colors
- Focused panes highlighted with cyan borders

## External Editor Integration

Scribble can seamlessly integrate with your favorite external editor for enhanced editing capabilities.

### Supported Editors
Scribble automatically detects and supports these editors:
1. **Helix** (`hx` or `helix`) - Modern modal editor with built-in LSP
2. **Neovim** (`nvim`) - Extensible Vim-based editor  
3. **Vim** (`vim`) - Classic modal editor
4. **Nano** (`nano`) - Simple, user-friendly editor
5. **Emacs** (`emacs`) - Extensible editor

### Configuration
- **Automatic**: Scribble detects available editors in this priority order
- **Manual**: Set the `EDITOR` environment variable to your preferred editor:
  ```bash
  export EDITOR=hx        # Use Helix
  export EDITOR=nvim      # Use Neovim
  export EDITOR="code -w" # Use VS Code (with wait flag)
  ```

### How It Works
1. Press `e` while viewing a note
2. Scribble creates a temporary `.md` file with the note content
3. Your external editor opens with syntax highlighting and full features
4. When you save and exit, changes are automatically imported back
5. The temporary file is cleaned up

### Benefits
- **Full editor features**: LSP, plugins, advanced editing capabilities
- **Familiar workflow**: Use the editor you know and love
- **Syntax highlighting**: Proper markdown highlighting in your editor
- **Seamless integration**: No manual file management needed

## Data Storage

Scribble stores your notes in platform-appropriate locations:
- **Linux**: `~/.local/share/scribble/`
- **macOS**: `~/Library/Application Support/scribble/`

Data is stored in JSON format and automatically saved when you quit the application.

## Keyboard Shortcuts Reference

### Normal Mode
- `j/k, ↓/↑` - Navigate
- `g/G` - Go to top/bottom
- `Enter` - Open/expand item
- `Tab` - Switch panes
- `n` - New note
- `f` - New folder
- `d` - Delete item
- `i` - Insert mode
- `/` - Search
- `:` - Command mode
- `Ctrl+S` - Save
- `q` - Quit
- `?` - Help

### Insert Mode
- `Esc` - Return to normal mode
- `Ctrl+S` - Save
- Regular typing for content
- Arrow keys for navigation
- `Tab` - Insert 4 spaces
- `Backspace` - Delete character

### Search Mode
- Type to search
- `Enter` - Execute search
- `Esc` - Cancel search

### Command Mode
- `:w` - Write/save
- `:q` - Quit
- `:wq` - Save and quit
- `Esc` - Cancel command

## Building

### Development
```bash
# Run with debug info
cargo run

# Run tests
cargo test

# Build release version
cargo build --release
```

### Dependencies
- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `serde/serde_json` - Data serialization
- `chrono` - Date/time handling
- `uuid` - Unique identifiers
- `syntect` - Syntax highlighting
- `pulldown-cmark` - Markdown parsing
- `dirs` - Platform directories

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Troubleshooting

### Common Issues
- **Terminal compatibility**: Works best with modern terminals that support Unicode
- **Colors not showing**: Ensure your terminal supports ANSI color codes
- **Permission denied**: Make sure you have write permissions to the data directory

### Getting Help
- Press `?` in the app for quick help
- Check the keyboard shortcuts reference above
- Look for error messages in the status bar

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for the terminal UI
- Syntax highlighting powered by [Syntect](https://github.com/trishume/syntect)
- Inspired by vim and other terminal-based editors

---

**Happy note-taking! ✏️✨**
