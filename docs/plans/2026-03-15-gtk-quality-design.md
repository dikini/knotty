# GTK Quality Hardening Design

## Metadata

- Created: `2026-03-15`
- Scope: `tooling`, `maintenance`
- Status: `approved`
- Spec: `docs/specs/component/gtk-quality-010.md`

## Goal

Define a bounded cleanup slice that improves the signal quality of local verification and reduces maintenance drag without turning into an unbounded refactor.

## Design

### Primary outcomes

1. Lower warning noise
- Reduce current clippy noise in active GTK code so real regressions are easier to see.
- Favor contained fixes over broad stylistic churn.

2. Better test confidence
- Strengthen weak tests around areas that already shipped.
- Remove duplication or flakiness where tests are doing unnecessary work.

3. Cleaner active codepaths
- Remove stale helpers, unused fields, and outdated glue left behind by completed slices.
- Keep future-slice placeholders only when they are still intentional and documented.

### Cleanup priorities

1. Low-risk clippy fixes
- redundant closures
- manual string prefix stripping
- needless borrows
- items-after-test-module issues

2. Medium-risk structural simplifications
- callback type aliases for repeated `Rc<RefCell<Option<Box<...>>>>` patterns
- argument-bundle structs for oversized helper signatures
- local helper extraction where repeated widget/reset logic drifts

3. Verification tightening
- fill obvious test gaps discovered during cleanup
- add or improve regression tests where simplification changes logic

### Explicit constraint

This slice should not try to solve every warning class in one pass. It should reduce the warning surface materially, document what remains, and leave the repo in a better place to tighten clippy later.

## Completion signal

- clippy output is meaningfully smaller
- touched subsystems are simpler to review
- remaining debt is explicit rather than ambient
