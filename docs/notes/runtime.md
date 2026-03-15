# Runtime Notes

## 2026-03-15

- GTK startup now retains the main `KnotWindow` wrapper for the application lifetime. Earlier activation code could drop the wrapper immediately after `present()`, which made window lifetime debugging harder and risked startup regressions.
- Note decoding is intentionally tolerant of partial nested payloads from `knotd` for `embed`, `headings`, `backlinks`, `media`, and `available_modes`. Future contract tightening should happen only after the daemon payload shape is stable and verified across note types.
