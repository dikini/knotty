use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use tracing as log;

use crate::client::{ExplorerFolderNode, ExplorerNoteNode, ExplorerTree, KnotdClient};

pub struct ExplorerView {
    widget: gtk::ScrolledWindow,
    tree_view: gtk::TreeView,
    store: gtk::TreeStore,
    client: Rc<KnotdClient>,
    on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    on_folder_toggled: Rc<RefCell<Option<Box<dyn Fn(&str, bool)>>>>,
}

#[derive(Debug, Clone)]
pub struct ExplorerModel {
    pub path: String,
    pub name: String,
    pub is_folder: bool,
    pub expanded: bool,
}

impl ExplorerView {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        // Store columns: icon, name, path, is_folder, expanded, type_badge
        let store = gtk::TreeStore::new(&[
            String::static_type(), // icon name
            String::static_type(), // display name
            String::static_type(), // path
            bool::static_type(),   // is folder
            bool::static_type(),   // expanded
            String::static_type(), // type badge (pill text)
        ]);

        let tree_view = gtk::TreeView::builder()
            .model(&store)
            .headers_visible(false)
            .build();

        // Icon column
        let icon_renderer = gtk::CellRendererPixbuf::new();
        let icon_column = gtk::TreeViewColumn::builder().title("").build();
        icon_column.pack_start(&icon_renderer, false);
        icon_column.add_attribute(&icon_renderer, "icon-name", 0);
        tree_view.append_column(&icon_column);

        // Name column
        let name_column = gtk::TreeViewColumn::builder()
            .title("Name")
            .expand(true)
            .build();

        let text_renderer = gtk::CellRendererText::new();
        text_renderer.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name_column.pack_start(&text_renderer, true);
        name_column.add_attribute(&text_renderer, "text", 1);

        tree_view.append_column(&name_column);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&tree_view)
            .build();

        let on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
        let on_folder_toggled: Rc<RefCell<Option<Box<dyn Fn(&str, bool)>>>> =
            Rc::new(RefCell::new(None));

        // Handle row expansion/collapse
        let on_folder_toggled_clone = Rc::clone(&on_folder_toggled);
        tree_view.connect_row_expanded(move |_, iter, _| {
            if let Some(ref cb) = *on_folder_toggled_clone.borrow() {
                // Get path from model
                // cb(path, true);
            }
        });

        let on_folder_toggled_clone = Rc::clone(&on_folder_toggled);
        tree_view.connect_row_collapsed(move |_, iter, _| {
            if let Some(ref cb) = *on_folder_toggled_clone.borrow() {
                // Get path from model
                // cb(path, false);
            }
        });

        // Handle selection (single click)
        let on_note_selected_clone = Rc::clone(&on_note_selected);
        tree_view.connect_cursor_changed(move |view| {
            if let Some((model, iter)) = view.selection().selected() {
                let store = model.downcast_ref::<gtk::TreeStore>().unwrap();
                let is_folder: bool = store.get_value(&iter, 3).get().unwrap_or(false);
                let item_path: String = store.get_value(&iter, 2).get().unwrap_or_default();

                if !is_folder {
                    if let Some(ref cb) = *on_note_selected_clone.borrow() {
                        cb(&item_path);
                    }
                }
            }
        });

        Self {
            widget: scrolled,
            tree_view,
            store,
            client,
            on_note_selected,
            on_folder_toggled,
        }
    }

    pub fn load_explorer_tree(&self, tree: &ExplorerTree) {
        self.store.clear();
        self.add_folder_node(None, &tree.root);
    }

    fn add_folder_node(&self, parent: Option<&gtk::TreeIter>, folder: &ExplorerFolderNode) {
        let icon = "folder";

        let iter = self.store.insert(parent, -1);
        self.store.set(
            &iter,
            &[
                (0, &icon),
                (1, &folder.name),
                (2, &folder.path),
                (3, &true),
                (4, &folder.expanded),
                (5, &""), // No badge for folders
            ],
        );

        // Add subfolders
        for subfolder in &folder.folders {
            self.add_folder_node(Some(&iter), subfolder);
        }

        // Add notes
        for note in &folder.notes {
            self.add_note_node(&iter, note);
        }

        // Expand if needed
        if folder.expanded {
            let path = self.store.path(&iter);
            self.tree_view.expand_row(&path, false);
        }
    }

    fn add_note_node(&self, parent: &gtk::TreeIter, note: &ExplorerNoteNode) {
        let (icon, badge) = Self::get_note_type_indicator(&note.type_badge);

        // Combine name and badge for display
        let display_text = if badge.is_empty() {
            note.display_title.clone()
        } else {
            format!("{}  [{}]", note.display_title, badge)
        };

        let iter = self.store.insert(Some(parent), -1);
        self.store.set(
            &iter,
            &[
                (0, &icon),
                (1, &display_text),
                (2, &note.path),
                (3, &false),
                (4, &false),
                (5, &badge),
            ],
        );
    }

    /// Get icon and badge text for a note type
    fn get_note_type_indicator(type_badge: &Option<String>) -> (String, String) {
        log::debug!("Getting indicator for type_badge: {:?}", type_badge);
        // Convert to lowercase for comparison
        let badge_lower = type_badge.as_ref().map(|s| s.to_lowercase());
        match badge_lower.as_deref() {
            Some("youtube") => ("video-x-generic".to_string(), "YT".to_string()),
            Some("pdf") => ("application-pdf".to_string(), "PDF".to_string()),
            // Handle specific image formats (case insensitive)
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("svg") => {
                ("image-x-generic".to_string(), "".to_string())
            }
            Some("image") => ("image-x-generic".to_string(), "".to_string()),
            Some(other) => {
                log::debug!("Unknown note type: {}", other);
                (
                    "text-x-generic".to_string(),
                    other.to_string().to_uppercase(),
                )
            }
            None => ("text-x-markdown".to_string(), "".to_string()),
        }
    }

    pub fn refresh(&self) {
        match self.client.get_explorer_tree() {
            Ok(tree) => {
                self.load_explorer_tree(&tree);
            }
            Err(e) => {
                log::error!("Failed to load explorer tree: {}", e);
            }
        }
    }

    pub fn connect_note_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_note_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_folder_toggled<F>(&self, f: F)
    where
        F: Fn(&str, bool) + 'static,
    {
        *self.on_folder_toggled.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.widget
    }
}

