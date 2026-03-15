use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::{KnotdClient, NoteData};
use crate::ui::async_bridge;
use crate::ui::context_panel::ContextPanel;
use crate::ui::editor::NoteEditor;
use crate::ui::inspector_rail::InspectorRail;
use crate::ui::request_state::RequestState;
use crate::ui::tool_rail::{ToolMode, ToolRail};

type NoteLoadState = RequestState<NoteData, String>;
type NoteLoadResult = Result<NoteData, String>;

pub struct KnotWindow {
    window: libadwaita::ApplicationWindow,
    client: Rc<KnotdClient>,
    tool_rail: ToolRail,
    context_panel: Rc<RefCell<ContextPanel>>,
    inspector_rail: InspectorRail,
    editor: Rc<NoteEditor>,
    content_stack: gtk::Stack,
    current_note: RefCell<Option<NoteData>>,
    note_load_state: Rc<RefCell<NoteLoadState>>,
    note_load_generation: Rc<RefCell<u64>>,
}

fn update_note_load_state(state: &Rc<RefCell<NoteLoadState>>, result: &NoteLoadResult) {
    *state.borrow_mut() = match result {
        Ok(note) => RequestState::success(note.clone()),
        Err(error) => RequestState::error(error.clone()),
    };
}

fn should_route_note_load_to_editor(current_surface: &str) -> bool {
    !matches!(current_surface, "graph" | "settings")
}

fn begin_note_load_with_dispatch<Dispatch, OnResult>(
    client: KnotdClient,
    path: String,
    state: Rc<RefCell<NoteLoadState>>,
    generation: u64,
    active_generation: Rc<RefCell<u64>>,
    dispatch: Dispatch,
    on_result: OnResult,
) where
    Dispatch: FnOnce(Box<dyn FnOnce() -> NoteLoadResult + Send>, Box<dyn FnOnce(NoteLoadResult)>),
    OnResult: FnOnce(NoteLoadResult) + 'static,
{
    *state.borrow_mut() = RequestState::loading();

    let state_for_ui = Rc::clone(&state);
    dispatch(
        Box::new(move || client.get_note(&path).map_err(|error| error.to_string())),
        Box::new(move |result| {
            if *active_generation.borrow() != generation {
                return;
            }
            update_note_load_state(&state_for_ui, &result);
            on_result(result);
        }),
    );
}

