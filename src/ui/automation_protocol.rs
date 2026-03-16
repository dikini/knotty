use serde_json::Value;

use crate::ui::automation_controller;
use crate::ui::automation_controller::UiAutomationApi;
use crate::ui::automation_state::UiAutomationAction;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UiAutomationToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UiAutomationJsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: UiAutomationToolCallParams,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAutomationProtocolError {
    NoActiveWindow,
    UnknownTool(String),
    InvalidArguments(String),
    InvalidRequest(String),
}

impl UiAutomationProtocolError {
    pub fn code(&self) -> &'static str {
        match self {
            UiAutomationProtocolError::NoActiveWindow => "no_active_window",
            UiAutomationProtocolError::UnknownTool(_) => "unknown_tool",
            UiAutomationProtocolError::InvalidArguments(_) => "invalid_arguments",
            UiAutomationProtocolError::InvalidRequest(_) => "invalid_request",
        }
    }

    pub fn message(&self) -> String {
        match self {
            UiAutomationProtocolError::NoActiveWindow => {
                "no active GTK window is registered for automation".to_string()
            }
            UiAutomationProtocolError::UnknownTool(name) => {
                format!("unknown GTK automation tool: {name}")
            }
            UiAutomationProtocolError::InvalidArguments(error) => {
                format!("invalid GTK automation arguments: {error}")
            }
            UiAutomationProtocolError::InvalidRequest(error) => {
                format!("invalid GTK automation request: {error}")
            }
        }
    }
}

pub fn handle_ui_automation_tool_call(
    api: &UiAutomationApi,
    name: &str,
    arguments: Value,
) -> Result<Value, UiAutomationProtocolError> {
    match name {
        "describe_ui_automation" => serialize_protocol_result(
            (api.describe)().ok_or(UiAutomationProtocolError::NoActiveWindow)?,
        ),
        "get_ui_snapshot" => serialize_protocol_result(
            (api.snapshot)().ok_or(UiAutomationProtocolError::NoActiveWindow)?,
        ),
        "dispatch_ui_action" => {
            let action = serde_json::from_value::<UiAutomationAction>(arguments)
                .map_err(|error| UiAutomationProtocolError::InvalidArguments(error.to_string()))?;
            serialize_protocol_result(
                (api.dispatch)(action).ok_or(UiAutomationProtocolError::NoActiveWindow)?,
            )
        }
        other => Err(UiAutomationProtocolError::UnknownTool(other.to_string())),
    }
}

pub fn handle_registered_ui_automation_tool_call(
    name: &str,
    arguments: Value,
) -> Result<Value, UiAutomationProtocolError> {
    let api = automation_controller::protocol_api();
    handle_ui_automation_tool_call(&api, name, arguments)
}

pub fn handle_registered_ui_automation_jsonrpc(request: Value) -> Value {
    match serde_json::from_value::<UiAutomationJsonRpcRequest>(request) {
        Ok(request) => {
            if request.jsonrpc != "2.0" {
                return jsonrpc_error(
                    request.id,
                    UiAutomationProtocolError::InvalidRequest(format!(
                        "unsupported jsonrpc version {}",
                        request.jsonrpc
                    )),
                );
            }

            if request.method != "tools/call" {
                return jsonrpc_error(
                    request.id,
                    UiAutomationProtocolError::InvalidRequest(format!(
                        "unsupported method {}",
                        request.method
                    )),
                );
            }

            match handle_registered_ui_automation_tool_call(
                &request.params.name,
                request.params.arguments,
            ) {
                Ok(result) => jsonrpc_result(request.id, result),
                Err(error) => jsonrpc_error(request.id, error),
            }
        }
        Err(error) => jsonrpc_error(
            Value::Null,
            UiAutomationProtocolError::InvalidRequest(error.to_string()),
        ),
    }
}

fn serialize_protocol_result<T: serde::Serialize>(
    result: T,
) -> Result<Value, UiAutomationProtocolError> {
    serde_json::to_value(result)
        .map_err(|error| UiAutomationProtocolError::InvalidArguments(error.to_string()))
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": result.to_string(),
                }
            ]
        }
    })
}

