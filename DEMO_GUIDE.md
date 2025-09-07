# 🎬 Scribble Demo Guide

This guide will walk you through demonstrating all the key features of Scribble.

## Pre-Demo Setup

1. **Build the project:**
   ```bash
   cargo build --release
   ```

2. **Ensure demo files exist:**
   - The demo files should already be in `demo_files/` directory
   - Contains: `hello.rs`, `example.py`, `notes.md`

## Demo Script

### Part 1: Basic Navigation (30 seconds)

1. **Start Scribble:**
   ```bash
   ./target/release/scribble demo_files/
   ```

2. **Show file browsing:**
   - Use `↑/↓` or `j/k` to navigate between files
   - Show the clean file browser interface
   - Highlight the different file types (`.rs`, `.py`, `.md`)

3. **Open a file:**
   - Navigate to `hello.rs`
   - Press `Enter` to open the file
   - Show the **syntax highlighting** for Rust code

### Part 2: Syntax Highlighting Tour (45 seconds)

4. **Navigate between files:**
   - Press `Tab` to switch to folder pane
   - Navigate to `example.py`
   - Press `Enter` to show **Python syntax highlighting**
   - Notice the different colors for keywords, strings, comments

5. **Show Markdown support:**
   - Navigate to `notes.md`
   - Press `Enter` to view
   - Show how headers, lists, and formatting are highlighted

### Part 3: Search Features (30 seconds)

6. **Basic search:**
   - Press `/` to enter search mode
   - Type a search term (e.g., "function" or "import")
   - Show search results highlighting

7. **Advanced search:**
   - Press `Ctrl+F` to open advanced search
   - Demonstrate more complex search options

### Part 4: Navigation & Controls (15 seconds)

8. **Show scrolling:**
   - Use `j/k` to scroll through files
   - Use `g` to go to top, `G` to go to bottom
   - Use `Ctrl+U/Ctrl+D` for half-page scrolling

9. **Pane switching:**
   - Press `Tab` to cycle between panes
   - Show the preview pane toggle with `Ctrl+M`

### Part 5: Exit (5 seconds)

10. **Clean exit:**
    - Press `q` to quit cleanly
    - Show the terminal returns to normal state

## Key Bindings Cheat Sheet

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate/Scroll |
| `Enter` | Open file/folder |
| `Tab` | Switch panes |
| `/` | Basic search |
| `Ctrl+F` | Advanced search |
| `Ctrl+M` | Toggle preview |
| `g/G` | Go to top/bottom |
| `Ctrl+U/D` | Half-page scroll |
| `q` | Quit |

## Tips for Recording

- **Prepare your terminal:** Use a large font size and clean terminal theme
- **Go slow:** Allow viewers to see each action clearly
- **Narrate:** If doing a live demo, explain what each feature does
- **Show variety:** Use all the different file types to showcase syntax highlighting
- **Emphasize colors:** The Tokyo Night theme looks great - make sure it's visible

## Alternative Demo Methods

### 1. Asciinema Recording
```bash
./create_demo.sh
```

### 2. Screen Recording
- Use OBS Studio or similar for video recording
- Focus on the terminal window
- Record at 1080p for clarity

### 3. Animated GIF
- Use asciinema + asciicast2gif to create GIFs
- Perfect for README files and documentation

## Troubleshooting

- **Colors not showing:** Ensure terminal supports 256 colors
- **App won't start:** Check that demo files exist
- **Performance issues:** Use release build (`--release` flag)

---

*This demo showcases Scribble as a modern, feature-rich terminal text editor with beautiful syntax highlighting and intuitive navigation.*
