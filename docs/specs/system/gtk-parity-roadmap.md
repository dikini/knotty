# GTK Parity Roadmap

## Metadata
- ID: `SYS-GTK-PARITY-001`
- Scope: `system`
- Status: `proposed`
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Define the delivery order for bringing `knotty` to functional parity with the Tauri UI while minimizing cross-cutting work and merge conflicts.

## Delivery Strategy

The parity effort should not copy the Tauri repo layout. It should deliver GTK functionality in dependency order:

1. stabilize contracts and execution model
2. build a real shell around those contracts
3. make note browsing and note loading reliable
4. make editing and mode switching reliable
5. add note-type-specific rendering
6. add graph functionality
7. add settings and maintenance tooling
8. add automation and parity verification

When a slice needs contract or parity behavior guidance, use the local `docs/reference/` bundle rather than assuming access to another repository checkout.

## Slice Dependency Graph

```text
COMP-GTK-RUNTIME-001
  -> COMP-GTK-SHELL-002
  -> COMP-GTK-EXPLORER-003

COMP-GTK-SHELL-002
  -> COMP-GTK-EDITOR-004
  -> COMP-GTK-GRAPH-006
  -> COMP-GTK-SETTINGS-007

COMP-GTK-EXPLORER-003
  -> COMP-GTK-EDITOR-004

COMP-GTK-EDITOR-004
  -> COMP-GTK-NOTE-TYPES-005

COMP-GTK-RUNTIME-001
  -> COMP-GTK-AUTOMATION-008

COMP-GTK-SHELL-002
  -> COMP-GTK-AUTOMATION-008

COMP-GTK-SETTINGS-007
  -> COMP-GTK-AUTOMATION-008
```

## Parallelization Rules

- `COMP-GTK-SHELL-002` and `COMP-GTK-EXPLORER-003` can start after the runtime slice lands.
- `COMP-GTK-GRAPH-006` and `COMP-GTK-SETTINGS-007` should stay out of editor files and can run in parallel after the shell slice lands.
- `COMP-GTK-NOTE-TYPES-005` should wait until the editor-core slice lands because it extends mode-specific rendering and note contracts.
- `COMP-GTK-AUTOMATION-008` should land last because it depends on stable shell/view identifiers and settings integration.

## Non-Goals

- repo migration into `../knot`
- shared workspace tooling changes
- dev workflow redesign in the main repo
- visual identity cloning of the Tauri app

## Completion Criteria

The roadmap is complete when:

- every slice spec is implemented or explicitly descoped
- GTK can perform the same core workflows as Tauri
- parity tests and manual review checklists exist for each slice
- remaining gaps are documented as explicit non-goals instead of hidden drift
