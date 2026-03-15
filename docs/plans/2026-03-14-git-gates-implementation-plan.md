# Git Gates Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** add local pre-commit and full-check gate scripts, hook wiring, and documentation-evidence enforcement for `knot-gtk`.

**Architecture:** follow the script-first gate style used in neighboring repositories: small focused `scripts/check-*.sh` helpers, one composite pre-commit gate, one composite full gate, and repo-local `.githooks/` entrypoints. Keep each script narrow and testable, and operate on staged files for documentation-evidence policy.

**Tech Stack:** bash, git, cargo, optional cargo-nextest, repo-local hooks

---

## How To Use This Plan

- Read the design first: `docs/plans/2026-03-14-git-gates-design.md`
- Keep scripts small and single-purpose.
- Match the local script style used by `../sharo` and `../rarag`.
- Do not add extra policy checks not covered by the design.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GGG-001 | Add documentation-evidence check | - |
| GGG-002 | Add Rust verification helpers | GGG-001 |
| GGG-003 | Add property and fuzz detection helpers | GGG-002 |
| GGG-004 | Add composite gate scripts and markers | GGG-003 |
| GGG-005 | Add hook installer and git hooks | GGG-004 |
| GGG-006 | Add justfile and README integration | GGG-005 |
| GGG-007 | Run full verification and fix script regressions | GGG-006 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GGG-001A | GGG-001 | Write failing docs-evidence script test or fixture check | `scripts/check-doc-work-evidence.sh` | Rust source files |
| GGG-001B | GGG-001 | Implement staged file classification | `scripts/check-doc-work-evidence.sh` | `.githooks/*` |
| GGG-001C | GGG-001 | Implement failure messaging | `scripts/check-doc-work-evidence.sh` | Rust source files |
| GGG-002A | GGG-002 | Add `check-rust-format.sh` | `scripts/check-rust-format.sh` | `.githooks/*` |
| GGG-002B | GGG-002 | Add `check-rust-tests.sh` with nextest fallback | `scripts/check-rust-tests.sh` | docs files |
| GGG-002C | GGG-002 | Add build verification inside helper or gate | `scripts/check-pre-commit-gate.sh` or helper | source files |
| GGG-002D | GGG-002 | Add `check-rust-clippy.sh` and wire it into the local gate | `scripts/check-rust-clippy.sh`, `scripts/check-pre-commit-gate.sh` | docs files |
| GGG-003A | GGG-003 | Add property harness detector | `scripts/check-property-tests.sh` | Rust files |
| GGG-003B | GGG-003 | Add fuzz harness detector | `scripts/check-fuzz.sh` | Rust files |
| GGG-003C | GGG-003 | Make optional checks skip cleanly when absent | `scripts/check-property-tests.sh`, `scripts/check-fuzz.sh` | docs files |
| GGG-004A | GGG-004 | Add pre-commit composite gate | `scripts/check-pre-commit-gate.sh` | `.githooks/*` |
| GGG-004B | GGG-004 | Add full composite gate | `scripts/check-full-gate.sh` | `.githooks/*` |
| GGG-004C | GGG-004 | Add marker file writer | composite gate scripts | source files |
| GGG-005A | GGG-005 | Add `.githooks/pre-commit` | `.githooks/pre-commit` | Rust files |
| GGG-005B | GGG-005 | Add `.githooks/pre-push` | `.githooks/pre-push` | Rust files |
| GGG-005C | GGG-005 | Add hook installer | `scripts/install-hooks.sh` | docs files |
| GGG-006A | GGG-006 | Add `justfile` targets | `justfile` | Rust source files |
| GGG-006B | GGG-006 | Document gate workflow in README | `README.md` | docs/specs content |
| GGG-007A | GGG-007 | Run repo gate scripts end-to-end | repo-wide | - |
| GGG-007B | GGG-007 | Fix pathing or staging-edge-case regressions | touched script files only | unrelated docs |

### Task GGG-001: Add documentation-evidence check

