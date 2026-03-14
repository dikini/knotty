# GTK Note Types, Media, and Embed Surfaces Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** extend the GTK shell and editor so note types, media, and embed metadata are handled functionally the same way as the Tauri UI.

**Architecture:** build contract-driven note-type routing on top of the editor-core slice, keeping rendering concerns isolated from shell navigation and text-authoring internals.

**Tech Stack:** Rust, gtk4, libadwaita, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/note-contract.md` and `docs/reference/editor-behavior.md`
- Keep note-type routing centralized.
- Do not re-open core editor architecture questions in this slice.

## Delivery Notes

- Use simple but reliable GTK-native media surfaces first.
- Keep note-type routing centralized.
- Prefer safe fallback rendering over partial broken embeds.

## Rust Guidance For This Slice

- Keep note-type helpers pure and testable.
- Treat missing media metadata as a recoverable error.
- Avoid repeating `match note_type` logic in multiple widgets.

## Contract Types Used By This Slice

```rust
pub enum NoteType {
    Markdown,
    Youtube,
    Pdf,
    Image,
    Unknown,
}

pub struct NoteModeAvailability {
    pub meta: bool,
    pub source: bool,
    pub edit: bool,
    pub view: bool,
}

pub struct NoteMediaData {
    pub mime_type: String,
    pub file_path: String,
}
```

### Communication sequence for media-capable notes

1. shell loads note via `get_note`
2. editor inspects `note_type`, `available_modes`, `media`, and `embed`
3. editor routes to the correct GTK view surface
4. if required media metadata is missing, editor enters a tested error state instead of panicking

### Junior developer advice

- Do not infer note type from file extension if the payload already gives you `note_type`.
- `available_modes` is part of the contract. Respect it even if the UI could technically show more tabs.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTN-001 | Add tests for note-type-driven mode gating and iconography | - |
| GTN-002 | Implement image note rendering | GTN-001 |
| GTN-003 | Implement PDF note rendering | GTN-001 |
| GTN-004 | Implement YouTube note rendering and open action | GTN-001 |
| GTN-005 | Add embed rendering or fallback actions | GTN-002, GTN-003, GTN-004 |
| GTN-006 | Full verification and review fixes | GTN-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTN-001A | GTN-001 | Add failing mode-gating test for image/pdf/youtube | `src/ui/editor.rs` | core shell |
| GTN-001B | GTN-001 | Add failing explorer icon/badge test | `src/ui/explorer.rs` | graph/settings |
| GTN-001C | GTN-001 | Centralize note-type helper logic | `src/ui/editor.rs`, `src/ui/explorer.rs` | `src/ui/window.rs` |
| GTN-002A | GTN-002 | Add failing image-view happy-path test | `src/ui/editor.rs` | pdf/youtube |
| GTN-002B | GTN-002 | Add failing image-view error-state test | `src/ui/editor.rs` | pdf/youtube |
| GTN-002C | GTN-002 | Implement image renderer | `src/ui/editor.rs` | shell routing |
| GTN-003A | GTN-003 | Add failing PDF loading-state test | `src/ui/editor.rs` | image/youtube |
| GTN-003B | GTN-003 | Add failing PDF render-state test | `src/ui/editor.rs` | image/youtube |
| GTN-003C | GTN-003 | Add failing PDF error-state test | `src/ui/editor.rs` | image/youtube |
| GTN-003D | GTN-003 | Implement minimal PDF surface | `src/ui/editor.rs` | shell routing |
| GTN-004A | GTN-004 | Add failing YouTube metadata card test | `src/ui/editor.rs` | image/pdf |
| GTN-004B | GTN-004 | Add failing primary-action launch test | `src/ui/editor.rs` | image/pdf |
| GTN-004C | GTN-004 | Implement YouTube view surface | `src/ui/editor.rs` | shell routing |
| GTN-005A | GTN-005 | Add failing supported-embed test | `src/ui/editor.rs` | graph/settings |
| GTN-005B | GTN-005 | Add failing fallback-embed test | `src/ui/editor.rs` | graph/settings |
| GTN-005C | GTN-005 | Implement embed/fallback rendering | `src/ui/editor.rs` | core editor sync |
| GTN-006A | GTN-006 | Run slice verification | repo-wide | - |
| GTN-006B | GTN-006 | Fix slice-only regressions | touched files only | unrelated modules |

### Task GTN-001: Add tests for note-type-driven mode gating and iconography

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/explorer.rs`

**Steps**
1. Write failing tests for note-type-based icon/badge and mode availability behavior.
2. Confirm red.
3. Implement centralized note-type helpers rather than repeating filename checks.
4. Re-run targeted tests until green.
5. Review for stray ad hoc note-type logic in other files.

**Advice**

- A single helper module for note-type routing is easier for juniors to maintain than scattered conditionals.

### Task GTN-002: Implement image note rendering

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Add a failing test for image notes entering view mode and rendering a usable image surface.
2. Confirm red.
3. Implement the minimal image renderer.
4. Re-run targeted tests until green.
5. Review error handling for missing file paths.

**Example test skeleton**

```rust
#[test]
fn image_note_without_media_path_enters_error_state() {
    let note = test_image_note_without_media();
    let state = render_image_note(&note);
    assert!(state.is_error());
}
```

### Task GTN-003: Implement PDF note rendering

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Add failing tests for PDF view mode and basic page/loading behavior.
2. Confirm red.
3. Implement a minimal PDF surface with readable loading/error states.
4. Re-run targeted tests until green.
5. Review whether basic page navigation needs small helper widgets now or in a follow-up task.

**Advice**

- Keep the first PDF surface intentionally small: loading, error, visible page, maybe next/previous.
- Avoid building a full document viewer unless the tests require it.

### Task GTN-004: Implement YouTube note rendering and open action

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Write failing tests for YouTube metadata display and primary-action launch behavior.
2. Confirm red.
3. Implement a small YouTube card or details surface with launch action.
4. Re-run targeted tests until green.
5. Review fallback behavior when metadata is incomplete.

**Advice**

- Show the best available title and URL.
- If metadata is partial, the primary action must still work.

### Task GTN-005: Add embed rendering or fallback actions

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Add failing tests for at least one supported embed shape and one fallback path.
2. Confirm red.
3. Implement the minimal supported renderers and a clear fallback action surface.
4. Re-run targeted tests until green.
5. Review for unsafe or ambiguous external-action behavior.

**Fallback template**

```rust
pub struct EmbedFallbackViewModel {
    pub title: String,
    pub description: Option<String>,
    pub primary_action_label: String,
}
```

### Task GTN-006: Full verification and review fixes

**Steps**
1. Run `cargo fmt`.
2. Run targeted note-type tests.
3. Run `cargo test`.
4. Manual smoke-check at least one image note and one non-markdown note if environment allows.
5. Fix review findings.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Note type logic is centralized
- Unsupported modes are disabled correctly
- Media surfaces expose loading and error states
- Embed fallbacks preserve the primary action
- Explorer and editor use the same note-type decisions

## Commit Gate

Commit only when all verification commands are green.
