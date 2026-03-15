use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type CloseCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

#[derive(Clone)]
pub struct InspectorRail {
    widget: gtk::Box,
    title_label: gtk::Label,
    content_stack: gtk::Stack,
    is_open: RefCell<bool>,
    on_close: CloseCallback,
}

impl InspectorRail {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_name("inspector-rail")
            .width_request(280)
            .build();

        // Header
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(16)
            .margin_end(16)
            .build();

        let title_label = gtk::Label::builder()
            .label("Inspector")
            .css_classes(vec!["title-4".to_string()])
            .hexpand(true)
            .xalign(0.0)
            .build();

        let close_btn = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .css_classes(vec!["flat".to_string(), "circular".to_string()])
            .build();

        header.append(&title_label);
        header.append(&close_btn);

        // Content stack
        let content_stack = gtk::Stack::builder().vexpand(true).build();

        // Details view
        let details_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(16)
            .margin_end(16)
            .build();

        let details_placeholder = gtk::Label::builder()
            .label("Select a note to see details")
            .wrap(true)
            .css_classes(vec!["dim-label".to_string()])
            .build();
        details_view.append(&details_placeholder);

        // Settings view (placeholder)
        let settings_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(16)
            .margin_end(16)
            .build();

        let settings_label = gtk::Label::builder()
            .label("Settings will appear here")
            .wrap(true)
            .build();
        settings_view.append(&settings_label);

        content_stack.add_titled(&details_view, Some("details"), "Details");
        content_stack.add_titled(&settings_view, Some("settings"), "Settings");

        widget.append(&header);
        widget.append(&content_stack);

        let on_close: CloseCallback = Rc::new(RefCell::new(None));

        // Connect close button
        let on_close_clone = Rc::clone(&on_close);
        close_btn.connect_clicked(move |_| {
            if let Some(ref cb) = *on_close_clone.borrow() {
                cb();
            }
        });

        let rail = Self {
            widget,
            title_label,
            content_stack,
            is_open: RefCell::new(false),
            on_close,
        };

        rail.set_open(false); // Closed by default
        rail
    }

    pub fn set_open(&self, open: bool) {
        *self.is_open.borrow_mut() = open;
        self.widget.set_visible(open);
    }

    pub fn set_mode(&self, mode: &str) {
        self.content_stack.set_visible_child_name(mode);
        let title = match mode {
            "details" => "Details",
            "settings" => "Settings",
            _ => "Inspector",
        };
        self.title_label.set_label(title);
    }

    pub fn connect_close<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        *self.on_close.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

impl Default for InspectorRail {
    fn default() -> Self {
        Self::new()
    }
}
