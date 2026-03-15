# Automation Notes

## Deferred Follow-Ups

- `knotd` transport integration still needs to consume the in-process GTK automation API and enforce token validation at the daemon boundary.
- Text entry and generic button/click actions are intentionally out of scope for this slice.
- If `knotd` needs structured "no active window" reporting, extend the protocol with an explicit cross-process result code instead of relying on controller-local `None`.
