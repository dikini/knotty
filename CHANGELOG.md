# Changelog

This project follows Common Changelog: <https://common-changelog.org/>.

## Unreleased

### Fixed

- Ignore local `.worktrees/` directories so isolated feature worktrees do not pollute repository status.

### Added

- Add GTK parity specs, plans, local reference bundle, and repository gate scripts.
- Expand GTK note DTOs to include runtime contract fields for mode availability, metadata, embeds, media, and request-state helpers for async UI flows.
- Add a GTK-safe background bridge and migrate note loading to request-state-driven async execution.
- Add a discoverable `docs/notes/` area for per-subsystem future opportunities and seed the shell follow-up notes.

### Changed

- Clarify the local agent workflow docs to include a rust-skills and review pass in the non-trivial task execution flow, plus recording deferred future work in `docs/notes/`.

### Fixed

- Match the CLI default socket path with the documented `XDG_RUNTIME_DIR/knot/knot.sock` contract so the baseline Rust test suite passes.
- Align the GTK shell startup and routing implementation with the shell contract by adding a real search content surface, applying initial shell state on vault-open startup, and wiring actionable startup controls.
- Replace deprecated GTK file chooser usage in the shell startup flow with `gtk::FileDialog` and document the project policy against introducing deprecated GTK APIs.
- Keep the GTK shell usable when vault-info lookup fails after a successful daemon connection.
- Keep async note-load completion from overriding a newer Graph or Settings navigation choice.
- Complete the GTK shell search contract by routing search RPCs through the background bridge, adding explicit search view states, wiring the search focus shortcut, and simplifying startup-state refresh handling.
- Keep startup-only surfaces from being bypassed by the search shortcut, and avoid sticky search suppression when clearing an already-empty query.
- Keep the search shortcut cheap by consulting cached startup state, and refresh daemon-unavailable detail text when startup diagnostics change.
