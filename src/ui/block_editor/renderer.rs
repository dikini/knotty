//! Block rendering to GTK4 widgets

use super::types::*;
use gtk::prelude::*;

/// Renders Block elements to GTK4 widgets
pub struct BlockRenderer {
    pub code_theme: String,
    pub base_path: Option<std::path::PathBuf>,
    pub document_style: DocumentStyle,
    /// Width ratio for main column in Tufte mode (0.0-1.0)
    pub main_column_ratio: f32,
}

impl BlockRenderer {
    pub fn new() -> Self {
        Self {
            code_theme: "default".to_string(),
            base_path: None,
            document_style: DocumentStyle::Standard,
            main_column_ratio: 0.65, // Tufte: 65% main, 35% margin
        }
    }

    /// Create a Tufte-style renderer
    pub fn tufte() -> Self {
        Self {
            code_theme: "default".to_string(),
            base_path: None,
            document_style: DocumentStyle::Tufte,
            main_column_ratio: 0.65,
        }
    }

    /// Render a Block to a GTK widget
    pub fn render(&self, block: &Block) -> gtk::Widget {
        match block {
            Block::Header(level, inlines) => self.render_header(*level, inlines),
            Block::Paragraph(inlines) => self.render_paragraph(inlines),
            Block::CodeBlock { language, content } => {
                self.render_code_block(language.as_deref(), content)
            }
            Block::BlockQuote(blocks) => self.render_block_quote(blocks),
            Block::OrderedList { start, items } => self.render_ordered_list(*start, items),
            Block::BulletList(items) => self.render_bullet_list(items),
            Block::Image { path, alt, width } => self.render_image(path, alt, *width),
            Block::Diagram {
                content,
                diagram_type,
                width,
            } => self.render_diagram(content, *diagram_type, *width),
            Block::HorizontalRule => self.render_horizontal_rule(),
            Block::Table { headers, rows } => self.render_table(headers, rows),
            Block::MathBlock(content) => self.render_math_block(content),
            Block::RawBlock(content) => self.render_raw_block(content),
            Block::Spacer => self.render_spacer(),
            Block::Grid {
                columns,
                rows,
                gap,
                cells,
            } => self.render_grid(*columns, *rows, *gap, cells),
            Block::MarginNote { id, content } => self.render_margin_note(id, content),
            Block::FullWidthFigure {
                content,
                caption,
                margin_note,
            } => self.render_full_width_figure(content, caption, margin_note.as_ref()),
            Block::Epigraph { quote, attribution } => {
                self.render_epigraph(quote, attribution.as_ref())
            }
        }
    }

    fn render_header(&self, level: u8, inlines: &[Inline]) -> gtk::Widget {
        let label = gtk::Label::new(None);
        let text = inlines_to_pango(inlines);
        label.set_markup(&text);
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);

        // Apply heading style based on level
        let css_class = match level {
            1 => "title-1",
            2 => "title-2",
            3 => "title-3",
            4 => "heading-4",
            5 => "heading-5",
            _ => "heading-6",
        };
        label.add_css_class(css_class);

        // Margins
        let margin = match level {
            1 => 16,
            2 => 12,
            _ => 8,
        };
        label.set_margin_top(margin);
        label.set_margin_bottom(margin / 2);
        label.set_margin_start(12);
        label.set_margin_end(12);

