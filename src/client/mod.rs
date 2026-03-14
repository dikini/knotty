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

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub fn is_connected(&self) -> bool {
        UnixStream::connect(&self.socket_path).is_ok()
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
        serde_json::from_str(text).map_err(|e| ClientError::Json(e))
    }

    /// List available tools from knotd
    pub fn list_tools(&self) -> Result<Vec<ToolDescriptor>> {
        let result = self.call_jsonrpc("tools/list", json!({}))?;
        let tools = result.get("tools").cloned().unwrap_or_else(|| json!([]));
        serde_json::from_value(tools).map_err(|e| ClientError::Json(e))
    }

    // ===== Vault Operations =====

    pub fn get_vault_info(&self) -> Result<VaultInfo> {
        let value = self.call_tool("get_vault_info", json!({}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn is_vault_open(&self) -> Result<bool> {
        let value = self.call_tool("is_vault_open", json!({}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn open_vault(&self, path: &str) -> Result<VaultInfo> {
        let value = self.call_tool("open_vault", json!({"path": path}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn create_vault(&self, path: &str) -> Result<VaultInfo> {
        let value = self.call_tool("create_vault", json!({"path": path}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn close_vault(&self) -> Result<()> {
        self.call_tool("close_vault", json!({}))?;
        Ok(())
    }

    // ===== Note Operations =====

    pub fn list_notes(&self) -> Result<Vec<NoteSummary>> {
        let value = self.call_tool("list_notes", json!({}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn get_note(&self, path: &str) -> Result<NoteData> {
        let value = self.call_tool("get_note", json!({"path": path}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
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
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
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

    pub fn get_recent_notes(&self, limit: Option<usize>) -> Result<Vec<NoteSummary>> {
        let mut args = json!({});
        if let Some(limit) = limit {
            args["limit"] = json!(limit);
        }
        let value = self.call_tool("get_recent_notes", args)?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    // ===== Search Operations =====

    pub fn search_notes(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>> {
        let mut args = json!({"query": query});
        if let Some(limit) = limit {
            args["limit"] = json!(limit);
        }
        let value = self.call_tool("search_notes", args)?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn search_suggestions(&self, query: &str, limit: Option<usize>) -> Result<Vec<String>> {
        let mut args = json!({"query": query});
        if let Some(limit) = limit {
            args["limit"] = json!(limit);
        }
        let value = self.call_tool("search_suggestions", args)?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    // ===== Explorer Operations =====

    pub fn get_explorer_tree(&self) -> Result<ExplorerTree> {
        let value = self.call_tool("get_explorer_tree", json!({}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
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
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn graph_neighbors(&self, path: &str, depth: Option<usize>) -> Result<Value> {
        let mut args = json!({"path": path});
        if let Some(depth) = depth {
            args["depth"] = json!(depth);
        }
        self.call_tool("graph_neighbors", args)
    }

    // ===== Settings Operations =====

    pub fn get_vault_settings(&self) -> Result<VaultSettings> {
        let value = self.call_tool("get_vault_settings", json!({}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }

    pub fn update_vault_settings(&self, patch: Value) -> Result<VaultSettings> {
        let value = self.call_tool("update_vault_settings", json!({"patch": patch}))?;
        serde_json::from_value(value).map_err(|e| ClientError::Json(e))
    }
}

impl Default for KnotdClient {
    fn default() -> Self {
        Self::new()
    }
}

fn default_socket_path() -> String {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir)
            .join("knot")
            .join("knot.sock")
            .to_string_lossy()
            .to_string()
    } else {
        "/run/user/1000/knot/knot.sock".to_string()
    }
}

// ===== Data Types =====

#[derive(Debug, Clone, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolSchema,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSchema {
    #[serde(rename = "type", default)]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub properties: Value,
    #[serde(default)]
    pub required: Vec<String>,
}

// Vault types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub path: String,
    pub name: String,
    #[serde(rename = "note_count")]
    pub note_count: usize,
    #[serde(rename = "last_modified")]
    pub last_modified: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSettings {
    pub name: String,
    #[serde(rename = "plugins_enabled")]
    pub plugins_enabled: bool,
    #[serde(rename = "file_visibility")]
    pub file_visibility: String,
    pub editor: VaultEditorSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEditorSettings {
    #[serde(rename = "font_size")]
    pub font_size: i32,
    #[serde(rename = "tab_size")]
    pub tab_size: i32,
}

// Note types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(rename = "created_at")]
    pub created_at: i64,
    #[serde(rename = "modified_at")]
    pub modified_at: i64,
    #[serde(rename = "word_count")]
    pub word_count: usize,
    #[serde(rename = "note_type")]
    pub note_type: Option<NoteType>,
    #[serde(rename = "type_badge")]
    pub type_badge: Option<String>,
    #[serde(default)]
    pub is_dimmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub headings: Vec<Heading>,
    pub backlinks: Vec<Backlink>,
    #[serde(rename = "note_type")]
    pub note_type: Option<NoteType>,
    #[serde(rename = "type_badge")]
    pub type_badge: Option<String>,
    #[serde(default)]
    pub is_dimmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Markdown,
    Youtube,
    Pdf,
    Image,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: i32,
    pub text: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    #[serde(rename = "source_path")]
    pub source_path: String,
    #[serde(rename = "source_title")]
    pub source_title: String,
    pub context: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLayout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}
