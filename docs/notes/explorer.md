# Explorer Notes

- `NoteSwitchDecision::Deny` and `SaveThenAllow` remain part of the public explorer/editor switch-guard contract even though the current production flow only constructs `Allow` and `Prompt`. Keep them until the editor-driven save-then-switch path is finalized or the contract is simplified explicitly.
- Add a focused GTK harness for explorer mutation callbacks so create/rename/delete and expansion-replay behavior are covered above the current helper-level tests.
- Revisit the custom prompt windows used for create/rename/delete once the project adopts a preferred non-deprecated dialog pattern for GTK 4.10+.
- Consider extracting the `TreeListModel` row binding and expansion-observer glue into smaller helpers if the graph/settings slices need similar hierarchical GTK list behavior later.
