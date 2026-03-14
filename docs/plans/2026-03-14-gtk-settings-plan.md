# GTK Settings, Plugins, and Maintenance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** implement the settings and maintenance surface required for GTK parity without coupling it tightly to unrelated shell or editor code.

**Architecture:** build a dedicated settings view module with data-driven sections and small async loaders, keeping shell integration limited to view routing and inspector-mode selection.

**Tech Stack:** Rust, gtk4, libadwaita, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/settings-behavior.md`, `docs/reference/shell-behavior.md`, and `docs/reference/knotd-protocol.md`
- Keep the settings surface modular and section-based.
- Do not mix maintenance actions into unrelated editor or shell modules.

## Delivery Notes

- Keep settings section definitions in one place.
- Separate data loading from widget construction where practical.
- Reindex and other maintenance actions must show explicit progress/result states.

## Rust Guidance For This Slice

- Keep vault-settings patch construction explicit and typed.
- Avoid ad hoc JSON manipulation in many places; centralize it.
- Treat plugin and maintenance failures as user-visible states.

## knotd Calls Used By This Slice

- `get_vault_settings`
- `update_vault_settings`
- `list_vault_plugins`
- `reindex_vault`
- optionally `sync_external_changes` if a maintenance surface includes it

### Settings request examples

```json
{
  "name": "get_vault_settings",
  "arguments": {}
}
```

```json
{
  "name": "update_vault_settings",
  "arguments": {
    "patch": {
      "editor": {
        "font_size": 16
      }
    }
  }
}
```

### Rust types to keep explicit

```rust
pub struct VaultSettings {
    pub name: String,
    pub plugins_enabled: bool,
    pub file_visibility: String,
    pub editor: VaultEditorSettings,
}

pub struct VaultEditorSettings {
    pub font_size: i32,
    pub tab_size: i32,
}
```

### Settings communication sequence

1. settings view opens
2. background worker fetches settings and plugin list
3. main thread populates form view models
4. user edits a field
5. UI builds a small patch object
6. background worker sends `update_vault_settings`
7. success refreshes the local view model, failure preserves editable state and shows an error

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTSM-001 | Add settings section model and routing tests | - |
| GTSM-002 | Implement vault settings load/update flow | GTSM-001 |
| GTSM-003 | Implement plugin list and refresh flow | GTSM-001 |
| GTSM-004 | Implement maintenance actions and feedback | GTSM-001 |
| GTSM-005 | Add app-level shell preference persistence | GTSM-001 |
| GTSM-006 | Full verification and review fixes | GTSM-002, GTSM-003, GTSM-004, GTSM-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTSM-001A | GTSM-001 | Add failing settings-view activation test | `src/ui/settings_view.rs`, `src/ui/window.rs` | editor files |
| GTSM-001B | GTSM-001 | Add failing section-routing test | `src/ui/settings_view.rs` | graph files |
| GTSM-001C | GTSM-001 | Implement section model | `src/ui/settings_view.rs`, `src/ui/mod.rs` | `src/client/*` |
| GTSM-002A | GTSM-002 | Add failing settings-load test | `src/ui/settings_view.rs` | plugin/maintenance sections |
| GTSM-002B | GTSM-002 | Add failing patch-update test | `src/ui/settings_view.rs` | plugin/maintenance sections |
| GTSM-002C | GTSM-002 | Implement vault settings form/view model | `src/ui/settings_view.rs` | shell state |
| GTSM-003A | GTSM-003 | Add failing plugin-list loading test | `src/ui/settings_view.rs` | vault settings |
| GTSM-003B | GTSM-003 | Add failing plugin refresh test | `src/ui/settings_view.rs` | vault settings |
| GTSM-003C | GTSM-003 | Implement plugin section | `src/ui/settings_view.rs` | shell state |
| GTSM-004A | GTSM-004 | Add failing reindex loading test | `src/ui/settings_view.rs` | plugin section |
| GTSM-004B | GTSM-004 | Add failing reindex success/error tests | `src/ui/settings_view.rs` | plugin section |
| GTSM-004C | GTSM-004 | Implement maintenance section | `src/ui/settings_view.rs` | shell state |
| GTSM-005A | GTSM-005 | Add failing label-visibility preference test | `src/ui/settings_view.rs`, `src/ui/shell_state.rs` | vault settings |
| GTSM-005B | GTSM-005 | Add failing panel-width preference test | `src/ui/settings_view.rs`, `src/ui/shell_state.rs` | vault settings |
| GTSM-005C | GTSM-005 | Implement app-level preference persistence | `src/ui/settings_view.rs`, `src/ui/shell_state.rs` | daemon settings |
| GTSM-006A | GTSM-006 | Run slice verification | repo-wide | - |
| GTSM-006B | GTSM-006 | Fix slice-only regressions | touched files only | unrelated modules |

### Task GTSM-001: Add settings section model and routing tests

**Files**
- Create or modify: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/mod.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Write failing tests for section routing and settings-view activation.
2. Confirm red.
3. Implement the minimal settings section model.
4. Re-run targeted tests until green.
5. Review for duplicated section labels or IDs.

**Template**

```rust
pub enum SettingsSection {
    General,
    Appearance,
    Plugins,
    Vault,
    Maintenance,
}
```

### Task GTSM-002: Implement vault settings load/update flow

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/client/mod.rs` only if helper methods are missing

**Steps**
1. Add failing tests for loading current settings and applying a patch.
2. Confirm red.
3. Implement minimal form fields and update dispatch.
4. Re-run targeted tests until green.
5. Review partial-update behavior to ensure unchanged fields are not overwritten accidentally.

**Advice**

- Build small view models for forms.
- Apply patches only from explicit field values, not from raw widget state dumps.

### Task GTSM-003: Implement plugin list and refresh flow

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`

**Steps**
1. Write failing tests for plugin list rendering and refresh.
2. Confirm red.
3. Implement the smallest plugin section needed for parity.
4. Re-run targeted tests until green.
5. Review empty-state and error-state behavior.

**Advice**

- Plugin lists often fail in edge cases. Add an explicit empty/error/loading state test.

### Task GTSM-004: Implement maintenance actions and feedback

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`

**Steps**
1. Add failing tests for reindex action loading, success, and error states.
2. Confirm red.
3. Implement the action and feedback UI.
4. Re-run targeted tests until green.
5. Review whether other maintenance actions belong now or should stay out of scope.

**Example test skeleton**

```rust
#[test]
fn reindex_success_updates_status_message() {
    let mut state = MaintenanceState::default();
    state.apply_result(Ok(42));
    assert_eq!(state.status_text(), Some("Reindexed 42 items".into()));
}
```

### Task GTSM-005: Add app-level shell preference persistence

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/shell_state.rs`

**Steps**
1. Write failing tests for one or two shell preferences, such as label visibility or panel width.
2. Confirm red.
3. Implement the smallest persistence path.
4. Re-run targeted tests until green.
5. Review for separation between app-level preferences and vault-level settings.

**Advice**

- App preferences belong to GTK shell state.
- Vault settings belong to daemon-backed configuration.
- Do not mix them in the same save path.

### Task GTSM-006: Full verification and review fixes

**Steps**
1. Run `cargo fmt`.
2. Run targeted settings tests.
3. Run `cargo test`.
4. Manual smoke-check settings load/update if the environment allows.
5. Fix review findings.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Section routing is explicit
- Vault settings patch flow is safe
- Plugin and maintenance states are visible
- App-level preferences stay separate from vault settings
- JSON patch construction is centralized and tested

## Commit Gate

Commit only when all verification commands are green.
