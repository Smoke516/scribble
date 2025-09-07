# External Editor Integration Guide

## 🎯 Overview

Scribble now supports seamless integration with external editors like **Helix**, **Neovim**, **Vim**, and more! This feature combines Scribble's excellent organization with the full power of your favorite editor.

## ✨ Key Features

### 🔍 Automatic Detection
Scribble automatically detects available editors in this priority order:
1. **Helix** (`hx` or `helix`) - Preferred choice
2. **Neovim** (`nvim`)
3. **Vim** (`vim`) 
4. **Nano** (`nano`)
5. **Emacs** (`emacs`)

### ⚡ Quick Access
- Press `e` to open the current note in your external editor
- Seamless terminal switching - no complex setup required
- Automatic cleanup of temporary files

### 🔄 Smart Synchronization
- Creates temporary `.md` file with current note content
- Preserves all your changes when you save and exit
- Automatic import back to Scribble
- No manual file management needed

## 🚀 How to Use

### Basic Usage
1. **Navigate** to a note in Scribble
2. **Press `e`** to open in external editor
3. **Edit** with full editor features (LSP, plugins, etc.)
4. **Save and quit** (`:wq` in Vim/Helix, `Ctrl+X` in Nano)
5. **Return to Scribble** - changes are automatically saved!

### Example Workflow
```
Scribble → Press 'e' → Helix opens → Edit with full features → :wq → Back to Scribble
```

## ⚙️ Configuration

### Environment Variable (Recommended)
Set your preferred editor globally:
```bash
# Add to ~/.zshrc or ~/.bashrc
export EDITOR=hx          # Use Helix
export EDITOR=nvim        # Use Neovim  
export EDITOR=vim         # Use Vim
export EDITOR="code -w"   # Use VS Code with wait flag
```

### Manual Override
You can also use any editor by setting the `EDITOR` environment variable before running Scribble:
```bash
EDITOR=micro scribble     # Use Micro editor
EDITOR=emacs scribble     # Use Emacs
```

## 💡 Editor-Specific Tips

### Helix (`hx`)
- **Best choice** - Modern modal editor with built-in LSP
- Excellent markdown support out of the box
- Tree-sitter syntax highlighting
- No configuration needed

### Neovim (`nvim`)
- Highly extensible with plugins
- Great for developers familiar with Vim
- Can add markdown plugins for enhanced experience

### Vim (`vim`)
- Classic and reliable
- Available on virtually every system
- Familiar to many users

### VS Code (`code -w`)
- Full IDE features
- Excellent markdown preview
- Rich plugin ecosystem
- Note: Use the `-w` flag to wait for file closure

## 🔧 Technical Details

### How It Works
1. **Temporary file creation**: `scribble_NoteName_PID.md` in system temp directory
2. **Terminal handoff**: Raw mode disabled, editor gets full terminal control
3. **Process waiting**: Scribble waits for editor to exit
4. **Content import**: Modified file content read back into Scribble
5. **Cleanup**: Temporary file automatically deleted

### File Naming
- Format: `scribble_{sanitized_title}_{process_id}.md`
- Example: `scribble_My_Great_Note_12345.md`
- Automatically cleaned up after editing

### Error Handling
- Editor not found → Clear error message
- Editor crashes → Graceful recovery, temp file preserved for manual recovery
- File read errors → Error displayed, original content preserved

## 🎨 Benefits

### For Note-Taking
- **Rich editing**: Full markdown support with syntax highlighting
- **Productivity**: Use familiar keybindings and workflows  
- **Features**: LSP completion, snippets, advanced search/replace
- **Flexibility**: Switch between simple and advanced editing as needed

### For Developers
- **Familiar tools**: Use your daily editor for notes too
- **Consistency**: Same editor configuration and plugins
- **Integration**: Works with your existing development environment
- **Efficiency**: No context switching between different editing paradigms

## 🛠️ Troubleshooting

### Editor Not Detected
- Check if editor is in PATH: `which hx` or `which nvim`
- Set EDITOR environment variable manually
- Install your preferred editor

### Editor Won't Start
- Verify editor command works independently
- Check for terminal compatibility issues
- Try a different editor as fallback

### Changes Not Saved
- Ensure you properly save and exit the editor
- Check that temp file permissions are correct
- Verify editor exits with success status

## 🌟 Best Practices

### Recommended Setup
1. **Install Helix** for the best out-of-box experience
2. **Set EDITOR** environment variable in your shell profile
3. **Test integration** with a simple note first
4. **Customize editor** with markdown-specific settings if desired

### Workflow Tips
- Use Scribble's built-in editor for quick edits
- Use external editor for longer writing sessions
- Leverage external editor's features for complex formatting
- Switch between modes based on your current needs

---

**The external editor integration makes Scribble incredibly powerful - you get the best of both worlds: Scribble's organization and your favorite editor's features! 🎯✨**
