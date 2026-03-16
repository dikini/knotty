# GTK Deprecated API Modernization Design

## Goal
Restore GTK/libadwaita deprecation warnings as a reliable maintenance signal by removing deprecated API usage from repository code and documenting a project rule against introducing new deprecated APIs.

## Problem
`knotty` currently emits GTK deprecation warnings during normal verification, with explorer tree widgets as the largest source. That creates two problems:

1. real warnings are buried under known noise
2. new work can keep landing on deprecated APIs because the warning stream is already normalized

The project already states a preference against introducing deprecated APIs, but the repository still contains active deprecated codepaths. That means the policy is directionally right but not yet enforced by the codebase state.

## Recommended Approach
Use a dedicated modernization slice with one umbrella spec and one implementation plan, but execute the work in subsystem-shaped tasks.

This keeps the acceptance target simple:
- no GTK/libadwaita deprecation warnings from repository code

At the same time, it avoids turning the work into one undifferentiated refactor. Explorer migration should be treated as the main structural task, while smaller deprecated dialog or widget replacements can be handled in contained follow-up tasks under the same slice.

## Scope Strategy

### Immediate target
- remove the current warning-producing GTK/libadwaita APIs from repository code

### Primary structural migration
- replace the explorer `TreeView`/`TreeStore` implementation with a non-deprecated GTK list/tree pattern

### Contained call-site migrations
- replace any remaining deprecated dialog, picker, or widget APIs that are still in active use

### Policy and verification
- keep the project rule explicit in docs
- make `cargo check` deprecation output the operational signal for this slice

## Priority Guidance
This is a maintenance slice, not a user-facing parity slice, so it does not automatically outrank missing end-user functionality. But it should not be left vague either.

Recommended priority guidance:
- if warning noise is slowing down active GTK feature work, schedule this slice before additional large UI slices
- otherwise, schedule it immediately after the next highest-value behavior gap

That keeps priorities explicit without forcing an all-maintenance-first roadmap.

## Risks
- explorer migration can expand if selection and expansion behavior are not preserved with strong tests
- broad modernization can drift into unrelated cleanup unless the plan stays anchored to actual deprecation warnings

## Success Signal
- `cargo check` no longer reports GTK/libadwaita deprecations from repository code
- feature behavior remains intact after the explorer/widget migrations
