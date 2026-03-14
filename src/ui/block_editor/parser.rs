//! Markdown parser using pulldown-cmark
//!
//! Converts CommonMark + extensions to our Block AST.

use super::types::*;
use pulldown_cmark::{Event, Parser, Tag};

/// Parse markdown string to Document
pub fn parse_markdown(input: &str) -> Document {
    let parser = Parser::new(input);
    let mut blocks = Vec::new();
    let mut current_block: Option<BlockAccumulator> = None;

    for event in parser {
        match event {
            Event::Start(tag) => {
                // Starting a new block-level element
                match tag {
                    Tag::Paragraph => {
                        current_block = Some(BlockAccumulator::Paragraph(Vec::new()));
                    }
                    Tag::Heading { level, .. } => {
                        current_block = Some(BlockAccumulator::Heading(level as u8, Vec::new()));
                    }
                    Tag::BlockQuote(_) => {
                        current_block = Some(BlockAccumulator::BlockQuote(Vec::new()));
                    }
                    Tag::CodeBlock(lang) => {
                        let language = match lang {
                            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                                let lang_str = lang.to_string();
                                if lang_str.is_empty() {
                                    None
                                } else {
                                    Some(lang_str)
                                }
                            }
                            _ => None,
                        };
                        current_block = Some(BlockAccumulator::CodeBlock {
                            language,
                            content: String::new(),
                        });
                    }
                    Tag::List(start_num) => {
                        current_block = Some(BlockAccumulator::List {
                            ordered: start_num.is_some(),
                            start: start_num.unwrap_or(1) as u32,
                            items: Vec::new(),
                            current_item: Vec::new(),
                        });
                    }
                    Tag::Item => {
                        // Start of list item - handled in List accumulator
                    }
                    _ => {}
                }
            }

            Event::End(_) => {
                // Finish current block
                if let Some(acc) = current_block.take() {
                    if let Some(block) = acc.finish() {
                        blocks.push(block);
                    }
                }
            }

            Event::Text(text) => {
                if let Some(ref mut acc) = current_block {
                    acc.push_inline(Inline::Text(text.to_string()));
                }
            }

            Event::Code(code) => {
                if let Some(ref mut acc) = current_block {
                    acc.push_inline(Inline::Code(code.to_string()));
                }
            }

            Event::Html(html) => {
                // Raw HTML as block
                blocks.push(Block::RawBlock(html.to_string()));
            }

            Event::SoftBreak => {
                if let Some(ref mut acc) = current_block {
                    acc.push_inline(Inline::SoftBreak);
                }
            }

            Event::HardBreak => {
                if let Some(ref mut acc) = current_block {
                    acc.push_inline(Inline::LineBreak);
                }
            }

            Event::Rule => {
                blocks.push(Block::HorizontalRule);
            }

            _ => {}
        }
    }

    // Handle any remaining block
    if let Some(acc) = current_block {
        if let Some(block) = acc.finish() {
            blocks.push(block);
        }
    }

    // Post-process to detect special blocks (diagrams, etc.)
    let blocks = post_process_blocks(blocks);

    Document::from_blocks(blocks)
}

/// Accumulator for building blocks during parsing
enum BlockAccumulator {
    Paragraph(Vec<Inline>),
    Heading(u8, Vec<Inline>),
    BlockQuote(Vec<Block>),
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    List {
        ordered: bool,
        start: u32,
        items: Vec<Vec<Block>>,
        current_item: Vec<Block>,
    },
}

impl BlockAccumulator {
    fn push_inline(&mut self, inline: Inline) {
        match self {
            BlockAccumulator::Paragraph(inlines) | BlockAccumulator::Heading(_, inlines) => {
                inlines.push(inline);
            }
            _ => {}
        }
    }

    fn finish(self) -> Option<Block> {
        match self {
            BlockAccumulator::Paragraph(inlines) => {
                if inlines.is_empty() {
                    None
                } else {
                    Some(Block::Paragraph(inlines))
                }
            }
            BlockAccumulator::Heading(level, inlines) => {
                if inlines.is_empty() {
                    None
                } else {
                    Some(Block::Header(level, inlines))
                }
            }
            BlockAccumulator::BlockQuote(blocks) => Some(Block::BlockQuote(blocks)),
            BlockAccumulator::CodeBlock { language, content } => {
                // Check if this is actually a diagram
                if let Some(ref lang) = language {
                    if let Some(diag_type) = DiagramType::from_language(lang) {
                        return Some(Block::Diagram {
                            content,
                            diagram_type: diag_type,
                            width: ImageWidth::Full, // Default to full width
                        });
                    }
                }
                Some(Block::CodeBlock { language, content })
            }
            BlockAccumulator::List {
                ordered,
                start,
                mut items,
                current_item,
            } => {
                // Add the last item if present
                let mut items = items;
                if !current_item.is_empty() {
                    items.push(current_item);
                }

                if ordered {
                    Some(Block::OrderedList { start, items })
                } else {
                    Some(Block::BulletList(items))
                }
            }
        }
    }
}

/// Post-process blocks to detect images and other special cases
fn post_process_blocks(blocks: Vec<Block>) -> Vec<Block> {
    blocks
        .into_iter()
        .map(|block| match block {
            Block::Paragraph(inlines) => {
                // Check if paragraph is just an image reference
                if let Some(image_block) = try_extract_image_block(&inlines) {
                    image_block
                } else {
                    Block::Paragraph(inlines)
                }
            }
            other => other,
        })
        .collect()
}

/// Try to extract an image block from a paragraph
/// If a paragraph contains only an image link, convert to Image block
fn try_extract_image_block(inlines: &[Inline]) -> Option<Block> {
    if inlines.len() == 1 {
        if let Inline::Link {
            text: _text,
            url: _url,
        } = &inlines[0]
        {
            // TODO: Check if link text is also the URL (image syntax in markdown)
            // In markdown: ![alt](path) becomes Link with text=[Image alt]
            // Actually pulldown-cmark should handle this as Event::Start(Tag::Image)
        }
    }
    None
}

/// Extract inline formatting from text
/// This is a simplified version - full implementation would parse markdown inline syntax
pub fn parse_inline(text: &str) -> Vec<Inline> {
    // For now, just return as plain text
    // Full implementation would parse **bold**, *italic*, `code`, [links](url), etc.
    vec![Inline::Text(text.to_string())]
}
