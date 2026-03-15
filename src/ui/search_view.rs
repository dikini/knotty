use gtk::gdk;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use tracing as log;

use crate::client::{KnotdClient, SearchResult};
use crate::ui::async_bridge;

const MAX_RESULTS: usize = 10;
const DEBOUNCE_MS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SearchState {
    Idle,
    Loading { query: String },
    Empty { query: String },
    Results { query: String, count: usize },
    Error { message: String },
}

impl SearchState {
    fn status_text(&self) -> String {
        match self {
            SearchState::Idle => "Type to search".to_string(),
            SearchState::Loading { .. } => "Searching...".to_string(),
            SearchState::Empty { query } => format!("No results for '{}'", query),
            SearchState::Results { query, count } => format!(
                "{} result{} for '{}'",
                count,
                if *count == 1 { "" } else { "s" },
                query
            ),
            SearchState::Error { message } => format!("Search error: {}", message),
        }
    }
}

fn apply_search_state(
    state: &SearchState,
    status_label: &gtk::Label,
    results: &RefCell<Vec<SearchResult>>,
    selected_index: &RefCell<i32>,
    results_list: &gtk::ListBox,
    query_for_results: Option<&str>,
) {
    status_label.set_label(&state.status_text());

    match state {
        SearchState::Idle | SearchState::Loading { .. } | SearchState::Empty { .. } => {
            while let Some(child) = results_list.first_child() {
                results_list.remove(&child);
            }
            *results.borrow_mut() = Vec::new();
            *selected_index.borrow_mut() = -1;
        }
        SearchState::Error { .. } => {
            while let Some(child) = results_list.first_child() {
                results_list.remove(&child);
            }
            *results.borrow_mut() = Vec::new();
            *selected_index.borrow_mut() = -1;
        }
        SearchState::Results { .. } => {
            while let Some(child) = results_list.first_child() {
                results_list.remove(&child);
            }
            *selected_index.borrow_mut() = if results.borrow().is_empty() { -1 } else { 0 };
            if let Some(query) = query_for_results {
                for (idx, result) in results.borrow().iter().enumerate() {
                    let row = create_search_result_row(result, query, idx == 0);
                    results_list.append(&row);
                }
            }
        }
    }
}

fn set_search_state(
    state_cell: &RefCell<SearchState>,
    next_state: SearchState,
    status_label: &gtk::Label,
    results: &RefCell<Vec<SearchResult>>,
    selected_index: &RefCell<i32>,
    results_list: &gtk::ListBox,
    query_for_results: Option<&str>,
) {
    *state_cell.borrow_mut() = next_state.clone();
    apply_search_state(
        &next_state,
        status_label,
        results,
        selected_index,
        results_list,
        query_for_results,
    );
}

fn clear_search_view(
    search_entry: &gtk::SearchEntry,
    state: &RefCell<SearchState>,
    results: &RefCell<Vec<SearchResult>>,
    selected_index: &RefCell<i32>,
    search_generation: &RefCell<u64>,
    suppress_search_changed: &RefCell<bool>,
    status_label: &gtk::Label,
    results_list: &gtk::ListBox,
) {
    *search_generation.borrow_mut() += 1;
    *suppress_search_changed.borrow_mut() = true;
    search_entry.set_text("");
    set_search_state(
        state,
        SearchState::Idle,
        status_label,
        results,
        selected_index,
        results_list,
        None,
    );
}

pub struct SearchView {
    widget: gtk::Box,
    search_entry: gtk::SearchEntry,
    results_list: gtk::ListBox,
    status_label: gtk::Label,
    on_result_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    selected_index: RefCell<i32>,
    results: RefCell<Vec<SearchResult>>,
    search_generation: RefCell<u64>,
    suppress_search_changed: RefCell<bool>,
    state: RefCell<SearchState>,
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
        let selected_index = RefCell::new(-1i32);
        let results = RefCell::new(Vec::new());
        let search_generation = RefCell::new(0u64);
        let suppress_search_changed = RefCell::new(false);
        let state = RefCell::new(SearchState::Idle);

