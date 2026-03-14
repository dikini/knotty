# Note Contract Reference

## Purpose

Define the GTK-side data contract required for note loading, editing, note-type routing, and metadata handling.

## Core Note Payload

```json
{
  "id": "note-1",
  "path": "notes/example.md",
  "title": "Example",
  "content": "# Example\n\nBody",
  "created_at": 1730000000,
  "modified_at": 1730000100,
  "word_count": 3,
  "headings": [],
  "backlinks": [],
  "note_type": "markdown",
  "available_modes": {
    "meta": true,
    "source": true,
    "edit": true,
    "view": true
  },
  "metadata": {
    "frontmatter": {
      "title": "Example"
    },
    "tags": ["demo"]
  },
  "embed": null,
  "media": null
}
```

## Rust Type Template

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteData {
    pub id: String,
    pub path: String,
    pub title: String,
    pub content: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub word_count: usize,
    pub headings: Vec<Heading>,
    pub backlinks: Vec<Backlink>,
    pub note_type: Option<NoteType>,
    pub available_modes: Option<NoteModeAvailability>,
    pub metadata: Option<NoteMetadata>,
    pub embed: Option<NoteEmbedDescriptor>,
    pub media: Option<NoteMediaData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteType {
    Markdown,
    Pdf,
    Image,
    Youtube,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NoteModeAvailability {
    pub meta: bool,
    pub source: bool,
    pub edit: bool,
    pub view: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoteMetadata {
    #[serde(default)]
    pub frontmatter: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoteEmbedDescriptor {
    pub kind: String,
    pub source: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoteMediaData {
    pub mime_type: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
}
```

## Supporting Types

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Backlink {
    pub path: String,
    pub title: String,
    pub excerpt: Option<String>,
}
```

## Mode Defaults By Note Type

| Note Type | Meta | Source | Edit | View |
|---|---|---|---|---|
| `markdown` | true | true | true | true |
| `pdf` | false | false | false | true |
| `image` | false | false | false | true |
| `youtube` | true | false | false | true |
| `unknown` | true | true | false | true |

If `available_modes` is present, it overrides the default table.

## Routing Rules

- use `note_type` and `available_modes` from the payload
- do not guess note type from file extension if the payload already provides a type
- prefer payload capability flags over hard-coded UI assumptions
- treat unknown optional fields as forward-compatible and ignore them

## Save Contract

### Request

```json
{
  "name": "save_note",
  "arguments": {
    "path": "notes/example.md",
    "content": "# Example\n\nUpdated body"
  }
}
```

### Behavioral Rule

- only `content` is written in the editor slice
- metadata editing may either update the markdown frontmatter or a dedicated metadata structure, but the persisted result must round-trip through `get_note`

## Test Advice

- add deserialization tests with optional fields present and absent
- add mode-routing tests that cover both note-type defaults and payload override behavior
- add save round-trip tests for metadata-bearing notes
