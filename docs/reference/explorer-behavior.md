# Explorer Behavior Reference

## Purpose

Define the expected tree, selection, and mutation behavior for the GTK explorer.

## Explorer Tree Payload

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplorerTree {
    pub roots: Vec<ExplorerNode>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExplorerNode {
    Folder(ExplorerFolderNode),
    Note(ExplorerNoteNode),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplorerFolderNode {
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub children: Vec<ExplorerNode>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplorerNoteNode {
    pub path: String,
    pub name: String,
    pub note_type: Option<String>,
}
```

## Required Explorer Behaviors

- load the full tree from daemon data, not from direct filesystem walking inside GTK
- preserve folder expanded state across refreshes when the daemon reports the same paths
- keep note selection stable across non-destructive refreshes when the selected path still exists
- route note activation through the shared note-loading path
- block destructive mutations if the current note has unresolved dirty-state rules

## Folder Expansion Persistence

### Request Example

```json
{
  "name": "set_folder_expanded",
  "arguments": {
    "path": "notes/projects",
    "expanded": true
  }
}
```

## Mutation Tool Calls

- `create_note`
- `rename_note`
- `delete_note`
- `rename_directory`
- `remove_directory`

## Mutation Sequence

1. user triggers explorer action
2. guard checks for unsaved current note if needed
3. GTK sends mutation tool call
4. GTK waits for daemon success
5. GTK refreshes explorer tree
6. GTK restores selection if target still exists, otherwise selects the best fallback

## Test Cases

- expanding a folder emits the correct persistence call
- refresh preserves expanded folders
- delete current note clears editor state only after daemon success
- rename updates selection to the new path
