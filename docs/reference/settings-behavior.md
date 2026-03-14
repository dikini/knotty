# Settings Behavior Reference

## Purpose

Define the settings, plugin, and maintenance behavior expected from the GTK settings surface.

## Tool Calls

- `get_vault_settings`
- `update_vault_settings`
- `list_vault_plugins`
- `reindex_vault`
- optionally `sync_external_changes`

## Settings Type Templates

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultSettings {
    pub editor: VaultEditorSettings,
    #[serde(default)]
    pub graph: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub app: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultEditorSettings {
    pub default_mode: Option<String>,
    pub vim_mode: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultPluginInfo {
    pub id: String,
    pub title: String,
    pub enabled: bool,
}
```

## Request Examples

```json
{
  "name": "get_vault_settings",
  "arguments": {}
}
```

```json
{
  "name": "update_vault_settings",
  "arguments": {
    "patch": "{\"editor\":{\"default_mode\":\"edit\"}}"
  }
}
```

```json
{
  "name": "reindex_vault",
  "arguments": {}
}
```

## Required Behavior

- settings load asynchronously
- settings edits are explicit and reviewable
- plugin state is shown clearly even if toggling is initially read-only
- maintenance actions show progress or at least pending/success/error feedback
- settings changes route through shell settings mode and inspector settings mode consistently

## Test Cases

- default mode setting round-trips through patch generation
- maintenance action shows busy then success state
- plugin list empty state is explicit
- failed patch update keeps the prior visible state until reload