        label.upcast()
    }

    fn render_paragraph(&self, inlines: &[Inline]) -> gtk::Widget {
        let label = gtk::Label::new(None);
        let text = inlines_to_pango(inlines);
        label.set_markup(&text);
        label.set_xalign(0.0);
        label.set_yalign(0.0);
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        label.set_selectable(true);

        label.set_margin_top(4);
        label.set_margin_bottom(4);
        label.set_margin_start(12);
        label.set_margin_end(12);

        label.upcast()
    }

    fn render_code_block(&self, language: Option<&str>, content: &str) -> gtk::Widget {
        // For now, use a simple TextView. Later could use sourceview5
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let text_view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .build();

        text_view.buffer().set_text(content);

        // Style
        text_view.add_css_class("code-block");
        if let Some(lang) = language {
            text_view.set_tooltip_text(Some(&format!("Language: {}", lang)));
        }

        text_view.set_margin_top(8);
        text_view.set_margin_bottom(8);
        text_view.set_margin_start(12);
        text_view.set_margin_end(12);

        scrolled.set_child(Some(&text_view));
        scrolled.upcast()
    }

    fn render_block_quote(&self, blocks: &[Block]) -> gtk::Widget {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Left border indicator
        container.add_css_class("block-quote");

        // Add a left margin/spacer
        let spacer = gtk::Box::builder().width_request(4).build();
        spacer.add_css_class("quote-border");

        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        hbox.append(&spacer);

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(12)
            .build();

        for block in blocks {
            let widget = self.render(block);
            content_box.append(&widget);
        }

        hbox.append(&content_box);
        container.append(&hbox);

        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_margin_start(12);
        container.set_margin_end(12);

        container.upcast()
    }

    fn render_ordered_list(&self, start: u32, items: &[Vec<Block>]) -> gtk::Widget {
        let list_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        for (i, item_blocks) in items.iter().enumerate() {
            let number = start as usize + i;
            let row = self.render_list_item(&format!("{}.", number), item_blocks);
            list_box.append(&row);
        }

        list_box.set_margin_top(4);
        list_box.set_margin_bottom(4);
        list_box.set_margin_start(12);
        list_box.set_margin_end(12);

        list_box.upcast()
    }

    fn render_bullet_list(&self, items: &[Vec<Block>]) -> gtk::Widget {
        let list_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        for item_blocks in items {
            let row = self.render_list_item("•", item_blocks);
            list_box.append(&row);
        }

        list_box.set_margin_top(4);
        list_box.set_margin_bottom(4);
        list_box.set_margin_start(12);
        list_box.set_margin_end(12);

        list_box.upcast()
    }

    fn render_list_item(&self, marker: &str, blocks: &[Block]) -> gtk::Box {
        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        // Marker label
        let marker_label = gtk::Label::new(Some(marker));
        marker_label.set_width_chars(3);
        marker_label.set_xalign(0.0);
        marker_label.set_margin_end(8);
        hbox.append(&marker_label);

        // Content
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();

        for block in blocks {
            let widget = self.render(block);
            content_box.append(&widget);
        }

        hbox.append(&content_box);
        hbox
    }

    fn render_image(&self, path: &str, alt: &str, width: ImageWidth) -> gtk::Widget {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Image loading (async would be better in real implementation)
        let image = gtk::Image::new();

        // Try to load the image
        let full_path = if let Some(base) = &self.base_path {
            base.join(path)
        } else {
            std::path::PathBuf::from(path)
        };

        if full_path.exists() {
            image.set_from_file(Some(&full_path));
        } else {
            // Placeholder for missing image
            image.set_icon_name(Some("image-missing"));
            image.set_pixel_size(64);
        }

        // Set width constraint based on ImageWidth variant
        match width {
            ImageWidth::Full => {
                image.set_hexpand(true);
            }
            ImageWidth::Large => {
                // 75% width - use CSS or size request
                image.set_halign(gtk::Align::Center);
            }
            ImageWidth::Medium => {
                // 50% width
                image.set_halign(gtk::Align::Center);
            }
            ImageWidth::Small => {
                // 33% width
                image.set_halign(gtk::Align::Center);
            }
        }

        container.append(&image);

        // Alt text as caption if present
        if !alt.is_empty() {
            let caption = gtk::Label::new(Some(alt));
            caption.add_css_class("caption");
            caption.set_xalign(0.5);
            caption.set_margin_top(4);
            container.append(&caption);
        }

        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_margin_start(12);
        container.set_margin_end(12);

        container.upcast()
    }

    fn render_horizontal_rule(&self) -> gtk::Widget {
        let separator = gtk::Separator::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        separator.set_margin_top(16);
        separator.set_margin_bottom(16);
        separator.set_margin_start(24);
        separator.set_margin_end(24);

        separator.upcast()
    }

    fn render_table(&self, _headers: &[Vec<Inline>], _rows: &[Vec<Vec<Inline>>]) -> gtk::Widget {
        // Simplified - just show "Table" placeholder for now
        let label = gtk::Label::new(Some("[Table rendering coming soon]"));
        label.add_css_class("dim-label");
        label.set_margin_top(8);
        label.set_margin_bottom(8);
        label.upcast()
    }

    fn render_math_block(&self, content: &str) -> gtk::Widget {
        // Placeholder for math rendering
        let label = gtk::Label::new(Some(&format!("$${}$$", content)));
        label.set_selectable(true);
        label.set_margin_top(8);
        label.set_margin_bottom(8);
        label.upcast()
    }

    fn render_raw_block(&self, content: &str) -> gtk::Widget {
        let label = gtk::Label::new(Some(content));
        label.set_selectable(true);
        label.add_css_class("raw-block");
        label.set_margin_top(4);
        label.set_margin_bottom(4);
        label.upcast()
    }

    fn render_spacer(&self) -> gtk::Widget {
        let spacer = gtk::Box::builder().height_request(16).build();
        spacer.upcast()
    }

    fn render_grid(
        &self,
        columns: u32,
        _rows: Option<u32>,
        gap: u32,
        cells: &[GridCell],
    ) -> gtk::Widget {
        // Use GTK4's Grid layout
        let grid = gtk::Grid::builder()
            .row_spacing(gap as i32)
            .column_spacing(gap as i32)
            .build();

        // Determine column widths (equal distribution)
        // In a full implementation, this could be configurable per-column

        for cell in cells {
            // Create container for cell content
            let cell_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();

            // Add a subtle border/background to distinguish cells
            cell_box.add_css_class("grid-cell");

            // Render all blocks in the cell
            for block in &cell.content {
                let widget = self.render(block);
                cell_box.append(&widget);
            }

            // Attach to grid
            grid.attach(
                &cell_box,
                cell.col as i32,
                cell.row as i32,
                cell.col_span as i32,
                cell.row_span as i32,
            );

            // Make cell expand
            cell_box.set_hexpand(true);
            cell_box.set_vexpand(true);
        }

        // Set equal column weights by making all cells expand
        for i in 0..columns {
            grid.set_column_homogeneous(true);
        }

        grid.set_margin_top(12);
        grid.set_margin_bottom(12);
        grid.set_margin_start(12);
        grid.set_margin_end(12);

        // Wrap in a frame for visual distinction
        let frame = gtk::Frame::builder().child(&grid).build();
        frame.add_css_class("grid-container");

        frame.upcast()
    }

    fn render_diagram(
        &self,
        content: &str,
        diagram_type: DiagramType,
        width: ImageWidth,
    ) -> gtk::Widget {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Diagram type indicator
        let type_label = gtk::Label::new(Some(&format!("[{} diagram]", diagram_type.as_str())));
        type_label.add_css_class("dim-label");
        type_label.set_margin_bottom(4);
        container.append(&type_label);

        // For now, show the source code with a render button
        // In full implementation, this would render to SVG and display the image
        let text_view = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .height_request(100)
            .build();
        text_view.buffer().set_text(content);
        text_view.add_css_class("code-block");

        // Set width constraint
        match width {
            ImageWidth::Full => {
                text_view.set_hexpand(true);
            }
            _ => {
                text_view.set_halign(gtk::Align::Center);
            }
        }

        container.append(&text_view);

        // Render button (for future implementation)
        let button = gtk::Button::builder()
            .label("Render Diagram")
            .margin_top(8)
            .build();
        container.append(&button);

        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_margin_start(12);
        container.set_margin_end(12);

        container.upcast()
    }

    /// Render a Tufte-style margin note (aside)
    fn render_margin_note(&self, id: &str, content: &[Inline]) -> gtk::Widget {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Number indicator (superscript style)
        let number_label = gtk::Label::new(Some(&format!("{} ", id)));
        number_label.add_css_class("margin-note-number");
        number_label.set_xalign(0.0);

        // Content
        let content_label = gtk::Label::new(None);
        let text = inlines_to_pango(content);
        content_label.set_markup(&text);
        content_label.set_xalign(0.0);
        content_label.set_yalign(0.0);
        content_label.set_wrap(true);
        content_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        content_label.add_css_class("margin-note-text");

        // Horizontal layout: number + content
        let hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        hbox.append(&number_label);
        hbox.append(&content_label);

        container.append(&hbox);

        // Tufte: smaller text, tighter line spacing
        container.add_css_class("margin-note");
        container.set_margin_top(4);
        container.set_margin_bottom(4);
        container.set_margin_start(8);
        container.set_margin_end(8);

        container.upcast()
    }

    /// Render a Tufte-style full-width figure
    fn render_full_width_figure(
        &self,
        content: &Block,
        caption: &[Inline],
        margin_note: Option<&Vec<Inline>>,
    ) -> gtk::Widget {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Main content (Image, Diagram, etc.)
        let content_widget = self.render(content);
        container.append(&content_widget);

        // Caption row: main caption + optional margin note
        let caption_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();

        // Main caption (left side, ~65%)
        let caption_label = gtk::Label::new(None);
        let caption_text = inlines_to_pango(caption);
        caption_label.set_markup(&caption_text);
        caption_label.set_xalign(0.0);
        caption_label.set_wrap(true);
        caption_label.add_css_class("figure-caption");
        caption_label.set_hexpand(true);
        caption_row.append(&caption_label);

        // Optional margin note (right side, ~35%)
        if let Some(margin) = margin_note {
            let margin_label = gtk::Label::new(None);
            let margin_text = inlines_to_pango(margin);
            margin_label.set_markup(&margin_text);
            margin_label.set_xalign(0.0);
            margin_label.set_wrap(true);
            margin_label.add_css_class("margin-note-text");
            margin_label.set_width_request(200); // Fixed-ish width for margin
            caption_row.append(&margin_label);
        }

        container.append(&caption_row);

        // Full width styling
        container.add_css_class("full-width-figure");
        container.set_margin_top(16);
        container.set_margin_bottom(16);

        container.upcast()
    }

    /// Render a Tufte-style epigraph
    fn render_epigraph(&self, quote: &[Inline], attribution: Option<&Vec<Inline>>) -> gtk::Widget {
        let container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Quote text - italic, larger, with left border
        let quote_label = gtk::Label::new(None);
        let quote_text = format!("<i>{}</i>", inlines_to_pango(quote));
        quote_label.set_markup(&quote_text);
        quote_label.set_xalign(0.0);
        quote_label.set_wrap(true);
        quote_label.add_css_class("epigraph-quote");

        // Attribution - right-aligned, smaller
        if let Some(attr) = attribution {
            let attr_label = gtk::Label::new(None);
            let attr_text = format!("— {}", inlines_to_pango(attr));
            attr_label.set_markup(&attr_text);
            attr_label.set_xalign(1.0); // Right align
            attr_label.add_css_class("epigraph-attribution");

            container.append(&quote_label);
            container.append(&attr_label);
        } else {
            container.append(&quote_label);
        }

        container.add_css_class("epigraph");
        container.set_margin_top(24);
        container.set_margin_bottom(24);
        container.set_margin_start(48);
        container.set_margin_end(48);

        container.upcast()
    }
}

