# Note Types Notes

- The current PDF surface is intentionally limited to a system-open fallback. If a later slice needs in-app reading, add a dedicated PDF renderer rather than growing the fallback card into a partial viewer.
- Image notes currently open the primary file externally and rely on `gtk::Picture` for inline rendering. If very large media files cause memory or scaling issues, move image loading onto a smaller adapter with size-aware downscaling.
- Embed support currently treats YouTube as the only specialized embed shape and falls back to a safe action card for everything else. Add richer embed renderers only when the daemon contract stabilizes around more specific kinds.
