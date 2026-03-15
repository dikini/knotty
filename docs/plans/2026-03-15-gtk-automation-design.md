# GTK Automation Design Notes

## Purpose

Capture the approved design for `COMP-GTK-AUTOMATION-008` before implementation details drift between GTK, `knotd`, and the parity harnesses.

## Locked Decisions

- Automation is semantic and daemon-mediated, not widget-addressed.
- `knotd` must be able to query GTK state and dispatch semantic UI actions through a typed local automation contract.
- Automation is disabled by default and becomes available only when both are true:
  - local config enables it in `~/.config/knot/knotty.toml`
  - the app is started with a runtime automation token/enable flag
- GTK must show a visible automation-active indicator when the automation surface is live.
- This slice does not include text entry, generic button clicking, or arbitrary widget addressing.

## Protocol Shape

- Discovery call:
  - `describe_ui_automation`
  - returns protocol version, availability, gating status, snapshot schema version, supported actions, and stable result codes
- Snapshot call:
  - `get_ui_snapshot`
  - returns typed semantic fields plus a normalized property map
- Action call:
  - `dispatch_ui_action`
  - accepts a typed semantic action and returns a typed result plus optional updated snapshot

## Initial Action Surface

- switch tool
- focus search
- select note by path
- clear selection
- set editor mode
- open settings section
- set graph scope
- set graph depth
- reset graph

## Snapshot Expectations

- active tool
- active content view
- startup state
- active note path
- editor mode
- dirty state
- active settings section
- graph scope/depth
- inspector visibility
- automation availability/active state

## Implications

- GTK should project existing shell/editor/settings/graph state into one automation snapshot instead of creating a second independent UI state model.
- GTK action dispatch should reuse existing routing and guard paths instead of inventing parallel automation-only code paths.
- The protocol must be explicit enough that the later `knotd` updates can be implemented from local docs without inferring schemas from GTK internals.
