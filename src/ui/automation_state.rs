use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAutomationSnapshot {
    pub active_tool: String,
    pub active_content: String,
    pub startup_state: String,
    pub inspector_visible: bool,
    pub active_note_path: Option<String>,
    pub editor_mode: Option<String>,
    pub editor_dirty: bool,
    pub search_query: Option<String>,
    pub graph_scope: Option<String>,
    pub graph_depth: Option<u8>,
    pub graph_selected_path: Option<String>,
    pub settings_section: Option<String>,
    pub automation_active: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAutomationDescription {
    pub protocol_version: u32,
    pub snapshot_schema_version: u32,
    pub action_catalog_version: u32,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub requires_config_opt_in: bool,
    pub requires_runtime_token: bool,
    pub actions: Vec<UiAutomationActionDescription>,
    pub result_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAutomationActionDescription {
    pub action_id: String,
    pub title: String,
    pub description: String,
    pub argument_schema: Value,
    pub preconditions: Vec<String>,
    pub result_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiAutomationAction {
    SwitchTool { tool: String },
    FocusSearch,
    SelectNote { path: String },
    ClearSelection,
    SetEditorMode { mode: String },
    OpenSettingsSection { section: String },
    SetGraphScope { scope: String },
    SetGraphDepth { depth: u8 },
    ResetGraph,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAutomationActionResult {
    pub action_id: String,
    pub ok: bool,
    pub result_code: String,
    pub message: Option<String>,
    pub snapshot: Option<UiAutomationSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_snapshot_serializes_with_property_map() {
        let mut properties = BTreeMap::new();
        properties.insert("tool.active".to_string(), "settings".to_string());
        let snapshot = UiAutomationSnapshot {
            active_tool: "settings".to_string(),
            active_content: "settings".to_string(),
            startup_state: "vault_open".to_string(),
            inspector_visible: false,
            active_note_path: Some("notes/welcome.md".to_string()),
            editor_mode: Some("view".to_string()),
            editor_dirty: false,
            search_query: None,
            graph_scope: None,
            graph_depth: None,
            graph_selected_path: None,
            settings_section: Some("plugins".to_string()),
            automation_active: true,
            properties,
        };

        let value = serde_json::to_value(&snapshot).expect("snapshot should serialize");

        assert_eq!(value["active_tool"], "settings");
        assert_eq!(value["properties"]["tool.active"], "settings");
    }

    #[test]
    fn automation_description_includes_action_catalog_and_result_codes() {
        let description = UiAutomationDescription {
            protocol_version: 1,
            snapshot_schema_version: 1,
            action_catalog_version: 1,
            available: false,
            unavailable_reason: Some("automation_disabled".to_string()),
            requires_config_opt_in: true,
            requires_runtime_token: true,
            actions: vec![UiAutomationActionDescription {
                action_id: "switch_tool".to_string(),
                title: "Switch Tool".to_string(),
                description: "Switch the active shell tool.".to_string(),
                argument_schema: serde_json::json!({
                    "type": "object",
                    "required": ["tool"],
                }),
                preconditions: vec!["startup.state == vault_open".to_string()],
                result_codes: vec!["ok".to_string(), "startup_blocked".to_string()],
            }],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
            ],
        };

        let value = serde_json::to_value(&description).expect("description should serialize");

        assert_eq!(value["actions"][0]["action_id"], "switch_tool");
        assert_eq!(value["result_codes"][1], "automation_disabled");
    }

    #[test]
    fn action_result_serializes_stable_result_code_and_snapshot() {
        let result = UiAutomationActionResult {
            action_id: "focus_search".to_string(),
            ok: false,
            result_code: "startup_blocked".to_string(),
            message: Some("vault must be open".to_string()),
            snapshot: None,
        };

        let value = serde_json::to_value(&result).expect("result should serialize");

        assert_eq!(value["action_id"], "focus_search");
        assert_eq!(value["result_code"], "startup_blocked");
        assert_eq!(value["ok"], false);
    }
}