        // Connect search entry with debouncing
        let client_clone = Rc::clone(&client);
        let results_list_clone = results_list.clone();
        let status_label_clone = status_label.clone();
        let selected_index_clone = selected_index.clone();
        let results_clone = results.clone();
        let search_generation_clone = search_generation.clone();
        let suppress_search_changed_clone = suppress_search_changed.clone();
        let state_clone = state.clone();

        search_entry.connect_search_changed(move |entry| {
            if *suppress_search_changed_clone.borrow() {
                *suppress_search_changed_clone.borrow_mut() = false;
                return;
            }

            let query = entry.text().to_string();

            // Increment generation to invalidate previous search
            let current_gen = {
                let mut gen = search_generation_clone.borrow_mut();
                *gen += 1;
                *gen
            };

            if query.is_empty() {
                set_search_state(
                    &state_clone,
                    SearchState::Idle,
                    &status_label_clone,
                    &results_clone,
                    &selected_index_clone,
                    &results_list_clone,
                    None,
                );
                return;
            }

            if query.len() < 2 {
                set_search_state(
                    &state_clone,
                    SearchState::Idle,
                    &status_label_clone,
                    &results_clone,
                    &selected_index_clone,
                    &results_list_clone,
                    None,
                );
                return;
            }

            let loading_state = SearchState::Loading {
                query: query.clone(),
            };
            set_search_state(
                &state_clone,
                loading_state,
                &status_label_clone,
                &results_clone,
                &selected_index_clone,
                &results_list_clone,
                None,
            );

            // Debounce search
            let client_inner = Rc::clone(&client_clone);
            let results_list_inner = results_list_clone.clone();
            let status_label_inner = status_label_clone.clone();
            let query_inner = query.clone();
            let results_inner = results_clone.clone();
            let selected_index_inner = selected_index_clone.clone();
            let search_generation_inner = search_generation_clone.clone();
            let state_inner = state_clone.clone();

            glib::timeout_add_local(Duration::from_millis(DEBOUNCE_MS), move || {
                // Check if this search is still valid (generation hasn't changed)
                if *search_generation_inner.borrow() != current_gen {
                    return glib::ControlFlow::Break;
                }

                let client_for_work = client_inner.as_ref().clone();
                let query_for_work = query_inner.clone();
                let query_for_ui = query_inner.clone();
                let search_generation_for_ui = search_generation_inner.clone();
                let results_for_ui = results_inner.clone();
                let state_for_ui = state_inner.clone();
                let status_label_for_ui = status_label_inner.clone();
                let selected_index_for_ui = selected_index_inner.clone();
                let results_list_for_ui = results_list_inner.clone();

                async_bridge::run_background(move || {
                    client_for_work
                        .search_notes(&query_for_work, Some(MAX_RESULTS))
                        .map_err(|error| error.to_string())
                })
                .attach_local(move |result| {
                    if *search_generation_for_ui.borrow() != current_gen {
                        return;
                    }

                    match result {
                        Ok(search_results) => {
                            *results_for_ui.borrow_mut() = search_results;
                            let next_state = if results_for_ui.borrow().is_empty() {
                                SearchState::Empty {
                                    query: query_for_ui.clone(),
                                }
                            } else {
                                SearchState::Results {
                                    query: query_for_ui.clone(),
                                    count: results_for_ui.borrow().len(),
                                }
                            };
                            set_search_state(
                                &state_for_ui,
                                next_state,
                                &status_label_for_ui,
                                &results_for_ui,
                                &selected_index_for_ui,
                                &results_list_for_ui,
                                Some(&query_for_ui),
                            );
                        }
                        Err(error) => {
                            log::error!("Search failed: {}", error);
                            set_search_state(
                                &state_for_ui,
                                SearchState::Error { message: error },
                                &status_label_for_ui,
                                &results_for_ui,
                                &selected_index_for_ui,
                                &results_list_for_ui,
                                None,
                            );
                        }
                    }
                });

                glib::ControlFlow::Break
            });
        });

