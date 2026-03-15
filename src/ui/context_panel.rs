use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::KnotdClient;
use crate::ui::explorer::{ExplorerSelection, ExplorerView, NoteSwitchDecision};
use crate::ui::tool_rail::ToolMode;

type NoteSelectedCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;
type SelectionClearedCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

pub struct ContextPanel {
    widget: gtk::Box,
    header_label: gtk::Label,
    mode: RefCell<ToolMode>,
    stack: gtk::Stack,
    explorer: Rc<ExplorerView>,
    status_label: gtk::Label,
    rename_btn: gtk::Button,
    delete_btn: gtk::Button,
    on_note_selected: NoteSelectedCallback,
    on_selection_cleared: SelectionClearedCallback,
}

fn root_window(widget: &gtk::Widget) -> Option<gtk::Window> {
    widget.root()?.downcast::<gtk::Window>().ok()
}

fn selected_display_name(selection: &ExplorerSelection) -> String {
    selection
        .path()
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("root")
        .to_string()
}

fn show_status(label: &gtk::Label, message: &str, is_error: bool) {
    label.set_label(message);
    label.set_visible(!message.is_empty());
    if is_error {
        label.remove_css_class("dim-label");
        label.add_css_class("error");
    } else {
        label.remove_css_class("error");
        label.add_css_class("dim-label");
    }
}

fn prompt_text<F>(
    parent: Option<&gtk::Window>,
    title: &str,
    accept_label: &str,
    placeholder: &str,
    initial_text: Option<&str>,
    on_accept: F,
) where
    F: FnOnce(String) + 'static,
{
    let dialog = gtk::Window::builder()
        .title(title)
        .modal(true)
        .resizable(false)
        .default_width(360)
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let entry = gtk::Entry::builder()
        .placeholder_text(placeholder)
        .hexpand(true)
        .build();
    if let Some(initial_text) = initial_text {
        entry.set_text(initial_text);
        entry.select_region(0, -1);
    }
    content.append(&entry);
    let actions = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::End)
        .spacing(6)
        .build();
    let cancel_btn = gtk::Button::with_label("Cancel");
    let accept_btn = gtk::Button::with_label(accept_label);
    actions.append(&cancel_btn);
    actions.append(&accept_btn);
    content.append(&actions);
    dialog.set_child(Some(&content));

    let entry_for_accept = entry.clone();
    let on_accept = Rc::new(RefCell::new(Some(on_accept)));
    let dialog_for_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_for_cancel.close();
    });

    let dialog_for_accept = dialog.clone();
    let on_accept_for_click = Rc::clone(&on_accept);
    accept_btn.connect_clicked(move |_| {
        if let Some(callback) = on_accept_for_click.borrow_mut().take() {
            callback(entry_for_accept.text().to_string());
        }
        dialog_for_accept.close();
    });

    let accept_btn_for_enter = accept_btn.clone();
    entry.connect_activate(move |_| {
        accept_btn_for_enter.emit_clicked();
    });

    dialog.present();
    entry.grab_focus();
}

fn confirm_action<F>(
    parent: Option<&gtk::Window>,
    title: &str,
    message: &str,
    accept_label: &str,
    on_confirm: F,
) where
    F: FnOnce() + 'static,
{
    let dialog = gtk::Window::builder()
        .title(title)
        .modal(true)
        .resizable(false)
        .default_width(360)
        .build();
    if let Some(parent) = parent {
        dialog.set_transient_for(Some(parent));
    }
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(
        &gtk::Label::builder()
            .label(message)
            .wrap(true)
            .max_width_chars(48)
            .xalign(0.0)
            .build(),
    );
    let actions = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::End)
        .spacing(6)
        .build();
    let cancel_btn = gtk::Button::with_label("Cancel");
    let accept_btn = gtk::Button::with_label(accept_label);
    actions.append(&cancel_btn);
    actions.append(&accept_btn);
    content.append(&actions);
    dialog.set_child(Some(&content));

    let on_confirm = Rc::new(RefCell::new(Some(on_confirm)));
    let dialog_for_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_for_cancel.close();
    });

    let dialog_for_accept = dialog.clone();
    accept_btn.connect_clicked(move |_| {
        if let Some(callback) = on_confirm.borrow_mut().take() {
            callback();
        }
        dialog_for_accept.close();
    });

    dialog.present();
}

