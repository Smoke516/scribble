# Scribble ✏️ v2.1.0

A powerful terminal-based note-taking app with advanced tag management, Obsidian vault integration, real-time file watching, folder organization, markdown support, and syntax highlighting.

![Version](https://img.shields.io/badge/version-2.1.0-brightgreen)
![Terminal Interface](https://img.shields.io/badge/interface-terminal-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS-lightgrey)
![Language](https://img.shields.io/badge/language-Rust-orange)

## Features

### Core Features
- **🏷️ Advanced Tag System** - Comprehensive tag management with YAML frontmatter and inline hashtag support (`Ctrl+T`)
- **📁 Obsidian Vault Integration** - Native compatibility with Obsidian vaults and seamless vault switching (`Ctrl+V`)
- **🔄 Real-time File Watching** - Live sync with external changes and file system monitoring
- **📁 Folder organization** - Hierarchical folder structure to organize your notes
- **📝 Markdown support** - Write notes in markdown with live syntax highlighting
- **🎨 Theme System** - Multiple beautiful themes with interactive browser (`F3` or `:theme list`)
- **✏️ Rename Support** - Easily rename folders and notes with duplicate checking (`r`)
- **✏️ External editor support** - Edit notes in Helix, Neovim, or your favorite editor
- **💾 Crash-safe auto-save** - Notes are persisted to disk continuously (on edit, periodic autosave, and structural changes like move/rename/delete) — not just on exit, so your work survives an unexpected exit
- **🔍 Full-text search** - Search through all your notes by title or content, with advanced regex/field search (`Ctrl+A`) and search & replace (`Ctrl+R`)
- **🔤 Spell check** - Optional aspell-backed spell checking with inline error highlighting and Vim-style `z=` suggestions
- **⌨️ Vim-like navigation** - Familiar keyboard shortcuts, including a Visual selection mode (`v`)

### 🚀 Productivity Features
- **🏷️ Tag Filtering** - Filter notes by tags with interactive browser and analytics (`Ctrl+T`)
- **🎨 Theme Browser** - Interactive theme selection with live preview (`F3` or `:theme list`)
- **✏️ Quick Rename** - Rename folders and notes instantly with `r` key
- **📁 Vault Management** - Switch between multiple Obsidian vaults instantly (`Ctrl+V`)
- **🔔 Change Notifications** - Real-time alerts for external file modifications
- **🔍 Fuzzy Search** - Intelligent search that finds matches even with typos (`Ctrl+F`)
- **⚡ Quick Jump** - Instant note navigation with `Ctrl+J` - like VS Code's quick open
- **🗂️ Outline Panel** - Jump to any heading in the current note (`Ctrl+G`)
- **☑️ Task Checkboxes** - Toggle `- [ ]`/`- [x]` on the current line with `Space`
- **📅 Daily Notes** - One-key open-or-create of today's dated note (`F4` or `:daily`)
- **📋 Recent Files** - Quick access to recently opened notes (`Ctrl+O`)
- **👁️ Live Preview** - Real-time markdown preview in split-screen mode (`Ctrl+P`)
- **↩️ Undo/Delete Safety** - Safely delete with full undo capability (`u` to undo)
- **🔗 Note Linking** - Wiki-style `[[Note Title]]` links between notes (`Ctrl+L` to follow)
- **🔙 Backlinks Panel** - See which notes link *to* the current note and where it links *out*; `Tab` switches sections, `Enter` opens either, and opening a broken outgoing link creates the missing note (`Ctrl+B`)
- **🔎 Advanced Search** - Regex and scoped search like `case:`/`folder:Name` (`Ctrl+A`), plus in-note search with `/` and `n`/`N` to step through matches
- **🔁 Search & Replace** - Find-and-replace across notes (`Ctrl+R`)
- **📑 Templates** - Create notes from Blank/Daily/Meeting/Project templates (`N`)
- **💾 Session Persistence** - Remembers your last opened note and editor state
- **🆔 Version Display** - Always know what version you're running (`--version` or in-app help)
- **🎯 Smart Autocompletion** - Intelligent markdown completion with context awareness
- **📖 Scrollable Help** - Navigate help with `j/k`, `g/G`, PageUp/Down (`?`)

### Interface
- **Multiple Themes** - 11 beautiful themes including Tokyo Night, Gruvbox, Dracula, and more
- **Theme Browser** - Interactive theme switcher with live preview (`F3`)
- **Two-pane layout** - Left pane for folder/note navigation, right pane for editing
- **Clean, responsive UI** - Works beautifully in any terminal size
- **Enhanced Welcome Screen** - Professional onboarding with organized quick-start guide
- **Multiple modes** - Normal, Insert, Search, Command, and Rename modes like in Vim
- **Real-time updates** - See your changes reflected immediately
- **Scrollable Help** - Full keyboard navigation in help dialog
- **Version-aware Interface** - Always shows current version in help and welcome screens

## 🎬 Demo

See Scribble in action! We've included comprehensive demo materials:

### Quick Demo
```bash
# Build the project
cargo build --release

# Run the demo with sample files
./target/release/scribble demo_files/
```

**Try these features:**
- Navigate with `↑/↓` or `j/k`
- Press `Enter` to open files and see **beautiful syntax highlighting**
- Use `Tab` to switch between panes
- Press `/` for search, `Ctrl+F` for advanced search
- Press `q` to quit

### Create a Recording
```bash
./create_demo.sh
```
This creates an `asciinema` recording perfect for sharing!

📖 **Full demo guide:** See [DEMO_GUIDE.md](DEMO_GUIDE.md) for detailed instructions

---

## Installation

### Prerequisites
- **Rust** (1.70+) - Install from [rustup.rs](https://rustup.rs/)
- **aspell** *(optional)* - Enables spell check (`:spell`, `z=`). Install with `sudo apt install aspell` (Debian/Ubuntu), `sudo dnf install aspell` (Fedora), or `brew install aspell` (macOS)

### Quick Install (Recommended)

```bash
# Clone the repository
git clone https://github.com/Smoke516/scribble.git
cd scribble

# Run the automated install script
./install.sh
```

The install script will:
- Build the release binary
- Install to your system PATH
- Create desktop application entry
- Set up configuration directories

### Manual Build from Source

```bash
# Clone the repository
git clone https://github.com/Smoke516/scribble.git
cd scribble

# Build and install manually
cargo build --release
cargo install --path .
```

### Verify Installation
```bash
# Check version
scribble --version
# Output: scribble 2.1.0

# View help
scribble --help
```

### Run without installing
```bash
cargo run
```

## External Changes and Conflicts

Scribble assumes it is not the only thing writing to your vault — a sync client,
another device, an editor, or another scribble instance may all touch the same
files. The rule is simple: **it never overwrites a file it cannot account for.**

- **You have nothing unsaved for that note** — the disk version is taken silently.
  There is nothing to lose, and a stale in-memory copy is how a later edit quietly
  reverts somebody else's work.
- **You have unsaved changes** — that is a real conflict. Your version keeps the
  filename, because you are mid-sentence in it, and the version found on disk is
  copied to `Note (scribble conflict 2026-08-16 131829).md` before yours is
  written. Nothing is lost, the editor never jumps, and the status line tells you
  the file exists.

Nothing is ever merged or discarded automatically — which version wins is your
call, made whenever you get to it. Conflict files are recognised as artefacts, so
they never displace the note they were forked from in the sidebar.

## Command Line Options

Scribble supports several command line options:

```bash
scribble              # Start the application
scribble --version    # Show version information  
scribble -v           # Show version (short form)
scribble --help       # Show detailed help
scribble -h           # Show help (short form)
```

### Command Line Help Output
```
scribble v2.1.0

A powerful terminal-based note-taking app with folder organization,
markdown support, and syntax highlighting.

USAGE:
    scribble              Start the application
    scribble --version    Show version information
    scribble --help       Show this help message

FEATURES:
  • 📝 Rich markdown editing with live preview
  • 🗂️  Hierarchical folder organization
  • 🔍 Fuzzy search and quick jump navigation
  • 🔗 Wiki-style [[note linking]]
  • ⚡ Auto-save and intelligent autocompletion
  • 🎨 Beautiful Tokyo Night theme
  • 🚀 Vim-inspired keybindings

Once started, press '?' for in-app help.
```

## Quick Start

1. **Launch the app:**
   ```bash
   scribble
   ```

2. **First time setup:**
   - The app starts with an enhanced welcome screen showing version 2.1.0
   - The welcome screen provides organized quick-start categories:
     - 📝 **Creating Content**: `n` (new note), `f` (folder), `i` (edit)
     - 🧭 **Navigation**: `Tab` (switch panes), `j/k` (move), `Enter` (open)
     - 🔍 **Essential Tools**: `/` (search), `Ctrl+P` (preview), `?` (help)
   - Use `j/k` or arrow keys to navigate the folder tree

3. **Create your first note:**
   - Press `n` to create a new note
   - Enter insert mode automatically and start writing in markdown
   - Press `Ctrl+P` to see live preview while editing

4. **Navigate and organize:**
   - Press `f` to create a new folder
   - Use `Enter` to open notes or expand/collapse folders
   - Press `Tab` to switch between panes
   - Try `Ctrl+J` for instant note jumping
   - Use `Ctrl+O` to access recent files

5. **Use external editor:**
   - Press `e` to open the current note in your external editor
   - Supports Helix (`hx`), Neovim, Vim, and more
   - Changes are automatically saved back to Scribble

## 🆕 What's New in v2.1.0

### 🗂️ Outline Panel (`Ctrl+G`)
- Jump to any heading in the current note; navigate with `j/k`, `Enter` to jump
- Skips headings inside fenced code blocks

### ☑️ Task Checkboxes (`Space`)
- Toggle `- [ ]` / `- [x]` on the current editor line with a single keystroke

### 📅 Daily Notes (`F4` or `:daily`)
- Open or create today's `YYYY-MM-DD` note in one step (no duplicates)

### 🔗 Backlinks: navigable outgoing links (`Ctrl+B`)
- `Tab` switches between the incoming and outgoing sections; `Enter` opens either
- Broken outgoing links are flagged, and opening one creates the missing note

### 🔎 Folder-scoped search (`Ctrl+A` → `folder:Name term`)
- Restrict advanced search to a folder; an empty term lists the whole folder

### 🐛 Fixes
- Live preview decorations (heading underlines, code-block borders, horizontal
  rules) now size to the pane width instead of wrapping into stray stub lines
- Advanced search no longer hangs on an empty query

## 🆕 What's New in v2.0.0

### 🎨 Theme System
- **Access:** Press `F3` or use `:theme list` to open the theme browser
- **11 Themes:** Tokyo Night, Gruvbox (Dark/Light), Dracula, Nord, Solarized (Dark/Light), Monokai, One Dark, Catppuccin, and Ayu
- **Live Preview:** See theme changes instantly as you navigate
- **Persistent:** Theme selection is saved across sessions
- **Commands:** `:theme <name>` to switch directly, `:theme current` to see active theme
- **See:** [THEMES.md](THEMES.md) for complete theme showcase

### ✏️ Quick Rename
- **Access:** Press `r` on any folder or note
- **Smart Validation:** Prevents duplicate names at the same level
- **Live Feedback:** Shows old and new names side-by-side
- **Safe Updates:** Properly updates all references when renaming

### 📖 Scrollable Help System
- **Navigation:** Use `j/k`, arrow keys, `g/G`, PageUp/PageDown in help dialog
- **Organized:** Well-structured sections for easy reference
- **Complete:** All features and shortcuts documented in-app

### 🏷️ Advanced Tag Management
- **Access:** Press `Ctrl+T` to open the tag browser
- **Dual Support:** Use YAML frontmatter (`tags: [work, project]`) or inline hashtags (`#work #project`)
- **Smart Filtering:** Filter notes by single or multiple tags
- **Analytics:** View tag frequency and usage statistics
- **Quick Selection:** Use number keys (1-9) for instant tag selection

### 📁 Obsidian Vault Integration
- **Access:** Press `Ctrl+V` to switch between vaults
- **Auto-detection:** Automatically finds Obsidian vaults in current/parent directories
- **Full Compatibility:** Seamlessly work with existing Obsidian vaults
- **YAML Support:** Preserves all metadata and frontmatter
- **Wiki Links:** Complete `[[note linking]]` support

### 🔄 Real-time File Watching
- **Live Sync:** External file changes appear instantly
- **Smart Notifications:** Visual alerts for file modifications
- **Status Indicators:** File watcher status shown in status bar
- **Multi-format Support:** Handles create, modify, delete, and rename events

## Usage

### Basic Operations
| Key | Action |
|-----|--------|
| `j/k` or `↓/↑` | Navigate up/down in folder tree |
| `g/G` | Go to top/bottom |
| `Enter` | Open note or toggle folder expansion |
| `Tab` | Switch between folder pane and editor |
| `n` | Create new note |
| `N` | Create new note from a template |
| `f` | Create new folder |
| `r` | Rename selected folder or note |
| `d` | Delete selected item |
| `m` | Move selected item |
| `t` | Edit the current note's tags |
| `i` | Enter insert mode (edit note) |
| `v` | Enter Visual selection mode |
| `e` | Open note in external editor |
| `Esc` | Return to normal mode |

### 🚀 Productivity & Navigation
| Key | Action |
|-----|--------|
| `F3` | Theme Browser - browse and switch themes interactively |
| `Ctrl+T` | Tag Browser - manage and filter notes by tags |
| `Ctrl+V` | Vault Switcher - switch between multiple vaults |
| `/` | Search notes by content or title (regular search) |
| `Ctrl+F` | Fuzzy search mode - intelligent search with typo tolerance |
| `Ctrl+A` | Advanced search - regex and scopes (e.g. `case:`, `folder:Name`) |
| `Ctrl+R` | Search & replace across notes |
| `/` then `n`/`N` | In-note search; jump to next/previous match |
| `Tab` (in search) | Switch between regular and fuzzy search |
| `Ctrl+J` | Quick Jump - instant fuzzy search across all notes |
| `Ctrl+O` | Recent files - quick access to recently opened notes |
| `Ctrl+L` | Follow [[wiki-style]] link at cursor |
| `Ctrl+B` | Links panel - backlinks (in) and outgoing links (out) |
| `Ctrl+G` | Outline panel - jump to any heading in the current note |
| `F4` | Open (or create) today's daily note (`YYYY-MM-DD`) |
| `u` | Undo last delete operation (restore from trash) |
| `?` | Show comprehensive scrollable help with all shortcuts |

### Live Preview
| Key | Action |
|-----|--------|
| `Ctrl+P` / `F2` | Toggle live markdown preview (split-screen) |

### File Operations & Commands
| Key | Action |
|-----|--------|
| `:w` or `Ctrl+S` | Save current note |
| `:q` | Quit application |
| `:wq` | Save and quit |
| `:N` | Jump to line number N |
| `:export [path]` | Export all notes as markdown files |
| `:export html [path]` | Export all notes as HTML (default `~/Documents/scribble_export`) |
| `:import <dir>` | Import markdown files from a directory |
| `:backup` / `:backups` | Create a backup / list existing backups |
| `:spell` / `:nospell` | Enable / disable spell check |
| `:theme list` | Open theme browser |
| `:theme <name>` | Switch to specific theme |
| `:theme current` | Show current theme name |
| `:vault` | Open vault switcher |
| `:daily` / `:today` | Open or create today's daily note |

### Note Format
Write notes in standard markdown:

````markdown
# This is a heading

## Subheading

- List item 1
- List item 2

> This is a blockquote

**Bold text** and *italic text*

`inline code` and fenced code blocks:

```
code blocks
```
````

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

Scribble has two storage backends:

- **Default notebook** - a single `notebook.json` in a platform-appropriate location:
  - **Linux**: `~/.local/share/scribble/`
  - **macOS**: `~/Library/Application Support/scribble/`
- **Obsidian vault** - when working in a vault, notes are individual Markdown files with YAML frontmatter, fully compatible with Obsidian.

Either way, changes are written to disk continuously — on edit, on a periodic autosave, and after structural operations (move/rename/delete) — so your work is not lost if the app exits unexpectedly. Vault saves are incremental: only changed files are rewritten.

## Keyboard Shortcuts Reference

### Normal Mode
- `j/k, ↓/↑` - Navigate
- `g/G` - Go to top/bottom
- `Enter` - Open/expand item
- `Tab` - Switch panes
- `n` / `N` - New note / new note from template
- `f` - New folder
- `r` / `m` / `d` - Rename / move / delete item
- `t` - Edit current note's tags
- `i` - Insert mode
- `v` - Visual selection mode
- `Space` - Toggle task checkbox `[ ]`/`[x]` on the current line (editor)
- `e` - Open in external editor
- `/` - Search
- `Ctrl+B` - Links panel (backlinks + outgoing)
- `Ctrl+G` - Outline panel (jump to heading)
- `F4` - Open today's daily note
- `:` - Command mode
- `Ctrl+S` - Save
- `q` - Quit
- `?` - Help

### Insert Mode
- `Esc` - Return to normal mode (auto-saves and runs spell check)
- `Ctrl+S` - Save
- `Ctrl+P` - Toggle live preview
- `Ctrl+V` - Paste from the system clipboard
- `Ctrl+L` - Follow `[[wiki link]]` at cursor
- `Ctrl+Z` / `Ctrl+Y` - Undo / redo
- `[[` - Wiki-link autocomplete; `Tab` accepts a suggestion (or inserts 4 spaces)
- Regular typing for content; arrow keys for navigation
- `Backspace` - Delete character

### Visual Mode (`v`)
- `h/j/k/l`, `w/b`, `0/$`, `g/G` - Extend the selection
- `y` - Yank selection
- `d` - Delete selection
- `c` - Change (delete then enter Insert mode)
- `Esc` / `v` - Cancel

### Spell Check (requires aspell)
- `:spell` / `:nospell` - Enable / disable
- `z=` - Suggestions for the word at the cursor; `1`-`9` to pick, `Enter` to apply

### Search Mode
- Type to search
- `Enter` - Execute search
- `Esc` - Cancel search

### Command Mode
- `:w` / `:q` / `:wq` - Write / quit / save and quit
- `:N` - Jump to line N
- `:export [path]` / `:export html [path]` - Export as markdown / HTML
- `:import <dir>` - Import markdown files
- `:backup` / `:backups` - Create / list backups
- `:spell` / `:nospell` - Toggle spell check
- `:theme list|<name>|current` - Theme browser / switch / show active
- `:vault` - Vault switcher
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
- `serde_yaml` - YAML frontmatter support
- `toml` - Configuration file handling
- `chrono` - Date/time handling
- `uuid` - Unique identifiers
- `syntect` - Syntax highlighting
- `pulldown-cmark` - Markdown parsing
- `dirs` - Platform directories
- `notify` - File system watching
- `walkdir` - Directory traversal
- `regex` - Pattern matching
- `fuzzy-matcher` - Fuzzy search capabilities
- `textwrap` - Text formatting

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
