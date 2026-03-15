# GTK App Shell, Startup, Navigation, and Search

## Metadata
- ID: `COMP-GTK-SHELL-002`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, UX]
- Created: `2026-03-14`
- Updated: `2026-03-15`

## Purpose
Turn the GTK prototype window into a real application shell with startup states, tool/context/inspector behavior, and search UX that matches the functional capabilities of the Tauri app.

## Scope

### In Scope
- no-vault startup and connected-vault states
- tool rail, context panel, content area, and inspector rail interaction rules
- shell mode transitions between notes, search, graph, and settings
- recent-vault and open/create entry points if daemon contract supports them
- search surface, focus shortcut, result activation, and keyboard navigation

### Out of Scope
- explorer tree mutations
- full editor behavior
- graph rendering internals
- settings form implementation details

## Contract

### Functional Requirements

**FR-1**: Distinct startup states
- The window must differentiate: daemon unavailable, daemon available with no vault, and vault open.
- No-vault state must expose the vault-opening path rather than only header text.

**FR-2**: Shell regions
- GTK shell must expose tool rail, context panel, main content, and optional inspector rail.
- Each region must have explicit ownership of what it renders and when it is visible.

**FR-3**: Mode routing
- Tool selection must switch shell routing consistently between notes, search, and graph.
- Settings must be reachable without corrupting note/editor state.

**FR-4**: Search UX
- Search must include keyboard focus, debounce, result list, activation, and empty/error states.
- Search activation must open the selected note in the main content area.

**FR-5**: Inspector behavior
- Inspector rail must support at least `details` and `settings` modes, even if details content stays minimal at first.

**FR-6**: Supported GTK APIs only
- Shell startup and navigation work must not introduce deprecated GTK or libadwaita APIs when a supported replacement exists in the current baseline.
- File and folder selection flows must use the non-deprecated async dialog API for the configured GTK baseline.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Deliver shell before deeper feature parity | Later slices need stable navigation and state ownership | Delays some user-facing details |
| Keep GTK-native layout widgets | Preserves native behavior and avoids HTML-style state modeling | Requires GTK-specific testing patterns |
| Search belongs to shell slice | Search depends on routing and selection more than explorer internals | Slightly larger shell slice |

## Acceptance Criteria

- [ ] GTK startup presents actionable no-vault and vault-open states.
- [ ] Tool/context/inspector interactions follow a documented routing policy.
- [ ] Search can focus, query, navigate, activate, and show empty/error states.
- [ ] Settings mode can be opened from shell chrome.
- [ ] Shell tests cover mode routing and startup transitions.

## Related

- Depends on: `COMP-GTK-RUNTIME-001`
- Enables: `COMP-GTK-EDITOR-004`, `COMP-GTK-GRAPH-006`, `COMP-GTK-SETTINGS-007`
