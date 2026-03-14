# GTK Explorer Tree and Note Lifecycle

## Metadata
- ID: `COMP-GTK-EXPLORER-003`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, UX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Provide a robust explorer tree and note-selection lifecycle so GTK can browse, create, rename, move, delete, and refresh notes and folders with the same functional coverage as the Tauri app.

## Scope

### In Scope
- tree rendering based on `get_explorer_tree`
- folder expansion persistence
- note selection and note reload flow
- create, rename, move, and delete for notes and directories
- refresh behavior after mutations
- unsaved-change guard hook points for later editor integration

### Out of Scope
- drag-and-drop tree authoring if it creates excessive GTK-specific churn
- final media/note-type rendering

## Contract

### Functional Requirements

**FR-1**: Explorer tree rendering
- Tree view must render folders and note leaves using the daemon explorer payload.
- Expanded state must be restored from daemon-backed data.

**FR-2**: Expansion persistence
- Expanding or collapsing a folder must persist through `set_folder_expanded`.

**FR-3**: Note lifecycle actions
- User can create, rename, move, and delete notes.
- User can create, rename, and remove directories.

**FR-4**: Selection contract
- Note selection must trigger a single consistent load path.
- Reload after mutation must preserve or deliberately clear selection according to the action.

**FR-5**: Guard points
- The explorer must expose a hook for “allow switch / reject switch / save then switch” decisions even if the first implementation only uses a simple boolean callback.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Replace deprecated tree widgets if practical inside this slice | Avoid building parity work on removed APIs | Slightly larger initial refactor |
| Keep explorer mutations in one slice | Mutation flows share the same tree and selection state | Larger explorer slice, but less later overlap |
| Add unsaved-change hook early | Prevents future editor rework when note switching becomes guarded | First implementation may feel abstract |

## Acceptance Criteria

- [ ] Explorer renders folders and notes from daemon data.
- [ ] Folder expanded state round-trips through the daemon.
- [ ] Note and directory mutations are supported and covered by tests.
- [ ] Selection behavior after mutation is deterministic and tested.
- [ ] Explorer code has a switch-guard extension point for editor integration.

## Related

- Depends on: `COMP-GTK-RUNTIME-001`
- Enables: `COMP-GTK-EDITOR-004`
