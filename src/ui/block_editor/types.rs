//! Block-based document model inspired by Pandoc AST
//!
//! Documents are sequences of Block elements.
//! Each Block renders as a discrete GTK widget.
//! Images are always block-level, never inline.

use std::collections::HashMap;

/// A document is a sequence of blocks
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub metadata: HashMap<String, String>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        Self {
            blocks,
            metadata: HashMap::new(),
        }
    }
}

/// Block-level elements (structural)
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// Header with level (1-6) and content
    Header(u8, Vec<Inline>),

    /// Paragraph of inline content
    Paragraph(Vec<Inline>),

    /// Code block with optional language
    CodeBlock {
        language: Option<String>,
        content: String,
    },

    /// Block quote containing other blocks
    BlockQuote(Vec<Block>),

    /// Ordered list with items
    OrderedList { start: u32, items: Vec<Vec<Block>> },

    /// Bullet list with items  
    BulletList(Vec<Vec<Block>>),

    /// Image block (always full-width or predefined sizes)
    Image {
        path: String,
        alt: String,
        width: ImageWidth,
    },

    /// Diagram block (Mermaid, D2, etc.)
    Diagram {
        content: String,
        diagram_type: DiagramType,
        width: ImageWidth,
    },

    /// Horizontal rule / separator
    HorizontalRule,

    /// Table (simplified)
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },

    /// Math block (display mode)
    MathBlock(String),

    /// Raw HTML or other content
    RawBlock(String),

    /// Spacer/empty block
    Spacer,

    /// Grid layout container
    Grid {
        columns: u32,
        rows: Option<u32>, // None = auto
        gap: u32,          // pixels
        cells: Vec<GridCell>,
    },

    /// Tufte-style margin note (sidenote)
    MarginNote { id: String, content: Vec<Inline> },

    /// Tufte-style full-width figure (spans main + margin)
    FullWidthFigure {
        content: Box<Block>, // Image, Diagram, or Table
        caption: Vec<Inline>,
        margin_note: Option<Vec<Inline>>,
    },

    /// Tufte epigraph (quotation at chapter start)
    Epigraph {
        quote: Vec<Inline>,
        attribution: Option<Vec<Inline>>,
    },
}

/// Document layout style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DocumentStyle {
    /// Standard vertical flow
    Standard,
    /// Tufte-style: main column + margin column
    Tufte,
    /// Two-column academic
    TwoColumn,
}

/// A cell in a grid layout
#[derive(Debug, Clone, PartialEq)]
pub struct GridCell {
    /// Column position (0-indexed)
    pub col: u32,
    /// Row position (0-indexed)
    pub row: u32,
    /// Column span (default 1)
    pub col_span: u32,
    /// Row span (default 1)
    pub row_span: u32,
    /// Content blocks
    pub content: Vec<Block>,
}

/// Image width variants - no inline floating
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageWidth {
    /// Full container width
    Full,
    /// 75% width, centered
    Large,
    /// 50% width, centered  
    Medium,
    /// 33% width, centered
    Small,
}

/// Diagram types supported
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagramType {
    Mermaid,
    D2,
    Graphviz,
    Plantuml,
}

impl DiagramType {
    pub fn from_language(lang: &str) -> Option<Self> {
        match lang.to_lowercase().as_str() {
            "mermaid" => Some(DiagramType::Mermaid),
            "d2" => Some(DiagramType::D2),
            "graphviz" | "dot" => Some(DiagramType::Graphviz),
            "plantuml" | "puml" => Some(DiagramType::Plantuml),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DiagramType::Mermaid => "mermaid",
            DiagramType::D2 => "d2",
            DiagramType::Graphviz => "graphviz",
            DiagramType::Plantuml => "plantuml",
        }
    }
}

impl ImageWidth {
    /// Get CSS/percentage value
    pub fn as_percent(&self) -> f32 {
        match self {
            ImageWidth::Full => 100.0,
            ImageWidth::Large => 75.0,
            ImageWidth::Medium => 50.0,
            ImageWidth::Small => 33.0,
        }
    }
}

/// Inline elements (textual content within blocks)
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// Plain text
    Text(String),

    /// Emphasized (italic)
    Emph(Vec<Inline>),

    /// Strong (bold)
    Strong(Vec<Inline>),

    /// Code span (inline code)
    Code(String),

    /// Strikethrough
    Strikeout(Vec<Inline>),

    /// Superscript
    Superscript(Vec<Inline>),

    /// Subscript
    Subscript(Vec<Inline>),

    /// Hyperlink
    Link { text: Vec<Inline>, url: String },

    /// Hard line break
    LineBreak,

    /// Soft line break (space)
    SoftBreak,

    /// Non-breaking space
    Nbsp,
}

/// Position in source document (for syncing)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePos {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

impl SourcePos {
    pub fn new(start_byte: usize, end_byte: usize) -> Self {
        Self {
            start_byte,
            end_byte,
            start_line: 0,
            end_line: 0,
        }
    }
}

/// A block with its source position (for mapping)
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedBlock {
    pub block: Block,
    pub position: SourcePos,
}

/// Cursor position in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentCursor {
    /// Index of the block
    pub block_index: usize,
    /// Offset within the block (byte position or character count)
    pub offset: usize,
}

impl DocumentCursor {
    pub fn new(block_index: usize, offset: usize) -> Self {
        Self {
            block_index,
            offset,
        }
    }
}

/// Selection range in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentSelection {
    pub start: DocumentCursor,
    pub end: DocumentCursor,
}

impl DocumentSelection {
    pub fn new(start: DocumentCursor, end: DocumentCursor) -> Self {
        Self { start, end }
    }

    pub fn is_collapsed(&self) -> bool {
        self.start.block_index == self.end.block_index && self.start.offset == self.end.offset
    }
}
