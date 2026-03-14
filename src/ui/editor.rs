use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::{KnotdClient, NoteData};
use crate::ui::block_editor::{BlockEditor, DocumentStyle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorMode {
    Source,
    Preview,
    Blocks,
}

/// Position in the document for syncing between modes
/// Tracks what's visible at the top of the viewport
#[derive(Debug, Clone, Copy, Default)]
pub struct EditorPosition {
    /// Line number at top of viewport
    pub top_line: usize,
    /// Character offset within that line
    pub line_offset: usize,
    /// Approximate scroll fraction as fallback
    pub scroll_fraction: f64,
}

/// Note editor widget with Source, Preview, and Blocks modes
pub struct NoteEditor {
    widget: gtk::Box,
    title_entry: gtk::Entry,
    content_stack: gtk::Stack,
    text_view: gtk::TextView,
    preview_box: gtk::Box,
    block_editor: BlockEditor,
    mode_buttons: gtk::Box,
    client: Rc<KnotdClient>,
    current_note: RefCell<Option<NoteData>>,
    modified: RefCell<bool>,
    current_mode: RefCell<EditorMode>,
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

        // Mode buttons container
        let mode_buttons = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .margin_start(8)
            .build();

        // Source button
        let source_btn = gtk::ToggleButton::builder()
            .icon_name("accessories-text-editor-symbolic")
            .tooltip_text("Source")
            .active(true)
            .build();

        // Preview button
        let preview_btn = gtk::ToggleButton::builder()
            .icon_name("view-paged-symbolic")
            .tooltip_text("Preview")
            .group(&source_btn)
            .build();

        // Blocks button (Tufte-style)
        let blocks_btn = gtk::ToggleButton::builder()
            .icon_name("view-grid-symbolic")
            .tooltip_text("Blocks (Tufte)")
            .group(&source_btn)
            .build();

        mode_buttons.append(&source_btn);
        mode_buttons.append(&preview_btn);
        mode_buttons.append(&blocks_btn);

        header.append(&title_entry);
        header.append(&mode_buttons);

        // Content stack: Source | Preview | Blocks
        let content_stack = gtk::Stack::builder()
            .vexpand(true)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .build();

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

        // Preview view - line-based for accurate position sync
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
        content_stack.add_titled(&preview_scroll, Some("preview"), "Preview");

        // Blocks view (BlockEditor)
        let block_editor = BlockEditor::new();
        content_stack.add_titled(block_editor.widget(), Some("blocks"), "Blocks");

        // Show source by default
        content_stack.set_visible_child_name("source");

        widget.append(&header);
        widget.append(&content_stack);

        let editor = Self {
            widget,
            title_entry,
            content_stack,
            text_view,
            preview_box,
            block_editor,
            mode_buttons,
            client,
            current_note: RefCell::new(None),
            modified: RefCell::new(false),
            current_mode: RefCell::new(EditorMode::Source),
            position: RefCell::new(EditorPosition::default()),
        };

        editor.setup_signals(&source_btn, &preview_btn, &blocks_btn);
        editor
    }

    fn setup_signals(
        &self,
        source_btn: &gtk::ToggleButton,
        preview_btn: &gtk::ToggleButton,
        blocks_btn: &gtk::ToggleButton,
    ) {
        // Track title changes
        let modified = self.modified.clone();
        self.title_entry.connect_changed(move |_| {
            *modified.borrow_mut() = true;
        });

        // Track content changes
        let text_view = self.text_view.clone();
        let modified = self.modified.clone();
        text_view.buffer().connect_changed(move |_| {
            *modified.borrow_mut() = true;
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

        // Preview mode
        preview_btn.connect_toggled({
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

                    *current_mode.borrow_mut() = EditorMode::Preview;

                    // Update preview content - line by line
                    let start = buffer.start_iter();
                    let end = buffer.end_iter();
                    let content = buffer.text(&start, &end, false).to_string();

                    // Clear existing preview
                    while let Some(child) = preview_box.first_child() {
                        preview_box.remove(&child);
                    }

                    // Render each line as a separate label for accurate positioning
                    let lines: Vec<&str> = content.lines().collect();
                    for (i, line) in lines.iter().enumerate() {
                        let label = gtk::Label::new(None);
                        label.set_xalign(0.0);
                        label.set_wrap(true);
                        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);

                        // Simple formatting based on line content
                        let formatted = if line.starts_with("# ") {
                            format!(
                                "<span size='large' weight='bold'>{}</span>",
                                escape_markup(&line[2..])
                            )
                        } else if line.starts_with("## ") {
                            format!("<span weight='bold'>{}</span>", escape_markup(&line[3..]))
                        } else if line.starts_with("- ") || line.starts_with("* ") {
                            format!("  • {}", escape_markup(&line[2..]))
                        } else {
                            escape_markup(line)
                        };

                        label.set_markup(&formatted);
                        label.set_margin_bottom(2);

                        // Tag the label with its line number for scrolling
                        label.set_widget_name(&format!("preview-line-{}", i));

                        preview_box.append(&label);
                    }

                    content_stack.set_visible_child_name("preview");

                    // Restore: scroll to the saved line
                    let target_line = position
                        .borrow()
                        .top_line
                        .min(lines.len().saturating_sub(1));
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

        // Blocks mode
        blocks_btn.connect_toggled({
            let content_stack = self.content_stack.clone();
            let text_view = self.text_view.clone();
            let block_editor = self.block_editor.clone();
            let current_mode = self.current_mode.clone();
            let position = self.position.clone();
            move |btn| {
                if btn.is_active() {
                    // Save: get the first visible line in source
                    let buffer = text_view.buffer();
                    let mut iter = buffer.start_iter();
                    // Get visible region
                    if let Some(adj) = text_view.vadjustment() {
                        // Estimate line from scroll position
                        let total_lines = buffer.line_count() as f64;
                        let scroll_frac = adj.value() / (adj.upper() - adj.page_size()).max(1.0);
                        let estimated_line = (scroll_frac * total_lines) as usize;
                        position.borrow_mut().top_line = estimated_line;
                        position.borrow_mut().scroll_fraction = scroll_frac;
                    }

                    *current_mode.borrow_mut() = EditorMode::Blocks;

                    // Update block editor content
                    let start = buffer.start_iter();
                    let end = buffer.end_iter();
                    let content = buffer.text(&start, &end, false).to_string();
                    block_editor.set_markdown(&content);

                    content_stack.set_visible_child_name("blocks");

                    // Restore: scroll to estimated block position
                    // Rough heuristic: ~4 lines per block on average
                    let top_line = position.borrow().top_line;
                    let estimated_block = top_line / 4;
                    let block_editor_clone = block_editor.clone();
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(100),
                        move || {
                            block_editor_clone.scroll_to_block(estimated_block);
                        },
                    );
                }
            }
        });
    }

    pub fn load_note(&self, note: &NoteData) {
        let title = if note.title.is_empty() {
            extract_title_from_markdown(&note.content)
        } else {
            note.title.clone()
        };

        self.title_entry.set_text(&title);

        // Display raw markdown source
        let buffer = self.text_view.buffer();
        buffer.set_text(&note.content);

        *self.current_note.borrow_mut() = Some(note.clone());
        *self.modified.borrow_mut() = false;

        // Update other views if needed
        match *self.current_mode.borrow() {
            EditorMode::Preview => {
                // Clear and rebuild preview
                while let Some(child) = self.preview_box.first_child() {
                    self.preview_box.remove(&child);
                }

                for line in note.content.lines() {
                    let label = gtk::Label::new(None);
                    label.set_xalign(0.0);
                    label.set_wrap(true);
                    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);

                    let formatted = if line.starts_with("# ") {
                        format!(
                            "<span size='large' weight='bold'>{}</span>",
                            escape_markup(&line[2..])
                        )
                    } else if line.starts_with("## ") {
                        format!("<span weight='bold'>{}</span>", escape_markup(&line[3..]))
                    } else if line.starts_with("- ") || line.starts_with("* ") {
                        format!("  • {}", escape_markup(&line[2..]))
                    } else {
                        escape_markup(line)
                    };

                    label.set_markup(&formatted);
                    label.set_margin_bottom(2);
                    self.preview_box.append(&label);
                }
            }
            EditorMode::Blocks => {
                self.block_editor.set_markdown(&note.content);
            }
            _ => {}
        }
    }

    pub fn get_note_content(&self) -> (String, String) {
        let title = self.title_entry.text().to_string();

        let buffer = self.text_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let content = buffer.text(&start, &end, false).to_string();

        (title, content)
    }

    pub fn is_modified(&self) -> bool {
        *self.modified.borrow()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if !self.is_modified() {
            return Ok(());
        }

        let (_title, _content) = self.get_note_content();

        // TODO: Call knotd to save note

        *self.modified.borrow_mut() = false;
        Ok(())
    }

    pub fn clear(&self) {
        self.title_entry.set_text("");
        self.text_view.buffer().set_text("");
        *self.current_note.borrow_mut() = None;
        *self.modified.borrow_mut() = false;

        // Reset to source mode
        self.content_stack.set_visible_child_name("source");
        *self.current_mode.borrow_mut() = EditorMode::Source;
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

/// Simple markdown to Pango markup converter
fn markdown_to_pango_markup(markdown: &str) -> String {
    let mut result = String::with_capacity(markdown.len() * 2);

    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                result.push_str("</tt></span>\n");
                in_code_block = false;
            } else {
                result.push_str("\n<span face='monospace' background='#f0f0f0'><tt>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            result.push_str(&escape_markup(line));
            result.push('\n');
            continue;
        }

        if trimmed.is_empty() {
            result.push('\n');
            continue;
        }

        // Headers
        if trimmed.starts_with("# ") {
            let text = &trimmed[2..];
            result.push_str(&format!(
                "<span size='x-large' weight='bold'>{}</span>\n\n",
                escape_markup(text)
            ));
            continue;
        }
        if trimmed.starts_with("## ") {
            let text = &trimmed[3..];
            result.push_str(&format!(
                "<span size='large' weight='bold'>{}</span>\n\n",
                escape_markup(text)
            ));
            continue;
        }
        if trimmed.starts_with("### ") {
            let text = &trimmed[4..];
            result.push_str(&format!(
                "<span weight='bold'>{}</span>\n\n",
                escape_markup(text)
            ));
            continue;
        }

        // Lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            result.push_str(&format!("  • {}\n", escape_markup(text)));
            continue;
        }

        // Regular paragraph - simple escape
        result.push_str(&format!("{}\n", escape_markup(trimmed)));
    }

    if in_code_block {
        result.push_str("</tt></span>\n");
    }

    result
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
