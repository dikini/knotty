# AGENTS

This file defines project-level execution policy for coding agents and contributors.

## Project Language Constraint

- Primary implementation language: `rust`
- Current project baseline: `edition 2021`, `rust-version 1.70`
- New runtime or core logic should use Rust unless explicitly waived in a spec or plan.

## Changelog and Commits

- Use Common Changelog format: <https://common-changelog.org/>.
- Task-completion work MUST update `CHANGELOG.md`.
- Any staged change in code, tooling, hooks, docs policy, or project workflow MUST stage a matching `CHANGELOG.md` update.
- Use Conventional Commits when possible for commit messages.

## Documentation Workflow

- `docs/specs/` stores canonical behavior and invariant specs.
- `docs/plans/` stores implementation and alignment plans.
- `docs/reference/` stores frozen contract and behavior references needed to work locally.

Default flow for non-trivial work:

1. confirm or update the relevant spec
2. create or update a plan
3. execute against that plan
4. review work using rust-skills and review skills 
4. update `CHANGELOG.md`
5. run the verification gates

## Policy Enforcement

- Install hooks once per clone:
  - `scripts/install-hooks.sh`
- Local `pre-commit` enforces:
  - staged docs evidence
  - staged `CHANGELOG.md` updates for staged implementation/tooling/docs-policy changes
  - `CHANGELOG.md` structure policy
  - Rust formatting
  - `cargo check`
  - Rust tests
  - property-test detection
  - fuzz checks when a runnable harness exists
- Local `pre-push` runs the full local gate.

## Testing and Verification

- Every behavior or workflow change must include verification evidence.
- If verification cannot run, document why and what residual risk remains.
