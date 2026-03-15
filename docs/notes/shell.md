# Shell Notes

Future opportunities for the GTK shell and search subsystem that are intentionally out of the current slice.

## Search

- Simplify `SearchState` so it stores only state that the UI cannot already derive. `Loading { query }` is currently not rendered, and `Results { query, count }` duplicates data already available from the results list.
- Consider replacing full result-list rebuilds on every state transition with a smaller diff or model-driven update path if the result cap grows beyond the current small list.

## Shell

- Keep watching for opportunities to reduce callback capture size in `src/ui/window.rs` if more shell actions are added. The latest cleanup centralized startup refresh handles, but note-load routing still carries a fairly wide UI context through the shared selection handler.
