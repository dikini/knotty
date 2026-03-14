# GTK Parity Reference Bundle

This directory is the local source of truth for implementing GTK parity work without requiring access to the main `knot` repository.

## Use This Directory For

- daemon transport and JSON-RPC framing
- frozen request and response examples
- Rust type templates for GTK-side DTOs
- parity behavior rules for shell, explorer, editor, graph, settings, and automation

## Reference Files

- `knotd-protocol.md`: transport, framing, request flow, and common daemon calls
- `note-contract.md`: note payload shapes, enums, metadata, media, and embed DTOs
- `shell-behavior.md`: startup states, tool routing, content routing, and search behavior
- `explorer-behavior.md`: tree loading, folder state, note activation, and mutation behavior
- `editor-behavior.md`: mode routing, dirty-state rules, save behavior, and synchronization rules
- `graph-behavior.md`: graph layout data, graph scope behavior, and activation rules
- `settings-behavior.md`: settings payloads, plugin state, and maintenance action behavior
- `automation-behavior.md`: semantic snapshot model and parity verification hooks

## Working Rule

If a plan tells a developer to match a contract field or behavior, the developer should use the matching reference file in this directory rather than checking another repository.
