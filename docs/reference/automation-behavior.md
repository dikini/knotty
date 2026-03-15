# Automation Behavior Reference

## Purpose

Define the gated semantic automation and parity verification surface expected from `knot-gtk`.

## Automation Goal

Favor semantic state capture and semantic UI actions over raw widget scraping so parity checks, `knotd`, and LLM-driven helpers can survive GTK implementation changes.

## Availability Gate

- automation is disabled by default
- automation becomes available only when both are true:
  - local config enables automation in `~/.config/knot/knotty.toml` via `automation.enabled = true`
  - the process is started with `--enable-automation --automation-token <TOKEN>`
- GTK must surface a visible automation-active indicator when automation is live

## Discovery Surface

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiAutomationDescription {
    pub protocol_version: u32,
    pub snapshot_schema_version: u32,
    pub action_catalog_version: u32,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub requires_config_opt_in: bool,
    pub requires_runtime_token: bool,
    pub actions: Vec<UiAutomationActionDescription>,
    pub result_codes: Vec<String>,
}
```

## Semantic Snapshot Type

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiAutomationSnapshot {
    pub active_tool: String,
    pub active_content: String,
    pub startup_state: String,
    pub inspector_visible: bool,
    pub active_note_path: Option<String>,
    pub editor_mode: Option<String>,
    pub editor_dirty: bool,
    pub search_query: Option<String>,
    pub graph_scope: Option<String>,
    pub graph_depth: Option<u8>,
    pub graph_selected_path: Option<String>,
    pub settings_section: Option<String>,
    pub automation_active: bool,
    pub properties: std::collections::BTreeMap<String, String>,
}
```

## Action Surface

```rust
pub enum UiAutomationAction {
    SwitchTool { tool: String },
    FocusSearch,
    SelectNote { path: String },
    ClearSelection,
    SetEditorMode { mode: String },
    OpenSettingsSection { section: String },
    SetGraphScope { scope: String },
    SetGraphDepth { depth: u8 },
    ResetGraph,
}
```

## Action Result Shape

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiAutomationActionResult {
    pub action_id: String,
    pub ok: bool,
    pub result_code: String,
    pub message: Option<String>,
    pub snapshot: Option<UiAutomationSnapshot>,
}
```

## Required Automation Behaviors

- stable view identifiers for major shell surfaces
- semantic state export for daemon callers and parity tests
- discoverable protocol metadata and action schemas
- semantic action execution for navigation and mode changes
- no dependence on display-specific pixel coordinates for core workflow verification
- manual smoke checklist docs for slices that are difficult to verify headlessly

## Snapshot Update Rules

- update snapshot state whenever shell routing changes
- update selected note on shared note-open success
- update startup state whenever startup routing changes
- update editor mode and dirty state whenever editor state changes
- update search query and results mode on search interactions
- update graph-selected path on graph node selection
- update graph scope/depth on graph context changes
- update settings section when settings navigation changes

## Test Cases

- automation remains unavailable unless both config opt-in and runtime token are present
- discovery reports action catalog and gating status
- selecting settings tool updates snapshot
- activating a search result updates selected note path
- graph selection updates graph-selected path
- note switch with dirty-state denial does not update selected note path
- unsupported actions return stable result codes instead of generic errors
