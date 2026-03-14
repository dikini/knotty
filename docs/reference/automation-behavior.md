# Automation Behavior Reference

## Purpose

Define the semantic automation and parity verification surface expected from `knot-gtk`.

## Automation Goal

Favor semantic state capture over raw widget scraping so parity checks can survive GTK implementation changes.

## Semantic Snapshot Type

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiAutomationSnapshot {
    pub shell_tool_mode: String,
    pub content_mode: String,
    pub context_mode: String,
    pub inspector_mode: String,
    pub selected_note_path: Option<String>,
    pub current_query: Option<String>,
    pub graph_selected_path: Option<String>,
    pub settings_section: Option<String>,
}
```

## Required Automation Behaviors

- stable view identifiers for major shell surfaces
- semantic state export for parity tests
- no dependence on display-specific pixel coordinates for core workflow verification
- manual smoke checklist docs for slices that are difficult to verify headlessly

## Snapshot Update Rules

- update snapshot state whenever shell routing changes
- update selected note on shared note-open success
- update search query and results mode on search interactions
- update graph-selected path on graph node selection
- update settings section when settings navigation changes

## Test Cases

- selecting settings tool updates snapshot
- activating a search result updates selected note path
- graph selection updates graph-selected path
- note switch with dirty-state denial does not update selected note path
