# knot-gtk Docs

This directory tracks the design and planning work required to bring `knot-gtk` to functional parity with the Tauri UI in the upstream `knot` project, using GTK-native delivery slices.

## Structure

- `specs/component/`: one spec per delivery concern
- `specs/system/`: roadmap and dependency overview
- `plans/`: one implementation plan per spec
- `reference/`: frozen contracts and behavior references needed to implement the slices locally
- `notes/`: per-subsystem future opportunities, cleanup notes, and follow-up observations that are intentionally out of the current slice

## Slice Order

1. `COMP-GTK-RUNTIME-001` - runtime contract and async execution
2. `COMP-GTK-SHELL-002` - app shell, startup, navigation, search
3. `COMP-GTK-EXPLORER-003` - explorer tree and note lifecycle
4. `COMP-GTK-EDITOR-004` - editor core and mode handling
5. `COMP-GTK-NOTE-TYPES-005` - note types, media, embeds
6. `COMP-GTK-GRAPH-006` - graph UI and graph context
7. `COMP-GTK-SETTINGS-007` - settings, plugins, maintenance
8. `COMP-GTK-AUTOMATION-008` - UI automation and parity harnesses

## Working Rules

- Slice boundaries are chosen for GTK delivery order, not for 1:1 mirroring of Tauri components.
- The local source of truth for implementation is `docs/reference/`, not the upstream repository.
- Review-driven future opportunities that are not part of the current slice should be recorded in `docs/notes/`.
- Each slice must follow TDD: write failing tests, verify red, implement, verify green, review, fix, repeat.
- Each slice ends with a full relevant test run, a review pass, and fixes before completion.
- Commit only after the slice test gate is green.
