# GTK Deprecated API Modernization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** remove deprecated GTK/libadwaita API usage from repository code and make deprecation warnings a meaningful maintenance signal again.

**Architecture:** treat this as one modernization slice with subsystem-scoped tasks. Replace contained deprecated APIs directly, and handle explorer as the main structural migration with behavior-preserving tests around selection, expansion persistence, and mutation flows.

**Tech Stack:** Rust, gtk4, libadwaita, cargo test, cargo check

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/specs/component/gtk-deprecations-009.md`, `docs/specs/component/gtk-explorer-003.md`, `docs/reference/explorer-behavior.md`
- Treat compile-time GTK/libadwaita deprecation warnings from repository code as the signal for this slice.
- Do not mix unrelated warning cleanup into this slice.

## Delivery Notes

- Start with an inventory so the warning set is explicit before changing code.
- Keep explorer migration behavior-first: preserve rendering, selection, expansion persistence, and mutation refresh semantics.
- Prefer current supported GTK/libadwaita APIs over custom workaround layers.
- Review-complete delivery should leave `cargo check` free of GTK/libadwaita deprecation warnings from repository code.

## Rust Guidance For This Slice

- Replace deprecated widget families with small adapter layers only when they reduce churn; do not add abstraction for its own sake.
- Keep selection and expansion state explicit during explorer migration.
- Prefer typed row/view-model helpers over stringly widget state.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTD-001 | Inventory deprecated GTK/libadwaita API usage | - |
| GTD-002 | Migrate explorer away from deprecated tree widgets | GTD-001 |
| GTD-003 | Replace remaining deprecated dialogs, pickers, and widget APIs | GTD-001 |
| GTD-004 | Tighten docs and verification policy | GTD-001 |
| GTD-005 | Run full verification and review sweep | GTD-002, GTD-003, GTD-004 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTD-001A | GTD-001 | Capture current `cargo check` deprecation inventory | repo-wide | implementation code |
| GTD-001B | GTD-001 | Map each warning to its owning module and replacement API | `docs/plans/2026-03-15-gtk-deprecations-plan.md` | runtime/client code |
| GTD-002A | GTD-002 | Add failing explorer behavior tests around rendering and selection | `src/ui/explorer.rs` | `src/ui/editor.rs` |
| GTD-002B | GTD-002 | Add failing expansion persistence and mutation refresh tests | `src/ui/explorer.rs` | `src/client/*` |
| GTD-002C | GTD-002 | Replace deprecated tree widgets with a supported GTK list/tree pattern | `src/ui/explorer.rs` | note editor |
| GTD-002D | GTD-002 | Re-run explorer-focused verification and simplify callback wiring | `src/ui/explorer.rs` | unrelated UI |
| GTD-003A | GTD-003 | Add failing tests for any remaining deprecated dialog/picker behavior where practical | affected UI modules | explorer core |
| GTD-003B | GTD-003 | Replace remaining deprecated GTK/libadwaita APIs outside explorer | affected UI modules | daemon protocol |
| GTD-004A | GTD-004 | Update docs to state the enforced no-new-deprecated-APIs policy | `AGENTS.md`, `docs/README.md`, related notes/specs | code |
| GTD-004B | GTD-004 | Document any temporary exception only if absolutely required | `docs/notes/*.md` | feature behavior |
| GTD-005A | GTD-005 | Run `cargo fmt --check`, `cargo check`, and targeted/full tests | repo-wide | - |
| GTD-005B | GTD-005 | Review the final diff for spec alignment, code smell, and simplification | repo-wide | - |
| GTD-005C | GTD-005 | Fix review findings until no actionable issues remain | repo-wide | - |

### Task GTD-001: Inventory deprecated GTK/libadwaita API usage

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/docs/plans/2026-03-15-gtk-deprecations-plan.md`

**Steps**
1. Run `cargo check` and capture every GTK/libadwaita deprecation warning emitted from repository code.
2. Group warnings by module and API family.
3. Record the owning file and intended replacement for each group.
4. Confirm the inventory is complete before starting migrations.

### Task GTD-002: Migrate explorer away from deprecated tree widgets

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`
- Modify only if required: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`
- Modify only if required: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add failing tests for explorer row rendering, selection, expansion persistence, and mutation refresh behavior.
2. Confirm red.
3. Replace `TreeView`/`TreeStore`-based explorer code with a supported GTK list/tree pattern.
4. Re-run targeted tests until green.
5. Review the migration for duplicate selection paths or refresh churn.

**Advice**
- Keep the public explorer behavior stable.
- Do not reintroduce stringly row ownership or duplicated selection dispatch.

### Task GTD-003: Replace remaining deprecated dialogs, pickers, and widget APIs

**Files**
- Modify: exact modules identified by GTD-001

**Steps**
1. Add focused tests when behavior can regress.
2. Confirm red where practical.
3. Replace each remaining deprecated API with the supported GTK/libadwaita alternative.
4. Re-run focused verification after each contained migration.
5. Remove dead compatibility code if the replacement fully supersedes it.

### Task GTD-004: Tighten docs and verification policy

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/AGENTS.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/README.md`
- Modify as needed: related notes/spec docs

**Steps**
1. State that deprecated GTK/libadwaita APIs are not allowed when a supported replacement exists.
2. Clarify that deprecation warnings in repository code are a maintenance signal, not background noise.
3. Document any temporary exception only if the migration is blocked and the exit path is explicit.

### Task GTD-005: Run full verification and review sweep

**Files**
- Modify as needed: repo-wide

**Steps**
1. Run `cargo fmt --check`.
2. Run `cargo check`.
3. Run targeted tests for touched modules, then `cargo test`.
4. Review for spec alignment, code smell, and simplification opportunities.
5. Fix findings and repeat until no actionable issues remain.

## Slice Verification

```bash
cargo fmt --check
cargo check
cargo test
```

## Completion Criteria

- `cargo check` emits no GTK/libadwaita deprecation warnings from repository code.
- Explorer behavior remains functionally aligned with its spec after the widget migration.
- Docs make the no-new-deprecated-APIs policy discoverable and explicit.
