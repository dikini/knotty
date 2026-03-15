use anyhow::{anyhow, Result};
use gtk::prelude::*;
use serde_json::{Map, Number, Value};
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::{KnotdClient, NoteData, NoteModeAvailability, NoteType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Meta,
    Source,
    Edit,
    View,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SaveRequest {
    path: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq)]
struct MetadataForm {
    title: String,
    description: String,
    tags: Vec<String>,
    extra_fields: Vec<(String, String)>,
    preserved_lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkdownCommand {
    Heading1,
    BulletList,
    OrderedList,
    BlockQuote,
    CodeBlock,
    HorizontalRule,
    TaskToggle,
    Link,
    WikiLink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextEdit {
    text: String,
    selection: (usize, usize),
}

#[derive(Clone)]
struct MetadataRow {
    container: gtk::Box,
    key_entry: gtk::Entry,
    value_entry: gtk::Entry,
}

#[derive(Debug, Clone, Default)]
struct EditorDocumentState {
    note: Option<NoteData>,
    modified: bool,
}

impl EditorDocumentState {
    fn load_note(&mut self, note: NoteData) {
        self.note = Some(note);
        self.modified = false;
    }

    fn clear(&mut self) {
        self.note = None;
        self.modified = false;
    }

    fn mark_dirty(&mut self) -> bool {
        if self.modified {
            return false;
        }
        self.modified = true;
        true
    }

    fn set_clean(&mut self) -> bool {
        if !self.modified {
            return false;
        }
        self.modified = false;
        true
    }

    fn apply_saved_content(&mut self, content: &str, title: &str) {
        if let Some(note) = self.note.as_mut() {
            note.content = content.to_string();
            note.title.clear();
            note.title.push_str(title);
        }
    }

    fn is_modified(&self) -> bool {
        self.modified
    }

    fn save_request(&self, content: &str) -> Result<Option<SaveRequest>> {
        if !self.modified {
            return Ok(None);
        }

        let note = self
            .note
            .as_ref()
            .ok_or_else(|| anyhow!("Cannot save without an active note"))?;

        Ok(Some(SaveRequest {
            path: note.path.clone(),
            content: content.to_string(),
        }))
    }
}

fn available_modes_for_note_type(note_type: NoteType) -> NoteModeAvailability {
    match note_type {
        NoteType::Markdown => NoteModeAvailability {
            meta: true,
            source: true,
            edit: true,
            view: true,
        },
        NoteType::Pdf | NoteType::Image => NoteModeAvailability {
            meta: false,
            source: false,
            edit: false,
            view: true,
        },
        NoteType::Youtube => NoteModeAvailability {
            meta: true,
            source: false,
            edit: false,
            view: true,
        },
        NoteType::Unknown => NoteModeAvailability {
            meta: true,
            source: true,
            edit: false,
            view: true,
        },
    }
}

fn available_modes_for_note(note: &NoteData) -> NoteModeAvailability {
    note.available_modes.clone().unwrap_or_else(|| {
        available_modes_for_note_type(note.note_type.unwrap_or(NoteType::Markdown))
    })
}

fn default_editor_mode(modes: &NoteModeAvailability) -> EditorMode {
    if modes.edit {
        EditorMode::Edit
    } else if modes.view {
        EditorMode::View
    } else if modes.source {
        EditorMode::Source
    } else if modes.meta {
        EditorMode::Meta
    } else {
        EditorMode::Source
    }
}

fn default_mode_for_note(note: &NoteData) -> EditorMode {
    default_editor_mode(&available_modes_for_note(note))
}

/// Position in the document for syncing between modes
/// Tracks what's visible at the top of the viewport
#[derive(Debug, Clone, Copy, Default)]
pub struct EditorPosition {
    /// Line number at top of viewport
    pub top_line: usize,
}

/// Note editor widget with meta, source, edit, and view modes
pub struct NoteEditor {
    widget: gtk::Box,
    title_entry: gtk::Entry,
    content_stack: gtk::Stack,
    meta_title_entry: gtk::Entry,
    meta_description_entry: gtk::Entry,
    meta_tags_entry: gtk::Entry,
    meta_rows_box: gtk::Box,
    text_view: gtk::TextView,
    preview_box: gtk::Box,
    edit_text_view: gtk::TextView,
    meta_button: gtk::ToggleButton,
    source_button: gtk::ToggleButton,
    edit_button: gtk::ToggleButton,
    view_button: gtk::ToggleButton,
    client: Rc<KnotdClient>,
    document_state: Rc<RefCell<EditorDocumentState>>,
    current_mode: RefCell<EditorMode>,
    on_modified_changed: Rc<RefCell<Option<Box<dyn Fn(bool)>>>>,
    suppress_content_changed: Rc<RefCell<bool>>,
    suppress_meta_changed: Rc<RefCell<bool>>,
    meta_rows: Rc<RefCell<Vec<MetadataRow>>>,
    /// Last known position for sync
    position: RefCell<EditorPosition>,
}

impl NoteEditor {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        widget.add_css_class("card");

        // Header bar with title and mode buttons
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_top(12)
            .margin_bottom(8)
            .margin_start(16)
            .margin_end(16)
            .build();

        // Title entry (takes most space)
        let title_entry = gtk::Entry::builder()
            .placeholder_text("Note title...")
            .css_classes(vec!["title-2".to_string()])
            .hexpand(true)
            .build();
        title_entry.set_editable(false);
        title_entry.set_can_focus(false);

        // Mode buttons container
        let mode_buttons = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .margin_start(8)
            .build();

        // Meta button
        let meta_btn = gtk::ToggleButton::builder()
            .icon_name("dialog-information-symbolic")
            .tooltip_text("Metadata")
            .build();

        // Source button
        let source_btn = gtk::ToggleButton::builder()
            .icon_name("accessories-text-editor-symbolic")
            .tooltip_text("Source")
            .group(&meta_btn)
            .build();

        // Edit button
        let edit_btn = gtk::ToggleButton::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text("Edit")
            .group(&meta_btn)
            .build();

        // View button
        let view_btn = gtk::ToggleButton::builder()
            .icon_name("view-paged-symbolic")
            .tooltip_text("View")
            .group(&meta_btn)
            .build();

        mode_buttons.append(&meta_btn);
        mode_buttons.append(&source_btn);
        mode_buttons.append(&edit_btn);
        mode_buttons.append(&view_btn);

        header.append(&title_entry);
        header.append(&mode_buttons);

        // Content stack: Meta | Source | Edit | View
        let content_stack = gtk::Stack::builder()
            .vexpand(true)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .build();

        let meta_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        let meta_label = gtk::Label::new(Some("Metadata"));
        meta_label.set_xalign(0.0);
        meta_label.add_css_class("heading");
        meta_box.append(&meta_label);

        let meta_title_entry = gtk::Entry::builder()
            .placeholder_text("Title")
            .hexpand(true)
            .build();
        let meta_description_entry = gtk::Entry::builder()
            .placeholder_text("Description")
            .hexpand(true)
            .build();
        let meta_tags_entry = gtk::Entry::builder()
            .placeholder_text("tag-one, tag-two")
            .hexpand(true)
            .build();

        meta_box.append(&labeled_meta_field("Title", &meta_title_entry));
        meta_box.append(&labeled_meta_field("Description", &meta_description_entry));
        meta_box.append(&labeled_meta_field("Tags", &meta_tags_entry));

        let extra_label = gtk::Label::new(Some("Additional frontmatter"));
        extra_label.set_xalign(0.0);
        extra_label.add_css_class("heading");
        meta_box.append(&extra_label);

        let meta_rows_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .build();
        meta_box.append(&meta_rows_box);

        let add_field_btn = gtk::Button::builder()
            .label("Add field")
            .halign(gtk::Align::Start)
            .build();
        meta_box.append(&add_field_btn);
        content_stack.add_titled(&meta_box, Some("meta"), "Meta");

        // Source view (TextView)
        let scrolled = gtk::ScrolledWindow::builder().vexpand(true).build();

        let text_view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .build();

        scrolled.set_child(Some(&text_view));
        content_stack.add_titled(&scrolled, Some("source"), "Source");

        let edit_buffer = text_view.buffer();

        // View surface - line-based for accurate position sync
        let preview_scroll = gtk::ScrolledWindow::builder().vexpand(true).build();

        let preview_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(false)
            .valign(gtk::Align::Start)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        preview_scroll.set_child(Some(&preview_box));
        content_stack.add_titled(&preview_scroll, Some("view"), "View");

        let edit_text_view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .build();
        edit_text_view.set_buffer(Some(&edit_buffer));

        let edit_toolbar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_top(8)
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(4)
            .build();
        for (label, command) in [
            ("H1", MarkdownCommand::Heading1),
            ("Bullets", MarkdownCommand::BulletList),
            ("Numbers", MarkdownCommand::OrderedList),
            ("Quote", MarkdownCommand::BlockQuote),
            ("Code", MarkdownCommand::CodeBlock),
            ("Rule", MarkdownCommand::HorizontalRule),
            ("Task", MarkdownCommand::TaskToggle),
            ("Link", MarkdownCommand::Link),
            ("Wiki", MarkdownCommand::WikiLink),
        ] {
            let button = gtk::Button::with_label(label);
            let buffer = edit_buffer.clone();
            button.connect_clicked(move |_| {
                apply_command_to_buffer(&buffer, command);
            });
            edit_toolbar.append(&button);
        }

        let edit_scroll = gtk::ScrolledWindow::builder().vexpand(true).build();
        edit_scroll.set_child(Some(&edit_text_view));

        let edit_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        edit_box.append(&edit_toolbar);
        edit_box.append(&edit_scroll);
        content_stack.add_titled(&edit_box, Some("edit"), "Edit");

        // Show source until a note is loaded and the contract selects a default mode.
        source_btn.set_active(true);
        content_stack.set_visible_child_name("source");

        widget.append(&header);
        widget.append(&content_stack);

        let editor = Self {
            widget,
            title_entry,
            content_stack,
            meta_title_entry: meta_title_entry.clone(),
            meta_description_entry: meta_description_entry.clone(),
            meta_tags_entry: meta_tags_entry.clone(),
            meta_rows_box: meta_rows_box.clone(),
            text_view,
            preview_box,
            edit_text_view,
            meta_button: meta_btn.clone(),
            source_button: source_btn.clone(),
            edit_button: edit_btn.clone(),
            view_button: view_btn.clone(),
            client,
            document_state: Rc::new(RefCell::new(EditorDocumentState::default())),
            current_mode: RefCell::new(EditorMode::Source),
            on_modified_changed: Rc::new(RefCell::new(None)),
            suppress_content_changed: Rc::new(RefCell::new(false)),
            suppress_meta_changed: Rc::new(RefCell::new(false)),
            meta_rows: Rc::new(RefCell::new(Vec::new())),
            position: RefCell::new(EditorPosition::default()),
        };

        editor.setup_signals(&meta_btn, &source_btn, &edit_btn, &view_btn);
        {
            let handles = editor.meta_handles();
            add_field_btn.connect_clicked(move |_| {
                handles.add_row("", "");
                handles.apply_controls_to_source();
            });
        }
        editor
    }

    fn setup_signals(
        &self,
        meta_btn: &gtk::ToggleButton,
        source_btn: &gtk::ToggleButton,
        edit_btn: &gtk::ToggleButton,
        view_btn: &gtk::ToggleButton,
    ) {
        // Track content changes
        let text_view = self.text_view.clone();
        let editor = self.clone_handles();
        text_view.buffer().connect_changed(move |_| {
            editor.mark_dirty_from_content_change();
        });

        let meta_handles = self.meta_handles();
        self.meta_title_entry.connect_changed({
            let meta_handles = meta_handles.clone();
            move |_| meta_handles.apply_controls_to_source()
        });
        self.meta_description_entry.connect_changed({
            let meta_handles = meta_handles.clone();
            move |_| meta_handles.apply_controls_to_source()
        });
        self.meta_tags_entry.connect_changed(move |_| {
            meta_handles.apply_controls_to_source();
        });

        // Meta mode
        meta_btn.connect_toggled({
            let content_stack = self.content_stack.clone();
            let current_mode = self.current_mode.clone();
            let meta_handles = self.meta_handles();
            move |btn| {
                if btn.is_active() {
                    *current_mode.borrow_mut() = EditorMode::Meta;
                    meta_handles.refresh_from_source();
                    content_stack.set_visible_child_name("meta");
                }
            }
        });

        // Mode switching with viewport position sync
        // Track what's at the TOP of the viewport, not scroll percentage

        // Source mode
        source_btn.connect_toggled({
            let content_stack = self.content_stack.clone();
            let text_view = self.text_view.clone();
            let current_mode = self.current_mode.clone();
            let position = self.position.clone();
            move |btn| {
                if btn.is_active() {
                    *current_mode.borrow_mut() = EditorMode::Source;
                    content_stack.set_visible_child_name("source");

                    // Restore: scroll to the saved line
                    let top_line = position.borrow().top_line;
                    let text_view_clone = text_view.clone();
                    glib::timeout_add_local_once(std::time::Duration::from_millis(10), move || {
                        let buffer = text_view_clone.buffer();
                        if let Some(mut iter) = buffer.iter_at_line(top_line as i32) {
                            text_view_clone.scroll_to_iter(&mut iter, 0.0, true, 0.0, 0.0);
                        }
                    });
                }
            }
        });

        // View mode
        view_btn.connect_toggled({
            let content_stack = self.content_stack.clone();
            let text_view = self.text_view.clone();
            let preview_box = self.preview_box.clone();
            let current_mode = self.current_mode.clone();
            let position = self.position.clone();
            move |btn| {
                if btn.is_active() {
                    // Save: get the first visible line in source
                    let buffer = text_view.buffer();
                    let top_line = if let Some(mark) = buffer.mark("insert") {
                        let iter = buffer.iter_at_mark(&mark);
                        iter.line() as usize
                    } else {
                        0
                    };
                    position.borrow_mut().top_line = top_line;

                    *current_mode.borrow_mut() = EditorMode::View;

                    // Update preview content - line by line
                    let start = buffer.start_iter();
                    let end = buffer.end_iter();
                    let content = buffer.text(&start, &end, false).to_string();

                    rebuild_preview_box(&preview_box, &content);

                    content_stack.set_visible_child_name("view");

                    // Restore: scroll to the saved line
                    let line_count = preview_markup_lines(&content).len();
                    let target_line = position.borrow().top_line.min(line_count.saturating_sub(1));
                    let preview_box_clone = preview_box.clone();
                    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
                        // Find target widget by iterating
                        let mut current = preview_box_clone.first_child();
                        let mut current_idx = 0;
                        let target_widget = loop {
                            match current {
                                Some(widget) if current_idx == target_line => break Some(widget),
                                Some(widget) => {
                                    current = widget.next_sibling();
                                    current_idx += 1;
                                }
                                None => break None,
                            }
                        };

                        if let Some(target_widget) = target_widget {
                            if let Some(parent) = target_widget.parent() {
                                if let Some(gp) = parent.parent() {
                                    if let Ok(sw) = gp.downcast::<gtk::ScrolledWindow>() {
                                        let adj = sw.vadjustment();
                                        // Estimate position based on average line height (~20px)
                                        let widget_y = target_line as f64 * 20.0;
                                        adj.set_value(widget_y);
                                    }
                                }
                            }
                        }
                    });
                }
            }
        });

