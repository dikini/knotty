# Changelog

This project follows Common Changelog: <https://common-changelog.org/>.

## Unreleased

### Fixed

- Ignore local `.worktrees/` directories so isolated feature worktrees do not pollute repository status.

### Added

- Add GTK parity specs, plans, local reference bundle, and repository gate scripts.
- Expand GTK note DTOs to include runtime contract fields for mode availability, metadata, embeds, media, and request-state helpers for async UI flows.
- Add a GTK-safe background bridge and migrate note loading to request-state-driven async execution.

### Changed

- Clarify the local agent workflow docs to include a rust-skills and review pass in the non-trivial task execution flow.

### Fixed

- Match the CLI default socket path with the documented `XDG_RUNTIME_DIR/knot/knot.sock` contract so the baseline Rust test suite passes.
- Keep async note-load completion from overriding a newer Graph or Settings navigation choice.
