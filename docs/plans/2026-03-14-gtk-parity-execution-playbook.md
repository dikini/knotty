# GTK Parity Execution Playbook

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement slice plans task-by-task.

**Goal:** give junior developers a repeatable way to execute the GTK parity slice plans safely and consistently.

**Audience:** engineers who know Rust and GTK basics, but do not yet know this codebase well.

---

## Core Rules

### TDD Rule

For every behavior change:

1. write the failing test
2. run the test and confirm it fails for the expected reason
3. write the smallest implementation that can pass
4. run the targeted test until green
5. run the local slice test set
6. review the code and fix any obvious problems

Do not skip the red step. A test that never failed is not strong evidence.

### Rust Rules

Use these rules from `rust-skills` throughout the parity work:

- `err-result-over-panic`: return `Result` for expected failures
- `err-no-unwrap-prod`: do not add `unwrap()` in production code
- `err-context-chain`: add context when returning errors across boundaries
- `own-borrow-over-clone`: prefer borrowing over cloning unless ownership is required
- `own-slice-over-vec`: accept slices and `&str` in helpers where practical
- `test-descriptive-names`: test names should describe behavior
- `test-arrange-act-assert`: keep tests easy to scan

### GTK Threading Rule

- background work must not touch GTK widgets directly
- widget updates must happen on the GTK main thread
- if you are unsure whether code is on the main thread, assume it is not safe and add an explicit handoff

## Standard Task Workflow

### Before starting a task

1. Read the task and identify exact files.
2. Read the existing code in those files before editing.
3. Write down the expected user-visible behavior in one sentence.
4. Decide what the smallest testable unit is.

### During the task

1. Add or update tests first.
2. Run only the new or affected tests.
3. Implement the smallest passing change.
4. Re-run the targeted tests.
5. Refactor only if tests are green.

### After the task

1. Run the slice-local test set.
2. Read the diff and remove dead code.
3. Check whether names, error messages, and comments are still clear.

## Test Templates

### Rust unit test template

```rust
#[test]
fn selects_settings_mode_when_settings_button_pressed() {
    // Arrange
    let mut state = ShellState::default();

    // Act
    state.open_settings();

    // Assert
    assert_eq!(state.main_view(), MainView::Settings);
}
```

### Result-returning helper test template

```rust
#[test]
fn returns_error_when_payload_is_missing_required_field() {
    // Arrange
    let payload = serde_json::json!({
        "id": "note-1",
        "path": "notes/a.md"
    });

    // Act
    let result = serde_json::from_value::<NoteData>(payload);

    // Assert
    assert!(result.is_err());
}
```

### Async bridge test template

```rust
#[test]
fn background_result_is_forwarded_back_to_ui_handler() {
    // Arrange
    let (tx, rx) = std::sync::mpsc::channel();

    // Act
    run_background(
        move || Ok::<_, anyhow::Error>(41_u32),
        move |result| {
            tx.send(result.map(|value| value + 1)).unwrap();
        },
    );

    // Assert
    let received = rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
    assert_eq!(received.unwrap(), 42);
}
```

### Integration-style behavior checklist

- does the targeted callback fire once
- does the UI state move through loading/success/error cleanly
- does selection state stay consistent after reload or mutation
- does a failure leave the previous stable state intact

## Implementation Templates

### Request-state enum template

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestState<T, E> {
    Idle,
    Loading,
    Success(T),
    Error(E),
}
```

### Small error type template

```rust
#[derive(Debug, thiserror::Error)]
pub enum UiLoadError {
    #[error("daemon request failed: {0}")]
    Request(String),
}
```

### Main-thread handoff sketch

```rust
pub fn run_background<T, E, Work, Ui>(work: Work, ui: Ui)
where
    T: Send + 'static,
    E: Send + 'static,
    Work: FnOnce() -> Result<T, E> + Send + 'static,
    Ui: FnOnce(Result<T, E>) + 'static,
{
    let context = glib::MainContext::default();
    std::thread::spawn(move || {
        let result = work();
        context.invoke(move || ui(result));
    });
}
```

Use this only as a sketch. Adapt it to the actual slice.

## Review Checklist

Run this after each task and again after each slice:

- Is the behavior covered by at least one focused test?
- Did the new test fail before implementation?
- Is any new `clone()` necessary?
- Did I add `unwrap()` or `expect()` to non-test production code?
- Did I log an error instead of surfacing it to state where the slice requires user-visible feedback?
- Did I leave TODOs without documenting them in the plan or spec?
- Did I accidentally edit files owned by another slice?

## Full Slice Gate

At the end of a slice:

1. run `cargo fmt --check`
2. run the targeted slice tests
3. run `cargo test`
4. run `cargo check`
5. do a manual smoke test if the UI can be launched
6. review the diff
7. only then commit
