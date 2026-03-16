# GTK Runtime Contract and Async Execution Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** align `knotty` contracts and execution flow with parity needs so later UI slices can build on stable async data access.

**Architecture:** keep `knotty` as a daemon-backed GTK frontend, but add a small async/request-state layer between UI callbacks and daemon RPC. Expand client DTOs to match needed parity fields before feature slices depend on them.

**Tech Stack:** Rust, gtk4, libadwaita, glib main-context messaging, tokio/background tasks, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/knotd-protocol.md` and `docs/reference/note-contract.md`
- This slice is the foundation for later slices. Do not take shortcuts here.
- Prefer one small helper at a time over a large runtime abstraction.

## Delivery Notes

- Mandatory workflow for every task: write tests, confirm red, implement minimal code, confirm green, review, fix.
- Prefer adding helper code in small units instead of introducing a large framework.
- Use the local reference docs as the source of truth for protocol framing and note DTO field names.
- Do not commit until the full slice verification section is green.

## Execution Status

- Completed in worktree `feature/gtk-runtime-contract`.
- `GTR-001` was already satisfied in the branch baseline and was verified rather than reimplemented.
- `GTR-002` through `GTR-006` were implemented, including async note loading, stale-result protection, and bridge disconnect handling.
- Review follow-up: async note-load completion now preserves a newer Graph or Settings surface instead of forcing the editor route after late results.
- Contract correction: the daemon-backed default socket path is `XDG_RUNTIME_DIR/knot/knotd.sock`, matching the running `knotd` service and other clients without guessing a machine-specific `/run/user/<uid>` fallback.
- Final verification: `cargo fmt --check`, `cargo test`, and `cargo check` passed on the completed slice.

## Rust Guidance For This Slice

- Use `Result` and explicit error types for request and decode failures.
- Do not add `unwrap()` to production code while decoding daemon payloads.
- Prefer borrowing when building request helpers; do not clone payload strings unless they must cross thread boundaries.
- Add context when converting daemon errors into UI-facing errors.

## knotd Protocol Notes

### Transport

- Unix domain socket
- JSON-RPC `2.0`
- framed with `Content-Length: <bytes>\r\n\r\n<payload>`

### Base request shape

```json
{
  "jsonrpc": "2.0",
  "id": 1001,
  "method": "tools/call",
  "params": {
    "name": "get_note",
    "arguments": {
      "path": "notes/example.md"
    }
  }
}
```

### Base response shape

```json
{
  "jsonrpc": "2.0",
  "id": 1001,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"id\":\"...\",\"path\":\"notes/example.md\"}"
      }
    ]
  }
}
```

### Communication sequence

1. open Unix socket
2. write framed JSON-RPC request
3. read response headers
4. read `Content-Length` bytes
5. decode JSON-RPC response
6. extract `result.content[0].text`
7. deserialize that JSON string into a Rust type

### Rust types to use or extend

```rust
pub enum ClientError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Utf8(std::string::FromUtf8Error),
    Rpc(String),
    Connection(String),
    MissingResult,
    NoVaultOpen,
}

