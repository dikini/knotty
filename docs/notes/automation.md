# Automation Notes

## Deferred Follow-Ups

- When `knotd` wiring lands, route daemon tool calls through `ui::automation_protocol::handle_ui_automation_tool_call` instead of duplicating GTK-side argument parsing.
- Text entry and generic button/click actions are intentionally out of scope for this slice.
- If `knotd` needs structured "no active window" reporting, extend the protocol with an explicit cross-process result code instead of relying on controller-local `None`.