impl KnotWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        let client = KnotdClient::new();
        Self::with_client(app, client)
    }

    pub fn with_client(app: &libadwaita::Application, client: KnotdClient) -> Self {
        let client = Rc::new(client);

        // Create window
        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("Knot")
            .default_width(1400)
            .default_height(900)
            .build();

        // Check daemon connection and vault status
        let vault_status = match client.is_vault_open() {
            Ok(true) => match client.get_vault_info() {
                Ok(info) => format!("Connected to vault: {}", info.name),
                Err(_) => "Connected (no vault open)".to_string(),
            },
            Ok(false) => "No vault open".to_string(),
            Err(e) => {
                log::warn!("knotd not running: {}", e);
                format!("Not connected: {}", e)
            }
        };
        log::info!("Vault status: {}", vault_status);

        // Create toolbar view
        let toolbar_view = libadwaita::ToolbarView::new();

        // Header bar
        let header = libadwaita::HeaderBar::new();

        // Vault info label in header
        let vault_label = gtk::Label::builder()
            .label(&vault_status)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(40)
            .build();
        header.set_title_widget(Some(&vault_label));

        // Menu button
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();
        header.pack_end(&menu_btn);

        toolbar_view.add_top_bar(&header);

        // Main horizontal box: ToolRail | ContextPanel | Content | InspectorRail
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        // ToolRail (left)
        let tool_rail = ToolRail::new();
        main_box.append(tool_rail.widget());

        // ContextPanel (left-center)
        let context_panel = Rc::new(RefCell::new(ContextPanel::new(Rc::clone(&client))));
        main_box.append(context_panel.borrow().widget());

        // Content area (center)
        let content_stack = gtk::Stack::builder().vexpand(true).hexpand(true).build();

        // Empty state view (shown when no note selected)
        let empty_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(12)
            .build();

        let empty_icon = gtk::Image::builder()
            .icon_name("emblem-documents-symbolic")
            .pixel_size(64)
            .opacity(0.5)
            .build();

        let empty_label = gtk::Label::builder()
            .label("No note selected")
            .css_classes(vec!["title-3".to_string()])
            .build();

        let empty_hint = gtk::Label::builder()
            .label("Select a note from the sidebar to view or edit")
            .css_classes(vec!["dim-label".to_string()])
            .build();

        empty_view.append(&empty_icon);
        empty_view.append(&empty_label);
        empty_view.append(&empty_hint);
        content_stack.add_titled(&empty_view, Some("empty"), "Empty");

        // Editor view
        let editor = Rc::new(NoteEditor::new(Rc::clone(&client)));
        content_stack.add_titled(editor.widget(), Some("editor"), "Editor");

        // Graph view (placeholder)
        let graph_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let graph_label = gtk::Label::builder()
            .label("Graph view coming soon")
            .vexpand(true)
            .build();
        graph_view.append(&graph_label);
        content_stack.add_titled(&graph_view, Some("graph"), "Graph");

        // Settings view (placeholder)
        let settings_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let settings_label = gtk::Label::builder()
            .label("Settings view coming soon")
            .vexpand(true)
            .build();
        settings_view.append(&settings_label);
        content_stack.add_titled(&settings_view, Some("settings"), "Settings");

        // Start with empty view
        content_stack.set_visible_child_name("empty");

        main_box.append(&content_stack);

        // InspectorRail (right)
        let inspector_rail = InspectorRail::new();
        inspector_rail.set_open(false); // Closed by default
        main_box.append(inspector_rail.widget());

        toolbar_view.set_content(Some(&main_box));
        window.set_content(Some(&toolbar_view));

        let win = Self {
            window,
            client,
            tool_rail,
            context_panel,
            inspector_rail,
            editor,
            content_stack,
            current_note: RefCell::new(None),
            note_load_state: Rc::new(RefCell::new(RequestState::idle())),
            note_load_generation: Rc::new(RefCell::new(0)),
        };

        // Connect signals
        win.setup_signals();

        win
    }

    fn setup_signals(&self) {
        // Tool mode changes
        let content_stack = self.content_stack.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let current_note_ref = self.current_note.clone();
        self.tool_rail.connect_mode_changed(move |mode| {
            // Update context panel mode
            context_panel_ref.borrow().set_mode(mode);

            // Update content view
            match mode {
                ToolMode::Notes => {
                    // Show editor if note selected, otherwise empty state
                    if current_note_ref.borrow().is_some() {
                        content_stack.set_visible_child_name("editor");
                    } else {
                        content_stack.set_visible_child_name("empty");
                    }
                }
                ToolMode::Search => {
                    // Show editor if note selected, otherwise empty state
                    if current_note_ref.borrow().is_some() {
                        content_stack.set_visible_child_name("editor");
                    } else {
                        content_stack.set_visible_child_name("empty");
                    }
                }
                ToolMode::Graph => {
                    content_stack.set_visible_child_name("graph");
                }
            }
        });

        // Settings button
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.widget().clone();
        self.tool_rail.connect_settings(move || {
            content_stack.set_visible_child_name("settings");
            // Also show inspector rail for settings
            inspector_rail.set_visible(true);
        });

        // Note selection from context panel
        let client = Rc::clone(&self.client);
        let editor_ref = Rc::clone(&self.editor);
        let current_note = self.current_note.clone();
        let note_load_state = Rc::clone(&self.note_load_state);
        let note_load_generation = Rc::clone(&self.note_load_generation);
        let window = self.window.clone();
        let content_stack = self.content_stack.clone();

        self.context_panel
            .borrow()
            .connect_note_selected(move |path| {
                log::info!("Loading note: {}", path);
                window.set_title(Some("Loading note... — Knot"));
                let load_path = path.to_string();
                let log_path = load_path.clone();
                let generation = {
                    let mut current = note_load_generation.borrow_mut();
                    *current += 1;
                    *current
                };

                begin_note_load_with_dispatch(
                    client.as_ref().clone(),
                    load_path,
                    Rc::clone(&note_load_state),
                    generation,
                    Rc::clone(&note_load_generation),
                    |work, ui| {
                        async_bridge::run_background(move || work()).attach_local(move |result| {
                            ui(result);
                        });
                    },
                    {
                        let window = window.clone();
                        let editor_ref = Rc::clone(&editor_ref);
                        let current_note = current_note.clone();
                        let content_stack = content_stack.clone();
                        move |result| match result {
                            Ok(note) => {
                                window.set_title(Some(&format!("{} — Knot", note.title)));
                                editor_ref.load_note(&note);
                                *current_note.borrow_mut() = Some(note);
                                let current_surface = content_stack
                                    .visible_child_name()
                                    .map(|name| name.to_string())
                                    .unwrap_or_else(|| "empty".to_string());
                                if should_route_note_load_to_editor(&current_surface) {
                                    content_stack.set_visible_child_name("editor");
                                }
                            }
                            Err(error) => {
                                window.set_title(Some("Failed to load note — Knot"));
                                log::error!("Failed to load note {}: {}", log_path, error);
                            }
                        }
                    },
                );
            });

        // Inspector close
        let inspector = self.inspector_rail.widget().clone();
        self.inspector_rail.connect_close(move || {
            inspector.set_visible(false);
        });
    }

    pub fn widget(&self) -> &libadwaita::ApplicationWindow {
        &self.window
    }

    pub fn present(&self) {
        self.window.present();
    }
}

