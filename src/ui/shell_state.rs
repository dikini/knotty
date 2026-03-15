//! Pure shell routing state for the GTK application shell.

use crate::ui::tool_rail::ToolMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMode {
    Notes,
    Search,
    Graph,
    Settings,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    Welcome,
    Note,
    Search,
    Graph,
    Settings,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorMode {
    Hidden,
    Details,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellState {
    tool_mode: ToolMode,
    has_selected_note: bool,
}

impl ShellState {
    pub fn select_tool(&mut self, mode: ToolMode) {
        self.tool_mode = mode;
    }

    pub fn set_note_selected(&mut self, selected: bool) {
        self.has_selected_note = selected;
    }

    pub fn tool_mode(&self) -> ToolMode {
        self.tool_mode
    }

    pub fn context_mode(&self) -> ContextMode {
        match self.tool_mode {
            ToolMode::Notes => ContextMode::Notes,
            ToolMode::Search => ContextMode::Search,
            ToolMode::Graph => ContextMode::Graph,
            ToolMode::Settings => ContextMode::Settings,
        }
    }

    pub fn content_mode(&self) -> ContentMode {
        match self.tool_mode {
            ToolMode::Notes => {
                if self.has_selected_note {
                    ContentMode::Note
                } else {
                    ContentMode::Welcome
                }
            }
            ToolMode::Search => ContentMode::Search,
            ToolMode::Graph => ContentMode::Graph,
            ToolMode::Settings => ContentMode::Settings,
        }
    }

    pub fn inspector_mode(&self) -> InspectorMode {
        match self.tool_mode {
            ToolMode::Notes | ToolMode::Graph => InspectorMode::Details,
            ToolMode::Search => InspectorMode::Hidden,
            ToolMode::Settings => InspectorMode::Settings,
        }
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            tool_mode: ToolMode::Notes,
            has_selected_note: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_notes_mode_uses_welcome_until_note_selected() {
        let mut shell = ShellState::default();

        assert_eq!(shell.tool_mode(), ToolMode::Notes);
        assert_eq!(shell.context_mode(), ContextMode::Notes);
        assert_eq!(shell.content_mode(), ContentMode::Welcome);
        assert_eq!(shell.inspector_mode(), InspectorMode::Details);

        shell.set_note_selected(true);

        assert_eq!(shell.content_mode(), ContentMode::Note);
    }

    #[test]
    fn selecting_graph_tool_switches_context_panel_to_graph() {
        let mut shell = ShellState::default();

        shell.select_tool(ToolMode::Graph);

        assert_eq!(shell.tool_mode(), ToolMode::Graph);
        assert_eq!(shell.context_mode(), ContextMode::Graph);
        assert_eq!(shell.content_mode(), ContentMode::Graph);
        assert_eq!(shell.inspector_mode(), InspectorMode::Details);
    }

    #[test]
    fn selecting_search_tool_hides_inspector_and_shows_search_content() {
        let mut shell = ShellState::default();
        shell.set_note_selected(true);

        shell.select_tool(ToolMode::Search);

        assert_eq!(shell.context_mode(), ContextMode::Search);
        assert_eq!(shell.content_mode(), ContentMode::Search);
        assert_eq!(shell.inspector_mode(), InspectorMode::Hidden);
    }

    #[test]
    fn selecting_settings_tool_routes_all_surfaces_to_settings() {
        let mut shell = ShellState::default();

        shell.select_tool(ToolMode::Settings);

        assert_eq!(shell.tool_mode(), ToolMode::Settings);
        assert_eq!(shell.context_mode(), ContextMode::Settings);
        assert_eq!(shell.content_mode(), ContentMode::Settings);
        assert_eq!(shell.inspector_mode(), InspectorMode::Settings);
    }
}
