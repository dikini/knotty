use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::{KnotdClient, NoteData};
use crate::ui::context_panel::ContextPanel;
use crate::ui::editor::NoteEditor;
use crate::ui::inspector_rail::InspectorRail;
use crate::ui::tool_rail::{ToolMode, ToolRail};

pub struct KnotWindow {
    window: libadwaita::ApplicationWindow,
    client: Rc<KnotdClient>,
    tool_rail: ToolRail,
    context_panel: Rc<RefCell<ContextPanel>>,
    inspector_rail: InspectorRail,
    editor: Rc<NoteEditor>,
    content_stack: gtk::Stack,
    current_note: RefCell<Option<NoteData>>,
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
        let window = self.window.clone();
        let content_stack = self.content_stack.clone();

        self.context_panel
            .borrow()
            .connect_note_selected(move |path| {
                log::info!("Loading note: {}", path);

                match client.get_note(path) {
                    Ok(note) => {
                        // Update window title
                        window.set_title(Some(&format!("{} — Knot", note.title)));

                        // Load into editor
                        editor_ref.load_note(&note);

                        *current_note.borrow_mut() = Some(note);

                        // Show editor view
                        content_stack.set_visible_child_name("editor");
                    }
                    Err(e) => {
                        log::error!("Failed to load note {}: {}", path, e);
                    }
                }
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
