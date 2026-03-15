# GTK Parity Notes

This directory stores per-subsystem notes about future opportunities discovered during implementation or review.

## Purpose

- preserve non-blocking follow-up ideas without inflating the active slice scope
- keep review findings that are worth revisiting discoverable
- provide a lightweight handoff trail for future cleanup or refinement work

## Rules

- keep one note file per subsystem or slice area when practical
- record only items that are intentionally deferred or optional for the current slice
- prefer concise bullets with enough context to act on the note later
- if a note becomes committed work, move it into the relevant spec or plan and remove or update the note

## Current Files

- `shell.md`: shell and search follow-up opportunities
- `explorer.md`: explorer tree and mutation follow-up opportunities
- `editor.md`: editor follow-up opportunities and possible structured-edit refinements
