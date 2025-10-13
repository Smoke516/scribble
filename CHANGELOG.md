# Changelog

All notable changes to Scribble will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2024-10-13

### 🎉 Major New Features

#### 🏷️ Advanced Tag Management System
- **Tag Browser**: Press `Ctrl+T` to open comprehensive tag management interface
- **Dual Format Support**: Both YAML frontmatter (`tags: [example, test]`) and inline hashtags (`#example #test`)
- **Smart Tag Detection**: Automatic parsing and indexing of tags across all notes
- **Tag Analytics**: Frequency statistics and usage insights
- **Interactive Filtering**: Filter notes by tags with visual feedback
- **Sort Modes**: Toggle between frequency-based and alphabetical sorting
- **Quick Selection**: Number keys (1-9) for instant tag selection
- **Visual Indicators**: Active filter display with checkmarks

#### 📁 Obsidian Vault Integration
- **Native Vault Support**: Seamless compatibility with Obsidian vault formats
- **Real-time File Watching**: Automatic detection of external file changes
- **Live Sync**: External changes appear instantly in Scribble
- **Vault Switching**: Press `Ctrl+V` to switch between multiple vaults
- **Auto-detection**: Automatically detects vaults in current/parent directories
- **YAML Frontmatter**: Full metadata preservation and compatibility
- **Wiki Links**: Complete `[[note linking]]` support with navigation

#### 🔄 File System Monitoring
- **Real-time Sync**: Live file watching with notification system
- **External Change Detection**: Visual indicators for external modifications
- **Sync Status**: Status bar shows file watching state
- **Change Notifications**: Pop-up alerts for file events
- **Multi-format Support**: Handles create, modify, delete, and rename events

### ✨ Enhanced User Experience

#### 📖 Comprehensive Help System
- **Complete Feature Guide**: Press `?` for exhaustive documentation
- **Organized Sections**: Grouped by feature category for easy navigation
- **Visual Formatting**: Rich colors, emojis, and clear hierarchy
- **Pro Tips**: Advanced usage suggestions and workflow optimization
- **All Keybindings**: Complete reference for every shortcut and command

#### 🎨 UI/UX Improvements
- **Enhanced Status Bar**: File watcher status and sync indicators
- **Mode Integration**: New modes for tag browser and vault switcher
- **Visual Feedback**: Operation results with icons and colors
- **Better Navigation**: Improved keyboard shortcuts and discoverability
- **Professional Polish**: Consistent theming and user experience

### 🔧 Technical Enhancements

#### ⚡ Performance & Architecture
- **Modular Design**: New dedicated modules for tags, watching, and configuration
- **Efficient Indexing**: Fast tag lookup and filtering algorithms  
- **Non-blocking I/O**: File watching doesn't impact app performance
- **Memory Optimization**: Efficient data structures for large vaults
- **Error Handling**: Robust error management for file operations

#### 🛠️ Configuration System
- **Extended Config**: New settings for vaults, tags, and behavior
- **Auto-detection**: Smart defaults for common scenarios
- **Recent Vaults**: Automatic tracking of vault usage
- **Customization**: Extensive personalization options

### 🚀 Installation & Deployment

#### 📦 Enhanced Installation
- **Automated Install Script**: `./install.sh` for complete setup
- **Desktop Integration**: Applications menu entry and GUI wrapper
- **Configuration Setup**: Automatic config directory creation
- **PATH Management**: Proper binary installation and access

#### 📋 Documentation
- **Installation Guide**: Comprehensive `INSTALL.md` with troubleshooting
- **Feature Documentation**: Detailed guides for all new features
- **Usage Examples**: Practical examples and workflows
- **System Integration**: Desktop environment setup instructions

### 🔄 Migration from 1.x

- **Backward Compatible**: All existing notes and folders preserved
- **Automatic Upgrade**: Seamless transition to new features
- **Enhanced Functionality**: Existing features work better with new systems
- **No Breaking Changes**: All previous keybindings and workflows preserved

### 📊 What's New by the Numbers

- **3 New Major Features**: Tag management, vault integration, file watching
- **15+ New Keybindings**: Enhanced navigation and functionality
- **4 New UI Modes**: Tag browser, vault switcher, enhanced help
- **50+ New Functions**: Expanded codebase with robust feature set
- **100% Vault Compatible**: Full Obsidian ecosystem integration

---

## [1.0.0] - 2024-09-26

### Initial Release Features

#### 📝 Core Note Management
- Hierarchical folder organization
- Rich markdown editing with syntax highlighting
- External editor integration (Helix, Neovim, VS Code)
- Auto-save functionality
- Search and filtering capabilities

#### 🎨 User Interface
- Tokyo Night theme
- Two-pane layout (folders + editor)
- Live markdown preview
- Vim-inspired keybindings
- Modal editing experience

#### 🔍 Navigation & Search
- Fuzzy search with typo tolerance
- Quick jump navigation (`Ctrl+J`)
- Recent files access (`Ctrl+O`)
- Wiki-style note linking (`[[links]]`)

#### 💾 Data Management
- JSON-based storage
- Import/export functionality
- Backup system
- Conflict resolution

#### ⚡ Productivity Features
- Smart autocompletion
- Undo/restore from trash
- Session persistence
- Multiple editing modes

---

## Version Numbering

- **Major version** (X.0.0): Breaking changes or major new features
- **Minor version** (0.X.0): New features, backward compatible
- **Patch version** (0.0.X): Bug fixes and minor improvements

## Links

- [GitHub Repository](https://github.com/username/scribble)
- [Installation Guide](INSTALL.md)
- [User Documentation](README.md)