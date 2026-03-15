# GTK Explorer Tree and Note Lifecycle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** make GTK note browsing and explorer mutations reliable enough for day-to-day parity workflows.

**Architecture:** treat the explorer as the single owner of tree rendering and mutation dispatch, and route note selection through a shared selection API that can later consult dirty-state guards from the editor slice.

**Tech Stack:** Rust, gtk4, libadwaita, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/explorer-behavior.md`, `docs/reference/knotd-protocol.md`, and `docs/reference/note-contract.md`
- Keep explorer ownership tight: tree state, mutation dispatch, selection dispatch.
- Do not mix editor save logic into this slice; only add the guard hook.

## Delivery Notes

- Prefer modern GTK list/tree patterns if a contained refactor is practical.
- Keep drag-and-drop out unless the contract and tests are already straightforward.
- Make guard hooks explicit even if they are initially simple.
- Review-complete explorer delivery should avoid replaying expansion persistence during tree refresh, route note activation through one shared path, and keep deletion fallbacks deterministic by reselecting the parent folder when possible.
- Rename flows may accept a full target path so the slice covers note moves without adding separate drag-and-drop or move-only UI.

## Rust Guidance For This Slice

- Use small data adapters for tree rows rather than reading raw daemon payloads in widget callbacks everywhere.
- Avoid duplicated refresh logic after mutations.
- Prefer explicit enums or structs for mutation intent over boolean flags with unclear meaning.

## knotd Calls Used By This Slice

- `get_explorer_tree`
- `set_folder_expanded`
- `create_note`
- `rename_note`
- `delete_note`
- `create_directory`
- `rename_directory`
- `remove_directory`
- `get_note` for note selection reloads

### Explorer payload types

```rust
pub struct ExplorerTree {
    pub root: ExplorerFolderNode,
    pub hidden_policy: String,
}

pub struct ExplorerFolderNode {
    pub path: String,
    pub name: String,
    pub expanded: bool,
    pub folders: Vec<ExplorerFolderNode>,
    pub notes: Vec<ExplorerNoteNode>,
}