**Files:**
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-doc-work-evidence.sh`

**Step 1: Write the failing test or manual fixture**

Create a simple temporary-repo harness or manual reproduction note that stages:

- one file under `src/`
- no docs changes

Expected result: the script exits non-zero and prints the missing docs requirement.

**Step 2: Run it to verify it fails**

Run:

```bash
bash scripts/check-doc-work-evidence.sh
```

Expected: fail in a staged-code/no-docs scenario.

**Step 3: Write minimal implementation**

Implementation requirements:

- gather staged files with `git diff --cached --name-only --diff-filter=ACMR`
- classify code files vs docs files
- if any staged code file exists and no staged docs file exists, exit `1`

**Step 4: Run it again**

Expected:

- fail when staged code has no staged docs
- pass when staged docs are present
- pass for docs-only commits

**Step 5: Review**

Review the path patterns against the approved design only:

- `src/**`
- `Cargo.toml`
- `Cargo.lock`
- `build.rs`
- `justfile`
- `.githooks/**`
- `scripts/**`

### Task GGG-002: Add Rust verification helpers

**Files:**
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-rust-format.sh`
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-rust-clippy.sh`
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-rust-tests.sh`

**Step 1: Write the failing test or dry-run expectation**

Document expected commands and exit behavior:

- `check-rust-format.sh` runs `cargo fmt --check`
- `check-rust-clippy.sh` runs `cargo clippy --workspace --all-targets --all-features`
- `check-rust-tests.sh` prefers `cargo nextest run` and falls back to `cargo test`

**Step 2: Run each script before implementation**

Expected: file not found or non-executable failure.

**Step 3: Write minimal implementation**

`check-rust-format.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
echo "rust-format: running cargo fmt --check"
cargo fmt --check
```

`check-rust-clippy.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
echo "check-rust-clippy: running cargo clippy --workspace --all-targets --all-features"
cargo clippy --workspace --all-targets --all-features
```

`check-rust-tests.sh` should support:

- `--workspace`
- `--args <cargo-test-args...>`

and should use the `../sharo` style nextest fallback.

**Step 4: Run scripts**

Expected: formatting and tests run from repo root regardless of current directory.

**Step 5: Review**

- no unnecessary flags
- clear usage on bad arguments
- same shell style as neighboring repos

### Task GGG-003: Add property and fuzz detection helpers

**Files:**
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-property-tests.sh`
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-fuzz.sh`

**Step 1: Write the failing/manual detection cases**

Describe and test these cases:

- no property harness found: script prints skip message and exits `0`
- no fuzz harness found: script prints skip message and exits `0`
- harness found but required runner missing: script exits non-zero with installation hint

**Step 2: Run the scripts before implementation**

Expected: file not found failure.

**Step 3: Write minimal implementation**

Property detection:

- search the repo with `rg` for `proptest`, `quickcheck`, `bolero`
- if found, run `scripts/check-rust-tests.sh --workspace`
- otherwise print skip

Fuzz detection:

- detect `fuzz/`, `fuzz_targets/`, `libfuzzer_sys`, `cargo fuzz`, `honggfuzz`
- if none found, print skip
- if found and `cargo fuzz` or other runner is unavailable, fail with hint
- if found, run a smoke command only

**Step 4: Run scripts**

Expected: current repo likely skips both checks cleanly.

**Step 5: Review**

- skip messages are explicit
- harness detection is centralized
- no false claim that a skipped check was executed

### Task GGG-004: Add composite gate scripts and markers

**Files:**
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-pre-commit-gate.sh`
- Create: `/home/dikini/Projects/knot-gtk/scripts/check-full-gate.sh`

**Step 1: Write the failing/manual gate expectations**

Define expected order:

Pre-commit:

1. docs evidence
2. rust format
3. cargo check
4. cargo clippy
5. rust tests
6. property checks
7. fuzz checks
8. marker write

Full gate:

1. pre-commit gate
2. explicit full rust test pass
3. property checks
4. fuzz checks
5. marker write

**Step 2: Run before implementation**

Expected: file not found failure.

**Step 3: Write minimal implementation**

Use a helper function to write marker files under `.git/`.

Marker examples:

- `.git/.pre-commit-gate.ok`
- `.git/.full-gate.ok`

Include:

- UTC timestamp
- `git rev-parse HEAD`
- content hash of tracked and staged changes if practical

**Step 4: Run scripts**

Expected: current repo should pass or fail with a genuine repo issue, not script bugs.

**Step 5: Review**

- no duplicate logic that should live in a helper later
- clear command echoing
- strict failure propagation

### Task GGG-005: Add hook installer and git hooks

**Files:**
- Create: `/home/dikini/Projects/knot-gtk/scripts/install-hooks.sh`
- Create: `/home/dikini/Projects/knot-gtk/.githooks/pre-commit`
- Create: `/home/dikini/Projects/knot-gtk/.githooks/pre-push`

**Step 1: Write expected behavior**

- installer sets `core.hooksPath=.githooks`
- installer marks hook and script files executable
- pre-commit hook runs pre-commit gate
- pre-push hook runs full gate

**Step 2: Run before implementation**

Expected: file not found failure.

**Step 3: Write minimal implementation**

Use the same style as `../sharo/scripts/install-hooks.sh`.

Hook example:

```bash
#!/usr/bin/env bash
set -euo pipefail
exec scripts/check-pre-commit-gate.sh
```

**Step 4: Run installer**

Run:

```bash
bash scripts/install-hooks.sh
```

Expected: hooks path configured and executable bits fixed.

**Step 5: Review**

- installer is idempotent
- no absolute paths

### Task GGG-006: Add justfile and README integration

**Files:**
- Create or modify: `/home/dikini/Projects/knot-gtk/justfile`
- Modify: `/home/dikini/Projects/knot-gtk/README.md`

**Step 1: Write expected commands**

Add targets:

- `verify`
- `pre-commit-gate`
- `full-gate`
- `install-hooks`

**Step 2: Run before implementation**

Expected: target absent.

**Step 3: Write minimal implementation**

`justfile` should use:

```just
set shell := ["bash", "-euo", "pipefail", "-c"]
```

README should document:

- what each gate does
- the strict docs-evidence rule
- how to install hooks

**Step 4: Run targets**

Expected: targets call the scripts correctly.

**Step 5: Review**

- README matches actual script names
- no undocumented hook behavior

### Task GGG-007: Run full verification and fix script regressions

**Step 1: Run formatting**

```bash
cargo fmt --check
```

**Step 2: Run gate scripts**

```bash
bash scripts/check-pre-commit-gate.sh
bash scripts/check-full-gate.sh
```

**Step 3: Run Rust verification explicitly**

```bash
cargo check
cargo clippy --workspace --all-targets --all-features
cargo test
```

**Step 4: Fix regressions**

Fix only issues introduced by the new gate scripts, hook wiring, or documentation updates.

**Step 5: Review**

- staged-code-without-docs policy behaves exactly as requested
- property/fuzz scripts skip cleanly when no harness exists
- hook installer is safe and repeatable
- README and justfile are aligned with the scripts

## Commit Guidance

Commit only after all verification commands are green.
