# Editor Notes

- The current `edit` mode intentionally stays on a shared markdown text buffer with command buttons rather than reviving the dormant `block_editor` module. Reintroduce a richer structured editor only if a later slice needs behavior that cannot stay source-authoritative with the simpler surface.
- The editor slice relies on pure synchronization and markdown-command tests more than widget-level interaction tests. If future editor work increases GTK-specific behavior, add a focused harness for mode-switch and viewport restoration flows.
- The metadata editor only exposes scalar frontmatter fields directly. Preserve unsupported frontmatter structure until a later slice can round-trip nested metadata safely, and keep discard aligned with the last successful save baseline.
- Dirty note-switch approval now dispatches the selected load directly instead of caching a one-shot allow token. Keep future switch-prompt work on that direct-dispatch path so stale approvals cannot bypass later dirty checks.