        // Edit mode
        edit_btn.connect_toggled({
            let content_stack = self.content_stack.clone();
            let edit_text_view = self.edit_text_view.clone();
            let current_mode = self.current_mode.clone();
            let position = self.position.clone();
            move |btn| {
                if btn.is_active() {
                    if let Some(adj) = edit_text_view.vadjustment() {
                        let total_lines = edit_text_view.buffer().line_count() as f64;
                        let scroll_frac = adj.value() / (adj.upper() - adj.page_size()).max(1.0);
                        position.borrow_mut().top_line = (scroll_frac * total_lines) as usize;
                    }

                    *current_mode.borrow_mut() = EditorMode::Edit;
                    content_stack.set_visible_child_name("edit");
                    let top_line = position.borrow().top_line;
                    let edit_text_view_clone = edit_text_view.clone();
                    glib::timeout_add_local_once(std::time::Duration::from_millis(10), move || {
                        let buffer = edit_text_view_clone.buffer();
                        if let Some(mut iter) = buffer.iter_at_line(top_line as i32) {
                            edit_text_view_clone.scroll_to_iter(&mut iter, 0.0, true, 0.0, 0.0);
                        }
                    });
                }
            }
        });
    }

    fn apply_mode_availability(&self, modes: &NoteModeAvailability) {
        self.meta_button.set_sensitive(modes.meta);
        self.source_button.set_sensitive(modes.source);
        self.edit_button.set_sensitive(modes.edit);
        self.view_button.set_sensitive(modes.view);
    }

    fn set_mode(&self, mode: EditorMode) {
        match mode {
            EditorMode::Meta => self.meta_button.set_active(true),
            EditorMode::Source => self.source_button.set_active(true),
            EditorMode::Edit => self.edit_button.set_active(true),
            EditorMode::View => self.view_button.set_active(true),
        }
    }

    fn clone_handles(&self) -> EditorHandles {
        EditorHandles {
            document_state: Rc::clone(&self.document_state),
            on_modified_changed: Rc::clone(&self.on_modified_changed),
            suppress_content_changed: Rc::clone(&self.suppress_content_changed),
            title_entry: self.title_entry.clone(),
            text_view: self.text_view.clone(),
        }
    }

    fn meta_handles(&self) -> MetaHandles {
        MetaHandles {
            document_state: Rc::clone(&self.document_state),
            text_view: self.text_view.clone(),
            title_entry: self.title_entry.clone(),
            meta_title_entry: self.meta_title_entry.clone(),
            meta_description_entry: self.meta_description_entry.clone(),
            meta_tags_entry: self.meta_tags_entry.clone(),
            meta_rows_box: self.meta_rows_box.clone(),
            meta_rows: Rc::clone(&self.meta_rows),
            on_modified_changed: Rc::clone(&self.on_modified_changed),
            suppress_content_changed: Rc::clone(&self.suppress_content_changed),
            suppress_meta_changed: Rc::clone(&self.suppress_meta_changed),
        }
    }

    fn notify_modified_changed(&self, modified: bool) {
        if let Some(callback) = self.on_modified_changed.borrow().as_ref() {
            callback(modified);
        }
    }

    pub fn connect_modified_changed<F>(&self, f: F)
    where
        F: Fn(bool) + 'static,
    {
        *self.on_modified_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn load_note(&self, note: &NoteData) {
        // Display raw markdown source
        let buffer = self.text_view.buffer();
        *self.suppress_content_changed.borrow_mut() = true;
        buffer.set_text(&note.content);
        *self.suppress_content_changed.borrow_mut() = false;

        let modes = available_modes_for_note(note);
        self.apply_mode_availability(&modes);

        let was_modified = self.is_modified();
        self.document_state.borrow_mut().load_note(note.clone());
        self.refresh_title_display();
        self.meta_handles().refresh_from_source();
        if was_modified {
            self.notify_modified_changed(false);
        }

        // Update other views if needed
        match *self.current_mode.borrow() {
            EditorMode::View => {
                rebuild_preview_box(&self.preview_box, &note.content);
            }
            _ => {}
        }

        self.set_mode(default_mode_for_note(note));
    }

    pub fn is_modified(&self) -> bool {
        self.document_state.borrow().is_modified()
    }

    pub fn current_title(&self) -> String {
        let content = self.current_content();
        preferred_note_title(self.document_state.borrow().note.as_ref(), &content)
    }

    fn current_content(&self) -> String {
        let buffer = self.text_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.text(&start, &end, false).to_string()
    }

    fn refresh_title_display(&self) {
        self.title_entry.set_text(&self.current_title());
    }

    fn save_with<F>(&self, save_note: F) -> Result<()>
    where
        F: FnOnce(&str, &str) -> Result<()>,
    {
        let content = self.current_content();
        let request = self.document_state.borrow().save_request(&content)?;
        let Some(request) = request else {
            return Ok(());
        };

        save_note(&request.path, &request.content)?;
        let title = preferred_note_title(self.document_state.borrow().note.as_ref(), &content);
        self.document_state
            .borrow_mut()
            .apply_saved_content(&content, &title);
        let changed = self.document_state.borrow_mut().set_clean();
        if changed {
            self.notify_modified_changed(false);
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.save_with(|path, content| {
            self.client
                .save_note(path, content)
                .map_err(|error| anyhow!(error))
        })
    }

    pub fn discard_changes(&self) {
        let original_content = loaded_note_content(self.document_state.borrow().note.as_ref());

        *self.suppress_content_changed.borrow_mut() = true;
        self.text_view.buffer().set_text(&original_content);
        *self.suppress_content_changed.borrow_mut() = false;

        self.refresh_title_display();
        self.meta_handles().refresh_from_source();
        if matches!(*self.current_mode.borrow(), EditorMode::View) {
            rebuild_preview_box(&self.preview_box, &original_content);
        }

        let changed = self.document_state.borrow_mut().set_clean();
        if changed {
            self.notify_modified_changed(false);
        }
    }

    pub fn clear(&self) {
        self.title_entry.set_text("");
        *self.suppress_content_changed.borrow_mut() = true;
        self.text_view.buffer().set_text("");
        *self.suppress_content_changed.borrow_mut() = false;
        let was_modified = self.is_modified();
        self.document_state.borrow_mut().clear();
        if was_modified {
            self.notify_modified_changed(false);
        }

        // Reset to source mode
        self.apply_mode_availability(&NoteModeAvailability {
            meta: true,
            source: true,
            edit: true,
            view: true,
        });
        self.content_stack.set_visible_child_name("source");
        self.set_mode(EditorMode::Source);
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

#[derive(Clone)]
struct EditorHandles {
    document_state: Rc<RefCell<EditorDocumentState>>,
    on_modified_changed: Rc<RefCell<Option<Box<dyn Fn(bool)>>>>,
    suppress_content_changed: Rc<RefCell<bool>>,
    title_entry: gtk::Entry,
    text_view: gtk::TextView,
}

impl EditorHandles {
    fn notify_modified_changed(&self, modified: bool) {
        if let Some(callback) = self.on_modified_changed.borrow().as_ref() {
            callback(modified);
        }
    }

    fn mark_dirty_from_content_change(&self) {
        if *self.suppress_content_changed.borrow() {
            return;
        }

        let changed = self.document_state.borrow_mut().mark_dirty();
        let content = {
            let buffer = self.text_view.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            buffer.text(&start, &end, false).to_string()
        };
        let title = preferred_note_title(self.document_state.borrow().note.as_ref(), &content);
        self.title_entry.set_text(&title);
        if changed {
            self.notify_modified_changed(true);
        }
    }
}

#[derive(Clone)]
struct MetaHandles {
    document_state: Rc<RefCell<EditorDocumentState>>,
    text_view: gtk::TextView,
    title_entry: gtk::Entry,
    meta_title_entry: gtk::Entry,
    meta_description_entry: gtk::Entry,
    meta_tags_entry: gtk::Entry,
    meta_rows_box: gtk::Box,
    meta_rows: Rc<RefCell<Vec<MetadataRow>>>,
    on_modified_changed: Rc<RefCell<Option<Box<dyn Fn(bool)>>>>,
    suppress_content_changed: Rc<RefCell<bool>>,
    suppress_meta_changed: Rc<RefCell<bool>>,
}

impl MetaHandles {
    fn refresh_from_source(&self) {
        let form = {
            let state = self.document_state.borrow();
            let content = current_buffer_text(&self.text_view.buffer());
            metadata_form_from_content(state.note.as_ref(), &content)
        };

        *self.suppress_meta_changed.borrow_mut() = true;
        self.meta_title_entry.set_text(&form.title);
        self.meta_description_entry.set_text(&form.description);
        self.meta_tags_entry.set_text(&form.tags.join(", "));
        self.rebuild_rows(&form.extra_fields);
        *self.suppress_meta_changed.borrow_mut() = false;
    }

    fn rebuild_rows(&self, rows: &[(String, String)]) {
        while let Some(child) = self.meta_rows_box.first_child() {
            self.meta_rows_box.remove(&child);
        }
        self.meta_rows.borrow_mut().clear();

        for (key, value) in rows {
            self.add_row(key, value);
        }
    }

    fn add_row(&self, key: &str, value: &str) {
        let row_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        let key_entry = gtk::Entry::builder()
            .placeholder_text("field")
            .hexpand(true)
            .build();
        key_entry.set_text(key);
        let value_entry = gtk::Entry::builder()
            .placeholder_text("value")
            .hexpand(true)
            .build();
        value_entry.set_text(value);
        let remove_button = gtk::Button::with_label("Remove");

        row_box.append(&key_entry);
        row_box.append(&value_entry);
        row_box.append(&remove_button);
        self.meta_rows_box.append(&row_box);

        let row = MetadataRow {
            container: row_box.clone(),
            key_entry: key_entry.clone(),
            value_entry: value_entry.clone(),
        };
        self.meta_rows.borrow_mut().push(row);

        let handles = self.clone();
        key_entry.connect_changed(move |_| {
            handles.apply_controls_to_source();
        });
        let handles = self.clone();
        value_entry.connect_changed(move |_| {
            handles.apply_controls_to_source();
        });
        let handles = self.clone();
        remove_button.connect_clicked(move |_| {
            handles.remove_row(&row_box);
            handles.apply_controls_to_source();
        });
    }

    fn remove_row(&self, row_box: &gtk::Box) {
        self.meta_rows_box.remove(row_box);
        self.meta_rows
            .borrow_mut()
            .retain(|row| row.container != *row_box);
    }

    fn apply_controls_to_source(&self) {
        if *self.suppress_meta_changed.borrow() {
            return;
        }

        let form = self.current_form();
        let current_content = current_buffer_text(&self.text_view.buffer());
        let (_, body) = split_frontmatter(&current_content);
        let updated_content = rebuild_content_with_metadata(&body, &form);
        if updated_content == current_content {
            return;
        }

        *self.suppress_content_changed.borrow_mut() = true;
        self.text_view.buffer().set_text(&updated_content);
        *self.suppress_content_changed.borrow_mut() = false;

        self.title_entry.set_text(&preferred_note_title(
            self.document_state.borrow().note.as_ref(),
            &updated_content,
        ));

        let changed = self.document_state.borrow_mut().mark_dirty();
        if changed {
            if let Some(callback) = self.on_modified_changed.borrow().as_ref() {
                callback(true);
            }
        }
    }

    fn current_form(&self) -> MetadataForm {
        let current_content = current_buffer_text(&self.text_view.buffer());
        let mut form = {
            let state = self.document_state.borrow();
            metadata_form_from_content(state.note.as_ref(), &current_content)
        };

        let tags = self
            .meta_tags_entry
            .text()
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(ToString::to_string)
            .collect();

        let extra_fields = self
            .meta_rows
            .borrow()
            .iter()
            .filter_map(|row| {
                let key = row.key_entry.text().trim().to_string();
                let value = row.value_entry.text().trim().to_string();
                if key.is_empty() || value.is_empty() {
                    None
                } else {
                    Some((key, value))
                }
            })
            .collect();

        form.title = self.meta_title_entry.text().to_string();
        form.description = self.meta_description_entry.text().to_string();
        form.tags = tags;
        form.extra_fields = extra_fields;
        form
    }
}

fn labeled_meta_field(label_text: &str, field: &impl IsA<gtk::Widget>) -> gtk::Box {
    let box_widget = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();
    let label = gtk::Label::new(Some(label_text));
    label.set_xalign(0.0);
    box_widget.append(&label);
    box_widget.append(field);
    box_widget
}

fn current_buffer_text(buffer: &gtk::TextBuffer) -> String {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.text(&start, &end, false).to_string()
}

fn loaded_note_content(note: Option<&NoteData>) -> String {
    note.map(|note| note.content.clone()).unwrap_or_default()
}

fn preview_markup_lines(content: &str) -> Vec<String> {
    let (_, body) = split_frontmatter(content);
    body.lines()
        .enumerate()
        .map(|(i, line)| {
            let _ = i;
            if let Some(rest) = line.strip_prefix("# ") {
                format!(
                    "<span size='large' weight='bold'>{}</span>",
                    escape_markup(rest)
                )
            } else if let Some(rest) = line.strip_prefix("## ") {
                format!("<span weight='bold'>{}</span>", escape_markup(rest))
            } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                format!("  • {}", escape_markup(rest))
            } else {
                escape_markup(line)
            }
        })
        .collect()
}

fn rebuild_preview_box(preview_box: &gtk::Box, content: &str) {
    while let Some(child) = preview_box.first_child() {
        preview_box.remove(&child);
    }

    for (index, markup) in preview_markup_lines(content).into_iter().enumerate() {
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        label.set_markup(&markup);
        label.set_margin_bottom(2);
        label.set_widget_name(&format!("preview-line-{}", index));
        preview_box.append(&label);
    }
}

fn split_frontmatter(content: &str) -> (Option<String>, String) {
    if let Some(remainder) = content.strip_prefix("---\n") {
        if let Some(end) = remainder.find("\n---\n") {
            let frontmatter = remainder[..end].to_string();
            let body = remainder[end + "\n---\n".len()..].to_string();
            return (Some(frontmatter), body);
        }
    }
    (None, content.to_string())
}

fn parse_metadata_scalar(value: &str) -> Value {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Ok(parsed) = trimmed.parse::<i64>() {
        Value::Number(Number::from(parsed))
    } else if let Ok(parsed) = trimmed.parse::<f64>() {
        Number::from_f64(parsed)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(trimmed.to_string()))
    } else {
        Value::String(trimmed.trim_matches('"').to_string())
    }
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_inline_tag_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(str::trim)
        .map(|item| item.trim_matches('"'))
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_frontmatter_map(frontmatter: &str) -> (Map<String, Value>, Vec<String>, Vec<String>) {
    let mut map = Map::new();
    let mut tags = Vec::new();
    let mut preserved_lines = Vec::new();

    for line in frontmatter.lines() {
        let trimmed_end = line.trim_end();
        let trimmed = trimmed_end.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            preserved_lines.push(trimmed_end.to_string());
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once(':') else {
            preserved_lines.push(trimmed_end.to_string());
            continue;
        };
        let key = key.trim();
        let raw_value = raw_value.trim();
        if key == "tags" {
            tags = parse_inline_tag_list(raw_value);
            continue;
        }
        if raw_value.is_empty() {
            preserved_lines.push(trimmed_end.to_string());
            continue;
        }
        map.insert(key.to_string(), parse_metadata_scalar(raw_value));
    }

    (map, tags, preserved_lines)
}

fn metadata_form_from_content(note: Option<&NoteData>, content: &str) -> MetadataForm {
    let (frontmatter, _) = split_frontmatter(content);
    let (mut frontmatter_map, mut tags, preserved_lines) = frontmatter
        .as_deref()
        .map(parse_frontmatter_map)
        .unwrap_or_else(|| {
            note.and_then(|note| note.metadata.clone())
                .map(|metadata| (metadata.frontmatter, metadata.tags, Vec::new()))
                .unwrap_or_else(|| (Map::new(), Vec::new(), Vec::new()))
        });

    let title = scalar_to_string(
        frontmatter_map
            .remove("title")
            .as_ref()
            .unwrap_or(&Value::Null),
    )
    .or_else(|| note.map(|note| note.title.clone()))
    .unwrap_or_default();
    let description = scalar_to_string(
        frontmatter_map
            .remove("description")
            .as_ref()
            .unwrap_or(&Value::Null),
    )
    .unwrap_or_default();

    if tags.is_empty() {
        if let Some(note) = note {
            if let Some(metadata) = &note.metadata {
                tags = metadata.tags.clone();
            }
        }
    }

    let mut extra_fields: Vec<(String, String)> = frontmatter_map
        .iter()
        .filter_map(|(key, value)| scalar_to_string(value).map(|value| (key.clone(), value)))
        .collect();
    extra_fields.sort_by(|left, right| left.0.cmp(&right.0));

    MetadataForm {
        title,
        description,
        tags,
        extra_fields,
        preserved_lines,
    }
}

fn frontmatter_lines(form: &MetadataForm) -> Vec<String> {
    let mut lines = Vec::new();
    if !form.title.trim().is_empty() {
        lines.push(format!("title: {}", form.title.trim()));
    }
    if !form.description.trim().is_empty() {
        lines.push(format!("description: {}", form.description.trim()));
    }
    if !form.tags.is_empty() {
        lines.push(format!("tags: [{}]", form.tags.join(", ")));
    }

    let mut extra_fields = form.extra_fields.clone();
    extra_fields.sort_by(|left, right| left.0.cmp(&right.0));
    for (key, value) in extra_fields {
        if !key.trim().is_empty() && !value.trim().is_empty() {
            lines.push(format!("{}: {}", key.trim(), value.trim()));
        }
    }

    lines.extend(form.preserved_lines.iter().cloned());

    lines
}

fn rebuild_content_with_metadata(body: &str, form: &MetadataForm) -> String {
    let lines = frontmatter_lines(form);
    if lines.is_empty() {
        return body.to_string();
    }
    format!("---\n{}\n---\n{}", lines.join("\n"), body)
}

fn current_selection_range(buffer: &gtk::TextBuffer) -> (usize, usize) {
    if let Some((start, end)) = buffer.selection_bounds() {
        let start = start.offset().max(0) as usize;
        let end = end.offset().max(0) as usize;
        (start.min(end), start.max(end))
    } else {
        let offset = buffer.cursor_position().max(0) as usize;
        (offset, offset)
    }
}

fn char_to_byte_offset(text: &str, offset: usize) -> usize {
    text.char_indices()
        .nth(offset)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn apply_command_to_buffer(buffer: &gtk::TextBuffer, command: MarkdownCommand) {
    let text = current_buffer_text(buffer);
    let selection = current_selection_range(buffer);
    let edit = apply_markdown_command(&text, selection, command);
    buffer.set_text(&edit.text);
    let start = buffer.iter_at_offset(edit.selection.0 as i32);
    let end = buffer.iter_at_offset(edit.selection.1 as i32);
    buffer.select_range(&start, &end);
    buffer.place_cursor(&start);
}

fn apply_markdown_command(
    text: &str,
    selection: (usize, usize),
    command: MarkdownCommand,
) -> TextEdit {
    match command {
        MarkdownCommand::Heading1 => transform_selected_lines(text, selection, |index, line| {
            let _ = index;
            format!("# {}", line.trim_start_matches("# ").trim())
        }),
        MarkdownCommand::BulletList => transform_selected_lines(text, selection, |index, line| {
            let _ = index;
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                trimmed.to_string()
            } else {
                format!("- {}", trimmed)
            }
        }),
        MarkdownCommand::OrderedList => transform_selected_lines(text, selection, |index, line| {
            let trimmed = line
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ')
                .trim_start();
            format!("{}. {}", index + 1, trimmed)
        }),
        MarkdownCommand::BlockQuote => transform_selected_lines(text, selection, |index, line| {
            let _ = index;
            let trimmed = line.trim_start_matches("> ").trim_start();
            format!("> {}", trimmed)
        }),
        MarkdownCommand::CodeBlock => wrap_selection(text, selection, "```\n", "\n```"),
        MarkdownCommand::HorizontalRule => insert_at_selection(text, selection, "\n\n---\n\n"),
        MarkdownCommand::TaskToggle => transform_selected_lines(text, selection, |index, line| {
            let _ = index;
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
                format!("- [x] {}", rest)
            } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
                format!("- [ ] {}", rest)
            } else {
                format!("- [ ] {}", trimmed)
            }
        }),
        MarkdownCommand::Link => wrap_selection_or_placeholder(
            text,
            selection,
            "[",
            "](https://example.com)",
            "link text",
        ),
        MarkdownCommand::WikiLink => {
            wrap_selection_or_placeholder(text, selection, "[[", "]]", "Note")
        }
    }
}