impl ContextPanel {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_name("context-panel")
            .width_request(280)
            .build();

        let header_label = gtk::Label::builder()
            .label("Notes")
            .css_classes(vec!["title-4".to_string()])
            .margin_top(12)
            .margin_bottom(8)
            .margin_start(16)
            .margin_end(16)
            .xalign(0.0)
            .build();

        let stack = gtk::Stack::builder().vexpand(true).build();

        let notes_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .build();

        let action_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(4)
            .build();

        let new_note_btn = gtk::Button::builder()
            .label("New Note")
            .icon_name("document-new-symbolic")
            .build();
        let new_folder_btn = gtk::Button::builder()
            .label("New Folder")
            .icon_name("folder-new-symbolic")
            .build();
        let rename_btn = gtk::Button::builder()
            .label("Rename")
            .sensitive(false)
            .build();
        let delete_btn = gtk::Button::builder()
            .label("Delete")
            .sensitive(false)
            .build();

        action_row.append(&new_note_btn);
        action_row.append(&new_folder_btn);
        action_row.append(&rename_btn);
        action_row.append(&delete_btn);

        let status_label = gtk::Label::builder()
            .label("")
            .xalign(0.0)
            .wrap(true)
            .margin_start(12)
            .margin_end(12)
            .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
            .visible(false)
            .build();

        let explorer = Rc::new(ExplorerView::new(Rc::clone(&client)));

        notes_view.append(&action_row);
        notes_view.append(&status_label);
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

        let graph_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .build();
        graph_view.append(
            &gtk::Label::builder()
                .label("Graph controls will appear here")
                .build(),
        );

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

        stack.add_titled(&notes_view, Some("notes"), "Notes");
        stack.add_titled(&search_view, Some("search"), "Search");
        stack.add_titled(&graph_view, Some("graph"), "Graph");
        stack.add_titled(&settings_view, Some("settings"), "Settings");

        widget.append(&header_label);
        widget.append(&stack);

        let on_note_selected: NoteSelectedCallback = Rc::new(RefCell::new(None));
        let on_selection_cleared: SelectionClearedCallback = Rc::new(RefCell::new(None));

        let on_note_selected_clone = Rc::clone(&on_note_selected);
        explorer.connect_note_selected(move |path| {
            if let Some(ref cb) = *on_note_selected_clone.borrow() {
                cb(path);
            }
        });

        let on_selection_cleared_clone = Rc::clone(&on_selection_cleared);
        explorer.connect_selection_cleared(move || {
            if let Some(ref cb) = *on_selection_cleared_clone.borrow() {
                cb();
            }
        });

        let status_label_for_status = status_label.clone();
        explorer.connect_status_changed(move |message, is_error| {
            show_status(&status_label_for_status, message, is_error);
        });

        let rename_btn_for_selection = rename_btn.clone();
        let delete_btn_for_selection = delete_btn.clone();
        explorer.connect_selection_changed(move |selection| {
            let enabled = selection.is_some();
            rename_btn_for_selection.set_sensitive(enabled);
            delete_btn_for_selection.set_sensitive(enabled);
        });

        let panel = Self {
            widget,
            header_label,
            mode: RefCell::new(ToolMode::Notes),
            stack,
            explorer,
            status_label,
            rename_btn,
            delete_btn,
            on_note_selected,
            on_selection_cleared,
        };

        panel.wire_note_actions(new_note_btn, new_folder_btn);
        panel.wire_selection_actions();
        panel.explorer.refresh();

