//! Block-based markdown editor for GTK4
//!
//! This module provides a WYSIWYM (What You See Is What You Mean) editor
//! where markdown is rendered as discrete block widgets rather than raw text.
//!
//! ## Key Design Decisions:
//!
//! - **Block-level only**: Images, diagrams, and other media are always blocks,
//!   never inline. No text wrapping around content.
//!   
//! - **Predefined widths**: Images/diagrams can be Full (100%), Large (75%),
//!   Medium (50%), or Small (33%), all centered.
//!   
//! - **Tufte-inspired layout**: Support for margin notes, full-width figures,
//!   and elegant typography.
//!   
//! - **Pandoc-inspired AST**: Document structure follows pandoc-types for
//!   compatibility and extensibility.

mod parser;
mod renderer;
mod types;

pub use parser::parse_markdown;
pub use renderer::BlockRenderer;
pub use types::*;

use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Block-based markdown editor widget
#[derive(Clone)]
pub struct BlockEditor {
    inner: Rc<BlockEditorInner>,
}

struct BlockEditorInner {
    widget: gtk::ScrolledWindow,
    content_box: gtk::Box,
    renderer: RefCell<BlockRenderer>,
    document: RefCell<Document>,
    on_change: RefCell<Option<Box<dyn Fn(&Document)>>>,
}

impl BlockEditor {
    pub fn new() -> Self {
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&content_box)
            .build();

        Self {
            inner: Rc::new(BlockEditorInner {
                widget: scrolled,
                content_box,
                renderer: RefCell::new(BlockRenderer::new()),
                document: RefCell::new(Document::new()),
                on_change: RefCell::new(None),
            }),
        }
    }

    /// Create with Tufte style
    pub fn tufte() -> Self {
        let editor = Self::new();
        *editor.inner.renderer.borrow_mut() = BlockRenderer::tufte();
        editor
    }

    /// Set the base path for resolving relative image paths
    pub fn set_base_path(&self, path: impl AsRef<std::path::Path>) {
        self.inner.renderer.borrow_mut().base_path = Some(path.as_ref().to_path_buf());
    }

    /// Load markdown content and render as blocks
    pub fn set_markdown(&self, content: &str) {
        let document = parse_markdown(content);
        self.set_document(document);
    }

    /// Set a document directly
    pub fn set_document(&self, document: Document) {
        // Clear existing content
        while let Some(child) = self.inner.content_box.first_child() {
            self.inner.content_box.remove(&child);
        }

        // Render each block
        let renderer = self.inner.renderer.borrow();
        for block in &document.blocks {
            let widget = renderer.render(block);
            self.inner.content_box.append(&widget);
        }
        drop(renderer);

        *self.inner.document.borrow_mut() = document;

        // Notify change handler
        if let Some(ref callback) = *self.inner.on_change.borrow() {
            callback(&self.inner.document.borrow());
        }
    }

    /// Get current document
    pub fn document(&self) -> Document {
        self.inner.document.borrow().clone()
    }

    /// Connect to change events
    pub fn connect_changed<F>(&self, f: F)
    where
        F: Fn(&Document) + 'static,
    {
        *self.inner.on_change.borrow_mut() = Some(Box::new(f));
    }

    /// Get the GTK widget
    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.inner.widget
    }

    /// Refresh the view (re-render current document)
    pub fn refresh(&self) {
        let doc = self.inner.document.borrow().clone();
        self.set_document(doc);
    }

    /// Scroll to a specific block by index
    pub fn scroll_to_block(&self, block_index: usize) {
        // Find the child widget at the specified index by iterating
        let mut current = self.inner.content_box.first_child();
        let mut current_idx = 0;
        let target_child = loop {
            match current {
                Some(child) if current_idx == block_index => break Some(child),
                Some(child) => {
                    current = child.next_sibling();
                    current_idx += 1;
                }
                None => break None,
            }
        };

        if let Some(_child) = target_child {
            // Scroll the scrolled window to estimated position
            let adjustment = self.inner.widget.vadjustment();

            // Estimate position based on average block height (~100px)
            let estimated_height = 100.0;
            let block_y = block_index as f64 * estimated_height;

            // Center the block in view if possible
            let page_size = adjustment.page_size();
            let value = (block_y - page_size / 2.0).max(0.0);

            adjustment.set_value(value);
        }
    }

    /// Scroll to a fraction of total height (0.0 = top, 1.0 = bottom)
    pub fn scroll_to_fraction(&self, fraction: f64) {
        let adjustment = self.inner.widget.vadjustment();
        let upper = adjustment.upper() - adjustment.page_size();
        if upper > 0.0 {
            adjustment.set_value(fraction * upper);
        }
    }

    /// Get current scroll fraction (0.0 = top, 1.0 = bottom)
    pub fn get_scroll_fraction(&self) -> Option<f64> {
        let adjustment = self.inner.widget.vadjustment();
        let upper = adjustment.upper() - adjustment.page_size();
        if upper > 0.0 {
            Some(adjustment.value() / upper)
        } else {
            None
        }
    }
}

impl Default for BlockEditor {
    fn default() -> Self {
        Self::new()
    }
}
