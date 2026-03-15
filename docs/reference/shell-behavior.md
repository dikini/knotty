# Shell Behavior Reference

## Purpose

Define the expected startup, routing, inspector, and search behavior for the GTK shell.

## Startup States

| State | Condition | Visible Surface | Required Action |
|---|---|---|---|
| daemon unavailable | cannot connect to `knotd` | startup error panel | retry |
| no vault open | daemon responds but no vault is active | no-vault panel | open vault or create vault |
| vault open | active vault exists | full shell | load notes/search/context |

## Shell Routing Model

### Tool Modes

- `notes`
- `search`
- `graph`
- `settings`

### Context Modes

- `notes`
- `search`
- `graph`
- `settings`
- `empty`

### Content Modes

- `welcome`
- `note`
- `search`
- `graph`
- `settings`
- `error`

### Inspector Modes

- `hidden`
- `details`
- `settings`

## Routing Rules

| Tool Mode | Context Mode | Content Mode | Inspector Default |
|---|---|---|---|
| notes | notes | note or welcome | details |
| search | search | search | hidden |
| graph | graph | graph | details |
| settings | settings | settings | settings |

## Search Behavior

### Required Rules

- search is part of shell routing, not a separate app mode outside shell state
- search runs on a background worker
- search shows explicit `idle`, `loading`, `empty`, `results`, and `error` states
- selecting a result routes through the central note-open path
- keyboard navigation and activation work without mouse-only flows

### Search Result Type

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub excerpt: String,
    pub score: f64,
}
```

### Search Request Example

```json
{
  "name": "search_notes",
  "arguments": {
    "query": "graph",
    "limit": 10
  }
}
```

## Shell Test Cases

- selecting the graph tool changes both context and content routing
- escape in search clears search query and results state
- no-vault startup does not show note editor chrome
- result activation reuses shared note loading
- settings tool hides the inspector and routes section navigation into the left context pane
