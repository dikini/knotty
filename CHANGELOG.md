# Changelog

This project follows Common Changelog: <https://common-changelog.org/>.

## Unreleased

### Added

- Add GTK parity specs, plans, local reference bundle, and repository gate scripts.
- Expand GTK note DTOs to include runtime contract fields for mode availability, metadata, embeds, media, and request-state helpers for async UI flows.
- Add a GTK-safe background bridge and migrate note loading to request-state-driven async execution.

### Changed

- Clarify the local agent workflow docs to include a rust-skills and review pass in the non-trivial task execution flow.

### Fixed

- Match the CLI default socket path with the documented `XDG_RUNTIME_DIR/knot/knot.sock` contract so the baseline Rust test suite passes.
- Align the GTK shell startup and routing implementation with the shell contract by adding a real search content surface, applying initial shell state on vault-open startup, and wiring actionable startup controls.
- Replace deprecated GTK file chooser usage in the shell startup flow with `gtk::FileDialog` and document the project policy against introducing deprecated GTK APIs.