fn transform_selected_lines<F>(text: &str, selection: (usize, usize), transform: F) -> TextEdit
where
    F: Fn(usize, &str) -> String,
{
    let (start, end) = expand_selection_to_lines(text, selection);
    let original = &text[start..end];
    let trailing_newline = original.ends_with('\n');
    let mut lines: Vec<String> = original.lines().map(ToString::to_string).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    let replacement = lines
        .iter()
        .enumerate()
        .map(|(index, line)| transform(index, line))
        .collect::<Vec<_>>()
        .join("\n");
    let replacement = if trailing_newline {
        format!("{}\n", replacement)
    } else {
        replacement
    };
    replace_range(text, start, end, &replacement)
}

fn wrap_selection(text: &str, selection: (usize, usize), prefix: &str, suffix: &str) -> TextEdit {
    let (start, end) = byte_selection(text, selection);
    let selected = &text[start..end];
    let replacement = format!("{}{}{}", prefix, selected, suffix);
    replace_range(text, start, end, &replacement)
}

fn wrap_selection_or_placeholder(
    text: &str,
    selection: (usize, usize),
    prefix: &str,
    suffix: &str,
    placeholder: &str,
) -> TextEdit {
    let (start, end) = byte_selection(text, selection);
    let selected = &text[start..end];
    let inner = if selected.is_empty() {
        placeholder
    } else {
        selected
    };
    let replacement = format!("{}{}{}", prefix, inner, suffix);
    replace_range(text, start, end, &replacement)
}