/// Convert inline elements to Pango markup
fn inlines_to_pango(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        result.push_str(&inline_to_pango(inline));
    }
    result
}

fn inline_to_pango(inline: &Inline) -> String {
    match inline {
        Inline::Text(s) => escape_pango(s),
        Inline::Emph(inlines) => {
            format!("<i>{}</i>", inlines_to_pango(inlines))
        }
        Inline::Strong(inlines) => {
            format!("<b>{}</b>", inlines_to_pango(inlines))
        }
        Inline::Code(s) => {
            format!("<tt>{}</tt>", escape_pango(s))
        }
        Inline::Strikeout(inlines) => {
            format!("<s>{}</s>", inlines_to_pango(inlines))
        }
        Inline::Superscript(inlines) => {
            format!("<sup>{}</sup>", inlines_to_pango(inlines))
        }
        Inline::Subscript(inlines) => {
            format!("<sub>{}</sub>", inlines_to_pango(inlines))
        }
        Inline::Link { text, url } => {
            // GTK Label links use <a href="..."> but they need to be handled specially
            // For now, show as blue text with URL in tooltip
            let link_text = inlines_to_pango(text);
            format!(
                "<span foreground='blue' underline='single'>{}</span>",
                link_text
            )
        }
        Inline::LineBreak => "\n".to_string(),
        Inline::SoftBreak => " ".to_string(),
        Inline::Nbsp => "&nbsp;".to_string(),
    }
}

fn escape_pango(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
