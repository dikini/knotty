use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolMode {
    Notes,
    Search,
    Graph,
}

type ModeChangeCallback = Rc<RefCell<Option<Box<dyn Fn(ToolMode)>>>>;
type SettingsCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

pub struct ToolRail {
    widget: gtk::Box,
    mode: RefCell<ToolMode>,
    notes_btn: gtk::Button,
    search_btn: gtk::Button,
    graph_btn: gtk::Button,
    on_mode_change: ModeChangeCallback,
    on_settings: SettingsCallback,
}

impl ToolRail {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_name("tool-rail")
            .width_request(56)
            .build();

        // Tools section
        let tools_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(6)
            .margin_end(6)
            .vexpand(true)
            .build();

        // Notes button
        let notes_btn = gtk::Button::builder()
            .icon_name("emblem-documents-symbolic")
            .tooltip_text("Notes")
            .build();
        notes_btn.add_css_class("tool-button");

        // Search button
        let search_btn = gtk::Button::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search")
            .build();
        search_btn.add_css_class("tool-button");

        // Graph button
        let graph_btn = gtk::Button::builder()
            .icon_name("network-workgroup-symbolic")
            .tooltip_text("Graph")
            .build();
        graph_btn.add_css_class("tool-button");

        tools_box.append(&notes_btn);
        tools_box.append(&search_btn);
        tools_box.append(&graph_btn);

        // Separator
        let separator = gtk::Separator::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(8)
            .build();

        // Footer with settings
        let footer = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_bottom(8)
            .margin_start(6)
            .margin_end(6)
            .build();

        let settings_btn = gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Settings")
            .build();
        settings_btn.add_css_class("tool-button");
        settings_btn.add_css_class("flat");

        footer.append(&settings_btn);

        widget.append(&tools_box);
        widget.append(&separator);
        widget.append(&footer);

        let on_mode_change: ModeChangeCallback = Rc::new(RefCell::new(None));
        let on_settings: SettingsCallback = Rc::new(RefCell::new(None));

        // Connect signals - manage active class
        // Connect signals - manage active class using standard closures
        let on_mode_change_clone = Rc::clone(&on_mode_change);
        let notes_btn_ref = notes_btn.clone();
        let search_btn_ref = search_btn.clone();
        let graph_btn_ref = graph_btn.clone();
        notes_btn.connect_clicked(move |_| {
            notes_btn_ref.add_css_class("active");
            search_btn_ref.remove_css_class("active");
            graph_btn_ref.remove_css_class("active");

            if let Some(ref cb) = *on_mode_change_clone.borrow() {
                cb(ToolMode::Notes);
            }
        });

        let on_mode_change_clone = Rc::clone(&on_mode_change);
        let notes_btn_ref = notes_btn.clone();
        let search_btn_ref = search_btn.clone();
        let graph_btn_ref = graph_btn.clone();
        search_btn.connect_clicked(move |_| {
            search_btn_ref.add_css_class("active");
            notes_btn_ref.remove_css_class("active");
            graph_btn_ref.remove_css_class("active");

            if let Some(ref cb) = *on_mode_change_clone.borrow() {
                cb(ToolMode::Search);
            }
        });

        let on_mode_change_clone = Rc::clone(&on_mode_change);
        let notes_btn_ref = notes_btn.clone();
        let search_btn_ref = search_btn.clone();
        let graph_btn_ref = graph_btn.clone();
        graph_btn.connect_clicked(move |_| {
            graph_btn_ref.add_css_class("active");
            notes_btn_ref.remove_css_class("active");
            search_btn_ref.remove_css_class("active");

            if let Some(ref cb) = *on_mode_change_clone.borrow() {
                cb(ToolMode::Graph);
            }
        });

        // Start with Notes as active
        notes_btn.add_css_class("active");

        let on_settings_clone = Rc::clone(&on_settings);
        settings_btn.connect_clicked(move |_| {
            if let Some(ref cb) = *on_settings_clone.borrow() {
                cb();
            }
        });

        Self {
            widget,
            mode: RefCell::new(ToolMode::Notes),
            notes_btn,
            search_btn,
            graph_btn,
            on_mode_change,
            on_settings,
        }
    }

    pub fn set_active_mode(&self, mode: ToolMode) {
        *self.mode.borrow_mut() = mode;

        // Update visual state
        self.notes_btn.remove_css_class("active");
        self.search_btn.remove_css_class("active");
        self.graph_btn.remove_css_class("active");

        match mode {
            ToolMode::Notes => self.notes_btn.add_css_class("active"),
            ToolMode::Search => self.search_btn.add_css_class("active"),
            ToolMode::Graph => self.graph_btn.add_css_class("active"),
        }
    }

    pub fn connect_mode_changed<F>(&self, f: F)
    where
        F: Fn(ToolMode) + 'static,
    {
        *self.on_mode_change.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_settings<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        *self.on_settings.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

impl Default for ToolRail {
    fn default() -> Self {
        Self::new()
    }
}
