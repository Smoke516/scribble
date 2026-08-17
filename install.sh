#!/bin/bash

# Scribble Installation Script
# This script installs Scribble and creates useful shortcuts

set -e

echo "🚀 Installing Scribble..."

# Install the binary via cargo
echo "📦 Building and installing Scribble binary..."
cargo install --path .

# Check if installation was successful
if ! command -v scribble &> /dev/null; then
    echo "❌ Installation failed - scribble not found in PATH"
    echo "💡 Make sure ~/.cargo/bin is in your PATH:"
    echo "    echo 'export PATH=\"\$HOME/.cargo/bin:\$PATH\"' >> ~/.zshrc"
    echo "    source ~/.zshrc"
    exit 1
fi

echo "✅ Scribble binary installed at: $(which scribble)"

# Create desktop entry
echo "🖥️  Creating desktop entry..."
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR"

cat > "$DESKTOP_DIR/scribble.desktop" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Scribble
Comment=A powerful terminal-based note-taking app with Obsidian vault support
Exec=sh -c 'cd && scribble'
Icon=accessories-text-editor
Terminal=true
Categories=Office;TextEditor;Utility;Development;
Keywords=notes;markdown;terminal;obsidian;vault;writing;tags;
MimeType=text/markdown;text/plain;
StartupNotify=true
EOF

chmod +x "$DESKTOP_DIR/scribble.desktop"
echo "✅ Desktop entry created at: $DESKTOP_DIR/scribble.desktop"

# Create a wrapper script for easy access
WRAPPER_SCRIPT="$HOME/.local/bin/scribble-gui"
mkdir -p "$HOME/.local/bin"

cat > "$WRAPPER_SCRIPT" << 'EOF'
#!/bin/bash
# Scribble GUI wrapper - opens scribble in a new terminal window

if command -v ghostty &> /dev/null; then
    ghostty --title="Scribble" -e scribble "$@"
elif command -v gnome-terminal &> /dev/null; then
    gnome-terminal --title="Scribble" -- scribble "$@"
elif command -v konsole &> /dev/null; then
    konsole --title "Scribble" -e scribble "$@"
elif command -v xfce4-terminal &> /dev/null; then
    xfce4-terminal --title="Scribble" --command="scribble $*"
elif command -v alacritty &> /dev/null; then
    alacritty --title "Scribble" -e scribble "$@"
elif command -v kitty &> /dev/null; then
    kitty --title "Scribble" scribble "$@"
else
    # Fallback to running in current terminal
    scribble "$@"
fi
EOF

chmod +x "$WRAPPER_SCRIPT"
echo "✅ GUI wrapper created at: $WRAPPER_SCRIPT"

# Create default config directory
CONFIG_DIR="$HOME/.config/scribble"
mkdir -p "$CONFIG_DIR"
echo "📁 Config directory created at: $CONFIG_DIR"

# Create example config if it doesn't exist
CONFIG_FILE="$CONFIG_DIR/config.toml"
if [[ ! -f "$CONFIG_FILE" ]]; then
    cat > "$CONFIG_FILE" << 'EOF'
# Scribble Configuration File
# This file is automatically created and can be customized

[editor]
# External editor command (defaults to $EDITOR environment variable)
# external_editor = "code"  # Use VS Code
# external_editor = "hx"    # Use Helix  
# external_editor = "nvim"  # Use Neovim

[ui]
theme = "tokyo-night"
preview_enabled = false
auto_save = true

[vaults]
# Auto-detect Obsidian vaults in current directory
auto_detect = true

# Default vault path (optional)
# default = "/path/to/your/vault"

# List of recent vaults (managed automatically)
recent = []

[behavior]
# Backup before import operations
backup_before_import = true

# Number of recent files to remember
max_recent_files = 10

# Auto-save interval in seconds
auto_save_interval = 5
EOF

    echo "📝 Example config created at: $CONFIG_FILE"
    echo "💡 You can customize Scribble by editing this file"
fi

# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
    echo "🔄 Updated desktop database"
fi

echo ""
echo "🎉 Scribble installation complete!"
echo ""
echo "📋 Quick Start:"
echo "  • Run 'scribble' in terminal"
echo "  • Run 'scribble --vault /path/to/obsidian/vault' for vault mode"
echo "  • Press '?' in the app for comprehensive help"
echo "  • Look for Scribble in your applications menu"
echo ""
echo "🔧 Configuration:"
echo "  • Config file: $CONFIG_FILE"
echo "  • Desktop entry: $DESKTOP_DIR/scribble.desktop"
echo "  • GUI wrapper: $WRAPPER_SCRIPT"
echo ""
echo "💡 Pro Tips:"
echo "  • Use Ctrl+T for tag management"
echo "  • Use Ctrl+J for quick note jumping"  
echo "  • Use Ctrl+V for vault switching"
echo "  • External editors work seamlessly with vault mode"
echo ""
echo "Happy note-taking! 📝✨"