# Changelog

This project follows Common Changelog: <https://common-changelog.org/>.

## Unreleased

### Added

- Add GTK parity specs, plans, local reference bundle, and repository gate scripts.
- Expand GTK note DTOs to include runtime contract fields for mode availability, metadata, embeds, media, and request-state helpers for async UI flows.
- Add a GTK-safe background bridge and migrate note loading to request-state-driven async execution.

### Fixed

- Match the CLI default socket path with the documented `XDG_RUNTIME_DIR/knot/knot.sock` contract so the baseline Rust test suite passes.
- Keep async note-load completion from overriding a newer Graph or Settings navigation choice.