        // Keyboard navigation using EventControllerKey (GTK4)
        let results_list_ref = results_list.clone();
        let selected_index_ref = selected_index.clone();
        let results_ref = results.clone();
        let on_result_selected_ref = Rc::clone(&on_result_selected);
        let search_entry_ref = search_entry.clone();
        let state_ref = state.clone();
        let search_generation_ref = search_generation.clone();
        let suppress_search_changed_ref = suppress_search_changed.clone();
        let status_label_ref = status_label.clone();
        let results_list_ref_for_clear = results_list.clone();

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| match key {
            gdk::Key::Down => {
                let next_selected = {
                    let results = results_ref.borrow();
                    let selected = *selected_index_ref.borrow();
                    if !results.is_empty() && selected < results.len() as i32 - 1 {
                        Some(selected + 1)
                    } else {
                        None
                    }
                };

                if let Some(next_selected) = next_selected {
                    *selected_index_ref.borrow_mut() = next_selected;
                    if let Some(row) = results_list_ref.row_at_index(next_selected) {
                        results_list_ref.select_row(Some(&row));
                        row.grab_focus();
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Up => {
                let next_selected = {
                    let selected = *selected_index_ref.borrow();
                    if selected > 0 {
                        Some(selected - 1)
                    } else {
                        None
                    }
                };

                if let Some(next_selected) = next_selected {
                    *selected_index_ref.borrow_mut() = next_selected;
                    if let Some(row) = results_list_ref.row_at_index(next_selected) {
                        results_list_ref.select_row(Some(&row));
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                let selected_path = {
                    let results = results_ref.borrow();
                    let selected = *selected_index_ref.borrow();
                    if selected >= 0 && selected < results.len() as i32 {
                        results
                            .get(selected as usize)
                            .map(|result| result.path.clone())
                    } else {
                        None
                    }
                };

                if let Some(path) = selected_path {
                    if let Some(ref cb) = *on_result_selected_ref.borrow() {
                        cb(&path);
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Escape => {
                clear_search_view(
                    &search_entry_ref,
                    &state_ref,
                    &results_ref,
                    &selected_index_ref,
                    &search_generation_ref,
                    &suppress_search_changed_ref,
                    &status_label_ref,
                    &results_list_ref_for_clear,
                );
                glib::Propagation::Proceed
            }
            _ => glib::Propagation::Proceed,
        });
        search_entry.add_controller(key_controller);

        // Connect result selection
        let on_result_selected_clone = Rc::clone(&on_result_selected);
        results_list.connect_row_activated(move |_list, row| {
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
            on_result_selected,
            selected_index,
            results,
            search_generation,
            suppress_search_changed,
            state,
        }
    }

    pub fn grab_focus(&self) {
        self.search_entry.grab_focus();
    }

    pub fn clear(&self) {
        clear_search_view(
            &self.search_entry,
            &self.state,
            &self.results,
            &self.selected_index,
            &self.search_generation,
            &self.suppress_search_changed,
            &self.status_label,
            &self.results_list,
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_state_exposes_required_status_messages() {
        assert_eq!(SearchState::Idle.status_text(), "Type to search");
        assert_eq!(
            SearchState::Loading {
                query: "graph".to_string()
            }
            .status_text(),
            "Searching..."
        );
        assert_eq!(
            SearchState::Empty {
                query: "graph".to_string()
            }
            .status_text(),
            "No results for 'graph'"
        );
        assert_eq!(
            SearchState::Results {
                query: "graph".to_string(),
                count: 2
            }
            .status_text(),
            "2 results for 'graph'"
        );
        assert_eq!(
            SearchState::Error {
                message: "offline".to_string()
            }
            .status_text(),
            "Search error: offline"
        );
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