pub struct KnotdClient {
    socket_path: String,
}
```

### Junior developer advice

- Do not bypass `call_tool()` with one-off socket code in UI modules.
- If you need a new tool method, add it to `KnotdClient` and add a typed return struct next to the other DTOs.

## Suggested Task Ownership

- One developer can own `src/client/mod.rs`.
- A second developer can own the async/request-state helpers under `src/ui/`.
- A third developer can own startup-contract tests in `src/cli.rs`.

Parallel work is allowed after `GTR-001` if the developers do not edit the same files.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTR-001 | Fix CLI startup contract tests | - |
| GTR-002 | Expand client DTO coverage | GTR-001 |
| GTR-003 | Add request-state types and unit tests | GTR-002 |
| GTR-004 | Add background execution bridge | GTR-003 |
| GTR-005 | Migrate one representative flow off blocking UI calls | GTR-004 |
| GTR-006 | Apply review fixes and full verification | GTR-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTR-001A | GTR-001 | Add failing XDG runtime-dir test | `src/cli.rs` | `src/ui/*` |
| GTR-001B | GTR-001 | Add failing missing-runtime test | `src/cli.rs` | `src/ui/*` |
| GTR-001C | GTR-001 | Fix path builder | `src/cli.rs` | `src/client/*` |
| GTR-002A | GTR-002 | Add failing `available_modes` decode test | `src/client/mod.rs` | `src/ui/*` |
| GTR-002B | GTR-002 | Add failing `media` decode test | `src/client/mod.rs` | `src/ui/*` |
| GTR-002C | GTR-002 | Add failing `metadata` and `embed` decode tests | `src/client/mod.rs` | `src/ui/*` |
| GTR-002D | GTR-002 | Implement note contract structs | `src/client/mod.rs` | `src/ui/*` |
| GTR-003A | GTR-003 | Add failing request-state enum tests | `src/ui/request_state.rs`, `src/ui/mod.rs` | `src/client/*` |
| GTR-003B | GTR-003 | Implement request-state type | `src/ui/request_state.rs` | `src/client/*` |
| GTR-004A | GTR-004 | Add failing background-work forwarding test | `src/ui/async_bridge.rs` | `src/ui/window.rs` |
| GTR-004B | GTR-004 | Implement bridge helper | `src/ui/async_bridge.rs`, `src/main.rs` | `src/client/*` |
| GTR-005A | GTR-005 | Add failing async note-load flow test | `src/ui/window.rs` | `src/ui/search_view.rs` |
| GTR-005B | GTR-005 | Migrate note load to bridge | `src/ui/window.rs` | `src/client/*` |
| GTR-005C | GTR-005 | Add request-state wiring to selected flow | `src/ui/window.rs` | `src/ui/explorer.rs` |
| GTR-006A | GTR-006 | Run slice verification | repo-wide | - |
| GTR-006B | GTR-006 | Fix slice-introduced warnings | touched files only | unrelated modules |

### Task GTR-001: Fix CLI startup contract tests

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/cli.rs`

**Steps**
1. Add or tighten tests for `XDG_RUNTIME_DIR` and missing-runtime behavior.
2. Run `cargo test cli::tests::test_default_socket_path -- --exact` and confirm red.
3. Fix `default_socket_path()` to match the documented `XDG_RUNTIME_DIR/knot/knotd.sock` contract and avoid a user-specific fallback path.
4. Re-run the same test until green.
5. Review for duplicated path-building logic and keep only the minimal helper extraction needed.

**Implementation notes**
- Keep the function tiny.
- Match README behavior exactly or update docs in a later docs-only cleanup task if the contract intentionally changes.

**Example test skeleton**

```rust
#[test]
fn default_socket_path_uses_xdg_runtime_dir_knot_subdirectory() {
    std::env::set_var("XDG_RUNTIME_DIR", "/tmp/runtime");
    assert_eq!(
        CliArgs::default_socket_path(),
        std::path::PathBuf::from("/tmp/runtime/knot/knotd.sock")
    );
}
```

### Task GTR-002: Expand client DTO coverage

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/client/mod.rs`
- Reference: `/home/dikini/Projects/knot-gtk/docs/reference/note-contract.md`

**Steps**
1. Add serde tests or focused deserialization tests for note payloads carrying `available_modes`, `metadata`, `embed`, and `media`.
2. Run the targeted test and confirm red because the fields are missing.
3. Add the missing Rust-side structs/enums and default handling.
4. Re-run targeted tests until green.
5. Review field naming against the local note contract reference and remove any guessed field names.

**Implementation notes**
- Junior-safe rule: copy the field names from `docs/reference/note-contract.md`, not from memory.
- Use optional fields unless the backend guarantees presence.

**Example test skeleton**

```rust
#[test]
fn note_data_deserializes_optional_media_and_mode_fields() {
    let payload = serde_json::json!({
        "id": "n1",
        "path": "notes/a.md",
        "title": "A",
        "content": "# A",
        "created_at": 0,
        "modified_at": 0,
        "word_count": 1,
        "headings": [],
        "backlinks": [],
        "note_type": "pdf",
        "available_modes": { "meta": false, "source": false, "edit": false, "view": true },
        "media": { "mime_type": "application/pdf", "file_path": "/tmp/a.pdf" }
    });

    let note: NoteData = serde_json::from_value(payload).expect("note payload should deserialize");
    assert!(matches!(note.note_type, Some(NoteType::Pdf)));
    assert!(note.media.is_some());
}
```

### Task GTR-003: Add request-state types and unit tests

**Files**
- Create: `/home/dikini/Projects/knot-gtk/src/ui/request_state.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/mod.rs`

**Steps**
1. Write unit tests for a small enum or struct representing idle/loading/success/error.
2. Confirm red because the module does not exist.
3. Implement the minimal request-state type and helper constructors.
4. Run the new tests until green.
5. Review names and keep the API intentionally small.

**Implementation notes**
- Avoid generic-heavy abstractions if they make the type harder for juniors to use.
- A simple `enum RequestState<T, E>` is enough.

**Advice**

- Add helper methods like `is_loading()` only if a current call site needs them.
- Do not invent retry logic in this slice.

### Task GTR-004: Add background execution bridge

**Files**
- Create: `/home/dikini/Projects/knot-gtk/src/ui/async_bridge.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/main.rs`

**Steps**
1. Write a focused unit/integration test for a helper that runs work off the main thread and returns a main-thread callback result.
2. Confirm red.
3. Implement a minimal bridge using `glib` channels or main-context invocation.
4. Re-run targeted tests until green.
5. Review for GTK-thread-safety mistakes: no widget mutation off the main thread.

**Implementation notes**
- The helper should own “run background closure, then invoke UI closure”.
- Keep error transport explicit rather than logging and swallowing failures.

**Advice**

- If a test against `glib` main-context behavior is awkward, test the pure helper logic and keep the GTK integration very small.
- Add one comment in the bridge explaining why widget updates must stay on the main thread.

### Task GTR-005: Migrate one representative flow off blocking UI calls

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/search_view.rs`

**Steps**
1. Pick one representative path, preferably note loading from selection.
2. Write a regression test or adapter-level test asserting the UI path uses the async bridge and updates request state.
3. Confirm red.
4. Replace the direct synchronous daemon call with bridge-based execution.
5. Run targeted tests until green.
6. Review for duplicated glue code that should move into a small helper.

**Implementation notes**
- Do not convert every call site in this slice.
- The goal is to prove the pattern and make later slices safe.

**Advice**

- Use note loading as the example path because later slices all depend on it.
- Keep search migration partial in this slice unless the same helper can be reused with almost no extra work.

### Task GTR-006: Apply review fixes and full verification

**Steps**
1. Run `cargo fmt`.
2. Run `cargo test`.
3. Run `cargo check`.
4. Review warnings and fix the ones introduced by this slice.
5. Perform a quick manual startup smoke run if the environment allows it.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- No direct widget updates from background threads
- DTO names match the daemon contract
- Error states can be surfaced to UI code
- Startup contract tests reflect real behavior
- Targeted tests clearly fail before implementation

## Commit Gate

Commit only when all commands above are green.
