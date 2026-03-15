# Settings Notes

- The `Controls` section is intentionally present but still a placeholder. Future work can add keybindings, mouse gestures, and other local interaction preferences there without reshaping the left-pane navigation.
- Plugin state is currently read-only. If knotd exposes safe toggle/update operations later, that work should extend the existing plugin list rather than introducing a separate plugin management surface.
- App preferences currently cover color scheme and panel widths. If more local GTK-only preferences are added later, keep them in `~/.config/knot/knotty.toml` and avoid introducing a second persistence system.
