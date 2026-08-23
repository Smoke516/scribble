# Changelog

All notable changes to Scribble will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### 🐛 Fixed — the markdown preview

An audit of the preview renderer, which had no tests, found eleven defects.
Three made a documented feature useless:

- **Nested lists collapsed onto one line.** Three levels of `-` rendered as
  `• abc`; nested task lists went the same way.
- **Obsidian-style callouts never rendered.** `[!note]` is a link reference to
  CommonMark, so the parser splits it and the marker was matched against the
  first fragment alone. Every callout showed its raw marker.
- **Table columns were not aligned**, and inline code in a cell leaked out of the
  table as a stray line beneath it.

The rest were visible but survivable: doubled blockquote bars (`▌ ▌ text`, and
four at depth two), a code fence that opened `╭──────── rust` and never lined up
with its own foot, two blank lines after every heading, a missing bullet on any
item starting with inline code or a link, a bullet on the second paragraph of an
item, inline code padding itself with spaces that doubled the ones around it, and
footnote definitions rendered as bare paragraphs.

### 🐛 Fixed — silent data loss

- **`Enter` in the editor jumped to another note**, and took up to two seconds of
  typing with it. The binding activated whatever the folder tree had selected
  regardless of which pane had focus — and the tree's selection does not follow
  the palette, the task panel, the outline or the landing page, so it usually
  pointed at some other note. `select_note` overwrites the buffer and clears the
  undo stack, and the autosave debounce is two seconds, so anything typed inside
  that window was gone. Activating the tree's selection is now the tree's
  business; in the editor `Enter` moves down a line to its first non-blank, as
  vim's `<CR>` does.

- **The tree's highlight did not follow the note you opened.** Only the routes
  through `open_note_by_id` pointed it; opening from the landing page, its `1`-`9`
  shortcuts, the explorer, or creating a note left the highlight on an unrelated
  row. Tab into the folder pane, press `Enter`, and you were reading a note you
  never asked for — with the same two-second autosave window at stake. Opening a
  note now points the tree at it, whichever route opened it.
- **Revealing a note inside a collapsed folder revealed nothing.**
  `navigate_to_note` looked for the note's row before expanding the folder that
  hid it, and a collapsed folder has no row to find. It expands first now, all
  the way up a nested chain.

### ✨ Changed

- Frontmatter is hidden in the preview rather than rendered as a rule and a
  heading.
- Smart punctuation is off: the preview shows the quotes and dashes you typed.
- A blockquote spanning two paragraphs keeps its bar across the gap.

### 🧹 Removed

- A second markdown formatter, `App::render_markdown_preview`, whose output no
  one read. It ran from seventeen call sites including every keystroke in insert
  mode, reformatted the whole note, and threw the result away.

Tests went 234 → 260: twenty-six for the preview, where there were none.

## [3.1.0] - 2026-08-16

Conflicts can be resolved in the app, and an audit of everything that already
claimed to work found fifteen features that did not. Five of those were losing
data.

### ✨ Added

- **Resolve conflicts** (`Ctrl+P` → "resolve conflicts"). The two versions side by
  side with differing lines marked, and three answers: keep yours, take theirs, or
  keep both with the copy promoted to an ordinary note. Every outcome leaves no
  marker behind, so a resolved conflict stays resolved. Nextcloud's
  `(conflicted copy)` and Syncthing's `.sync-conflict-` files resolve here too.
  Unresolved conflicts are announced on the landing page.
- The open note's tags in the status line.

### 🐛 Fixed — silent data loss

Each of these reported success and wrote nothing to disk:

- **Search and replace** said "Replaced N occurrences" and queued no write. It
  persisted only when something else had already marked the note dirty, so it
  worked most of the time.
- **Undo delete** said "Restored note: X" and put it back in the list while the
  file stayed deleted — the note was gone at the next launch.
- **Import** said "Import completed: N successful" and left the vault untouched.

### 🐛 Fixed — doing the wrong thing quietly

- **Renaming a note** rewrote the frontmatter title but left the file under its old
  name, so filename and title diverged permanently.
- **The external editor** was handed a copy in `/tmp` without frontmatter, so its
  file tree, git status, project search and LSP saw nothing of the vault. It now
  opens the real file, with anything unsaved flushed first and the note reloaded
  after.
- **Eight config settings were ignored entirely** — `editor.default`,
  `ui.preview_width`, `behavior.auto_save`, `file_watching`, `spell_check` and
  `backup_on_import` (whose backup ran unconditionally), plus two governing
  features that no longer exist, now removed.
- **Templates** were gated so the obvious route in — create a note, then press `N`
  — did nothing at all.
- **The palette** printed `Ctrl+P` as the shortcut for live preview, after 3.0.0
  moved preview to `F2`.
- The help screen taught `:spell on`, which the parser rejected.
- `:export` could panic while reporting success.
- The status bar measured its width in bytes, so a multi-byte tag pushed the
  left-hand side off screen.

### 🗑️ Removed

- The **session persistence** claim from the README. Nothing restored a note at
  startup; the feature never existed.

### 🛡️ Guards