// Helper module for logging
mod log {
    pub use tracing::{error, info, warn};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::request_state::RequestState;
    use std::cell::Cell;

    fn sample_note() -> NoteData {
        NoteData {
            id: "note-1".to_string(),
            path: "notes/example.md".to_string(),
            title: "Example".to_string(),
            content: "# Example".to_string(),
            created_at: 1,
            modified_at: 2,
            word_count: 2,
            headings: Vec::new(),
            backlinks: Vec::new(),
            note_type: Some(crate::client::NoteType::Markdown),
            available_modes: None,
            metadata: None,
            embed: None,
            media: None,
            type_badge: Some("MD".to_string()),
            is_dimmed: false,
        }
    }

    #[test]
    fn note_load_uses_dispatcher_and_updates_success_state() {
        let state = Rc::new(RefCell::new(RequestState::idle()));
        let generation = Rc::new(RefCell::new(1_u64));
        let dispatched = Rc::new(Cell::new(false));
        let note = sample_note();

        begin_note_load_with_dispatch(
            KnotdClient::with_socket_path("/tmp/knot.sock"),
            "notes/example.md".to_string(),
            Rc::clone(&state),
            1,
            Rc::clone(&generation),
            {
                let dispatched = Rc::clone(&dispatched);
                let note = note.clone();
                move |_work, ui| {
                    dispatched.set(true);
                    ui(Ok(note));
                }
            },
            |_| {},
        );

        assert!(dispatched.get());
        assert_eq!(*state.borrow(), RequestState::Success(note));
    }

    #[test]
    fn note_load_updates_error_state_without_clearing_previous_note() {
        let state = Rc::new(RefCell::new(RequestState::success(sample_note())));
        let generation = Rc::new(RefCell::new(1_u64));

        begin_note_load_with_dispatch(
            KnotdClient::with_socket_path("/tmp/knot.sock"),
            "notes/missing.md".to_string(),
            Rc::clone(&state),
            1,
            Rc::clone(&generation),
            move |_work, ui| {
                ui(Err("daemon request failed".to_string()));
            },
            |_| {},
        );

        assert_eq!(
            *state.borrow(),
            RequestState::Error("daemon request failed".to_string())
        );
    }

    #[test]
    fn stale_note_load_result_is_ignored_when_newer_selection_exists() {
        let first_state = Rc::new(RefCell::new(RequestState::idle()));
        let second_state = Rc::new(RefCell::new(RequestState::idle()));
        let generation = Rc::new(RefCell::new(2_u64));
        let stale_result: Rc<RefCell<Option<Box<dyn FnOnce()>>>> = Rc::new(RefCell::new(None));
        let fresh_result: Rc<RefCell<Option<Box<dyn FnOnce()>>>> = Rc::new(RefCell::new(None));
        let first_note = sample_note();
        let mut second_note = sample_note();
        second_note.title = "Second".to_string();

        begin_note_load_with_dispatch(
            KnotdClient::with_socket_path("/tmp/knot.sock"),
            "notes/first.md".to_string(),
            Rc::clone(&first_state),
            1,
            Rc::clone(&generation),
            {
                let stale_result = Rc::clone(&stale_result);
                let first_note = first_note.clone();
                move |_work, ui| {
                    *stale_result.borrow_mut() = Some(Box::new(move || ui(Ok(first_note))));
                }
            },
            |_| panic!("stale result should not update the UI"),
        );

        begin_note_load_with_dispatch(
            KnotdClient::with_socket_path("/tmp/knot.sock"),
            "notes/second.md".to_string(),
            Rc::clone(&second_state),
            2,
            Rc::clone(&generation),
            {
                let fresh_result = Rc::clone(&fresh_result);
                let second_note = second_note.clone();
                move |_work, ui| {
                    *fresh_result.borrow_mut() = Some(Box::new(move || ui(Ok(second_note))));
                }
            },
            |_| {},
        );

        fresh_result
            .borrow_mut()
            .take()
            .expect("fresh result should be captured")();
        stale_result
            .borrow_mut()
            .take()
            .expect("stale result should be captured")();

        assert_eq!(*first_state.borrow(), RequestState::Loading);
        assert_eq!(*second_state.borrow(), RequestState::Success(second_note));
    }

    #[test]
    fn note_load_completion_does_not_override_graph_or_settings_surface() {
        assert!(!should_route_note_load_to_editor("graph"));
        assert!(!should_route_note_load_to_editor("settings"));
        assert!(should_route_note_load_to_editor("empty"));
        assert!(should_route_note_load_to_editor("editor"));
    }
}
