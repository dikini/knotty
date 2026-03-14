# GTK UI Automation and Parity Harnesses Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** add the observability and parity-verification hooks needed to keep `knot-gtk` aligned with the delivered slice set.

**Architecture:** add stable semantic identifiers and lightweight state export hooks to the GTK shell, then build parity-oriented tests and manual review artifacts on top of those hooks.

**Tech Stack:** Rust, gtk4, libadwaita, cargo test, parity review docs

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/automation-behavior.md`, `docs/reference/shell-behavior.md`, and `docs/reference/editor-behavior.md`
- Keep automation semantic, not fragile.
- Use stable IDs and state snapshots rather than testing random widget details.

## Delivery Notes

- Do not start this slice before shell, settings, and runtime identifiers are stable.
- Favor semantic state exposure over brittle widget traversal.

## Rust Guidance For This Slice

- Define stable identifier constants in one place if they are reused.
- Keep automation state snapshots serializable and easy to inspect in tests.
- Do not expose private widget details that are likely to churn.

## Protocol and State Notes

This slice may not require new knotd tool calls, but it should describe GTK state in a way that later parity harnesses can consume consistently.

### Recommended snapshot type

```rust
pub struct AutomationSnapshot {
    pub active_view: &'static str,
    pub active_note_path: Option<String>,
    pub tool_mode: Option<&'static str>,
    pub inspector_open: bool,
}
```

### Snapshot update sequence

1. shell or editor state changes
2. GTK updates the semantic snapshot model
3. tests and parity harnesses read semantic state, not widget internals

### Junior developer advice

- If you are about to expose a widget pointer, widget name, or child index, stop and ask whether a semantic field would be more stable.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTA-001 | Define stable automation identifiers and tests | - |
| GTA-002 | Expose semantic shell/editor/view state | GTA-001 |
| GTA-003 | Add parity-focused integration tests for delivered slices | GTA-002 |
| GTA-004 | Write manual review checklists and artifacts | GTA-003 |
| GTA-005 | Wire automation-related settings if required | GTA-002 |
| GTA-006 | Full verification and review fixes | GTA-003, GTA-004, GTA-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTA-001A | GTA-001 | Add failing identifier test for major views | `src/ui/window.rs` | editor internals |
| GTA-001B | GTA-001 | Add failing identifier test for rails/panels | `src/ui/window.rs`, `src/ui/*` | graph logic |
| GTA-001C | GTA-001 | Implement stable ID constants | touched ui files only | settings data flow |
| GTA-002A | GTA-002 | Add failing snapshot test for active view/tool mode | `src/ui/automation_state.rs`, `src/ui/window.rs` | graph renderer |
| GTA-002B | GTA-002 | Add failing snapshot test for active note/inspector state | `src/ui/automation_state.rs`, `src/ui/window.rs` | graph renderer |
| GTA-002C | GTA-002 | Implement semantic snapshot model | `src/ui/automation_state.rs`, `src/ui/window.rs` | unrelated UI widgets |
| GTA-003A | GTA-003 | Add parity test for shell/startup | `tests/parity_shell.rs` | editor/media files |
| GTA-003B | GTA-003 | Add parity test for explorer/editor | `tests/parity_editor.rs` | graph/settings files |
| GTA-003C | GTA-003 | Add parity test for graph/settings | `tests/parity_graph_settings.rs` | explorer internals |
| GTA-004A | GTA-004 | Draft smoke checklist | `docs/testing/gtk-parity-smoke.md` | code files |
| GTA-004B | GTA-004 | Draft slice checklist artifact | `docs/audit/gtk-parity-slice-checklist-001.md` | code files |
| GTA-005A | GTA-005 | Confirm whether automation settings are required | `src/ui/settings_view.rs` only if needed | unrelated files |
| GTA-005B | GTA-005 | Add failing settings-toggle test if required | `src/ui/settings_view.rs` | unrelated files |
| GTA-005C | GTA-005 | Implement minimal automation settings if required | `src/ui/settings_view.rs` | unrelated files |
| GTA-006A | GTA-006 | Run slice verification | repo-wide | - |
| GTA-006B | GTA-006 | Fix slice-only regressions | touched files only | unrelated modules |

### Task GTA-001: Define stable automation identifiers and tests

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/*` as needed

**Steps**
1. Write failing tests for stable identifiers on major views and controls.
2. Confirm red.
3. Add the identifiers using a consistent naming scheme.
4. Re-run targeted tests until green.
5. Review naming consistency across shell, editor, graph, and settings.

**Advice**

- Use a naming scheme like `view.editor`, `view.graph`, `rail.inspector`, `panel.context.notes`.
- Put the scheme in comments or docs so future contributors keep it stable.

### Task GTA-002: Expose semantic shell/editor/view state

**Files**
- Create: `/home/dikini/Projects/knot-gtk/src/ui/automation_state.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add failing tests for exporting active view, active note, tool mode, and inspector state.
2. Confirm red.
3. Implement the smallest semantic state snapshot API.
4. Re-run targeted tests until green.
5. Review for accidental coupling to concrete widgets.

**Example snapshot shape**

```rust
pub struct AutomationSnapshot {
    pub active_view: &'static str,
    pub active_note_path: Option<String>,
    pub tool_mode: Option<&'static str>,
    pub inspector_open: bool,
}
```

### Task GTA-003: Add parity-focused integration tests for delivered slices

**Files**
- Create or modify: `/home/dikini/Projects/knot-gtk/tests/parity_*.rs`

**Steps**
1. Add one integration test per completed slice using the semantic identifiers/state where possible.
2. Confirm red for each new behavior test before implementation support is added.
3. Implement the missing glue only where needed.
4. Re-run targeted tests until green.
5. Review for overly brittle UI assertions.

**Advice**

- Prefer “active view is graph” over “third child widget is visible”.
- Prefer “active note path is X” over checking a label string if the label is just presentation.

### Task GTA-004: Write manual review checklists and artifacts

**Files**
- Create: `/home/dikini/Projects/knot-gtk/docs/testing/gtk-parity-smoke.md`
- Create: `/home/dikini/Projects/knot-gtk/docs/audit/gtk-parity-slice-checklist-001.md`

**Steps**
1. Draft a manual checklist for startup, search, explorer, editor, note types, graph, and settings.
2. Review the checklist against implemented slices.
3. Refine steps until a junior developer can execute them without guessing.

**Checklist advice**

- Write each step as a user action plus expected result.
- Example: “Open a vault with at least one PDF note. Expected: selecting the PDF note shows view mode only.”

### Task GTA-005: Wire automation-related settings if required

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`

**Steps**
1. Add failing tests only if the chosen automation contract requires settings toggles.
2. Confirm red.
3. Implement the minimal settings surface.
4. Re-run targeted tests until green.
5. Skip this task entirely if the agreed automation contract does not need a GTK toggle.

**Advice**

- It is acceptable to mark this task “not required” after confirming the contract.
- Do not invent new settings just to fill the slot.

### Task GTA-006: Full verification and review fixes

**Steps**
1. Run `cargo fmt`.
2. Run targeted parity/integration tests.
3. Run `cargo test`.
4. Review the manual checklist for missing steps.
5. Fix review findings.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Identifiers are stable and documented
- Semantic state export reflects actual shell state
- Integration tests check behavior rather than widget internals
- Manual checklist is runnable by a junior developer
- Automation hooks do not leak unstable widget implementation details

## Commit Gate

Commit only when all verification commands are green.
