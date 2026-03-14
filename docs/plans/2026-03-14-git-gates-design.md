# Git Gates Design

## Metadata

- Created: `2026-03-14`
- Scope: `tooling`
- Status: `approved`

## Goal

Add repository-local git gate scripts and hooks that:

- catch regressions caused by staged work before commit
- run the broadest available Rust test surface
- run property and fuzz checks when harnesses exist
- fail staged code changes that do not include matching staged docs changes in `docs/` or `README.md`
- provide a stronger full-check gate for pre-push or explicit verification

## Context

`knot-gtk` currently has no local gate script suite, no hook installer, and no clear repository policy for staged documentation evidence. Neighboring repositories `../sharo` and `../rarag` use a script-first approach:

- `scripts/check-*.sh` for focused checks
- one composite fast gate
- one hook installer
- `.githooks/` for repo-local hook wiring
- marker files under `.git/` to record successful gate runs

This design follows that style closely, but adapts the checks to a smaller Rust-only GTK project.

## Constraints

- gate scripts must work from the repository root regardless of the current working directory
- they must use plain `bash` with `set -euo pipefail`
- they must degrade cleanly when optional tools like `cargo-nextest` are not installed
- they must detect whether fuzz/property harnesses exist before trying to run them
- they must operate on staged changes for documentation evidence checks
- docs-only commits must remain valid

## Proposed Structure

### Files

- `scripts/check-doc-work-evidence.sh`
- `scripts/check-rust-format.sh`
- `scripts/check-rust-tests.sh`
- `scripts/check-property-tests.sh`
- `scripts/check-fuzz.sh`
- `scripts/check-pre-commit-gate.sh`
- `scripts/check-full-gate.sh`
- `scripts/install-hooks.sh`
- `.githooks/pre-commit`
- `.githooks/pre-push`
- `justfile`
- `README.md`

### Hook Model

- `.githooks/pre-commit` runs `scripts/check-pre-commit-gate.sh`
- `.githooks/pre-push` runs `scripts/check-full-gate.sh`
- `scripts/install-hooks.sh` sets `core.hooksPath=.githooks` and fixes executable bits

## Gate Behavior

### Pre-commit Gate

Purpose: catch errors caused by the staged work and enforce documentation evidence.

Checks:

1. documentation evidence check against staged changes
2. Rust formatting check
3. Rust compile check
4. unit and integration tests
5. property tests if present
6. fuzz smoke or fuzz harness checks if present
7. success marker write to `.git/.pre-commit-gate.ok`

### Full Gate

Purpose: run the strongest local verification available before push or manual release-quality validation.

Checks:

1. everything in pre-commit
2. full workspace test run again in `--all` mode if the pre-commit gate uses changed/worktree-specific behavior later
3. full property-test run if harnesses exist
4. full fuzz check if harnesses exist
5. success marker write to `.git/.full-gate.ok`

## Documentation Evidence Policy

### Required rule

If staged changes include any of:

- `src/**`
- `Cargo.toml`
- `Cargo.lock`
- `build.rs`
- `justfile`
- `.githooks/**`
- `scripts/**`

then staged changes must also include at least one of:

- `docs/**`
- `README.md`

### Allowed cases

- docs-only commit: allowed
- README-only docs update: allowed
- code plus docs: allowed

### Failure behavior

The check should fail with a message that lists:

- the staged code files that triggered the policy
- the required doc paths
- the exact remediation

## Property and Fuzz Detection

### Property tests

Detect by scanning for:

- `proptest` usage in Rust files
- `quickcheck` usage in Rust files
- dedicated property test files or modules if named clearly

If found:

- run the repository Rust test command and print that property tests were detected inside the normal test suite
- if a dedicated property command is added later, the script should have one clear place to plug it in

If not found:

- print `property-tests: no property test harness detected, skipping`

### Fuzzing

Detect by scanning for:

- `fuzz/`
- `fuzz_targets/`
- `cargo-fuzz` config
- `libfuzzer_sys`
- `honggfuzz`
- `bolero`

If found:

- run a smoke-level command appropriate to the detected harness
- fail if a harness exists but cannot be executed

If not found:

- print `fuzz: no fuzz harness detected, skipping`

## Rust Verification Policy

- prefer `cargo nextest run` if available
- otherwise use `cargo test`
- use `cargo check` for build verification
- use `cargo fmt --check` for formatting verification
- do not add clippy unless explicitly requested; this repo currently has many warnings and that would change the quality bar significantly

## Error Handling

- unknown script arguments should print usage and exit `2`
- missing optional harnesses should skip with a clear message
- missing required commands should fail with a clear installation hint
- each script should print the exact command it is about to run

## Testing Strategy

- add shell tests for the gate scripts themselves where practical
- unit-test staging logic with isolated temporary repos if needed later
- validate the documentation-evidence check against staged-only changes, not working tree changes

## Recommendation

Implement the strict staged-pairing policy now. It is simple, enforceable, and aligned with your stated requirement to fail code changes that are not documented.
