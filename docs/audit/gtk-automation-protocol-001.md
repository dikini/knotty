# GTK Automation Protocol Handoff

## Purpose

Concrete handoff note for `knotd` integration against the GTK semantic automation surface.

## Gate Rules

Automation is available only when both are true:

1. `~/.config/knot/knotty.toml`

```toml
[automation]
enabled = true
```

2. Runtime startup includes:

```bash
knot-gtk --enable-automation --automation-token <TOKEN>
```

## Discovery Call

### Shape

```json
{
  "protocol_version": 1,
  "snapshot_schema_version": 1,
  "action_catalog_version": 1,
  "available": true,
  "unavailable_reason": null,
  "requires_config_opt_in": true,
  "requires_runtime_token": true,
  "actions": [
    {
      "action_id": "switch_tool",
      "title": "Switch Tool",
      "description": "Switch the active shell tool.",
      "argument_schema": {
        "type": "object",
        "required": ["tool"],
        "properties": {
          "tool": {
            "type": "string",
            "enum": ["notes", "search", "graph", "settings"]
          }
        }
      },
      "preconditions": ["startup.state == vault_open"],
      "result_codes": ["ok", "automation_disabled", "startup_blocked", "invalid_arguments"]
    }
  ],
  "result_codes": [
    "ok",
    "automation_disabled",
    "invalid_token",
    "startup_blocked",
    "dirty_guard_blocked",
    "unsupported_context",
    "not_found",
    "invalid_arguments"
  ]
}
```

## Snapshot Call

### Shape

```json
{
  "active_tool": "settings",
  "active_content": "settings",
  "startup_state": "vault_open",
  "inspector_visible": false,
  "active_note_path": "notes/example.md",
  "editor_mode": "edit",
  "editor_dirty": false,
  "search_query": "graph",
  "graph_scope": "neighborhood",
  "graph_depth": 2,
  "graph_selected_path": "notes/example.md",
  "settings_section": "plugins",
  "automation_active": true,
  "properties": {
    "tool.active": "settings",
    "content.active": "settings",
    "startup.state": "vault_open",
    "editor.mode": "edit",
    "editor.dirty": "false",
    "settings.section": "plugins",
    "automation.active": "true"
  }
}
```

## Action Call

### Request examples

```json
{ "action": "switch_tool", "tool": "graph" }
```

```json
{ "action": "open_settings_section", "section": "plugins" }
```

```json
{ "action": "set_editor_mode", "mode": "view" }
```

### Result example

```json
{
  "action_id": "open_settings_section",
  "ok": true,
  "result_code": "ok",
  "message": null,
  "snapshot": {
    "active_tool": "settings",
    "active_content": "settings",
    "startup_state": "vault_open",
    "settings_section": "plugins",
    "automation_active": true,
    "properties": {
      "tool.active": "settings",
      "settings.section": "plugins",
      "automation.active": "true"
    }
  }
}
```

## Notes For `knotd`

- Use discovery first; do not assume all actions are available.
- Prefer stable result codes over message parsing.
- Treat `snapshot.properties` as the flexible compatibility layer for agent clients.
- `select_note` reuses the shared GTK note loader and is asynchronous from the user’s perspective.
- Dirty-note behavior is intentional:
  - automation does not bypass note-switch guards
  - blocked note-changing actions return `dirty_guard_blocked`
