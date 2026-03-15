# Editor Notes

- The current `edit` mode intentionally stays on a shared markdown text buffer with command buttons rather than reviving the dormant `block_editor` module. Reintroduce a richer structured editor only if a later slice needs behavior that cannot stay source-authoritative with the simpler surface.
- The editor slice relies on pure synchronization and markdown-command tests more than widget-level interaction tests. If future editor work increases GTK-specific behavior, add a focused harness for mode-switch and viewport restoration flows.