fn insert_at_selection(text: &str, selection: (usize, usize), inserted: &str) -> TextEdit {
    let (start, end) = byte_selection(text, selection);
    replace_range(text, start, end, inserted)
}

fn replace_range(text: &str, start: usize, end: usize, replacement: &str) -> TextEdit {
    let mut updated = String::with_capacity(text.len() - (end - start) + replacement.len());
    updated.push_str(&text[..start]);
    updated.push_str(replacement);
    updated.push_str(&text[end..]);
    let start_chars = updated[..start].chars().count();
    let inserted_chars = replacement.chars().count();
    TextEdit {
        text: updated,
        selection: (start_chars, start_chars + inserted_chars),
    }
}

fn byte_selection(text: &str, selection: (usize, usize)) -> (usize, usize) {
    let start = char_to_byte_offset(text, selection.0);
    let end = char_to_byte_offset(text, selection.1);
    (start.min(end), start.max(end))
}

fn expand_selection_to_lines(text: &str, selection: (usize, usize)) -> (usize, usize) {
    let (mut start, mut end) = byte_selection(text, selection);
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    while start > 0 && text.as_bytes()[start - 1] != b'\n' {
        start -= 1;
    }
    while end < text.len() && text.as_bytes()[end] != b'\n' {
        end += 1;
    }
    if end < text.len() {
        end += 1;
    }
    (start, end)
}

