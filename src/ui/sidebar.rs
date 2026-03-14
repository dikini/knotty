use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::{KnotdClient, NoteSummary};

/// Sidebar widget showing list of notes
pub struct Sidebar {
    widget: libadwaita::ToolbarView,
    list_box: gtk::ListBox,
    client: Rc<KnotdClient>,
    notes: Rc<RefCell<Vec<NoteSummary>>>,
    on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
}

impl Sidebar {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        // Create toolbar view for sidebar
        let widget = libadwaita::ToolbarView::builder()
            .width_request(250)
            .build();
        
        widget.add_css_class("sidebar");
        
        // Header bar for sidebar
        let header = libadwaita::HeaderBar::builder()
            .show_end_title_buttons(false)
            .show_start_title_buttons(false)
            .centering_policy(libadwaita::CenteringPolicy::Strict)
            .build();
        
        let title = libadwaita::WindowTitle::builder()
            .title("Notes")
            .build();
        header.set_title_widget(Some(&title));
        
        // New note button in sidebar header
        let new_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Note")
            .build();
        new_btn.add_css_class("flat");
        header.pack_end(&new_btn);
        
        widget.add_top_bar(&header);
        
        // Search entry
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search notes...")
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        
        // Scrolled window for list
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        
        // List box for notes
        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar".to_string()])
            .build();
        
        scrolled.set_child(Some(&list_box));
        
        // Content box
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        
        content.append(&search_entry);
        content.append(&scrolled);
        
        widget.set_content(Some(&content));
        
        let notes: Rc<RefCell<Vec<NoteSummary>>> = Rc::new(RefCell::new(Vec::new()));
        let on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
        
        // Setup signals
        Self::setup_signals(&list_box, &search_entry, &notes, &on_note_selected);
        
        Self {
            widget,
            list_box,
            client,
            notes,
            on_note_selected,
        }
    }
    
    fn setup_signals(
        list_box: &gtk::ListBox,
        search_entry: &gtk::SearchEntry,
        notes: &Rc<RefCell<Vec<NoteSummary>>>,
        callback: &Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    ) {
        // Handle row selection
        let callback_clone = Rc::clone(callback);
        list_box.connect_row_activated(move |_list, row| {
            if let Some(ref cb) = *callback_clone.borrow() {
                if let Some(id) = row.widget_name().as_str().strip_prefix("note-") {
                    cb(id);
                }
            }
        });
        
        // Handle search
        let _notes = Rc::clone(notes);
        let list = list_box.clone();
        search_entry.connect_search_changed(move |entry| {
            let _text = entry.text().to_lowercase();
            let _ = &list;
            // TODO: Filter list based on search text
        });
    }
    
    pub fn set_notes(&self, notes: Vec<NoteSummary>) {
        *self.notes.borrow_mut() = notes.clone();
        
        // Clear existing
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        
        // Add notes
        for note in notes {
            let row = self.create_note_row(&note);
            self.list_box.append(&row);
        }
    }
    
    fn create_note_row(&self, note: &NoteSummary) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(&format!("note-{}", note.id));
        
        let box_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .spacing(4)
            .build();
        
        let title_label = gtk::Label::builder()
            .label(&note.title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(vec!["title".to_string()])
            .build();
        
        let preview_label = gtk::Label::builder()
            .label(&note.preview)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(30)
            .lines(1)
            .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
            .build();
        
        box_container.append(&title_label);
        box_container.append(&preview_label);
        
        row.set_child(Some(&box_container));
        row
    }
    
    pub fn connect_note_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_note_selected.borrow_mut() = Some(Box::new(f));
    }
    
    pub fn widget(&self) -> &libadwaita::ToolbarView {
        &self.widget
    }
}