Three new tests, each verified to fail when the thing it guards is broken: every
config setting must be read somewhere, every palette command must print the chord
the keymap binds, and every Ctrl chord must appear in the help screen.

### 🧪 Internal

- Tests 217 → 234 in this release; 67 → 234 across the day.

## [3.0.1] - 2026-08-16

Fixes for two things 3.0.0 shipped broken.

### 🐛 Fixed

- **Opening the tag dialog destroyed the note's tags.** `sync_note_tags` assigned
  the tags extracted from `note.content` over `note.tags`, and storage strips
  frontmatter out of `content` before storing it — so the extraction found nothing
  and every frontmatter tag was dropped. A tag added through the dialog survived
  exactly until the next time you opened the dialog, and the next save wrote that
  loss to disk. It merges now, and sorts, so the `tags:` line stops being
  rewritten in a different order on every save.
- **The tag dialog disagreed with the rest of the app**, reading only the
  frontmatter field — so it could report no tags on a note the browser and the
  palette both listed as tagged. It uses the shared extractor now.
- **The palette printed the wrong shortcut for the live preview**, still showing
  `Ctrl+P` after 3.0.0 moved preview to `F2` and gave `Ctrl+P` to the palette
  itself. Since the palette exists partly to teach the chord, printing one that
  does nothing was worse than printing none. A test now walks every palette
  command against the keymap.
- The status bar's width was measured in **bytes**, so a multi-byte tag or note
  title over-reserved space and pushed the left-hand side off screen.

### ✨ Added

- The open note's tags are shown in the status line, capped at three with `+N`.
  Tagging something previously gave no sign it had worked outside the dialog you
  did it in.
- Screenshots on the README.

## [3.0.0] - 2026-08-16

A large release. Storage got the attention it needed, retrieval was consolidated
behind one door, editing became properly vim-like, and one unused feature was
removed rather than expanded.

### ⚠️ Breaking

- **Wiki links are gone.** The `[[ ]]` parser, link graph, backlinks panel,
  `[[` autocomplete and follow-link were all removed, along with `Ctrl+B` and
  `Ctrl+L`. Note content is untouched — any `[[text]]` already written stays in
  the file as ordinary text.
- **`Ctrl+P` now opens the Go to palette**, not the live preview. Preview keeps
  `F2`, which it already answered to.

### ✨ Added

- **Go to (`Ctrl+P`)** — one door in front of the six finders. Notes, tags,
  headings and commands ranked together. Prefixes: `>` commands, `#` tags,
  `?` full text, `@` headings. The original finders all still work as direct
  shortcuts.
- **Real vim operators** — `d`, `c` and `y` compose with motions, text objects
  and counts: `dw`, `ciw`, `d$`, `3dd`, `d3w`, `dG`, `daw`, plus `D`/`C`/`Y`.
  Counts work on plain motions too. Yanks remember whether they were linewise.
- **Task panel (`Ctrl+K`)** — every open task in the vault in one list, grouped
  by note. `Enter` jumps to it, `Space` ticks it in place, `a` shows completed.
- **Quick capture from the shell** — `scribble -n "..."` for a new note,
  `scribble -t "..."` to append to today's daily note, `scribble -t` to open it,
  and piped stdin. Each prints the path it wrote.
- **Live vault switching** — `Ctrl+V` switches immediately instead of asking for
  a restart, and remembers the vault as the default.
- **A file-conflict policy.** Notes carry a content hash of the file as last read
  or written, and nothing is overwritten that cannot be accounted for. Nothing
  unsaved: the disk version is taken silently. Unsaved changes: your version
  keeps the filename and the version found on disk is preserved beside it.

### 🐛 Fixed

- **CRLF notes lost their identity on every load.** The frontmatter parser
  required a literal `---\n`, so a note with Windows line endings parsed as
  having no frontmatter and was given a fresh id and creation date each time.
- **A sync client's conflicted copy could make the real note disappear.** It
  carries the original's `scribble_id`, and notes are keyed by id, so the pair
  collapsed into one — and walk order consistently favoured the copy.
- **Code comments were counted as tags.** The inline tag regex had no context
  rules, so `#` in Python, Bash and Ruby comments became tags. Against a real
  vault that was 21 tags across 5 notes before and 9 across 1 after.
- **External edits were silently overwritten**, and edits to notes that were not
  open never reached memory at all.
- `--today` and the in-app `F4` disagreed about where today's note lives, so
  using both could produce two notes for the same day.

### 🧪 Internal

- Tests 67 → 198.

## [2.1.0] - 2026-08-15

Recorded after the fact; this release was never written up at the time.

### ✨ Added
- Outline panel (`Ctrl+G`), task checkboxes (`Space`), daily notes (`F4`),
  folder-scoped advanced search, full-screen landing page, explorer overlay
  (`Ctrl+E`), and a move picker.

### 🐛 Fixed
- **Every save added a blank line to the top of every note.** The frontmatter
  delimiter is five bytes and the parser skipped four; 144 accumulated lines were
  cleaned across the vault.
- Atomic writes, panic-safe terminal restore, vault-relative `folder_path`, and a
  keymap table that fixed five silently shadowed Ctrl chords.

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