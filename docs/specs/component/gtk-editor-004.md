# GTK Editor Core and Mode Handling

## Metadata
- ID: `COMP-GTK-EDITOR-004`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, UX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Provide a stable editor core for GTK with the same functional mode model as the Tauri app, while allowing GTK to keep a native implementation rather than cloning the exact ProseMirror stack.

## Scope

### In Scope
- note load, dirty state, save flow, and save shortcuts
- mode switching across `meta`, `source`, `edit`, and `view`
- per-note mode availability rules
- note-switch guard integration
- predictable source/view synchronization
- baseline edit commands needed for core markdown authoring

### Out of Scope
- rich media surfaces for PDF/image/YouTube
- graph behavior
- settings UI

## Contract

### Functional Requirements

**FR-1**: Mode model
- GTK editor must support `meta`, `source`, `edit`, and `view` modes when the note contract allows them.
- View-only note types must disable unsupported modes.

**FR-2**: Dirty-state handling
- Content edits must mark a note dirty.
- Save must clear dirty state only after success.

**FR-3**: Note-switch guard
- Attempting to load another note while dirty must route through a guard callback.

**FR-4**: Mode synchronization
- Switching between source, edit, and view must preserve user content and keep a stable cursor/scroll restoration strategy.

**FR-5**: Baseline authoring
- GTK edit mode must support enough markdown authoring to achieve functional parity: headings, lists, quotes, code blocks, horizontal rules, task toggles, and links or wikilinks.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Preserve four-mode model | Matches Tauri functionality and note-type contract | More shell/editor state than the prototype |
| Allow GTK-native edit implementation | User asked for same functionality, not identical implementation | More design work to define equivalence |
| Treat edit and source as separate surfaces with synchronization tests | Reduces accidental state corruption | Requires explicit sync rules |

## Acceptance Criteria

- [ ] GTK editor respects per-note mode availability.
- [ ] Dirty state and save flow are tested and visible.
- [ ] Note switching can be blocked or confirmed through a guard.
- [ ] Core authoring commands exist and are tested.
- [ ] Mode switching preserves content and expected position state.

## Related

- Depends on: `COMP-GTK-SHELL-002`, `COMP-GTK-EXPLORER-003`
- Enables: `COMP-GTK-NOTE-TYPES-005`
