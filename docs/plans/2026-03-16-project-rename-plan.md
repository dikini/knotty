# Project Rename Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** rename the application, package, and crate identity from `knot-gtk`/`knot_gtk` to `knotty` across code, tests, and repo documentation.

**Architecture:** keep the code structure unchanged and perform a coordinated metadata-and-reference rename. Protect the externally visible name with focused CLI tests first, then update Cargo metadata, Rust imports, and docs in one slice.

**Tech Stack:** Rust, Cargo, gtk4/libadwaita, ripgrep, cargo test, cargo check, cargo clippy

---

## Delivery Notes

- Treat this as a behavior-preserving rename, not a feature rewrite.
- Use TDD for the public CLI name assertions before changing implementation strings.
- Update `CHANGELOG.md` in the same slice.
- Prefer broad textual replacement only after checking each hit still means the project identity, not a historical slice label that should stay GTK-specific.

### Task 1: Lock the public name with tests

**Files:**
- Modify: `/home/dikini/Projects/knot-gtk/src/cli.rs`

**Step 1: Write the failing test**

- Add focused tests for the help/version branding helpers so the expected project name is `knotty`.

**Step 2: Run test to verify it fails**

Run: `cargo test cli::tests::help_text_uses_knotty_branding -- --exact`

Expected: FAIL because the current help text still says `knot-gtk`.

**Step 3: Write minimal implementation**

- Extract string-building helpers if needed so tests can assert help/version text without spawning the process.
- Update the public CLI strings to `knotty`.

**Step 4: Run test to verify it passes**

Run: `cargo test cli::tests::help_text_uses_knotty_branding -- --exact`

Expected: PASS

### Task 2: Rename Cargo package and Rust crate references

**Files:**
- Modify: `/home/dikini/Projects/knot-gtk/Cargo.toml`
- Modify: `/home/dikini/Projects/knot-gtk/Cargo.lock`
- Modify: `/home/dikini/Projects/knot-gtk/src/main.rs`
- Modify: `/home/dikini/Projects/knot-gtk/tests/automation_protocol.rs`
- Modify: `/home/dikini/Projects/knot-gtk/fuzz/Cargo.toml`
- Modify: `/home/dikini/Projects/knot-gtk/fuzz/fuzz_targets/automation_protocol_jsonrpc.rs`

**Steps**

1. Change the package name to `knotty`.
2. Update direct crate imports from `knot_gtk` to `knotty`.
3. Update fuzz dependency references so the fuzz crate still depends on the renamed main package.
4. Regenerate lockfile metadata as needed with Cargo-aware commands.

### Task 3: Update runtime and user-facing references

**Files:**
- Modify: `/home/dikini/Projects/knot-gtk/src/cli.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/main.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/runtime_contract.rs`

**Steps**

1. Replace remaining public runtime strings that still mention `knot-gtk`.
2. Update log filter/thread naming to use the renamed crate target.
3. Re-run focused tests after each cluster if needed to catch stale references.

### Task 4: Update docs and changelog

**Files:**
- Modify: `/home/dikini/Projects/knot-gtk/README.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/README.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/reference/automation-behavior.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/reference/knotd-protocol.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/testing/gtk-parity-smoke.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/audit/gtk-automation-protocol-001.md`
- Modify: `/home/dikini/Projects/knot-gtk/CHANGELOG.md`

**Steps**

1. Update current user-facing documentation and command examples to use `knotty`.
2. Leave historical GTK slice names intact where they are part of document titles or component identifiers.
3. Add a concise changelog entry for the official rename.

### Task 5: Verify the slice

**Steps**

1. Run the focused CLI tests.
2. Run `cargo test`.
3. Run `cargo check`.
4. Run `cargo clippy --all-targets --all-features -- -D warnings` if the workspace supports it.
5. Review the diff for stale `knot-gtk` or `knot_gtk` references that should now be `knotty`.
