# 🎉 Scribble Installation Complete!

Scribble has been successfully installed on your system!

## 📍 Installation Details

- **Binary location**: `~/.cargo/bin/scribble`
- **Data storage**: `~/.local/share/scribble/`
- **Desktop entry**: `~/.local/share/applications/scribble.desktop`

## 🚀 How to Use

### Command Line
```bash
# Start Scribble from anywhere
scribble

# Start with a specific directory
scribble ~/my-notes

# Start with demo files
scribble ~/scribble/demo_files
```

### Application Menu
Look for "Scribble" in your applications menu under:
- **Office** applications
- **Text Editors**
- Search for "scribble" or "notes"

## ✨ Quick Start Guide

1. **Launch Scribble**: `scribble`
2. **Navigate**: Use `j/k` or arrow keys
3. **Create note**: Press `n`
4. **Create folder**: Press `f`
5. **Edit note**: Press `Enter` to select, then `i` to edit
6. **Search**: Press `/` for basic search, `Ctrl+F` for advanced
7. **Delete (safely)**: Press `d` → confirm with `y`
8. **Help**: Press `?` anytime
9. **Quit**: Press `q`

## 🛡️ Safety Features

- **Deletion confirmation**: No accidental deletions!
- **Auto-save**: Notes save automatically
- **External editor**: Press `e` to edit in your favorite editor
- **Backup-friendly**: All data stored in JSON format

## 🎨 Features Highlights

- **Beautiful Tokyo Night theme**
- **Syntax highlighting** for code blocks
- **Folder organization** with tree view
- **Powerful search** with regex support
- **Live markdown preview** (Ctrl+M)
- **Vim-like navigation**

## 🔧 Troubleshooting

### If `scribble` command not found:
```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export PATH="$HOME/.cargo/bin:$PATH"

# Then reload your shell
source ~/.bashrc  # or ~/.zshrc
```

### If you want to uninstall:
```bash
cargo uninstall scribble
rm ~/.local/share/applications/scribble.desktop
```

### For updates:
```bash
cd ~/scribble
git pull
cargo install --path .
```

## 📚 More Information

- **Demo files**: Try `scribble ~/scribble/demo_files`
- **Documentation**: See `DEMO_GUIDE.md` for detailed features
- **Source**: https://github.com/Smoke516/scribble

---

**Happy note-taking with Scribble!** ✏️✨
