# GTK App Shell, Startup, Navigation, and Search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** replace the prototype window shell with a stable GTK app shell covering startup states, tool routing, inspector routing, and search behavior.

**Architecture:** build a thin shell state layer around the existing window and panel widgets, then incrementally wire startup states, tool routing, and search interactions. Keep shell logic centralized so later slices do not duplicate mode-switch rules.

**Tech Stack:** Rust, gtk4, libadwaita, glib, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/shell-behavior.md` and `docs/reference/knotd-protocol.md`
- Treat this slice as shell-state work, not explorer or editor work.
- Keep view-routing logic centralized so later slices can depend on it.

## Delivery Notes

- Follow TDD for each task.
- Keep search in this slice because it depends on shell routing and focus behavior.
- Do not pull explorer mutation logic into this slice.

## Rust Guidance For This Slice

- Represent shell modes with small enums instead of stringly typed helpers.
- Prefer pure state-transition functions for routing rules so they are easy to test.
- Keep widget-building code separate from routing decisions when possible.

## knotd Calls Used By This Slice

### Startup-state related calls

- `is_vault_open`
- `get_vault_info`
- optionally `open_vault` or `create_vault` if shell actions are wired in this slice

### Search-related call

- `search_notes`

### Search request example

```json
{
  "name": "search_notes",
  "arguments": {
    "query": "graph",
    "limit": 10
  }
}
```

### Search response type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub excerpt: String,
    pub score: f64,
}
```

### Search communication sequence

1. user types in `SearchEntry`
2. debounce timer expires
3. shell or search view enters loading state
4. background worker calls `client.search_notes(query, Some(limit))`
5. result returns to main thread
6. UI updates result list or error state
7. activating a result delegates to the shared note-load path

## Suggested Task Ownership

- One developer can own shell state and routing.
- One developer can own startup-state rendering.
- One developer can own search behavior.

After `GTS-001`, `GTS-002` and `GTS-005` can proceed in parallel if file ownership is coordinated.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTS-001 | Add shell-state model and unit tests | - |
| GTS-002 | Implement startup-state rendering | GTS-001 |
| GTS-003 | Wire tool/context/content routing rules | GTS-002 |
| GTS-004 | Wire inspector details/settings modes | GTS-003 |
| GTS-005 | Strengthen search focus, empty, and activation flows | GTS-003 |
| GTS-006 | Add review fixes and full verification | GTS-004, GTS-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTS-001A | GTS-001 | Add failing shell mode transition tests | `src/ui/shell_state.rs` | `src/ui/editor.rs` |
| GTS-001B | GTS-001 | Implement tool/context/content enums | `src/ui/shell_state.rs` | `src/ui/window.rs` |
| GTS-001C | GTS-001 | Export shell-state module | `src/ui/mod.rs` | `src/client/*` |
| GTS-002A | GTS-002 | Add failing startup decision tests | `src/ui/window.rs` | `src/ui/search_view.rs` |
| GTS-002B | GTS-002 | Add daemon-unavailable state widget | `src/ui/window.rs` | `src/ui/tool_rail.rs` |
| GTS-002C | GTS-002 | Add no-vault state widget | `src/ui/window.rs` | `src/ui/context_panel.rs` |
| GTS-003A | GTS-003 | Add failing routing tests for notes/search/graph | `src/ui/window.rs`, `src/ui/context_panel.rs` | `src/ui/editor.rs` |
| GTS-003B | GTS-003 | Wire tool rail into shell state | `src/ui/tool_rail.rs`, `src/ui/window.rs` | `src/ui/explorer.rs` |
| GTS-003C | GTS-003 | Wire content stack visibility rules | `src/ui/window.rs` | `src/ui/editor.rs` |
| GTS-004A | GTS-004 | Add failing inspector mode tests | `src/ui/inspector_rail.rs`, `src/ui/window.rs` | `src/ui/settings_view.rs` |
| GTS-004B | GTS-004 | Implement inspector details/settings modes | `src/ui/inspector_rail.rs`, `src/ui/window.rs` | `src/ui/editor.rs` |
| GTS-005A | GTS-005 | Add failing search shortcut/focus tests | `src/ui/search_view.rs`, `src/main.rs` | `src/ui/explorer.rs` |
| GTS-005B | GTS-005 | Add failing empty/error state tests | `src/ui/search_view.rs` | `src/ui/window.rs` |
| GTS-005C | GTS-005 | Implement result activation through shared path | `src/ui/search_view.rs`, `src/ui/window.rs` | `src/client/*` |
| GTS-006A | GTS-006 | Run slice verification | repo-wide | - |
| GTS-006B | GTS-006 | Fix slice-only regressions | touched files only | unrelated modules |

