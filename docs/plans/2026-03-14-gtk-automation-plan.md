# GTK UI Automation and Parity Harnesses Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** add a gated, daemon-consumable semantic automation contract to `knot-gtk`, plus mocked transport harnesses, parity tests, and review artifacts that use the same automation contract.

**Architecture:** project existing shell/editor/graph/settings state into a serializable automation snapshot, dispatch typed semantic UI actions through one controller on the GTK main thread, expose discovery metadata suitable for later `knotd` consumption, and build parity tests/docs on that same layer rather than scraping widget internals.

**Tech Stack:** Rust, gtk4, libadwaita, serde, cargo test, cargo clippy, parity review docs

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/automation-behavior.md`, `docs/reference/shell-behavior.md`, `docs/reference/editor-behavior.md`, `docs/reference/settings-behavior.md`, and `docs/reference/graph-behavior.md`
- Keep the automation surface semantic, not widget-addressed.
- Reuse existing routing and guard paths instead of inventing automation-only state transitions.
- Do not add text entry or generic click primitives in this slice.

## Delivery Notes

- The protocol must be explicit enough to drive later `knotd` implementation without guessing argument/result shapes.
- Live `knotd` IPC/RPC wiring is out of scope for this slice; use mocked daemon-style callers instead.
- Automation must stay disabled unless both local config opt-in and runtime token/flag are present.
- Tests should assert semantic state and action results, not GTK child ordering or labels that are only presentation.

## Rust Guidance For This Slice

- Keep snapshot, action, description, and result types serializable and colocated.
- Use typed enums and structs for action modeling; do not use stringly internals for GTK dispatch.
- Keep controller failures semantic with stable result codes.
- Avoid holding `RefCell` borrows or mutable widget state across callbacks that may synchronously re-enter routing.

## Protocol Notes

### Required discovery call

- `describe_ui_automation`
- reports protocol version, gating state, action catalog, and stable result codes

### Required state call

- `get_ui_snapshot`
- returns typed semantic fields plus a property map for flexible daemon/LLM callers

### Required action call

- `dispatch_ui_action`
- returns a structured result plus optional updated snapshot

### Stable result codes

- `ok`
- `automation_disabled`
- `startup_blocked`
- `dirty_guard_blocked`
- `unsupported_context`
- `not_found`
- `invalid_arguments`

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTA-001 | Tighten the automation spec, reference, and protocol docs | - |
| GTA-002 | Add local config and CLI/runtime gate types | GTA-001 |
| GTA-003 | Define serializable snapshot, discovery, action, and result types | GTA-001 |
| GTA-004 | Implement window-owned automation projection and controller | GTA-002, GTA-003 |
| GTA-005 | Expose stable automation identifiers and visible active indicator | GTA-004 |
| GTA-006 | Add parity-focused automation tests for completed slices | GTA-004, GTA-005 |
| GTA-007 | Write manual review checklists and knotd handoff notes | GTA-006 |
| GTA-008 | Full verification and review fixes | GTA-006, GTA-007 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTA-001A | GTA-001 | Update spec for gated daemon-consumable automation | `docs/specs/component/gtk-automation-008.md` | code files |
| GTA-001B | GTA-001 | Expand automation reference with discovery/snapshot/action protocol | `docs/reference/automation-behavior.md` | code files |
| GTA-001C | GTA-001 | Add design note and align changelog | `docs/plans/2026-03-15-gtk-automation-design.md`, `CHANGELOG.md` | code files |
| GTA-002A | GTA-002 | Add failing config-path test for automation opt-in | `src/config/knotty_config.rs` | GTK UI files |
| GTA-002B | GTA-002 | Add failing CLI parse test for runtime token/flag | `src/cli.rs` | window/controller files |
| GTA-002C | GTA-002 | Implement config and CLI gate types | `src/config/knotty_config.rs`, `src/cli.rs`, `src/main.rs` | UI controller files |
| GTA-003A | GTA-003 | Add failing snapshot/discovery type tests | `src/ui/automation_state.rs` | window logic |
| GTA-003B | GTA-003 | Add failing action/result schema tests | `src/ui/automation_state.rs` | window logic |
| GTA-003C | GTA-003 | Implement serializable protocol types | `src/ui/automation_state.rs`, `src/ui/mod.rs` | shell/editor internals |
| GTA-004A | GTA-004 | Add failing snapshot projection test for shell/startup/editor state | `src/ui/window.rs`, `src/ui/automation_state.rs` | graph renderer |
| GTA-004B | GTA-004 | Add failing controller test for semantic actions and blocked result codes | `src/ui/window.rs`, `src/ui/automation_state.rs` | graph renderer |
| GTA-004C | GTA-004 | Implement window-owned automation projection and controller | `src/ui/window.rs` | unrelated widgets |
| GTA-005A | GTA-005 | Add failing stable-ID test for major surfaces | `src/ui/window.rs`, `src/ui/context_panel.rs`, `src/ui/settings_view.rs`, `src/ui/graph_view.rs`, `src/ui/search_view.rs` | protocol types |
| GTA-005B | GTA-005 | Add failing active-indicator test | `src/ui/window.rs` | editor internals |
| GTA-005C | GTA-005 | Implement IDs and automation-active indicator | touched UI files only | client code |
| GTA-006A | GTA-006 | Add parity test for startup/tool switching via automation | `src/ui/window.rs` tests or `tests/` if added | editor internals |
| GTA-006B | GTA-006 | Add parity test for note selection/editor mode/dirty guard | `src/ui/window.rs` tests or `tests/` if added | graph/settings files |
| GTA-006C | GTA-006 | Add parity test for settings/graph actions | `src/ui/window.rs` tests or `tests/` if added | explorer internals |
| GTA-007A | GTA-007 | Draft GTK parity smoke checklist using automation terms | `docs/testing/gtk-parity-smoke.md` | code files |
| GTA-007B | GTA-007 | Draft knotd integration handoff note with protocol examples | `docs/audit/gtk-automation-protocol-001.md` | code files |
| GTA-008A | GTA-008 | Run full slice verification | repo-wide | - |
| GTA-008B | GTA-008 | Fix slice-only regressions and review findings | touched files only | unrelated modules |

### Task GTA-001: Tighten the automation spec, reference, and protocol docs

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/docs/specs/component/gtk-automation-008.md`
- Modify: `/home/dikini/Projects/knot-gtk/docs/reference/automation-behavior.md`
- Create: `/home/dikini/Projects/knot-gtk/docs/plans/2026-03-15-gtk-automation-design.md`
- Modify: `/home/dikini/Projects/knot-gtk/CHANGELOG.md`