fn jsonrpc_error(id: Value, error: UiAutomationProtocolError) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32000,
            "message": error.message(),
            "data": {
                "code": error.code(),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::automation_state::{
        UiAutomationActionResult, UiAutomationDescription, UiAutomationSnapshot,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn describe() -> Option<UiAutomationDescription> {
        Some(UiAutomationDescription {
            protocol_version: 1,
            snapshot_schema_version: 1,
            action_catalog_version: 1,
            available: true,
            unavailable_reason: None,
            requires_config_opt_in: true,
            requires_runtime_token: true,
            actions: Vec::new(),
            result_codes: vec!["ok".to_string()],
        })
    }

    fn snapshot() -> Option<UiAutomationSnapshot> {
        Some(UiAutomationSnapshot {
            active_tool: "notes".to_string(),
            active_content: "editor".to_string(),
            startup_state: "vault_open".to_string(),
            inspector_visible: true,
            active_note_path: Some("notes/example.md".to_string()),
            editor_mode: Some("edit".to_string()),
            editor_dirty: false,
            search_query: None,
            graph_scope: None,
            graph_depth: None,
            graph_selected_path: None,
            settings_section: None,
            automation_active: true,
            properties: BTreeMap::new(),
        })
    }

    fn dispatch(action: UiAutomationAction) -> Option<UiAutomationActionResult> {
        Some(UiAutomationActionResult {
            action_id: match action {
                UiAutomationAction::FocusSearch => "focus_search".to_string(),
                _ => "other".to_string(),
            },
            ok: true,
            result_code: "ok".to_string(),
            message: None,
            snapshot: snapshot(),
        })
    }

    fn missing_description() -> Option<UiAutomationDescription> {
        None
    }

    fn missing_snapshot() -> Option<UiAutomationSnapshot> {
        None
    }

    fn missing_dispatch(_: UiAutomationAction) -> Option<UiAutomationActionResult> {
        None
    }

    fn mock_api() -> UiAutomationApi {
        UiAutomationApi {
            describe,
            snapshot,
            dispatch,
        }
    }

    fn missing_api() -> UiAutomationApi {
        UiAutomationApi {
            describe: missing_description,
            snapshot: missing_snapshot,
            dispatch: missing_dispatch,
        }
    }

    #[test]
    fn protocol_adapter_serializes_discovery_and_snapshot_tools() {
        let api = mock_api();

        let description = handle_ui_automation_tool_call(&api, "describe_ui_automation", json!({}))
            .expect("description should serialize");
        let snapshot = handle_ui_automation_tool_call(&api, "get_ui_snapshot", json!({}))
            .expect("snapshot should serialize");

        assert_eq!(description["protocol_version"], 1);
        assert_eq!(snapshot["active_note_path"], "notes/example.md");
    }

    #[test]
    fn protocol_adapter_parses_dispatch_actions_from_json() {
        let api = mock_api();

        let result = handle_ui_automation_tool_call(
            &api,
            "dispatch_ui_action",
            json!({"action": "focus_search"}),
        )
        .expect("dispatch should succeed");

        assert_eq!(result["action_id"], "focus_search");
        assert_eq!(result["result_code"], "ok");
    }

    #[test]
    fn protocol_adapter_rejects_unknown_tools_and_bad_arguments() {
        let api = mock_api();

        let unknown = handle_ui_automation_tool_call(&api, "unsupported", json!({}))
            .expect_err("unknown tool should fail");
        let invalid = handle_ui_automation_tool_call(
            &api,
            "dispatch_ui_action",
            json!({"action": "set_graph_depth", "depth": "bad"}),
        )
        .expect_err("invalid action payload should fail");

        assert_eq!(unknown.code(), "unknown_tool");
        assert_eq!(invalid.code(), "invalid_arguments");
    }

    #[test]
    fn protocol_adapter_reports_missing_active_window_explicitly() {
        let api = missing_api();

        let error = handle_ui_automation_tool_call(&api, "describe_ui_automation", json!({}))
            .expect_err("missing window should fail");

        assert_eq!(error.code(), "no_active_window");
    }
}
