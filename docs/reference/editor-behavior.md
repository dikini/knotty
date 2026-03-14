# Editor Behavior Reference

## Purpose

Define the expected mode, synchronization, dirty-state, and save behavior for the GTK editor.

## Editor Modes

- `meta`
- `source`
- `edit`
- `view`

These mode names should be used exactly in code, tests, and payload handling.

## Mode Routing Rules

- if `available_modes` exists, use it directly
- otherwise use the note-type defaults from `note-contract.md`
- default selected mode should be the first enabled mode in this priority order:
  1. `edit`
  2. `view`
  3. `source`
  4. `meta`

## Dirty-State Rules

- loading a note clears dirty state only after the new note is fully applied
- editing content marks the current note dirty
- successful save clears dirty state
- failed save keeps dirty state
- switching away from a dirty note requires an explicit decision:
  - deny switch
  - discard changes and switch
  - save and switch

## Save Rules

- there should be one authoritative save entry point
- save requests persist the note path and current authoritative text content
- unsaved changes must not be lost on failed save

## Synchronization Rules

- source markdown is the authoritative persisted representation
- view mode renders from source markdown
- edit mode either edits the source directly or round-trips through a controlled conversion layer
- mode switches must preserve content
- cursor or scroll restoration can be approximate, but it must be deterministic and tested

## Minimum Authoring Command Set

- headings
- unordered lists
- ordered lists
- block quotes
- fenced code blocks
- horizontal rules
- task list toggles
- links
- wikilinks

## Meta Mode Rules

- meta mode should allow inspection and editing of note metadata needed by parity flows
- start with a small explicit field set, such as title, tags, or frontmatter fields
- metadata edits must round-trip through save and reload

## Test Cases

- pdf note only enables `view`
- markdown note enables all modes by default
- failed save keeps dirty flag set
- blocked switch keeps current note selected
- mode switch preserves markdown text
- task toggle preserves markdown round-trip