// Simple list-based explorer (alternative to tree view for flatter UI)
pub struct SimpleNoteList {
    widget: gtk::ScrolledWindow,
    list_box: gtk::ListBox,
    client: Rc<KnotdClient>,
    on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
}

impl SimpleNoteList {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        let list_box = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_name("navigation-sidebar")
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list_box)
            .build();

        let on_note_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));

        let on_note_selected_clone = Rc::clone(&on_note_selected);
        list_box.connect_row_activated(move |_, row| {
            if let Some(name) = row.widget_name().as_str().strip_prefix("note-") {
                if let Some(ref cb) = *on_note_selected_clone.borrow() {
                    cb(name);
                }
            }
        });

        Self {
            widget: scrolled,
            list_box,
            client,
            on_note_selected,
        }
    }

    pub fn load_notes(&self, notes: &[crate::client::NoteSummary]) {
        // Clear existing
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        // Add notes
        for note in notes {
            let row = self.create_note_row(note);
            self.list_box.append(&row);
        }
    }

    fn create_note_row(&self, note: &crate::client::NoteSummary) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(&format!("note-{}", note.path));

        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .spacing(8)
            .build();

        // Icon based on type - check both note_type and type_badge
        let icon_name = match (note.note_type.clone(), note.type_badge.as_deref()) {
            (Some(crate::client::NoteType::Youtube), _) => "video-x-generic",
            (Some(crate::client::NoteType::Pdf), _) => "application-pdf",
            (Some(crate::client::NoteType::Image), _) => "image-x-generic",
            // Check type_badge for image formats (case insensitive)
            (_, Some(badge))
                if badge.eq_ignore_ascii_case("png")
                    || badge.eq_ignore_ascii_case("jpg")
                    || badge.eq_ignore_ascii_case("jpeg")
                    || badge.eq_ignore_ascii_case("gif")
                    || badge.eq_ignore_ascii_case("webp")
                    || badge.eq_ignore_ascii_case("svg") =>
            {
                "image-x-generic"
            }
            _ => "text-x-markdown",
        };

        let icon = gtk::Image::builder()
            .icon_name(icon_name)
            .pixel_size(24)
            .build();

        // Title and badge container
        let text_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();

        let title_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let title = gtk::Label::builder()
            .label(&note.title)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(vec!["title".to_string()])
            .hexpand(true)
            .build();

        title_row.append(&title);

        // Type badge (pill) for non-markdown types
        if let Some(ref badge) = note.type_badge {
            if !badge.is_empty() && badge != "markdown" {
                let css_class = match badge.as_str() {
                    "youtube" => "youtube",
                    "pdf" => "pdf",
                    "image" => "image",
                    _ => "",
                };

                let badge_label = gtk::Label::builder().label(&badge.to_uppercase()).build();
                badge_label.add_css_class("badge");
                if !css_class.is_empty() {
                    badge_label.add_css_class(css_class);
                }
                title_row.append(&badge_label);
            }
        }

        let meta = gtk::Label::builder()
            .label(&format!("{} words", note.word_count))
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(30)
            .lines(1)
            .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
            .build();

        text_box.append(&title_row);
        text_box.append(&meta);

        container.append(&icon);
        container.append(&text_box);
        row.set_child(Some(&container));

        row
    }

    pub fn refresh(&self) {
        match self.client.list_notes() {
            Ok(notes) => {
                self.load_notes(&notes);
            }
            Err(e) => {
                log::error!("Failed to load notes: {}", e);
            }
        }
    }

    pub fn connect_note_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_note_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.widget
    }
}