### Review Follow-up Checks

- Shared note-load completion must only route back to Notes when the initiating context requires it.
- Search-result note loads may return to Notes; sidebar/context note loads must not infer that from current tool mode alone.

### Task GTS-001: Add shell-state model and unit tests

**Files**
- Create: `/home/dikini/Projects/knot-gtk/src/ui/shell_state.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/mod.rs`

**Steps**
1. Write unit tests describing shell tool mode, main content mode, and inspector mode transitions.
2. Confirm red.
3. Implement the smallest shell-state model that can answer “what should be visible now?”
4. Re-run targeted tests until green.
5. Review for overengineering; this should be a small state holder, not a framework.

**Example test skeleton**

```rust
#[test]
fn selecting_graph_tool_switches_context_panel_to_graph() {
    let mut shell = ShellState::default();
    shell.select_tool(ToolMode::Graph);
    assert_eq!(shell.tool_mode(), ToolMode::Graph);
    assert_eq!(shell.context_mode(), ContextMode::Graph);
}
```

### Task GTS-002: Implement startup-state rendering

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add tests for daemon unavailable, no vault, and vault-open startup decisions.
2. Confirm red.
3. Add dedicated empty/startup widgets instead of header text only.
4. Re-run targeted tests until green.
5. Review text and action labels for clarity.

**Implementation notes**
- It is acceptable to start with buttons that call existing contract methods or placeholders if the daemon contract is missing a native dialog path.
- The key requirement is actionable startup states.

**Advice**

- Make each startup state visually distinct.
- Store state decision logic in a helper so tests do not need to instantiate the full window.

### Task GTS-003: Wire tool/context/content routing rules

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/tool_rail.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add tests for mode routing: notes, search, graph, settings.
2. Confirm red.
3. Apply shell-state rules to the window and panels.
4. Re-run targeted tests until green.
5. Review for duplicated switch statements; consolidate only the repeated routing logic.

**Advice**

- Define one authoritative function for “given shell state, what content should be visible”.
- Later slices should call into that logic, not recreate it.

### Task GTS-004: Wire inspector details/settings modes

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/inspector_rail.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Write tests for inspector hidden/open, details mode, and settings mode.
2. Confirm red.
3. Replace placeholder-only inspector behavior with shell-driven mode switching.
4. Re-run targeted tests until green.
5. Review for naming consistency with the shell-state model.

**Advice**

- Even if inspector details content remains simple, make the mode explicit now.
- This prevents a later “placeholder widget” trap.

### Task GTS-005: Strengthen search focus, empty, and activation flows

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/search_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/main.rs`

**Steps**
1. Add tests for focus shortcut, empty states, keyboard navigation, and result activation.
2. Confirm red.
3. Implement the missing shortcut/focus wiring, move `search_notes` off the GTK thread via the async bridge, and complete explicit `idle`/`loading`/`empty`/`results`/`error` state transitions.
4. Re-run targeted tests until green.
5. Review debounce behavior and ensure search activation delegates to the central note-load path.

**Example test skeleton**

```rust
#[test]
fn escape_clears_search_results_state() {
    let mut state = SearchState::with_results(vec!["notes/a.md".into()]);
    state.handle_escape();
    assert!(state.query().is_empty());
    assert!(state.results().is_empty());
}
```

### Task GTS-006: Add review fixes and full verification

**Steps**
1. Run `cargo fmt`.
2. Run targeted shell tests.
3. Run `cargo test`.
4. Manual smoke-check tool switching and search if a GTK environment is available.
5. Fix slice-introduced warnings and routing regressions.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Shell state owns routing, not scattered callbacks
- Startup states are actionable
- Vault-info lookup failures degrade to a connected shell state instead of blocking navigation
- Startup-only recovery surfaces cannot be bypassed by the search shortcut
- Startup shortcut guards do not make fresh blocking daemon RPCs on the GTK thread
- Search result activation goes through one note-open path
- Late note-load results do not override a newer Graph or Settings navigation choice
- Inspector modes are explicit
- Search has user-visible empty and error states
- Clearing an already-empty search query does not leave `search_changed` suppression stuck on
- Refreshed startup diagnostics update the daemon-unavailable detail text

## Commit Gate

Commit only when the full verification commands are green.
