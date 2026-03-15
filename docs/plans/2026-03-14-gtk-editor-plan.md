# GTK Editor Core and Mode Handling Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** give GTK a stable editor core with four-mode behavior, dirty/save handling, and enough markdown authoring to reach functional parity without cloning the Tauri editor internals.

**Architecture:** keep mode decisions contract-driven, keep content synchronization explicit, and separate “editor core” from “note-type/media views” so text-authoring work remains isolated.

**Tech Stack:** Rust, gtk4, libadwaita, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/editor-behavior.md` and `docs/reference/note-contract.md`
- Keep this slice focused on text authoring and mode behavior.
- Media and note-type-specific rendering belong to the next slice.

## Delivery Notes

- This slice should not add PDF/image/YouTube rendering; that belongs to the next slice.
- Keep authoring controls minimal but functionally complete.
- Add guard integration using the explorer hook from the previous slice.
- Canonical markdown source is the only authoritative note state; `view`, `edit`, and `meta` must derive from it.
- Keep the mode rail in a fixed four-icon layout for every note type; unavailable modes stay visible but disabled, with no hover or click affordance.
- `meta` should pin `title`, `description`, and `tags` at the top, then show a generic frontmatter editor below.

## Rust Guidance For This Slice

- Separate pure mode/state logic from GTK widget code wherever possible.
- Do not drop unsaved changes on failed save or mode switch.
- Keep markdown transformation helpers small and covered by tests.
- Avoid hidden side effects in mode-switch callbacks.
- Prefer deterministic rebuilds over clever incremental synchronization if that keeps the code smaller and easier to reason about.

## knotd Calls Used By This Slice

- `get_note`
- `save_note`

This slice should consume note-contract fields added in the runtime slice:

```rust
pub struct NoteData {
    pub id: String,
    pub path: String,
    pub title: String,
    pub content: String,
    pub headings: Vec<Heading>,
    pub backlinks: Vec<Backlink>,
    pub note_type: Option<NoteType>,
    pub available_modes: Option<NoteModeAvailability>,
    pub metadata: Option<NoteMetadata>,
    pub embed: Option<NoteEmbedDescriptor>,
    pub media: Option<NoteMediaData>,
}
```

### Save request example

```json
{
  "name": "save_note",
  "arguments": {
    "path": "notes/example.md",
    "content": "# Title\n\nBody"
  }
}
```

### Note-load communication sequence

1. shell requests note selection
2. editor enters loading state
3. background worker calls `get_note`
4. main thread receives typed `NoteData`
5. editor applies mode availability and content
6. dirty state resets only after a successful load

### UI contract for this slice

- The mode order is fixed: `meta`, `source`, `edit`, `view`.
- Mode selection uses icons rather than text labels.
- Unavailable modes remain visible for layout stability and are grayed out.
- Unavailable modes should not show hover affordances or explanatory messages.

## Suggested Task Ownership

- One developer can own mode and dirty-state logic.
- One developer can own synchronization behavior between source/edit/view.
- One developer can own markdown command coverage.

`GTC-002` and `GTC-004` should not be implemented in parallel by different developers unless they coordinate carefully on `src/ui/editor.rs`.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTC-001 | Add tests for mode availability and mode routing | - |
| GTC-002 | Implement dirty/save state model | GTC-001 |
| GTC-003 | Integrate note-switch guard | GTC-002 |
| GTC-004 | Stabilize source/view/edit synchronization | GTC-002 |
| GTC-005 | Add baseline markdown authoring commands | GTC-004 |
| GTC-006 | Add meta-mode editing and full verification | GTC-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTC-001A | GTC-001 | Add failing default-mode test | `src/ui/editor.rs` | media rendering |
| GTC-001B | GTC-001 | Add failing mode-availability test per note type | `src/ui/editor.rs` | `src/ui/window.rs` |
| GTC-001C | GTC-001 | Implement mode gate helper | `src/ui/editor.rs` | `src/ui/block_editor/*` |
| GTC-002A | GTC-002 | Add failing dirty-on-edit test | `src/ui/editor.rs` | `src/ui/window.rs` |
| GTC-002B | GTC-002 | Add failing save-success clears-dirty test | `src/ui/editor.rs` | `src/ui/explorer.rs` |
| GTC-002C | GTC-002 | Add failing save-error keeps-dirty test | `src/ui/editor.rs` | `src/ui/explorer.rs` |
| GTC-002D | GTC-002 | Implement dirty/save model | `src/ui/editor.rs`, `src/ui/window.rs` | media views |
| GTC-003A | GTC-003 | Add failing deny-switch test | `src/ui/editor.rs`, `src/ui/window.rs` | `src/ui/explorer.rs` |
| GTC-003B | GTC-003 | Add failing discard-and-switch test | `src/ui/editor.rs`, `src/ui/window.rs` | `src/ui/explorer.rs` |
| GTC-003C | GTC-003 | Add failing save-then-switch test | `src/ui/editor.rs`, `src/ui/window.rs` | `src/ui/explorer.rs` |
| GTC-003D | GTC-003 | Implement guard decisions | `src/ui/editor.rs`, `src/ui/window.rs` | note type rendering |
| GTC-004A | GTC-004 | Add failing content-preservation test | `src/ui/editor.rs` | `src/ui/window.rs` |
| GTC-004B | GTC-004 | Add failing scroll/cursor restoration test | `src/ui/editor.rs` | note type rendering |
| GTC-004C | GTC-004 | Implement source/view sync | `src/ui/editor.rs` | `src/ui/explorer.rs` |
| GTC-004D | GTC-004 | Implement source/edit sync | `src/ui/editor.rs`, `src/ui/block_editor/*` | media views |
| GTC-005A | GTC-005 | Add failing heading/list command tests | `src/ui/editor.rs`, `src/ui/block_editor/parser.rs` | meta mode |
| GTC-005B | GTC-005 | Add failing quote/code/hr tests | `src/ui/editor.rs`, `src/ui/block_editor/parser.rs` | meta mode |
| GTC-005C | GTC-005 | Add failing task-toggle/link tests | `src/ui/editor.rs`, `src/ui/block_editor/renderer.rs` | media views |
| GTC-005D | GTC-005 | Implement baseline commands minimally | touched files only | shell routing |
| GTC-006A | GTC-006 | Add failing meta-mode availability test | `src/ui/editor.rs` | `src/ui/window.rs` |
| GTC-006B | GTC-006 | Add failing metadata round-trip test | `src/ui/editor.rs` | note type rendering |
| GTC-006C | GTC-006 | Implement minimal meta mode | `src/ui/editor.rs` | graph/settings |
| GTC-006D | GTC-006 | Run slice verification and fix regressions | repo-wide | - |

### Task GTC-001: Add tests for mode availability and mode routing

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Write failing tests for per-note mode availability and default mode choice.
2. Confirm red.
3. Implement the mode gate logic with the smallest API necessary.
4. Re-run targeted tests until green.
5. Review mode names and ensure they match the contract exactly.

**Example test skeleton**

```rust
#[test]
fn pdf_note_only_allows_view_mode() {
    let modes = available_modes_for_note_type(NoteType::Pdf);
    assert!(!modes.meta);
    assert!(!modes.source);
    assert!(!modes.edit);
    assert!(modes.view);
}
```

### Task GTC-002: Implement dirty/save state model

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add failing tests for edit marks dirty, successful save clears dirty, failed save preserves dirty.
2. Confirm red.
3. Implement minimal dirty/save state handling and shell feedback.
4. Re-run targeted tests until green.
5. Review save-path ownership so there is one save entry point.

**Advice**

- Add tests for success and failure before wiring actual save UI.
- A visible dirty indicator is useful, but correctness matters more than styling in this slice.

### Task GTC-003: Integrate note-switch guard

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Add failing tests for blocked switch, discard-and-switch, and save-then-switch decisions.
2. Confirm red.
3. Implement the smallest guard integration using the explorer hook.
4. Re-run targeted tests until green.
5. Review edge cases around failed save and cancelled switch.

**Example guard test skeleton**

```rust
#[test]
fn denied_switch_keeps_current_note_selected() {
    let mut editor = EditorHarness::dirty_note("notes/a.md");
    let result = editor.request_switch("notes/b.md", NoteSwitchDecision::Deny);
    assert!(result.is_denied());
    assert_eq!(editor.current_note_path(), "notes/a.md");
}
```

### Task GTC-004: Stabilize source/view/edit synchronization

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/block_editor/*` only if required

**Steps**
1. Write failing tests for content preservation across mode switches.
2. Add a focused test for cursor or scroll restoration strategy.
3. Confirm red.
4. Implement minimal synchronization and restoration logic.
5. Re-run targeted tests until green.
6. Review for duplicated conversion code.

**Advice**

- Pick one synchronization direction as authoritative.
- Example: source text is the saved truth, and view/edit derive from it.
- Write that down in code comments if it is not obvious.
- Favor explicit reconstruction from source over partial in-place synchronization if that keeps behavior deterministic and the implementation simple.

### Task GTC-005: Add baseline markdown authoring commands

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/block_editor/parser.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/block_editor/renderer.rs`

**Steps**
1. Write failing tests for headings, lists, quotes, code blocks, horizontal rules, task toggles, and links/wikilinks.
2. Confirm red.
3. Implement the smallest authoring commands and rendering support needed.
4. Re-run targeted tests until green.
5. Review for unnecessary feature creep; keep this slice to baseline parity.

**Example command test ideas**

- toggling a paragraph into a heading
- inserting an unordered list marker
- serializing a quote block correctly
- toggling a markdown task item from unchecked to checked
- preserving a wikilink token round-trip

### Task GTC-006: Add meta-mode editing and full verification

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Write failing tests for metadata mode availability and metadata round-trip into note content or contract payload.
2. Confirm red.
3. Implement minimal meta-mode UI and update flow.
4. Re-run targeted tests until green.
5. Run full verification and fix review findings.

**Advice**

- Start meta mode with a tiny, clear surface.
- Keep `title`, `description`, and `tags` pinned at the top.
- The generic editor below should focus on simple scalar frontmatter values first; nested structures can be deferred if they would add disproportionate complexity.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Mode availability follows note contract
- Dirty/save behavior is never lost on failed save
- Switch guard cannot silently drop edits
- Mode transitions preserve content
- Markdown commands are covered by focused tests, not only manual UI checks

## Commit Gate

Commit only when all verification commands are green.
