# GTK Automation Smoke Checklist

## Purpose

Manual smoke checklist for the gated semantic automation surface in `knot-gtk`.

## Preconditions

- `knotd` is running and a vault is open.
- `~/.config/knot/knotty.toml` contains:

```toml
[automation]
enabled = true
```

- `knot-gtk` is started with:

```bash
knot-gtk --enable-automation --automation-token dev-token
```

## Expected Startup Signals

- Header shows `Automation active`.
- `knot-gtk` reaches `VaultOpen`.
- Search, graph, settings, and editor surfaces remain usable without automation attached.

## Discovery Smoke

- `describe_ui_automation` reports:
  - `protocol_version = 1`
  - `snapshot_schema_version = 1`
  - `action_catalog_version = 1`
  - `available = true`
- The action catalog includes:
  - `switch_tool`
  - `focus_search`
  - `select_note`
  - `clear_selection`
  - `set_editor_mode`
  - `open_settings_section`
  - `set_graph_scope`
  - `set_graph_depth`
  - `reset_graph`

## Snapshot Smoke

- `get_ui_snapshot` returns stable properties for:
  - `tool.active`
  - `content.active`
  - `startup.state`
  - `editor.dirty`
  - `automation.active`
- Settings mode exposes `settings.section`.
- Graph mode exposes `graph.scope`, `graph.depth`, and `graph.selected_path` when applicable.

## Action Smoke

1. `switch_tool(settings)`
- content switches to settings
- left context pane shows the settings section list
- inspector stays hidden

2. `open_settings_section(plugins)`
- settings pane switches to `Plugins`
- snapshot reports `settings.section = plugins`

3. `focus_search`
- tool switches to search
- search entry is focused

4. `set_graph_scope(vault)` then `set_graph_depth(2)`
- tool switches to graph
- graph status updates
- snapshot reports the requested scope and depth

5. `select_note(path)`
- when editor is clean, note load is dispatched through the shared note loader
- when editor is dirty, result returns `dirty_guard_blocked`

6. `set_editor_mode(view)`
- succeeds only when a note is loaded and the mode is available
- unsupported requests return `unsupported_context`

7. `clear_selection`
- clears the active note when the editor is clean
- returns `dirty_guard_blocked` when the editor is dirty

## Gate Smoke

- Start without config opt-in:
  - discovery reports `available = false`
  - reason is `config_opt_in_required`
- Start without runtime flag/token:
  - discovery reports `available = false`
  - reason is `runtime_token_required`
- In both disabled cases:
  - action dispatch returns `automation_disabled`
  - normal UI use still works