**Steps**
1. Write the approved gated daemon-consumable automation design into the spec.
2. Add protocol details for discovery, snapshot shape, actions, and result codes in the reference.
3. Save the design note so implementation decisions stay frozen.
4. Re-read the protocol docs and remove any vague or inferred-only behavior.

### Task GTA-002: Add local config and CLI/runtime gate types

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/config/knotty_config.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/cli.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/main.rs`

**Steps**
1. Write a failing config test for `automation.enabled`.
2. Run it to verify it fails.
3. Write a failing CLI parse test for the runtime automation flag/token.
4. Run it to verify it fails.
5. Implement the minimal config and CLI types.
6. Run targeted tests until green.
7. Review startup flow to ensure automation stays off unless both gates are satisfied.

### Task GTA-003: Define serializable snapshot, discovery, action, and result types

**Files**
- Create: `/home/dikini/Projects/knot-gtk/src/ui/automation_state.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/mod.rs`

**Steps**
1. Write failing tests for snapshot serialization, description serialization, and result-code stability.
2. Run them to verify they fail.
3. Implement the smallest serializable protocol types.
4. Run targeted tests until green.
5. Review names and field semantics so they are daemon-friendly, not GTK-widget-specific.

### Task GTA-004: Implement window-owned automation projection and controller

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`
- Modify if needed: `/home/dikini/Projects/knot-gtk/src/ui/shell_state.rs`
- Modify if needed: `/home/dikini/Projects/knot-gtk/src/ui/editor.rs`

