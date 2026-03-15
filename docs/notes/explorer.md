# Explorer Notes

- Replace the temporary `TreeView`/`TreeStore` explorer with a non-deprecated GTK list/tree pattern once the editor and graph slices stop depending on the old selection callbacks.
- Add a focused GTK harness for explorer mutation callbacks so create/rename/delete and expansion-replay behavior are covered above the current helper-level tests.
- Revisit the custom prompt windows used for create/rename/delete once the project adopts a preferred non-deprecated dialog pattern for GTK 4.10+.
