# knotd Protocol Reference

## Purpose

Define the transport and message framing used by `knot-gtk` to communicate with `knotd`.

## Transport

- Unix domain socket
- JSON-RPC `2.0`
- request and response body framed with `Content-Length`

## Wire Format

```text
Content-Length: 153\r
\r
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_note","arguments":{"path":"notes/example.md"}}}
```

## Request Envelope

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_note",
    "arguments": {
      "path": "notes/example.md"
    }
  }
}
```

## Response Envelope

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"path\":\"notes/example.md\"}"
      }
    ]
  }
}
```

## Error Envelope

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "no vault open"
  }
}
```

## Communication Sequence

1. open Unix socket
2. encode JSON-RPC request body to bytes
3. write `Content-Length` header
4. write request body
5. read headers until empty line
6. parse `Content-Length`
7. read exact response body bytes
8. deserialize JSON-RPC response
9. handle `error` or parse `result.content[0].text`
10. deserialize that string into a typed Rust value

## Common Tool Calls

| Tool Name | Purpose | Primary Slice |
|---|---|---|
| `is_vault_open` | determine startup state | shell |
| `get_vault_info` | show current vault state | shell |
| `search_notes` | return search results | shell |
| `get_note` | load note payload | runtime, editor |
| `save_note` | persist note content | editor |
| `get_explorer_tree` | load explorer tree | explorer |
| `set_folder_expanded` | persist folder expansion state | explorer |
| `create_note` | create markdown note | explorer |
| `rename_note` | rename or move a note | explorer |
| `delete_note` | delete note | explorer |
| `get_graph_layout` | load graph layout | graph |
| `graph_neighbors` | load note neighborhood | graph |
| `get_vault_settings` | read settings | settings |
| `update_vault_settings` | write settings patch | settings |
| `list_vault_plugins` | load plugin states | settings |
| `reindex_vault` | maintenance action | settings |
| `describe_ui_automation` | discover GTK automation protocol | automation |
| `get_ui_snapshot` | read semantic GTK automation state | automation |
| `dispatch_ui_action` | drive semantic GTK automation actions | automation |

## Rust Type Templates

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'static str,
    pub params: ToolCallParams<T>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolCallParams<T> {
    pub name: String,
    pub arguments: T,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: Option<ToolCallResult>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContentItem>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ToolContentItem {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}
```

## Error Handling Rules

- treat socket connection failure as a recoverable application error
- treat malformed daemon payloads as decode errors with context
- do not mutate GTK widgets from the background thread that performed I/O
- do not add one-off protocol parsing in UI modules

## Test Advice

- unit test frame encoding and header parsing with byte strings
- unit test response decoding with explicit JSON fixtures
- integration test one client method per tool family