**Steps**
1. Write failing tests for snapshot projection from current shell/editor/settings/graph state.
2. Run them to verify they fail.
3. Write failing tests for action dispatch success and blocked cases.
4. Run them to verify they fail.
5. Implement a small automation controller on `KnotWindow` that:
   - returns discovery metadata
   - returns current snapshot
   - dispatches semantic actions through existing routing helpers
6. Run targeted tests until green.
7. Review that action handling reuses existing guards for startup and dirty-note state.

### Task GTA-005: Expose stable automation identifiers and visible active indicator

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`
- Modify as needed: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`
- Modify as needed: `/home/dikini/Projects/knot-gtk/src/ui/settings_view.rs`
- Modify as needed: `/home/dikini/Projects/knot-gtk/src/ui/graph_view.rs`
- Modify as needed: `/home/dikini/Projects/knot-gtk/src/ui/search_view.rs`

**Steps**
1. Write failing tests for stable IDs on major views and rails.
2. Run them to verify they fail.
3. Write a failing test for the automation-active indicator.
4. Run it to verify it fails.
5. Add stable identifiers using one consistent naming scheme.
6. Add the smallest visible automation-active indicator.
7. Run targeted tests until green.
8. Review naming consistency across notes/search/graph/settings surfaces.

### Task GTA-006: Add parity-focused automation tests for completed slices

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs` test module, or create focused integration tests under `/home/dikini/Projects/knot-gtk/tests/` if a new integration harness is justified

**Steps**
1. Add a failing test for startup gating and tool switching through automation.
2. Run it to verify it fails.
3. Add a failing test for note selection, editor mode changes, and dirty-guard blocking.
4. Run it to verify it fails.
5. Add a failing test for settings-section and graph-scope/depth actions.
6. Run it to verify it fails.
7. Implement only the missing glue needed for those tests.
8. Run targeted tests until green.
9. Review assertions so they target semantic snapshot state and result codes, not widget structure.

### Task GTA-007: Write manual review checklists and knotd handoff notes

**Files**
- Create: `/home/dikini/Projects/knot-gtk/docs/testing/gtk-parity-smoke.md`
- Create: `/home/dikini/Projects/knot-gtk/docs/audit/gtk-automation-protocol-001.md`

**Steps**
1. Draft a manual smoke checklist for enabling automation and verifying core flows.
2. Draft a knotd handoff note with concrete discovery/snapshot/action examples.
3. Re-read both docs from a zero-context perspective and remove any guesswork.

### Task GTA-008: Full verification and review fixes

**Steps**
1. Run `cargo fmt --check`.
2. Run targeted automation tests.
3. Run `cargo check`.
4. Run `cargo clippy --workspace --all-targets --all-features`.
5. Run `cargo test`.
6. Run `cargo nextest run --workspace`.
7. Review the implementation specifically for:
   - spec/protocol drift
   - stringly action handling
   - duplicate routing code
   - hidden widget-coupling in tests
8. Fix findings before committing.

## Slice Verification

```bash
cargo fmt --check
cargo check
cargo clippy --workspace --all-targets --all-features
cargo test
cargo nextest run --workspace
```

## Review Checklist

- Discovery metadata is explicit and versioned.
- Snapshot fields reflect actual shell/editor/graph/settings state.
- Automation remains unavailable unless both config and runtime token gates are satisfied.
- Action results use stable semantic result codes.
- Tests assert semantic state and result codes rather than widget internals.
- Manual docs are sufficient for both human parity review and later `knotd` implementation.

## Commit Gate

Commit only when all verification commands are green and the protocol docs match the implementation.
