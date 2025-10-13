# Installation Guide 🚀

This guide covers various ways to install and set up Scribble with all its features.

## ✅ Prerequisites

- **Rust** (1.70+) - Install from [rustup.rs](https://rustup.rs/)
- A terminal emulator
- Optional: External editor like [Helix](https://helix-editor.com/), [Neovim](https://neovim.io/), VS Code

## 🎯 Quick Installation (Recommended)

### Option 1: Automated Install Script
```bash
# Clone and install in one command
git clone <repository-url>
cd scribble
./install.sh
```

This script will:
- ✅ Build and install the Scribble binary
- ✅ Create desktop entry for GUI access
- ✅ Set up configuration directory
- ✅ Create terminal wrapper scripts
- ✅ Add to applications menu

### Option 2: Manual Installation
```bash
# Install directly via cargo
cargo install --path .

# Verify installation
scribble --version
```

## 🔧 Advanced Setup

### Desktop Integration
The install script creates a desktop entry, but you can manually create one:

```bash
# Create desktop entry directory
mkdir -p ~/.local/share/applications

# Create the desktop file
cat > ~/.local/share/applications/scribble.desktop << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Scribble
Comment=Terminal-based note-taking with Obsidian vault support
Exec=sh -c 'cd && scribble'
Icon=accessories-text-editor
Terminal=true
Categories=Office;TextEditor;Utility;Development;
Keywords=notes;markdown;terminal;obsidian;vault;
StartupNotify=true
EOF

chmod +x ~/.local/share/applications/scribble.desktop
```

### GUI Wrapper Script
For easy access from desktop environments:

```bash
mkdir -p ~/.local/bin

cat > ~/.local/bin/scribble-gui << 'EOF'
#!/bin/bash
if command -v gnome-terminal &> /dev/null; then
    gnome-terminal --title="Scribble" -- scribble "$@"
elif command -v konsole &> /dev/null; then
    konsole --title "Scribble" -e scribble "$@"
elif command -v alacritty &> /dev/null; then
    alacritty --title "Scribble" -e scribble "$@"
else
    scribble "$@"
fi
EOF

chmod +x ~/.local/bin/scribble-gui
```

### Configuration Setup
Create configuration directory and default config:

```bash
mkdir -p ~/.config/scribble

cat > ~/.config/scribble/config.toml << 'EOF'
[editor]
# External editor (auto-detected from $EDITOR)
# external_editor = "hx"    # Helix
# external_editor = "nvim"  # Neovim  
# external_editor = "code"  # VS Code

[ui]
theme = "tokyo-night"
preview_enabled = false
auto_save = true

[vaults]
auto_detect = true
recent = []

[behavior]
backup_before_import = true
max_recent_files = 10
auto_save_interval = 5
EOF
```

## 🏗️ Build from Source

### Development Build
```bash
git clone <repository-url>
cd scribble

# Debug build (faster compile)
cargo build

# Run from source
cargo run

# Run with vault
cargo run -- --vault /path/to/vault
```

### Release Build
```bash
# Optimized build
cargo build --release

# Install from release build
cargo install --path . --force
```

### Testing Installation
```bash
# Run tests
cargo test

# Check binary location
which scribble

# Verify functionality
scribble --help
scribble --version
```

## 🛠️ Configuration Options

### External Editor Setup
Scribble automatically detects external editors in this order:
1. `$EDITOR` environment variable
2. `hx` (Helix)
3. `helix`
4. `nvim` (Neovim)
5. `vim`
6. `nano`

To set a specific editor:
```bash
# In your shell profile (.bashrc, .zshrc, etc.)
export EDITOR="hx"  # or "nvim", "code --wait", etc.

# Or in config file
echo 'external_editor = "hx"' >> ~/.config/scribble/config.toml
```

### PATH Configuration
Ensure Cargo's bin directory is in your PATH:

```bash
# For bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# For zsh  
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# For fish
fish_add_path ~/.cargo/bin
```

## 📁 Vault Mode Setup

### Obsidian Integration
```bash
# Auto-detect vault in current directory
cd /path/to/obsidian-vault
scribble

# Specify vault path
scribble --vault /path/to/vault

# Set default vault in config
echo 'default = "/path/to/your/main/vault"' >> ~/.config/scribble/config.toml
```

### Multiple Vaults
Scribble tracks recent vaults automatically. Use `Ctrl+V` to switch between them.

## 🚨 Troubleshooting

### Installation Issues

**Problem**: `scribble: command not found`
```bash
# Check if binary exists
ls ~/.cargo/bin/scribble

# Add to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Make permanent
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
```

**Problem**: Build errors
```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release
```

**Problem**: Missing external editor
```bash
# Install Helix (recommended)
# See: https://helix-editor.com/

# Or set EDITOR variable
export EDITOR="nano"  # or your preferred editor
```

### Runtime Issues

**Problem**: Can't open files in external editor
```bash
# Check EDITOR variable
echo $EDITOR

# Test editor directly
$EDITOR test.md

# Check Scribble config
cat ~/.config/scribble/config.toml
```

**Problem**: Desktop entry not appearing
```bash
# Update desktop database
update-desktop-database ~/.local/share/applications

# Check file permissions
ls -la ~/.local/share/applications/scribble.desktop
```

## 🔄 Updating

### Update from Git
```bash
cd scribble
git pull
cargo install --path . --force
```

### Reinstall
```bash
# Remove old installation
cargo uninstall scribble

# Clean reinstall  
cargo install --path . --force
```

## 🗑️ Uninstallation

### Remove Binary
```bash
cargo uninstall scribble
```

### Remove All Files
```bash
# Remove binary
cargo uninstall scribble

# Remove config
rm -rf ~/.config/scribble

# Remove desktop entry
rm ~/.local/share/applications/scribble.desktop

# Remove GUI wrapper
rm ~/.local/bin/scribble-gui
```

## 🎉 Post-Installation

After successful installation:

1. **Test basic functionality:**
   ```bash
   scribble --help
   scribble --version
   ```

2. **Try the app:**
   ```bash
   scribble
   # Press '?' for comprehensive help
   ```

3. **Set up your workflow:**
   - Configure external editor
   - Set up Obsidian vaults
   - Explore tag management with `Ctrl+T`

4. **Access from desktop:**
   - Look for "Scribble" in applications menu
   - Use `scribble-gui` for terminal window access

**Happy note-taking!** 📝✨

Need help? Press `?` in the application for comprehensive keybinding reference.