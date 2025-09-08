# Search Navigation Feature

## Overview

The Scribble editor includes intelligent search functionality that not only finds matching notes but automatically navigates to the first search result, providing a seamless search and navigation experience.

## How It Works

When you perform a search, Scribble will:

1. **Find all matching notes** using both basic and enhanced search algorithms
2. **Automatically open the first result** in the editor
3. **Navigate the tree view** to highlight the opened note
4. **Expand folders** if the note is contained within a collapsed folder
5. **Focus the editor** so you can immediately start reading or editing
6. **Display search statistics** showing how many results were found

## Usage

### Basic Search

1. **Press `/`** to enter search mode
2. **Type your search query** (searches in note titles, content, and tags)
3. **Press Enter** to execute the search
4. **First result automatically opens** in the editor
5. **Tree view highlights** the opened note

### Search Features

- **Multi-field search**: Searches note titles, content, and tags simultaneously
- **Case-insensitive**: Search is not case-sensitive by default
- **Partial matching**: Finds partial word matches
- **Relevance ranking**: Results are sorted by relevance (title matches first, then by number of matches)
- **Search history**: Previous searches are remembered for easy reuse

## User Feedback

After a successful search, you'll see a status message like:
```
Found 3 notes with 7 matches for 'markdown' - Opened first result: 'Markdown Guide'
```

If no results are found:
```
No matches found for 'nonexistent'
```

## Advanced Search Features

The search system includes an enhanced search engine that provides:

- **Match highlighting**: Identifies exact locations of matches
- **Context awareness**: Understands different types of content (headers, code blocks, etc.)
- **Structured results**: Organizes results by match type and relevance
- **Performance optimization**: Fast search even with many notes

## Navigation Behavior

### Tree View Updates
- Selected note is automatically highlighted in the tree
- Parent folders are expanded to show the note location
- Tree scroll position adjusts to keep the selected note visible

### Editor State
- Note content is loaded immediately
- Cursor is positioned at the beginning of the note
- Editor is focused and ready for editing
- Scroll position is reset to the top

### Folder Expansion
- If a found note is in a collapsed folder, the folder automatically expands
- Tree view refreshes to show the expanded structure
- Selection is maintained on the found note after refresh

## Search Tips

1. **Use specific terms**: More specific search terms yield more relevant results
2. **Try different keywords**: If you don't find what you're looking for, try synonyms
3. **Search in titles**: Notes with matching titles are prioritized in results
4. **Use tags**: Add and search for tags to organize and find notes more effectively

## Integration with Editor

The search navigation feature integrates seamlessly with other editor features:

- **Works with preview mode**: Search results open in both editor and preview
- **Compatible with vim-like navigation**: Use normal vim keys after opening a result
- **Respects editor modes**: Returns to normal mode after opening a search result
- **Maintains undo history**: Search navigation doesn't affect undo/redo

## Performance

The search system is optimized for:
- **Fast indexing**: Notes are efficiently indexed for quick searches
- **Memory efficiency**: Search results are cached intelligently
- **Real-time updates**: Search index updates when notes are modified
- **Scalability**: Performs well even with hundreds of notes

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `/` | Enter search mode |
| `Enter` | Execute search and navigate to first result |
| `Esc` | Cancel search (while in search mode) |
| `n` | Navigate to next search result (planned feature) |
| `N` | Navigate to previous search result (planned feature) |

This automatic navigation feature significantly improves the note-taking workflow by eliminating the manual step of locating and opening search results, making information retrieval fast and efficient.