        panel
    }

    fn wire_note_actions(&self, new_note_btn: gtk::Button, new_folder_btn: gtk::Button) {
        let panel_widget = self.widget.clone().upcast::<gtk::Widget>();
        let status_label = self.status_label.clone();
        new_note_btn.connect_clicked({
            let panel_widget = panel_widget.clone();
            let explorer = Rc::clone(&self.explorer);
            let status_label = status_label.clone();
            move |_| {
                let parent = root_window(&panel_widget);
                prompt_text(
                    parent.as_ref(),
                    "Create note",
                    "Create",
                    "Name or path",
                    None,
                    {
                        let status_label = status_label.clone();
                        let explorer = Rc::clone(&explorer);
                        move |name| {
                            if let Err(error) = explorer.create_note_in_selected_container(&name) {
                                show_status(&status_label, &error, true);
                            }
                        }
                    },
                );
            }
        });

        new_folder_btn.connect_clicked({
            let explorer = Rc::clone(&self.explorer);
            let status_label = status_label.clone();
            move |_| {
                let parent = root_window(&panel_widget);
                prompt_text(
                    parent.as_ref(),
                    "Create folder",
                    "Create",
                    "Folder name",
                    None,
                    {
                        let status_label = status_label.clone();
                        let explorer = Rc::clone(&explorer);
                        move |name| {
                            if let Err(error) =
                                explorer.create_directory_in_selected_container(&name)
                            {
                                show_status(&status_label, &error, true);
                            }
                        }
                    },
                );
            }
        });
    }

    fn wire_selection_actions(&self) {
        let panel_widget = self.widget.clone().upcast::<gtk::Widget>();
        let status_label = self.status_label.clone();
        self.rename_btn.connect_clicked({
            let explorer = Rc::clone(&self.explorer);
            let status_label = status_label.clone();
            move |_| {
                let selection = explorer.selected_item();
                let initial_text = selection.as_ref().map(selected_display_name);
                let parent = root_window(&panel_widget);
                prompt_text(
                    parent.as_ref(),
                    "Rename item",
                    "Rename",
                    "New name or path",
                    initial_text.as_deref(),
                    {
                        let explorer = Rc::clone(&explorer);
                        let status_label = status_label.clone();
                        move |name| {
                            if let Err(error) = explorer.rename_selected(&name) {
                                show_status(&status_label, &error, true);
                            }
                        }
                    },
                );
            }
        });

        let panel_widget = self.widget.clone().upcast::<gtk::Widget>();
        let status_label = self.status_label.clone();
        self.delete_btn.connect_clicked({
            let explorer = Rc::clone(&self.explorer);
            let status_label = status_label.clone();
            move |_| {
                let Some(selection) = explorer.selected_item() else {
                    return;
                };
                let title = match selection {
                    ExplorerSelection::Note { .. } => "Delete note",
                    ExplorerSelection::Folder { .. } => "Remove folder",
                };
                let noun = match selection {
                    ExplorerSelection::Note { .. } => "note",
                    ExplorerSelection::Folder { .. } => "folder",
                };
                let message = format!(
                    "Delete the selected {} \"{}\"?",
                    noun,
                    selected_display_name(&selection)
                );
                let parent = root_window(&panel_widget);
                confirm_action(parent.as_ref(), title, &message, "Delete", {
                    let explorer = Rc::clone(&explorer);
                    let status_label = status_label.clone();
                    move || {
                        if let Err(error) = explorer.delete_selected() {
                            show_status(&status_label, &error, true);
                        }
                    }
                });
            }
        });
    }

    pub fn set_mode(&self, mode: ToolMode) {
        *self.mode.borrow_mut() = mode;

        let (label, visible_child) = match mode {
            ToolMode::Notes => ("Notes", "notes"),
            ToolMode::Search => ("Search", "search"),
            ToolMode::Graph => ("Graph", "graph"),
            ToolMode::Settings => ("Settings", "settings"),
        };

        self.header_label.set_label(label);
        self.stack.set_visible_child_name(visible_child);

        if matches!(mode, ToolMode::Notes) {
            self.explorer.refresh();
        }
    }

    pub fn connect_note_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.on_note_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_selection_cleared<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        *self.on_selection_cleared.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_note_switch_guard<F>(&self, f: F)
    where
        F: Fn(&str) -> NoteSwitchDecision + 'static,
    {
        self.explorer.connect_note_switch_guard(f);
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}
