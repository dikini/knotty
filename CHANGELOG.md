# Changelog

This project follows Common Changelog: <https://common-changelog.org/>.

## Unreleased

### Fixed

- Ignore local `.worktrees/` directories so isolated feature worktrees do not pollute repository status.
- Centralize the GTK socket runtime contract so the CLI, client defaults, tests, and README all use the canonical `.../knot/knotd.sock` path.
- Generate the CLI default-socket help text from the shared runtime-contract definition so path help cannot drift from runtime behavior.
- Remove the machine-specific `/run/user/1000` socket fallback and require `XDG_RUNTIME_DIR`, `--socket`, or `KNOTD_SOCKET_PATH` for the GTK daemon connection contract.

### Added

- Add GTK parity specs, plans, local reference bundle, and repository gate scripts.
- Expand GTK note DTOs to include runtime contract fields for mode availability, metadata, embeds, media, and request-state helpers for async UI flows.
- Add a GTK-safe background bridge and migrate note loading to request-state-driven async execution.
- Add a discoverable `docs/notes/` area for per-subsystem future opportunities and seed the shell follow-up notes.
- Add the approved GTK editor design note and tighten the editor implementation plan around source-authoritative synchronization, fixed icon-only mode controls, and pinned meta fields.
- Add the GTK explorer slice with async tree refresh, mutation actions, dirty-state guard wiring, deterministic selection fallback, and explorer follow-up notes.
- Fix explorer review follow-ups so empty tree selection no longer suppresses the next note activation, folder removal clears stale active-note state, and cleared note loads reset back to an idle request state.
- Restore explicit regression coverage for note-load cancellation after rebasing the explorer review fixes.

### Changed

- Clarify the local agent workflow docs to include a rust-skills and review pass in the non-trivial task execution flow, plus recording deferred future work in `docs/notes/`.

### Fixed

- Match the CLI default socket path with the documented `XDG_RUNTIME_DIR/knot/knotd.sock` contract so the GTK app matches the daemon and other clients.
- Align the GTK shell startup and routing implementation with the shell contract by adding a real search content surface, applying initial shell state on vault-open startup, and wiring actionable startup controls.
- Replace deprecated GTK file chooser usage in the shell startup flow with `gtk::FileDialog` and document the project policy against introducing deprecated GTK APIs.
- Keep the GTK shell usable when vault-info lookup fails after a successful daemon connection.
- Keep async note-load completion from overriding a newer Graph or Settings navigation choice.
- Complete the GTK shell search contract by routing search RPCs through the background bridge, adding explicit search view states, wiring the search focus shortcut, and simplifying startup-state refresh handling.
- Keep startup-only surfaces from being bypassed by the search shortcut, and avoid sticky search suppression when clearing an already-empty query.
- Keep the search shortcut cheap by consulting cached startup state, and refresh daemon-unavailable detail text when startup diagnostics change.
- Keep async note-load completion from forcing Search back to Notes unless the load was explicitly initiated from a search result.
- Simplify GTK shell callback wiring by removing row-name-coupled search activation, collapsing repeated search reset logic, dropping unnecessary window-level `ContextPanel` interior mutability, and centralizing startup refresh handles.
- Prevent an in-flight note load from re-populating the editor after the note has been cleared or deleted by bumping the load generation in `clear_active_note`.
- Remove unused `SimpleNoteList` that performed a blocking RPC call on the GTK main thread.
- Clarify `select_tree_item` by removing the unused `suppress_note` parameter — note activation is always suppressed for programmatic selection.
