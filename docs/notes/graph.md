# Graph Notes

- The current graph surface auto-fits the rendered scene into the available drawing area but does not yet provide pan or zoom controls. If later slices need larger-graph ergonomics, add those as explicit follow-up work rather than expanding the current graph contract silently.
- Focused-neighborhood rendering currently reuses vault-graph positions when available and falls back to a generated circular layout otherwise. If knotd later returns positioned neighborhood nodes, prefer consuming those directly and remove the GTK fallback path.
