# GTK Settings, Plugins, and Maintenance

## Metadata
- ID: `COMP-GTK-SETTINGS-007`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, DX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Provide the settings and maintenance surface required for functional parity with the Tauri app, including vault settings, plugin state, maintenance actions, and app-level display options.

## Scope

### In Scope
- settings navigation structure in the left context pane
- vault settings load/update flow
- plugin list and enablement state
- maintenance actions such as reindex
- app-level shell preferences needed by GTK parity
- local app-preferences persistence in `~/.config/knot/knotty.toml`

### Out of Scope
- UI automation action execution
- repo workflow settings

## Contract

### Functional Requirements

**FR-1**: Settings navigation
- GTK must provide a stable settings surface with explicit sections.
- When `ToolMode::Settings` is active, the left context pane must render the settings section list and the main pane must render the selected section.
- The initial section set for this slice is `General`, `Appearance`, `Controls`, `Vault`, `Plugins`, and `Maintenance`.

**FR-2**: Vault settings
- GTK can load and update vault settings through the daemon contract.

**FR-3**: Plugin visibility
- GTK can list plugins and show their enablement/effective state.

**FR-4**: Maintenance actions
- GTK can run maintenance actions such as reindex with visible progress/result feedback.

**FR-5**: Shell preferences
- GTK can persist and restore app-level shell preferences needed by parity from `~/.config/knot/knotty.toml`.

**FR-6**: Settings shell layout
- GTK must hide the right inspector rail in settings mode for this slice.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Separate settings from shell slice | Keeps shell routing independent from form complexity | More slices to manage |
| Build settings as data-driven sections | Reduces repetitive view logic and junior confusion | Slightly more up-front structure |
| Maintenance belongs with settings | Same user mental model and same contract family | Slightly broader settings slice |
| Use the left context pane for section navigation | Preserves the existing shell layout and avoids a second competing settings navigation UI | Settings mode depends on context-panel integration |
| Use `~/.config/knot/knotty.toml` for app-level preferences | Keeps app preferences explicit and separate from daemon-backed vault settings | Requires a small local config layer |
| Ship `Controls` as an intentionally incomplete section | Locks the information architecture early without inventing fake settings | Includes an empty section during active development |

## Acceptance Criteria

- [ ] Settings surface has stable sections in the left context pane and tests for section routing.
- [ ] Vault settings load and patch-update successfully.
- [ ] Plugin list renders and refreshes.
- [ ] Maintenance actions show loading/success/error states.
- [ ] App-level preferences needed by parity can be persisted in `~/.config/knot/knotty.toml`.
- [ ] Settings mode hides the inspector rail.

## Related

- Depends on: `COMP-GTK-SHELL-002`
- Enables: `COMP-GTK-AUTOMATION-008`
