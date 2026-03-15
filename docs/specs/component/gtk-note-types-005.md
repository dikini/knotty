# GTK Note Types, Media, and Embed Surfaces

## Metadata
- ID: `COMP-GTK-NOTE-TYPES-005`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, UX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Add note-type-aware behavior so GTK can handle markdown, YouTube, image, PDF, and unknown note types with the same functional coverage as the Tauri UI.

## Scope

### In Scope
- note-type-specific mode availability
- image and PDF view surfaces
- YouTube note view surface
- embed metadata rendering when present
- note list and explorer badges/icons aligned with note types

### Out of Scope
- shell navigation rules already covered elsewhere
- UI automation commands

## Contract

### Functional Requirements

**FR-1**: Note type awareness
- Editor and shell must use note-type data from the contract, not filename guesses only.

**FR-2**: View-only note types
- PDF and image note types must disable unsupported modes and render stable view surfaces.
- Unknown note types must follow the contract defaults unless `available_modes` overrides them.

**FR-3**: YouTube note support
- YouTube notes must show title and outbound/open behavior using available metadata.

**FR-4**: Media surfaces
- Image notes must render the image safely.
- PDF notes must provide a stable fallback surface with a primary action that opens the document in the system PDF viewer.

**FR-5**: Embed support
- If note embed metadata is present, GTK must render a safe equivalent surface or a clear fallback that preserves the primary action.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Separate note-type slice after editor core | Keeps core editor work focused and avoids mixing text-edit and media-view concerns | Note types land later |
| Safe fallback for unsupported embed shapes | Functional parity matters more than visual cloning | Some embed views may be simpler than Tauri initially |
| Contract-driven mode gating | Prevents accidental access to invalid modes | Requires early contract alignment |

## Acceptance Criteria

- [ ] Note type data drives iconography, mode availability, and view routing.
- [ ] Image notes render with tests.
- [ ] PDF notes expose a tested system-open fallback.
- [ ] YouTube notes render and can open their primary link.
- [ ] Embed metadata has either a GTK rendering or a tested fallback action.

## Related

- Depends on: `COMP-GTK-EDITOR-004`
