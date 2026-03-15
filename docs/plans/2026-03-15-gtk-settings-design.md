# GTK Settings Design Notes

## Purpose

Capture the local design choices for `COMP-GTK-SETTINGS-007` before implementation details drift from the shell layout and settings contract.

## Locked Decisions

- When `ToolMode::Settings` is active, the left context pane renders the settings section list.
- The main pane renders the selected settings section content.
- The right inspector rail stays hidden in settings mode for this slice.
- The initial settings sections are `General`, `Appearance`, `Controls`, `Vault`, `Plugins`, and `Maintenance`.
- `Controls` ships now as an intentionally incomplete section so the information architecture stays stable during active development.
- Vault settings remain daemon-backed through `get_vault_settings` and `update_vault_settings`.
- The master `plugins_enabled` toggle remains vault-backed but lives in the `Plugins` section because that is the correct user-facing grouping.
- App-level preferences persist locally in `~/.config/knot/knotty.toml`.

## Implications

- Settings navigation should reuse existing context-panel plumbing instead of introducing a second settings-specific navigation strip in the main pane.
- The settings module should keep the split explicit:
  - daemon-backed vault state and maintenance actions
  - local app preferences loaded from and saved to `knotty.toml`
- Widget state should be derived from those stores rather than persisted independently through toolkit-specific settings APIs.
- Partial or placeholder sections are acceptable during this slice, but they must render intentionally and keep routing/tests stable.
