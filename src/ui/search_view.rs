use gtk::gdk;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use tracing as log;

use crate::client::{KnotdClient, SearchResult};

const MAX_RESULTS: usize = 10;
const DEBOUNCE_MS: u64 = 300;

pub struct SearchView {
    widget: gtk::Box,
    search_entry: gtk::SearchEntry,
    results_list: gtk::ListBox,
    status_label: gtk::Label,
    hint_label: gtk::Label,
    client: Rc<KnotdClient>,
    on_result_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    current_query: RefCell<String>,
    selected_index: RefCell<i32>,
    results: RefCell<Vec<SearchResult>>,
    search_generation: RefCell<u64>,
}

impl SearchView {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Search entry
        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search notes...")
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        // Status label (shows "Type to search", "Searching...", "N results", etc.)
        let status_label = gtk::Label::builder()
            .label("Type to search")
            .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(4)
            .xalign(0.0)
            .build();

        // Results list
        let results_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_name("navigation-sidebar")
            .vexpand(true)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&results_list)
            .build();

        // Hint label
        let hint_label = gtk::Label::builder()
            .label("Tip: Use quotes for phrases")
            .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .xalign(0.0)
            .wrap(true)
            .build();

        widget.append(&search_entry);
        widget.append(&status_label);
        widget.append(&scrolled);
        widget.append(&hint_label);

        let on_result_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> =
            Rc::new(RefCell::new(None));
        let current_query = RefCell::new(String::new());
        let selected_index = RefCell::new(-1i32);
        let results = RefCell::new(Vec::new());
        let search_generation = RefCell::new(0u64);

        // Connect search entry with debouncing
        let client_clone = Rc::clone(&client);
        let results_list_clone = results_list.clone();
        let status_label_clone = status_label.clone();
        let current_query_clone = current_query.clone();
        let selected_index_clone = selected_index.clone();
        let results_clone = results.clone();
        let search_generation_clone = search_generation.clone();

        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            *current_query_clone.borrow_mut() = query.clone();

            // Increment generation to invalidate previous search
            let current_gen = {
                let mut gen = search_generation_clone.borrow_mut();
                *gen += 1;
                *gen
            };

            if query.is_empty() {
                // Clear results
                while let Some(child) = results_list_clone.first_child() {
                    results_list_clone.remove(&child);
                }
                status_label_clone.set_label("Type to search");
                *results_clone.borrow_mut() = Vec::new();
                *selected_index_clone.borrow_mut() = -1;
                return;
            }

            if query.len() < 2 {
                status_label_clone.set_label("Type at least 2 characters...");
                return;
            }

            // Show searching state
            status_label_clone.set_label("Searching...");

            // Debounce search
            let client_inner = Rc::clone(&client_clone);
            let results_list_inner = results_list_clone.clone();
            let status_label_inner = status_label_clone.clone();
            let query_inner = query.clone();
            let results_inner = results_clone.clone();
            let selected_index_inner = selected_index_clone.clone();

            let search_generation_inner = search_generation_clone.clone();

            glib::timeout_add_local(Duration::from_millis(DEBOUNCE_MS), move || {
                // Check if this search is still valid (generation hasn't changed)
                if *search_generation_inner.borrow() != current_gen {
                    return glib::ControlFlow::Break;
                }

                // Perform search
                match client_inner.search_notes(&query_inner, Some(MAX_RESULTS)) {
                    Ok(search_results) => {
                        // Check again before updating UI
                        if *search_generation_inner.borrow() != current_gen {
                            return glib::ControlFlow::Break;
                        }

                        // Clear existing
                        while let Some(child) = results_list_inner.first_child() {
                            results_list_inner.remove(&child);
                        }

                        let count = search_results.len();
                        *results_inner.borrow_mut() = search_results.clone();

                        if count == 0 {
                            status_label_inner
                                .set_label(&format!("No results for '{}'", query_inner));
                            *selected_index_inner.borrow_mut() = -1;
                        } else {
                            status_label_inner.set_label(&format!(
                                "{} result{} for '{}'",
                                count,
                                if count == 1 { "" } else { "s" },
                                query_inner
                            ));
                            *selected_index_inner.borrow_mut() = 0;

                            // Add results
                            for (idx, result) in search_results.iter().enumerate() {
                                let row = create_search_result_row(result, &query_inner, idx == 0);
                                results_list_inner.append(&row);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Search failed: {}", e);
                        status_label_inner.set_label(&format!("Search error: {}", e));
                        *results_inner.borrow_mut() = Vec::new();
                        *selected_index_inner.borrow_mut() = -1;
                    }
                }

                glib::ControlFlow::Break
            });
        });

        // Keyboard navigation using EventControllerKey (GTK4)
        let results_list_ref = results_list.clone();
        let selected_index_ref = selected_index.clone();
        let results_ref = results.clone();
        let on_result_selected_ref = Rc::clone(&on_result_selected);
        let search_entry_ref = search_entry.clone();
        let status_label_ref = status_label.clone();

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            let results = results_ref.borrow();
            let mut selected = *selected_index_ref.borrow_mut();

            match key {
                gdk::Key::Down => {
                    if !results.is_empty() {
                        if selected < results.len() as i32 - 1 {
                            selected += 1;
                            *selected_index_ref.borrow_mut() = selected;
                            // Update visual selection
                            if let Some(row) = results_list_ref.row_at_index(selected) {
                                results_list_ref.select_row(Some(&row));
                                // Grab focus for keyboard navigation feedback
                                row.grab_focus();
                            }
                        }
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    if selected > 0 {
                        selected -= 1;
                        *selected_index_ref.borrow_mut() = selected;
                        if let Some(row) = results_list_ref.row_at_index(selected) {
                            results_list_ref.select_row(Some(&row));
                        }
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    if selected >= 0 && selected < results.len() as i32 {
                        if let Some(result) = results.get(selected as usize) {
                            if let Some(ref cb) = *on_result_selected_ref.borrow() {
                                cb(&result.path);
                            }
                        }
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Escape => {
                    // Clear search text and results
                    search_entry_ref.set_text("");
                    status_label_ref.set_label("Type to search");
                    glib::Propagation::Proceed
                }
                _ => glib::Propagation::Proceed,
            }
        });
        search_entry.add_controller(key_controller);

        // Connect result selection
        let on_result_selected_clone = Rc::clone(&on_result_selected);
        results_list.connect_row_activated(move |list, row| {
            if let Some(path) = row.widget_name().as_str().strip_prefix("search-") {
                // Find the result
                if let Some(ref cb) = *on_result_selected_clone.borrow() {
                    cb(path);
                }
            }
        });

        Self {
            widget,
            search_entry,
            results_list,
            status_label,
            hint_label,
            client,
            on_result_selected,
            current_query,
            selected_index,
            results,
            search_generation,
        }
    }

    pub fn grab_focus(&self) {
        self.search_entry.grab_focus();
    }

    pub fn clear(&self) {
        // Increment generation to invalidate any pending searches
        *self.search_generation.borrow_mut() += 1;

        self.search_entry.set_text("");
        while let Some(child) = self.results_list.first_child() {
            self.results_list.remove(&child);
        }
        self.status_label.set_label("Type to search");
        *self.results.borrow_mut() = Vec::new();
        *self.selected_index.borrow_mut() = -1;
    }

    pub fn connect_result_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_result_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

fn create_search_result_row(
    result: &SearchResult,
    query: &str,
    is_selected: bool,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(&format!("search-{}", result.path));

    if is_selected {
        row.add_css_class("selected");
    }

    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .spacing(4)
        .build();

    // Title with highlighted match
    let title = gtk::Label::builder()
        .label(&highlight_text(&result.title, query))
        .use_markup(true)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(vec!["title".to_string()])
        .build();

    // Excerpt
    let excerpt = gtk::Label::builder()
        .label(&result.excerpt)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .lines(2)
        .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
        .build();

    container.append(&title);
    container.append(&excerpt);
    row.set_child(Some(&container));

    row
}

/// Highlight matching text using Pango markup
fn highlight_text(text: &str, query: &str) -> String {
    if query.is_empty() {
        return glib::markup_escape_text(text).to_string();
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();

    let mut result = String::new();
    let mut last_end = 0;

    // Find all occurrences (case insensitive)
    while let Some(pos) = text_lower[last_end..].find(&query_lower) {
        let start = last_end + pos;
        let end = start + query.len();

        // Add text before match
        if start > last_end {
            result.push_str(&glib::markup_escape_text(&text[last_end..start]));
        }

        // Add highlighted match
        result.push_str("<b>");
        result.push_str(&glib::markup_escape_text(&text[start..end]));
        result.push_str("</b>");

        last_end = end;
    }

    // Add remaining text
    if last_end < text.len() {
        result.push_str(&glib::markup_escape_text(&text[last_end..]));
    }

    result
}