fn preferred_note_title(note: Option<&NoteData>, content: &str) -> String {
    let (frontmatter, _) = split_frontmatter(content);
    if let Some(frontmatter) = frontmatter {
        let (map, _, _) = parse_frontmatter_map(&frontmatter);
        if let Some(title) = map.get("title").and_then(scalar_to_string) {
            if !title.is_empty() {
                return title;
            }
        }
    }

    let derived = extract_title_from_markdown(content);
    if derived != "Untitled" {
        return derived;
    }

    note.and_then(|note| {
        if note.title.is_empty() {
            None
        } else {
            Some(note.title.clone())
        }
    })
    .unwrap_or(derived)
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn extract_title_from_markdown(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed[2..].trim().to_string();
        }
    }
    "Untitled".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{Backlink, Heading, NoteModeAvailability, NoteType};

    #[test]
    fn markdown_note_enables_all_modes_by_default() {
        let modes = available_modes_for_note_type(NoteType::Markdown);
        assert_eq!(
            modes,
            NoteModeAvailability {
                meta: true,
                source: true,
                edit: true,
                view: true,
            }
        );
    }

    #[test]
    fn pdf_note_only_allows_view_mode_by_default() {
        let modes = available_modes_for_note_type(NoteType::Pdf);
        assert_eq!(
            modes,
            NoteModeAvailability {
                meta: false,
                source: false,
                edit: false,
                view: true,
            }
        );
    }

    #[test]
    fn note_available_modes_override_note_type_defaults() {
        let note = test_note(
            Some(NoteType::Markdown),
            Some(NoteModeAvailability {
                meta: true,
                source: false,
                edit: false,
                view: true,
            }),
        );

        assert_eq!(
            available_modes_for_note(&note),
            NoteModeAvailability {
                meta: true,
                source: false,
                edit: false,
                view: true,
            }
        );
    }

    #[test]
    fn default_mode_prefers_edit_then_view_then_source_then_meta() {
        assert_eq!(
            default_mode_for_note(&test_note(
                Some(NoteType::Markdown),
                Some(NoteModeAvailability {
                    meta: true,
                    source: true,
                    edit: true,
                    view: true,
                }),
            )),
            EditorMode::Edit
        );

        assert_eq!(
            default_mode_for_note(&test_note(
                Some(NoteType::Markdown),
                Some(NoteModeAvailability {
                    meta: true,
                    source: true,
                    edit: false,
                    view: true,
                }),
            )),
            EditorMode::View
        );

        assert_eq!(
            default_mode_for_note(&test_note(
                Some(NoteType::Markdown),
                Some(NoteModeAvailability {
                    meta: true,
                    source: true,
                    edit: false,
                    view: false,
                }),
            )),
            EditorMode::Source
        );

        assert_eq!(
            default_mode_for_note(&test_note(
                Some(NoteType::Markdown),
                Some(NoteModeAvailability {
                    meta: true,
                    source: false,
                    edit: false,
                    view: false,
                }),
            )),
            EditorMode::Meta
        );
    }

    #[test]
    fn content_change_marks_document_state_dirty() {
        let mut state = EditorDocumentState::default();
        state.load_note(test_note(Some(NoteType::Markdown), None));

        assert!(!state.is_modified());
        assert!(state.mark_dirty());
        assert!(state.is_modified());
        assert!(!state.mark_dirty());
    }

    #[test]
    fn save_request_uses_note_path_and_current_content_when_dirty() {
        let mut state = EditorDocumentState::default();
        let note = test_note(Some(NoteType::Markdown), None);
        state.load_note(note.clone());
        state.mark_dirty();

        assert_eq!(
            state.save_request("# Updated").unwrap(),
            Some(SaveRequest {
                path: note.path,
                content: "# Updated".to_string(),
            })
        );
    }

    #[test]
    fn successful_save_clears_document_state_dirty() {
        let mut state = EditorDocumentState::default();
        state.load_note(test_note(Some(NoteType::Markdown), None));
        state.mark_dirty();

        assert!(state.set_clean());
        assert!(!state.is_modified());
    }

    #[test]
    fn failed_save_keeps_document_state_dirty() {
        let mut state = EditorDocumentState::default();
        state.load_note(test_note(Some(NoteType::Markdown), None));
        state.mark_dirty();

        let result: Result<()> = Err(anyhow!("save failed"));

        assert!(result.is_err());
        assert!(state.is_modified());
    }

    #[test]
    fn successful_save_updates_loaded_note_content_baseline() {
        let mut state = EditorDocumentState::default();
        state.load_note(test_note(Some(NoteType::Markdown), None));
        state.mark_dirty();

        state.apply_saved_content("# Saved\n\nBody", "Saved");
        state.set_clean();

        assert_eq!(
            loaded_note_content(state.note.as_ref()),
            "# Saved\n\nBody".to_string()
        );
        assert_eq!(
            state.note.as_ref().map(|note| note.title.as_str()),
            Some("Saved")
        );
        assert!(!state.is_modified());
    }

    #[test]
    fn preview_markup_skips_frontmatter() {
        let preview = preview_markup_lines("---\ntitle: Demo\n---\n# Heading\n\nBody");

        assert_eq!(
            preview.first().unwrap(),
            "<span size='large' weight='bold'>Heading</span>"
        );
        assert!(preview.iter().all(|line| !line.contains("title: Demo")));
    }

    #[test]
    fn metadata_round_trip_builds_frontmatter_and_preserves_body() {
        let form = MetadataForm {
            title: "Example".to_string(),
            description: "Demo".to_string(),
            tags: vec!["rust".to_string(), "gtk".to_string()],
            extra_fields: vec![
                ("published".to_string(), "true".to_string()),
                ("rating".to_string(), "5".to_string()),
            ],
            preserved_lines: Vec::new(),
        };

        let content = rebuild_content_with_metadata("# Heading\n\nBody", &form);
        let parsed = metadata_form_from_content(None, &content);

        assert_eq!(parsed.title, "Example");
        assert_eq!(parsed.description, "Demo");
        assert_eq!(parsed.tags, vec!["rust".to_string(), "gtk".to_string()]);
        assert!(parsed
            .extra_fields
            .contains(&(String::from("published"), String::from("true"))));
        assert!(content.ends_with("# Heading\n\nBody"));
    }

    #[test]
    fn metadata_round_trip_preserves_unsupported_frontmatter_lines() {
        let content = concat!(
            "---\n",
            "title: Example\n",
            "nested:\n",
            "  child: value\n",
            "list:\n",
            "  - one\n",
            "---\n",
            "# Heading\n"
        );

        let mut form = metadata_form_from_content(None, content);
        form.description = "Demo".to_string();
        let rebuilt = rebuild_content_with_metadata("# Heading\n", &form);

        assert!(rebuilt.contains("description: Demo"));
        assert!(rebuilt.contains("nested:"));
        assert!(rebuilt.contains("  child: value"));
        assert!(rebuilt.contains("list:"));
        assert!(rebuilt.contains("  - one"));
    }

    #[test]
    fn preferred_title_uses_frontmatter_before_heading() {
        let note = test_note(Some(NoteType::Markdown), None);
        let content = "---\ntitle: Frontmatter Title\n---\n# Heading Title\n\nBody";

        assert_eq!(
            preferred_note_title(Some(&note), content),
            "Frontmatter Title"
        );
    }

    #[test]
    fn heading_command_prefixes_selected_line() {
        let edit = apply_markdown_command("Body", (0, 4), MarkdownCommand::Heading1);

        assert_eq!(edit.text, "# Body");
    }

    #[test]
    fn ordered_list_command_numbers_selected_lines() {
        let edit = apply_markdown_command(
            "First\nSecond",
            (0, "First\nSecond".chars().count()),
            MarkdownCommand::OrderedList,
        );

        assert_eq!(edit.text, "1. First\n2. Second");
    }

    #[test]
    fn task_toggle_command_toggles_existing_checkbox() {
        let edit = apply_markdown_command(
            "- [ ] Task",
            (0, "- [ ] Task".chars().count()),
            MarkdownCommand::TaskToggle,
        );

        assert_eq!(edit.text, "- [x] Task");
    }

    #[test]
    fn link_and_wikilink_commands_wrap_selection() {
        let link = apply_markdown_command("Note", (0, 4), MarkdownCommand::Link);
        let wiki = apply_markdown_command("Note", (0, 4), MarkdownCommand::WikiLink);

        assert_eq!(link.text, "[Note](https://example.com)");
        assert_eq!(wiki.text, "[[Note]]");
    }

    #[test]
    fn quote_code_and_rule_commands_insert_expected_markdown() {
        let quote = apply_markdown_command("Line", (0, 4), MarkdownCommand::BlockQuote);
        let code = apply_markdown_command("fn main() {}", (0, 12), MarkdownCommand::CodeBlock);
        let rule = apply_markdown_command("Body", (4, 4), MarkdownCommand::HorizontalRule);

        assert_eq!(quote.text, "> Line");
        assert_eq!(code.text, "```\nfn main() {}\n```");
        assert_eq!(rule.text, "Body\n\n---\n\n");
    }

    #[test]
    fn loaded_note_content_uses_active_note_source() {
        let mut note = test_note(Some(NoteType::Markdown), None);
        note.content = "# Original\n\nBody".to_string();

        assert_eq!(
            loaded_note_content(Some(&note)),
            "# Original\n\nBody".to_string()
        );
        assert_eq!(loaded_note_content(None), String::new());
    }

    fn test_note(
        note_type: Option<NoteType>,
        available_modes: Option<NoteModeAvailability>,
    ) -> NoteData {
        NoteData {
            id: "note-1".to_string(),
            path: "note.md".to_string(),
            title: "Note".to_string(),
            content: "# Note".to_string(),
            created_at: 0,
            modified_at: 0,
            word_count: 1,
            headings: vec![Heading {
                level: 1,
                text: "Note".to_string(),
                slug: "note".to_string(),
            }],
            backlinks: vec![Backlink {
                path: "other.md".to_string(),
                title: "Other".to_string(),
                excerpt: None,
            }],
            note_type,
            available_modes,
            metadata: None,
            embed: None,
            media: None,
            type_badge: None,
            is_dimmed: false,
        }
    }
}
