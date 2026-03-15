# Settings Notes

- The `Controls` section is intentionally present but still a placeholder. Future work can add keybindings, mouse gestures, and other local interaction preferences there without reshaping the left-pane navigation.
- The plugin inventory list is still read-only. If knotd exposes per-plugin toggle/update operations later, that work should extend the existing plugin list rather than introducing a separate plugin management surface.
- The master `plugins_enabled` toggle lives in `Plugins` for UX clarity even though it still patches vault-backed daemon settings.
- App preferences currently cover color scheme and panel widths. If more local GTK-only preferences are added later, keep them in `~/.config/knot/knotty.toml` and avoid introducing a second persistence system.
