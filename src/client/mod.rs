//! knotd daemon client for GTK4 UI.
//!
//! Communicates with knotd via JSON-RPC over Unix socket.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1000);

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Missing result in response")]
    MissingResult,

    #[error("No vault is open")]
    NoVaultOpen,
}

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug, Clone)]
pub struct KnotdClient {
    socket_path: String,
}

impl KnotdClient {
    pub fn new() -> Self {
        let socket_path = crate::SOCKET_PATH
            .get()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(default_socket_path);
        Self { socket_path }
    }

    pub fn with_socket_path(socket_path: impl AsRef<std::path::Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_string_lossy().to_string(),
        }
    }

    fn next_id() -> u64 {
        NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
    }

    fn read_framed_message<R: BufRead>(reader: &mut R) -> Result<String> {
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let read = reader.read_line(&mut line)?;
            if read == 0 {
                return Err(ClientError::Connection(
                    "Connection closed while reading headers".to_string(),
                ));
            }

            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }

            if let Some((name, value)) = trimmed.split_once(':') {
                if name.eq_ignore_ascii_case("Content-Length") {
                    content_length = value.trim().parse::<usize>().ok();
                }
            }
        }

        let len =
            content_length.ok_or_else(|| ClientError::Rpc("Missing Content-Length".to_string()))?;

        let mut body = vec![0_u8; len];
        reader.read_exact(&mut body)?;

        String::from_utf8(body).map_err(|e| ClientError::Connection(format!("Invalid UTF-8: {e}")))
    }

    fn write_framed_message<W: Write>(writer: &mut W, value: &Value) -> Result<()> {
        let payload = serde_json::to_vec(value)?;
        write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
        writer.write_all(&payload)?;
        writer.flush()?;
        Ok(())
    }

    fn call_jsonrpc(&self, method: &str, params: Value) -> Result<Value> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            ClientError::Connection(format!(
                "Failed to connect to knotd at {}: {}. Is knotd running?",
                self.socket_path, e
            ))
        })?;

        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": Self::next_id(),
            "method": method,
            "params": params
        });

        Self::write_framed_message(&mut stream, &request)?;

        let mut reader = BufReader::new(stream);
        let raw = Self::read_framed_message(&mut reader)?;
        let response: Value = serde_json::from_str(&raw)?;

        if let Some(err) = response.get("error") {
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("knotd RPC error");
            // Check for specific error types
            if message.contains("No vault is open") {
                return Err(ClientError::NoVaultOpen);
            }
            return Err(ClientError::Rpc(message.to_string()));
        }

        response
            .get("result")
            .cloned()
            .ok_or(ClientError::MissingResult)
    }

    /// Call a tool on the knotd daemon
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let result = self.call_jsonrpc(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments
            }),
        )?;

        // Extract text content from the result
        let text = result
            .get("content")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|v| v.get("text"))
            .and_then(Value::as_str)
            .ok_or_else(|| ClientError::Rpc(format!("Tool {name} returned no text payload")))?;

        // Parse the JSON text content
        serde_json::from_str(text).map_err(ClientError::Json)
    }

    // ===== Vault Operations =====

    pub fn get_vault_info(&self) -> Result<VaultInfo> {
        let value = self.call_tool("get_vault_info", json!({}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn is_vault_open(&self) -> Result<bool> {
        let value = self.call_tool("is_vault_open", json!({}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn open_vault(&self, path: &str) -> Result<VaultInfo> {
        let value = self.call_tool("open_vault", json!({"path": path}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn create_vault(&self, path: &str) -> Result<VaultInfo> {
        let value = self.call_tool("create_vault", json!({"path": path}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    // ===== Note Operations =====

    pub fn get_note(&self, path: &str) -> Result<NoteData> {
        let value = self.call_tool("get_note", json!({"path": path}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn save_note(&self, path: &str, content: &str) -> Result<()> {
        self.call_tool("save_note", json!({"path": path, "content": content}))?;
        Ok(())
    }

    pub fn create_note(&self, path: &str, content: Option<&str>) -> Result<NoteData> {
        let mut args = json!({"path": path});
        if let Some(content) = content {
            args["content"] = json!(content);
        }
        let value = self.call_tool("create_note", args)?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn delete_note(&self, path: &str) -> Result<()> {
        self.call_tool("delete_note", json!({"path": path}))?;
        Ok(())
    }

    pub fn rename_note(&self, old_path: &str, new_path: &str) -> Result<()> {
        self.call_tool(
            "rename_note",
            json!({"old_path": old_path, "new_path": new_path}),
        )?;
        Ok(())
    }

    // ===== Search Operations =====

    pub fn search_notes(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>> {
        let mut args = json!({"query": query});
        if let Some(limit) = limit {
            args["limit"] = json!(limit);
        }
        let value = self.call_tool("search_notes", args)?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    // ===== Explorer Operations =====

    pub fn get_explorer_tree(&self) -> Result<ExplorerTree> {
        let value = self.call_tool("get_explorer_tree", json!({}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn set_folder_expanded(&self, path: &str, expanded: bool) -> Result<()> {
        self.call_tool(
            "set_folder_expanded",
            json!({"path": path, "expanded": expanded}),
        )?;
        Ok(())
    }

    pub fn create_directory(&self, path: &str) -> Result<()> {
        self.call_tool("create_directory", json!({"path": path}))?;
        Ok(())
    }

    pub fn rename_directory(&self, old_path: &str, new_path: &str) -> Result<()> {
        self.call_tool(
            "rename_directory",
            json!({"old_path": old_path, "new_path": new_path}),
        )?;
        Ok(())
    }

    pub fn remove_directory(&self, path: &str, recursive: bool) -> Result<()> {
        self.call_tool(
            "remove_directory",
            json!({"path": path, "recursive": recursive}),
        )?;
        Ok(())
    }

    // ===== Graph Operations =====

    pub fn get_graph_layout(&self, width: f64, height: f64) -> Result<GraphLayout> {
        let value = self.call_tool(
            "get_graph_layout",
            json!({"width": width, "height": height}),
        )?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn graph_neighbors(&self, path: &str, depth: Option<usize>) -> Result<GraphNeighborhood> {
        let mut args = json!({"path": path});
        if let Some(depth) = depth {
            args["depth"] = json!(depth);
        }
        let value = self.call_tool("graph_neighbors", args)?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    // ===== Settings Operations =====

    #[allow(dead_code)]
    pub fn get_vault_settings(&self) -> Result<VaultSettings> {
        let value = self.call_tool("get_vault_settings", json!({}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    #[allow(dead_code)]
    pub fn update_vault_settings(&self, patch: Value) -> Result<VaultSettings> {
        let value = self.call_tool("update_vault_settings", json!({"patch": patch}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn list_vault_plugins(&self) -> Result<Vec<VaultPluginInfo>> {
        let value = self.call_tool("list_vault_plugins", json!({}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }

    pub fn reindex_vault(&self) -> Result<MaintenanceResult> {
        let value = self.call_tool("reindex_vault", json!({}))?;
        serde_json::from_value(value).map_err(ClientError::Json)
    }
}

impl Default for KnotdClient {
    fn default() -> Self {
        Self::new()
    }
}

fn default_socket_path() -> String {
    crate::runtime_contract::default_socket_path()
        .unwrap_or_else(|| std::path::PathBuf::from(crate::runtime_contract::default_socket_help()))
        .to_string_lossy()
        .into_owned()
}

// ===== Data Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub path: String,
    pub name: String,
    #[serde(rename = "note_count")]
    pub note_count: usize,
    #[serde(rename = "last_modified")]
    pub last_modified: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSettings {
    pub name: String,
    #[serde(rename = "plugins_enabled")]
    pub plugins_enabled: bool,
    #[serde(rename = "file_visibility")]
    pub file_visibility: String,
    pub editor: VaultEditorSettings,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEditorSettings {
    #[serde(rename = "font_size")]
    pub font_size: i32,
    #[serde(rename = "tab_size")]
    pub tab_size: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultPluginInfo {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub effective_enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaintenanceResult {
    Message(String),
    Count(i64),
    Object {
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        count: Option<i64>,
        #[serde(default)]
        reindexed: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteData {
    pub id: String,
    pub path: String,
    pub title: String,
    pub content: String,
    #[serde(rename = "created_at")]
    pub created_at: i64,
    #[serde(rename = "modified_at")]
    pub modified_at: i64,
    #[serde(rename = "word_count")]
    pub word_count: usize,
    #[serde(default)]
    pub headings: Vec<Heading>,
    #[serde(default)]
    pub backlinks: Vec<Backlink>,
    #[serde(rename = "note_type")]
    pub note_type: Option<NoteType>,
    pub available_modes: Option<NoteModeAvailability>,
    pub metadata: Option<NoteMetadata>,
    pub embed: Option<NoteEmbedDescriptor>,
    pub media: Option<NoteMediaData>,
    #[serde(rename = "type_badge")]
    pub type_badge: Option<String>,
    #[serde(default)]
    pub is_dimmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteType {
    Markdown,
    Youtube,
    Pdf,
    Image,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteModeAvailability {
    #[serde(default)]
    pub meta: bool,
    #[serde(default)]
    pub source: bool,
    #[serde(default)]
    pub edit: bool,
    #[serde(default)]
    pub view: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteMetadata {
    #[serde(default)]
    pub frontmatter: serde_json::Map<String, Value>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteEmbedDescriptor {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteMediaData {
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backlink {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub excerpt: Option<String>,
}

// Search types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub excerpt: String,
    pub score: f64,
}

// Explorer types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerTree {
    pub root: ExplorerFolderNode,
    #[serde(rename = "hidden_policy")]
    pub hidden_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerFolderNode {
    pub path: String,
    pub name: String,
    pub expanded: bool,
    pub folders: Vec<ExplorerFolderNode>,
    pub notes: Vec<ExplorerNoteNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerNoteNode {
    pub path: String,
    pub title: String,
    #[serde(rename = "display_title")]
    pub display_title: String,
    #[serde(rename = "modified_at")]
    pub modified_at: i64,
    #[serde(rename = "word_count")]
    pub word_count: usize,
    #[serde(rename = "type_badge")]
    pub type_badge: Option<String>,
    #[serde(default)]
    pub is_dimmed: bool,
}

// Graph types
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphLayout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNeighborhood {
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_note_payload() -> Value {
        json!({
            "id": "note-1",
            "path": "notes/example.md",
            "title": "Example",
            "content": "# Example",
            "created_at": 1730000000,
            "modified_at": 1730000100,
            "word_count": 2,
            "headings": [],
            "backlinks": [],
            "note_type": "markdown",
            "type_badge": "MD"
        })
    }

    #[test]
    fn note_data_deserializes_available_modes() {
        let mut payload = base_note_payload();
        payload["available_modes"] = json!({
            "meta": true,
            "source": true,
            "edit": false,
            "view": true
        });

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert_eq!(
            note.available_modes,
            Some(NoteModeAvailability {
                meta: true,
                source: true,
                edit: false,
                view: true,
            })
        );
    }

    #[test]
    fn note_data_deserializes_optional_media() {
        let mut payload = base_note_payload();
        payload["note_type"] = json!("pdf");
        payload["media"] = json!({
            "mime_type": "application/pdf",
            "file_path": "/tmp/example.pdf",
            "thumbnail_path": "/tmp/example.png"
        });

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert!(matches!(note.note_type, Some(NoteType::Pdf)));
        assert_eq!(
            note.media,
            Some(NoteMediaData {
                mime_type: "application/pdf".to_string(),
                file_path: Some("/tmp/example.pdf".to_string()),
                thumbnail_path: Some("/tmp/example.png".to_string()),
            })
        );
    }

    #[test]
    fn note_data_deserializes_metadata_and_embed() {
        let mut payload = base_note_payload();
        payload["note_type"] = json!("youtube");
        payload["metadata"] = json!({
            "frontmatter": {
                "title": "Example"
            },
            "tags": ["demo", "video"]
        });
        payload["embed"] = json!({
            "kind": "youtube",
            "source": "https://www.youtube.com/watch?v=test",
            "title": "Demo Video"
        });

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert_eq!(
            note.metadata,
            Some(NoteMetadata {
                frontmatter: serde_json::Map::from_iter([(
                    "title".to_string(),
                    Value::String("Example".to_string()),
                )]),
                tags: vec!["demo".to_string(), "video".to_string()],
            })
        );
        assert_eq!(
            note.embed,
            Some(NoteEmbedDescriptor {
                kind: "youtube".to_string(),
                source: "https://www.youtube.com/watch?v=test".to_string(),
                title: Some("Demo Video".to_string()),
            })
        );
    }

    #[test]
    fn note_data_tolerates_embed_without_kind() {
        let mut payload = base_note_payload();
        payload["note_type"] = json!("youtube");
        payload["embed"] = json!({
            "source": "https://www.youtube.com/watch?v=test",
            "title": "Demo Video"
        });

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert_eq!(
            note.embed,
            Some(NoteEmbedDescriptor {
                kind: String::new(),
                source: "https://www.youtube.com/watch?v=test".to_string(),
                title: Some("Demo Video".to_string()),
            })
        );
    }

    #[test]
    fn note_data_tolerates_missing_heading_slug_and_absent_collections() {
        let mut payload = base_note_payload();
        payload
            .as_object_mut()
            .expect("payload object")
            .remove("backlinks");
        payload
            .as_object_mut()
            .expect("payload object")
            .remove("headings");
        payload["headings"] = json!([
            {
                "level": 2,
                "text": "Section One"
            }
        ]);

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert_eq!(
            note.headings,
            vec![Heading {
                level: 2,
                text: "Section One".to_string(),
                slug: String::new(),
            }]
        );
        assert!(note.backlinks.is_empty());
    }

    #[test]
    fn note_data_tolerates_partial_media_and_available_modes() {
        let mut payload = base_note_payload();
        payload["note_type"] = json!("image");
        payload["available_modes"] = json!({
            "view": true
        });
        payload["media"] = json!({
            "file_path": "/tmp/example.png"
        });

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert_eq!(
            note.available_modes,
            Some(NoteModeAvailability {
                meta: false,
                source: false,
                edit: false,
                view: true,
            })
        );
        assert_eq!(
            note.media,
            Some(NoteMediaData {
                mime_type: String::new(),
                file_path: Some("/tmp/example.png".to_string()),
                thumbnail_path: None,
            })
        );
    }

    #[test]
    fn note_data_deserializes_non_empty_headings_and_backlinks() {
        let mut payload = base_note_payload();
        payload["headings"] = json!([
            {
                "level": 2,
                "text": "Section One",
                "slug": "section-one"
            }
        ]);
        payload["backlinks"] = json!([
            {
                "path": "notes/other.md",
                "title": "Other Note",
                "excerpt": "References Example"
            }
        ]);

        let note: NoteData =
            serde_json::from_value(payload).expect("note payload should deserialize");

        assert_eq!(
            note.headings,
            vec![Heading {
                level: 2,
                text: "Section One".to_string(),
                slug: "section-one".to_string(),
            }]
        );
        assert_eq!(
            note.backlinks,
            vec![Backlink {
                path: "notes/other.md".to_string(),
                title: "Other Note".to_string(),
                excerpt: Some("References Example".to_string()),
            }]
        );
    }

    #[test]
    fn graph_layout_deserializes_positioned_nodes() {
        let layout: GraphLayout = serde_json::from_value(json!({
            "nodes": [
                {
                    "id": "notes/example.md",
                    "label": "example",
                    "x": 12.5,
                    "y": 44.0
                }
            ],
            "edges": [
                {
                    "source": "notes/example.md",
                    "target": "notes/other.md"
                }
            ]
        }))
        .expect("graph layout should deserialize");

        assert_eq!(
            layout.nodes,
            vec![GraphNode {
                id: "notes/example.md".to_string(),
                label: "example".to_string(),
                x: 12.5,
                y: 44.0,
            }]
        );
        assert_eq!(
            layout.edges,
            vec![GraphEdge {
                source: "notes/example.md".to_string(),
                target: "notes/other.md".to_string(),
            }]
        );
    }

    #[test]
    fn graph_neighbors_deserializes_string_nodes() {
        let neighborhood: GraphNeighborhood = serde_json::from_value(json!({
            "nodes": ["notes/example.md", "notes/other.md"],
            "edges": [
                {
                    "source": "notes/example.md",
                    "target": "notes/other.md"
                }
            ]
        }))
        .expect("graph neighborhood should deserialize");

        assert_eq!(
            neighborhood.nodes,
            vec!["notes/example.md".to_string(), "notes/other.md".to_string()]
        );
        assert_eq!(
            neighborhood.edges,
            vec![GraphEdge {
                source: "notes/example.md".to_string(),
                target: "notes/other.md".to_string(),
            }]
        );
    }

    #[test]
    fn vault_plugin_info_deserializes_effective_state() {
        let plugins: Vec<VaultPluginInfo> = serde_json::from_value(json!([
            {
                "id": "daily-notes",
                "title": "Daily Notes",
                "enabled": true,
                "effective_enabled": false
            }
        ]))
        .expect("plugin list should deserialize");

        assert_eq!(
            plugins,
            vec![VaultPluginInfo {
                id: "daily-notes".to_string(),
                title: "Daily Notes".to_string(),
                enabled: true,
                effective_enabled: Some(false),
            }]
        );
    }

    #[test]
    fn maintenance_result_deserializes_message_and_count_shapes() {
        let message: MaintenanceResult =
            serde_json::from_value(json!("Reindex complete")).expect("message result");
        let count: MaintenanceResult = serde_json::from_value(json!(42)).expect("count result");

        assert_eq!(
            message,
            MaintenanceResult::Message("Reindex complete".to_string())
        );
        assert_eq!(count, MaintenanceResult::Count(42));
    }
}
