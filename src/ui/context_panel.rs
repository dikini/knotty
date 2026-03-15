use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::KnotdClient;
use crate::ui::explorer::ExplorerView;
use crate::ui::tool_rail::ToolMode;

pub struct ContextPanel {
    widget: gtk::Box,
    mode: RefCell<ToolMode>,
    stack: gtk::Stack,
    explorer: ExplorerView,
    on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    on_mode_changed: Rc<RefCell<Option<Box<dyn Fn(ToolMode)>>>>,
}

impl ContextPanel {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_name("context-panel")
            .width_request(280)
            .build();

        // Header with title
        let header = gtk::Label::builder()
            .label("Notes")
            .css_classes(vec!["title-4".to_string()])
            .margin_top(12)
            .margin_bottom(8)
            .margin_start(16)
            .margin_end(16)
            .xalign(0.0)
            .build();

        // Stack for different modes
        let stack = gtk::Stack::builder().vexpand(true).build();

        // Notes view - with directory tree
        let notes_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let new_note_btn = gtk::Button::builder()
            .label("New Note")
            .icon_name("document-new-symbolic")
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .build();

        let explorer = ExplorerView::new(Rc::clone(&client));

        notes_view.append(&new_note_btn);
        notes_view.append(explorer.widget());

        let search_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .build();
        search_view.append(
            &gtk::Label::builder()
                .label("Search is shown in the main panel.")
                .wrap(true)
                .css_classes(vec!["dim-label".to_string()])
                .build(),
        );

        // Graph view (placeholder)
        let graph_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .build();

        let graph_label = gtk::Label::builder()
            .label("Graph controls will appear here")
            .build();
        graph_view.append(&graph_label);

        let settings_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .build();
        settings_view.append(
            &gtk::Label::builder()
                .label("Settings context is shown in the main panel.")
                .wrap(true)
                .css_classes(vec!["dim-label".to_string()])
                .build(),
        );

        // Add to stack
        stack.add_titled(&notes_view, Some("notes"), "Notes");
        stack.add_titled(&search_view, Some("search"), "Search");
        stack.add_titled(&graph_view, Some("graph"), "Graph");
        stack.add_titled(&settings_view, Some("settings"), "Settings");

        widget.append(&header);
        widget.append(&stack);

        let on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
        let on_mode_changed: Rc<RefCell<Option<Box<dyn Fn(ToolMode)>>>> =
            Rc::new(RefCell::new(None));

        // Wire up explorer selection
        let on_note_selected_clone = Rc::clone(&on_note_selected);
        explorer.connect_note_selected(move |path| {
            if let Some(ref cb) = *on_note_selected_clone.borrow() {
                cb(path);
            }
        });

        let panel = Self {
            widget,
            mode: RefCell::new(ToolMode::Notes),
            stack,
            explorer,
            on_note_selected,
            on_mode_changed,
        };

        // Initial load
        panel.explorer.refresh();

        panel
    }

    pub fn set_mode(&self, mode: ToolMode) {
        *self.mode.borrow_mut() = mode;

        let (label, visible_child) = match mode {
            ToolMode::Notes => ("Notes", "notes"),
            ToolMode::Search => ("Search", "search"),
            ToolMode::Graph => ("Graph", "graph"),
            ToolMode::Settings => ("Settings", "settings"),
        };

        // Update header label
        if let Some(header) = self.widget.first_child().and_downcast::<gtk::Label>() {
            header.set_label(label);
        }

        // Switch stack
        self.stack.set_visible_child_name(visible_child);

        // Mode-specific actions
        match mode {
            ToolMode::Notes => {
                self.explorer.refresh();
            }
            ToolMode::Settings => {}
            _ => {}
        }

        // Notify listeners
        if let Some(ref cb) = *self.on_mode_changed.borrow() {
            cb(mode);
        }
    }

    pub fn refresh(&self) {
        self.explorer.refresh();
    }

    pub fn connect_note_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_note_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_mode_changed<F>(&self, f: F)
    where
        F: Fn(ToolMode) + 'static,
    {
        *self.on_mode_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}
