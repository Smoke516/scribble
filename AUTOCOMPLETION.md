# Markdown Autocompletion Feature

## Overview

The Scribble editor now includes intelligent markdown autocompletion that triggers as you type common markdown patterns. This feature helps speed up markdown writing by automatically suggesting completions for headers, lists, code blocks, and other markdown elements.

## How It Works

- **Automatic Triggering**: As you type specific markdown triggers (like `#`, `-`, `*`, etc.), a popup will appear showing available completions
- **Navigation**: Use `↑` and `↓` arrow keys to navigate through suggestions
- **Application**: Press `Tab` or `Enter` to apply the selected suggestion
- **Cancellation**: Press `Escape` to close the autocompletion popup

## Supported Completions

### Headers
- Type `#` → Suggests "# " (Heading 1)
- Type `##` → Suggests "## " (Heading 2)  
- Type `###` → Suggests "### " (Heading 3)

### Lists
- Type `-` → Suggests "- " (Bullet list item)
- Type `*` → Suggests "* " (Bullet list item - alternative)
- Type `1.` → Suggests "1. " (Numbered list item)

### Checkboxes
- Type `- [` → Suggests "- [ ] " (Unchecked todo item)
- Type `- [x` → Suggests "- [x] " (Checked todo item)

### Code
- Type ``` → Suggests code block with proper formatting
- Type ` → Suggests inline code with backticks

### Emphasis
- Type `**` → Suggests bold text formatting
- Type `*` → Suggests italic text formatting

### Links and Images
- Type `[` → Suggests link format "[](url)"
- Type `![` → Suggests image format "![alt text](image.png)"

### Other Elements
- Type `>` → Suggests "> " (Blockquote)
- Type `|` → Suggests complete table structure
- Type `---` → Suggests horizontal rule

## Smart Cursor Positioning

Many completions include smart cursor positioning:
- **Code blocks**: Cursor positioned inside the block
- **Emphasis**: Cursor positioned between the markers
- **Links**: Cursor positioned for easy text entry
- **Tables**: Cursor positioned at the first header cell

## Usage Tips

1. **Context Aware**: Autocompletion only triggers at the beginning of lines or after spaces for most patterns
2. **Immediate Feedback**: The popup appears instantly when a trigger pattern is detected
3. **Non-intrusive**: If you don't want to use a suggestion, simply continue typing or press Escape
4. **Visual Indicators**: Each suggestion shows an appropriate icon and description

## Integration with Editor

The autocompletion feature is seamlessly integrated with the editor's insert mode:
- Works alongside existing vim-like key bindings
- Respects the current cursor position and scroll state
- Automatically saves content when suggestions are applied
- Compatible with syntax highlighting and preview mode

This feature significantly improves the markdown editing experience by reducing typing overhead and ensuring consistent formatting across your notes.
