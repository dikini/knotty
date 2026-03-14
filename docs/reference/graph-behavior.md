# Graph Behavior Reference

## Purpose

Define the graph payloads and the user-facing behavior expected from the GTK graph surface.

## Graph Tool Calls

- `get_graph_layout`
- `graph_neighbors`
- optionally `get_note` when selection activates a note

## Graph Layout Types

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphLayout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub path: String,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub degree: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}
```

## Request Examples

```json
{
  "name": "get_graph_layout",
  "arguments": {
    "width": 1200,
    "height": 800
  }
}
```

```json
{
  "name": "graph_neighbors",
  "arguments": {
    "path": "notes/example.md",
    "depth": 1
  }
}
```

## Required Graph Behaviors

- render the daemon-provided layout rather than recalculating graph physics in GTK
- support global graph and focused-neighborhood graph flows
- allow node selection and note activation
- show graph context details in the shared context panel
- handle empty graph and graph error states explicitly

## Graph Interaction Rules

- single select node highlights the selected item and updates graph context
- activate node opens the corresponding note through the shared note-open path
- graph scope changes reload layout through daemon calls
- graph rendering style does not need to match Tauri pixel-for-pixel

## Test Cases

- selecting graph tool displays graph content mode
- selecting a node updates graph context panel state
- activating a node uses the central note-open path
- empty graph payload shows explicit empty state
