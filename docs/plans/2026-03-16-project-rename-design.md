# Project Rename Design Notes

## Purpose

Capture the approved project-identity rename from `knot-gtk` to `knotty` before implementation spreads across package metadata, crate imports, tests, and repository docs.

## Locked Decisions

- `knotty` is the official project name for this repository.
- The Cargo package name changes from `knot-gtk` to `knotty`.
- The Rust library crate path changes from `knot_gtk` to `knotty`.
- The produced application binary changes from `knot-gtk` to `knotty`.
- User-facing strings, help text, logging targets, fuzz-package references, and repository docs should prefer `knotty` over `knot-gtk`.
- Existing local app-preferences naming stays as `knotty.toml`; it already matches the target identity and does not need a second rename.

## Implications

- Any direct crate imports in `src/`, `tests/`, and `fuzz/` must be updated together so the rename remains buildable at every layer.
- CLI-oriented tests should assert the new public name so future regressions do not silently reintroduce `knot-gtk` in shipped output.
- Existing planning and reference docs should be updated where they describe the current application identity or invocation examples, even if older slice names remain GTK-specific.
- This rename is behavior-preserving apart from the exposed project identity, so verification should focus on name-bearing outputs plus normal Rust build/test gates.