pub struct ExplorerNoteNode {
    pub path: String,
    pub title: String,
    pub display_title: String,
    pub modified_at: i64,
    pub word_count: usize,
    pub type_badge: Option<String>,
    pub is_dimmed: bool,
}
```

### Folder-expand request example

```json
{
  "name": "set_folder_expanded",
  "arguments": {
    "path": "Projects",
    "expanded": true
  }
}
```

### Mutation communication sequence

1. user triggers explorer action
2. explorer validates local selection/input
3. background worker sends the tool call
4. on success, explorer refreshes from `get_explorer_tree`
5. selection is restored, updated, or cleared according to the action contract
6. on failure, existing tree stays stable and an error state is shown

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTE-001 | Choose tree widget approach and lock tests around rendering | - |
| GTE-002 | Implement folder expansion persistence | GTE-001 |
| GTE-003 | Centralize note selection and reload behavior | GTE-001 |
| GTE-004 | Add note create/rename/delete flows | GTE-003 |
| GTE-005 | Add directory create/rename/remove flows | GTE-003 |
| GTE-006 | Add switch-guard hook and finish verification | GTE-004, GTE-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTE-001A | GTE-001 | Add failing folder-row rendering test | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-001B | GTE-001 | Add failing note-row rendering and badge test | `src/ui/explorer.rs` | `src/client/*` |
| GTE-001C | GTE-001 | Stabilize tree widget approach | `src/ui/explorer.rs` | `src/ui/window.rs` |
| GTE-002A | GTE-002 | Add failing expand callback test | `src/ui/explorer.rs` | `src/ui/context_panel.rs` |
| GTE-002B | GTE-002 | Add failing collapse callback test | `src/ui/explorer.rs` | `src/ui/window.rs` |
| GTE-002C | GTE-002 | Implement expansion persistence | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-003A | GTE-003 | Add failing single note-selection path test | `src/ui/explorer.rs`, `src/ui/window.rs` | `src/ui/search_view.rs` |
| GTE-003B | GTE-003 | Centralize note-load dispatch | `src/ui/window.rs`, `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-004A | GTE-004 | Add failing create-note flow test | `src/ui/explorer.rs`, `src/ui/context_panel.rs` | `src/ui/window.rs` |
| GTE-004B | GTE-004 | Add failing rename-note flow test | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-004C | GTE-004 | Add failing delete-note flow test | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-004D | GTE-004 | Implement note mutation actions | `src/ui/explorer.rs`, `src/ui/context_panel.rs` | `src/client/*` |
| GTE-005A | GTE-005 | Add failing create-directory flow test | `src/ui/explorer.rs` | `src/ui/window.rs` |
| GTE-005B | GTE-005 | Add failing rename-directory flow test | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-005C | GTE-005 | Add failing remove-directory flow test | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTE-005D | GTE-005 | Implement directory mutation actions | `src/ui/explorer.rs` | `src/client/*` |
| GTE-006A | GTE-006 | Add failing guard-callback decision test | `src/ui/explorer.rs`, `src/ui/window.rs` | `src/ui/editor.rs` |
| GTE-006B | GTE-006 | Implement guard hook only | `src/ui/explorer.rs`, `src/ui/window.rs` | save logic |
| GTE-006C | GTE-006 | Run slice verification and fix regressions | repo-wide | - |

### Task GTE-001: Choose tree widget approach and lock tests around rendering

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`

**Steps**
1. Write rendering tests for folder rows, note rows, badges, and selection behavior.
2. Confirm red.
3. Either keep the existing tree temporarily or refactor to a non-deprecated GTK pattern if it stays contained to this file/module.
4. Re-run targeted tests until green.
5. Review API choices and document the chosen widget approach in a module comment.

**Advice**

- If replacing deprecated widgets would touch too many files, stabilize behavior first and leave the widget refactor documented.
- The key requirement is deterministic rendering and selection.

### Task GTE-002: Implement folder expansion persistence

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/client/mod.rs` only if required by test support

**Steps**
1. Write a failing test for expand/collapse dispatch.
2. Confirm red.
3. Wire expand/collapse callbacks to `set_folder_expanded`.
4. Re-run targeted tests until green.
5. Review for duplicate reload logic.

**Example test skeleton**

```rust
#[test]
fn folder_expand_event_requests_persistence() {
    let mut events = Vec::new();
    let explorer = ExplorerHarness::new(|path, expanded| {
        events.push((path.to_string(), expanded));
    });

    explorer.simulate_expand("Projects");

    assert_eq!(events, vec![("Projects".to_string(), true)]);
}
```

### Task GTE-003: Centralize note selection and reload behavior

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add a failing test for selecting a note through the explorer and routing through a single note-load path.
2. Confirm red.
3. Implement the central note selection flow.
4. Re-run targeted tests until green.
5. Review for double-loading or duplicate callbacks.

**Advice**

- Keep one public “request note selection” entry point.
- Do not let tree rows load notes directly.

### Task GTE-004: Add note create/rename/delete flows

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`

**Steps**
1. Write one failing test per note mutation flow.
2. Confirm each test is red for the expected missing behavior.
3. Implement the smallest mutation UI and daemon dispatch needed for each flow.
4. Re-run targeted tests until green.
5. Review post-mutation selection behavior and refresh policy.

**Implementation notes**
- Use simple dialogs or prompts first; avoid fancy context menus if they slow the slice down.

**Advice**

- For each mutation flow, write the success path first, then the error path.
- Be explicit about what selection should happen after create or rename.

### Task GTE-005: Add directory create/rename/remove flows

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`

**Steps**
1. Add failing tests for directory create, rename, and remove.
2. Confirm red.
3. Implement minimal UI plus daemon dispatch.
4. Re-run targeted tests until green.
5. Review nested-path behavior and refresh correctness.

**Advice**

- Create tests for nested folder names early.
- Directory operations are easy to get wrong if you only test root-level paths.

### Task GTE-006: Add switch-guard hook and finish verification

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Write a failing test for “selection requests can be blocked or deferred by a guard callback”.
2. Confirm red.
3. Add the hook without hard-coding editor behavior yet.
4. Re-run targeted tests until green.
5. Run full slice verification and apply review fixes.

**Example guard contract sketch**

```rust
pub enum NoteSwitchDecision {
    Allow,
    Deny,
    SaveThenAllow,
}
```

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Tree rendering and selection are deterministic
- Expansion state round-trips
- Mutations refresh the tree correctly
- Guard hook exists without editor coupling
- Selection behavior after rename and delete is tested

## Commit Gate

Commit only when all verification commands are green.
