# GTK Graph Design Notes

## Purpose

Capture the local design choices for `COMP-GTK-GRAPH-006` before implementation details drift from the daemon contract.

## Locked Decisions

- Vault graph uses `get_graph_layout` directly and renders daemon-provided positions.
- Neighborhood graph uses `graph_neighbors`, which currently returns node-path strings plus edges rather than positioned node objects.
- GTK normalizes both payload shapes into one internal scene model so rendering, selection, and context-details logic stay shared.
- When a neighborhood payload omits positions, GTK reuses vault-graph positions when available and otherwise falls back to a simple generated layout.
- Graph selection updates shared context details; note activation routes through the existing note-loading path and dirty-note prompt logic.
- Reset restores vault scope, depth `1`, and clears the focused node.

## Implications

- The graph context panel can derive neighbors and backlinks from the normalized scene instead of depending on a separate details payload.
- Graph rendering does not need to match the Tauri surface pixel-for-pixel, but routing and interaction semantics must match the frozen references.
