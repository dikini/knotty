# GTK Editor Design

## Purpose

Lock the GTK editor behavior before implementation so the editor slice can stay focused on correctness, mode behavior, and synchronization instead of growing a second document truth model.

## Core Decisions

### Source Is Authoritative

- Canonical markdown source is the single authoritative note state.
- `view`, `edit`, and `meta` derive from canonical source.
- Save continues to persist one `content` field through `save_note`.
- Mode transitions must round-trip through canonical source rather than keeping a second hidden truth.

### Fixed Mode Rail

- The mode rail always shows four controls in the same order: `meta`, `source`, `edit`, `view`.
- Modes are icon-based rather than text-based.
- `meta` uses an information-style icon if there is no dedicated metadata icon.
- Unavailable modes stay visible but disabled.
- Disabled modes have no hover affordance and no click behavior.

### Edit Surface

- `source` shows editable raw markdown.
- `view` renders canonical markdown without editing controls.
- `edit` uses a GTK-native markdown editing surface with command buttons over the same canonical text buffer used by `source`.
- Changes made in `edit` update canonical source directly.
- Simplicity rule: prefer deterministic rebuild/refresh behavior over clever incremental synchronization.

### Meta Surface

- `meta` starts with a pinned top section for `title`, `description`, and `tags`.
- Remaining frontmatter is shown below in a generic metadata editor.
- The first implementation should stay conservative and focus on scalar values before supporting nested structures.
- Meta edits still update canonical source so reload/save behavior matches the source-authoritative model.

### Dirty, Save, and Guard Behavior

- Any change in `source`, `edit`, or `meta` marks the note dirty.
- Save clears dirty state only after success.
- Failed save preserves dirty state and current content.
- Switching notes while dirty must route through the existing explorer/window guard integration.

## Scope Constraints

- Keep the mode and synchronization logic explicit and testable.
- Do not add media rendering in this slice.
- Remove dead code only when it is clearly superseded by the editor implementation in this slice.
- Planned code for later slices should remain if the editor slice still depends on it.
