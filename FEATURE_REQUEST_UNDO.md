# Feature Request: Undo Deleted Files

## Problem
Currently, when a user deletes a file or folder in Scribble (using `d` key), it's permanently removed with no way to recover it. This can lead to accidental data loss.

## Proposed Solution

### 1. Trash/Recycle Bin System
- Move deleted items to a "trash" folder instead of permanent deletion
- Add a "Recently Deleted" view accessible via command mode
- Auto-cleanup trash after X days (configurable)

### 2. Undo History
- Maintain a history of recent delete operations
- Add `u` key binding to undo last delete operation
- Support multiple levels of undo (configurable limit)

## Implementation Ideas

### Data Structure Changes
```rust
#[derive(Debug, Clone)]
pub struct DeletedItem {
    pub item: DeletedItemType,
    pub deleted_at: DateTime<Utc>,
    pub original_parent: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum DeletedItemType {
    Note(Note),
    Folder(Folder),
}

pub struct TrashBin {
    pub items: Vec<DeletedItem>,
    pub max_items: usize,
    pub retention_days: u32,
}
```

### Key Bindings
- `d` - Move to trash (with confirmation)
- `u` - Undo last delete
- `:trash` - Show recently deleted items
- `:restore <item>` - Restore specific item
- `:empty-trash` - Permanently delete all trash items

### UI Changes
- Add confirmation dialog for deletions
- Show trash count in status bar
- Add "Recently Deleted" section to folder tree

## Benefits
- Prevents accidental data loss
- Familiar workflow (similar to file managers)
- Maintains current delete behavior but makes it safer
- Optional feature (can be disabled for minimal mode)

## Configuration
```rust
pub struct UndoConfig {
    pub enabled: bool,
    pub max_undo_levels: usize,
    pub trash_retention_days: u32,
    pub confirm_delete: bool,
}
```

This would make Scribble much safer for daily use while maintaining its fast, keyboard-driven workflow.
