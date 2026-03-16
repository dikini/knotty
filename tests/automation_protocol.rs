use knot_gtk::ui::automation_controller::{
    clear_registration, register_protocol_api, UiAutomationApi,
};
use knot_gtk::ui::automation_protocol::{
    handle_registered_ui_automation_jsonrpc, handle_registered_ui_automation_tool_call,
};
use knot_gtk::ui::automation_state::{
    UiAutomationAction, UiAutomationActionResult, UiAutomationDescription, UiAutomationSnapshot,
};
use serde_json::json;
use std::cell::RefCell;
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
        result_codes: vec!["ok".to_string(), "automation_disabled".to_string()],
    })
}

fn snapshot() -> Option<UiAutomationSnapshot> {
    Some(UiAutomationSnapshot {
        active_tool: "settings".to_string(),
        active_content: "settings".to_string(),
        startup_state: "vault_open".to_string(),
        inspector_visible: false,
        active_note_path: Some("notes/example.md".to_string()),
        editor_mode: Some("view".to_string()),
        editor_dirty: false,
        search_query: None,
        graph_scope: Some("vault".to_string()),
        graph_depth: Some(1),
        graph_selected_path: None,
        settings_section: Some("plugins".to_string()),
        automation_active: true,
        properties: BTreeMap::from([
            ("tool.active".to_string(), "settings".to_string()),
            ("settings.section".to_string(), "plugins".to_string()),
        ]),
    })
}

fn dispatch(action: UiAutomationAction) -> Option<UiAutomationActionResult> {
    Some(UiAutomationActionResult {
        action_id: match action {
            UiAutomationAction::OpenSettingsSection { .. } => "open_settings_section".to_string(),
            _ => "unexpected".to_string(),
        },
        ok: true,
        result_code: "ok".to_string(),
        message: None,
        snapshot: snapshot(),
    })
}

fn protocol_api() -> UiAutomationApi {
    UiAutomationApi {
        describe,
        snapshot,
        dispatch,
    }
}

#[derive(Clone)]
struct FakeParityState {
    available: bool,
    unavailable_reason: Option<String>,
    startup_state: String,
    active_tool: String,
    active_content: String,
    inspector_visible: bool,
    active_note_path: Option<String>,
    editor_mode: Option<String>,
    editor_dirty: bool,
    search_query: Option<String>,
    graph_scope: Option<String>,
    graph_depth: Option<u8>,
    graph_selected_path: Option<String>,
    settings_section: Option<String>,
    automation_active: bool,
}

impl Default for FakeParityState {
    fn default() -> Self {
        Self {
            available: true,
            unavailable_reason: None,
            startup_state: "vault_open".to_string(),
            active_tool: "notes".to_string(),
            active_content: "empty".to_string(),
            inspector_visible: false,
            active_note_path: None,
            editor_mode: None,
            editor_dirty: false,
            search_query: None,
            graph_scope: Some("vault".to_string()),
            graph_depth: Some(1),
            graph_selected_path: None,
            settings_section: None,
            automation_active: true,
        }
    }
}

impl FakeParityState {
    fn snapshot(&self) -> UiAutomationSnapshot {
        let mut properties = BTreeMap::from([
            ("tool.active".to_string(), self.active_tool.clone()),
            ("content.active".to_string(), self.active_content.clone()),
            ("startup.state".to_string(), self.startup_state.clone()),
            ("editor.dirty".to_string(), self.editor_dirty.to_string()),
            (
                "automation.active".to_string(),
                self.automation_active.to_string(),
            ),
        ]);

        if let Some(editor_mode) = &self.editor_mode {
            properties.insert("editor.mode".to_string(), editor_mode.clone());
        }
        if let Some(section) = &self.settings_section {
            properties.insert("settings.section".to_string(), section.clone());
        }
        if let Some(scope) = &self.graph_scope {
            properties.insert("graph.scope".to_string(), scope.clone());
        }
        if let Some(depth) = self.graph_depth {
            properties.insert("graph.depth".to_string(), depth.to_string());
        }
        if let Some(path) = &self.active_note_path {
            properties.insert("note.path".to_string(), path.clone());
        }

        UiAutomationSnapshot {
            active_tool: self.active_tool.clone(),
            active_content: self.active_content.clone(),
            startup_state: self.startup_state.clone(),
            inspector_visible: self.inspector_visible,
            active_note_path: self.active_note_path.clone(),
            editor_mode: self.editor_mode.clone(),
            editor_dirty: self.editor_dirty,
            search_query: self.search_query.clone(),
            graph_scope: self.graph_scope.clone(),
            graph_depth: self.graph_depth,
            graph_selected_path: self.graph_selected_path.clone(),
            settings_section: self.settings_section.clone(),
            automation_active: self.automation_active,
            properties,
        }
    }

    fn ok_result(&self, action_id: &str) -> UiAutomationActionResult {
        UiAutomationActionResult {
            action_id: action_id.to_string(),
            ok: true,
            result_code: "ok".to_string(),
            message: None,
            snapshot: Some(self.snapshot()),
        }
    }

    fn error_result(
        &self,
        action_id: &str,
        result_code: &str,
        message: &str,
    ) -> UiAutomationActionResult {
        UiAutomationActionResult {
            action_id: action_id.to_string(),
            ok: false,
            result_code: result_code.to_string(),
            message: Some(message.to_string()),
            snapshot: None,
        }
    }

    fn dispatch(&mut self, action: UiAutomationAction) -> UiAutomationActionResult {
        let action_id = match &action {
            UiAutomationAction::SwitchTool { .. } => "switch_tool",
            UiAutomationAction::FocusSearch => "focus_search",
            UiAutomationAction::SelectNote { .. } => "select_note",
            UiAutomationAction::ClearSelection => "clear_selection",
            UiAutomationAction::SetEditorMode { .. } => "set_editor_mode",
            UiAutomationAction::OpenSettingsSection { .. } => "open_settings_section",
            UiAutomationAction::SetGraphScope { .. } => "set_graph_scope",
            UiAutomationAction::SetGraphDepth { .. } => "set_graph_depth",
            UiAutomationAction::ResetGraph => "reset_graph",
        };

        if !self.available {
            return self.error_result(
                action_id,
                "automation_disabled",
                "automation is disabled for this session",
            );
        }
        if self.startup_state != "vault_open" {
            return self.error_result(
                action_id,
                "startup_blocked",
                "vault must be open before automation can drive the shell",
            );
        }

        match action {
            UiAutomationAction::SwitchTool { tool } => {
                if !matches!(tool.as_str(), "notes" | "search" | "graph" | "settings") {
                    return self.error_result(action_id, "invalid_arguments", "unknown tool");
                }
                self.active_tool = tool.clone();
                self.active_content = match tool.as_str() {
                    "notes" => {
                        if self.active_note_path.is_some() {
                            "editor".to_string()
                        } else {
                            "empty".to_string()
                        }
                    }
                    "search" => "search".to_string(),
                    "graph" => "graph".to_string(),
                    "settings" => "settings".to_string(),
                    _ => unreachable!(),
                };
                self.inspector_visible = false;
                self.ok_result(action_id)
            }
            UiAutomationAction::FocusSearch => {
                self.active_tool = "search".to_string();
                self.active_content = "search".to_string();
                self.inspector_visible = false;
                self.ok_result(action_id)
            }
            UiAutomationAction::SelectNote { path } => {
                if self.editor_dirty {
                    return self.error_result(
                        action_id,
                        "dirty_guard_blocked",
                        "dirty note guard blocked note selection",
                    );
                }
                self.active_tool = "notes".to_string();
                self.active_content = "editor".to_string();
                self.active_note_path = Some(path);
                if self.editor_mode.is_none() {
                    self.editor_mode = Some("edit".to_string());
                }
                self.ok_result(action_id)
            }
            UiAutomationAction::ClearSelection => {
                if self.editor_dirty {
                    return self.error_result(
                        action_id,
                        "dirty_guard_blocked",
                        "dirty note guard blocked selection clear",
                    );
                }
                self.active_note_path = None;
                self.active_content = "empty".to_string();
                self.editor_mode = None;
                self.ok_result(action_id)
            }
            UiAutomationAction::SetEditorMode { mode } => {
                if self.active_note_path.is_none() {
                    return self.error_result(
                        action_id,
                        "unsupported_context",
                        "no active note is loaded",
                    );
                }
                if !matches!(mode.as_str(), "view" | "edit" | "source" | "meta") {
                    return self.error_result(
                        action_id,
                        "invalid_arguments",
                        "unknown editor mode",
                    );
                }
                self.editor_mode = Some(mode);
                self.ok_result(action_id)
            }
            UiAutomationAction::OpenSettingsSection { section } => {
                if !matches!(
                    section.as_str(),
                    "general" | "appearance" | "controls" | "vault" | "plugins" | "maintenance"
                ) {
                    return self.error_result(
                        action_id,
                        "invalid_arguments",
                        "unknown settings section",
                    );
                }
                self.active_tool = "settings".to_string();
                self.active_content = "settings".to_string();
                self.settings_section = Some(section);
                self.inspector_visible = false;
                self.ok_result(action_id)
            }
            UiAutomationAction::SetGraphScope { scope } => {
                if !matches!(scope.as_str(), "vault" | "neighborhood") {
                    return self.error_result(
                        action_id,
                        "invalid_arguments",
                        "unknown graph scope",
                    );
                }
                self.active_tool = "graph".to_string();
                self.active_content = "graph".to_string();
                self.graph_scope = Some(scope);
                self.ok_result(action_id)
            }
            UiAutomationAction::SetGraphDepth { depth } => {
                if depth == 0 {
                    return self.error_result(
                        action_id,
                        "invalid_arguments",
                        "graph depth must be at least 1",
                    );
                }
                self.active_tool = "graph".to_string();
                self.active_content = "graph".to_string();
                self.graph_depth = Some(depth);
                self.ok_result(action_id)
            }
            UiAutomationAction::ResetGraph => {
                self.active_tool = "graph".to_string();
                self.active_content = "graph".to_string();
                self.graph_scope = Some("vault".to_string());
                self.graph_depth = Some(1);
                self.graph_selected_path = None;
                self.ok_result(action_id)
            }
        }
    }
}

thread_local! {
    static FAKE_PARITY_STATE: RefCell<FakeParityState> = RefCell::new(FakeParityState::default());
}

fn reset_fake_parity_state() {
    FAKE_PARITY_STATE.with(|state| {
        *state.borrow_mut() = FakeParityState::default();
    });
}

fn with_fake_parity_state(f: impl FnOnce(&mut FakeParityState)) {
    FAKE_PARITY_STATE.with(|state| f(&mut state.borrow_mut()));
}

fn parity_describe() -> Option<UiAutomationDescription> {
    Some(FAKE_PARITY_STATE.with(|state| {
        let state = state.borrow();
        UiAutomationDescription {
            protocol_version: 1,
            snapshot_schema_version: 1,
            action_catalog_version: 1,
            available: state.available,
            unavailable_reason: state.unavailable_reason.clone(),
            requires_config_opt_in: true,
            requires_runtime_token: true,
            actions: vec![],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "dirty_guard_blocked".to_string(),
                "unsupported_context".to_string(),
                "invalid_arguments".to_string(),
            ],
        }
    }))
}

fn parity_snapshot() -> Option<UiAutomationSnapshot> {
    Some(FAKE_PARITY_STATE.with(|state| state.borrow().snapshot()))
}

fn parity_dispatch(action: UiAutomationAction) -> Option<UiAutomationActionResult> {
    Some(FAKE_PARITY_STATE.with(|state| state.borrow_mut().dispatch(action)))
}

fn parity_protocol_api() -> UiAutomationApi {
    UiAutomationApi {
        describe: parity_describe,
        snapshot: parity_snapshot,
        dispatch: parity_dispatch,
    }
}

#[test]
fn mocked_daemon_can_query_discovery_and_snapshot_via_protocol_adapter() {
    clear_registration();
    register_protocol_api(protocol_api());

    let description =
        handle_registered_ui_automation_tool_call("describe_ui_automation", json!({}))
            .expect("description should succeed");
    let snapshot = handle_registered_ui_automation_tool_call("get_ui_snapshot", json!({}))
        .expect("snapshot should succeed");

    assert_eq!(description["available"], true);
    assert_eq!(snapshot["properties"]["settings.section"], "plugins");

    clear_registration();
}

#[test]
fn mocked_daemon_can_dispatch_semantic_action_via_protocol_adapter() {
    clear_registration();
    register_protocol_api(protocol_api());

    let result = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "open_settings_section", "section": "plugins"}),
    )
    .expect("dispatch should succeed");

    assert_eq!(result["action_id"], "open_settings_section");
    assert_eq!(result["snapshot"]["active_tool"], "settings");

    clear_registration();
}

#[test]
fn registered_protocol_entrypoint_reports_missing_window_or_mock_explicitly() {
    clear_registration();

    let error = handle_registered_ui_automation_tool_call("describe_ui_automation", json!({}))
        .expect_err("missing registration should fail");

    assert_eq!(error.code(), "no_active_window");
}

#[test]
fn mocked_daemon_can_drive_registered_protocol_through_jsonrpc_tool_call_envelopes() {
    clear_registration();
    register_protocol_api(protocol_api());

    let description_response = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": {
            "name": "describe_ui_automation",
            "arguments": {}
        }
    }));
    let dispatch_response = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "tools/call",
        "params": {
            "name": "dispatch_ui_action",
            "arguments": {
                "action": "open_settings_section",
                "section": "plugins"
            }
        }
    }));

    let description_text = description_response["result"]["content"][0]["text"]
        .as_str()
        .expect("description text response");
    let dispatch_text = dispatch_response["result"]["content"][0]["text"]
        .as_str()
        .expect("dispatch text response");
    let description: serde_json::Value =
        serde_json::from_str(description_text).expect("description payload");
    let dispatch: serde_json::Value =
        serde_json::from_str(dispatch_text).expect("dispatch payload");

    assert_eq!(description_response["id"], 7);
    assert_eq!(description["available"], true);
    assert_eq!(dispatch_response["id"], 8);
    assert_eq!(dispatch["snapshot"]["settings_section"], "plugins");

    clear_registration();
}

#[test]
fn jsonrpc_protocol_reports_stable_error_codes_for_bad_requests() {
    clear_registration();

    let invalid_method = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "unsupported",
        "params": {
            "name": "describe_ui_automation",
            "arguments": {}
        }
    }));
    let missing_registration = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "describe_ui_automation",
            "arguments": {}
        }
    }));

    assert_eq!(invalid_method["error"]["data"]["code"], "invalid_request");
    assert_eq!(
        missing_registration["error"]["data"]["code"],
        "no_active_window"
    );
}

#[test]
fn parity_harness_covers_startup_gate_and_tool_switching() {
    clear_registration();
    reset_fake_parity_state();
    register_protocol_api(parity_protocol_api());

    with_fake_parity_state(|state| {
        state.startup_state = "no_vault".to_string();
    });

    let blocked = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "switch_tool", "tool": "settings"}),
    )
    .expect("dispatch should serialize");
    assert_eq!(blocked["result_code"], "startup_blocked");

    with_fake_parity_state(|state| {
        state.startup_state = "vault_open".to_string();
    });

    let switched = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "switch_tool", "tool": "settings"}),
    )
    .expect("switch tool should succeed");
    let snapshot = handle_registered_ui_automation_tool_call("get_ui_snapshot", json!({}))
        .expect("snapshot should succeed");

    assert_eq!(switched["result_code"], "ok");
    assert_eq!(snapshot["active_tool"], "settings");
    assert_eq!(snapshot["active_content"], "settings");
    assert_eq!(snapshot["inspector_visible"], false);

    clear_registration();
}

#[test]
fn parity_harness_covers_note_selection_editor_modes_and_dirty_guard() {
    clear_registration();
    reset_fake_parity_state();
    register_protocol_api(parity_protocol_api());

    let unsupported = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "set_editor_mode", "mode": "view"}),
    )
    .expect("set editor mode should serialize");
    assert_eq!(unsupported["result_code"], "unsupported_context");

    let selected = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "select_note", "path": "notes/example.md"}),
    )
    .expect("select note should succeed");
    assert_eq!(selected["snapshot"]["active_note_path"], "notes/example.md");
    assert_eq!(selected["snapshot"]["active_content"], "editor");

    let mode_changed = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "set_editor_mode", "mode": "view"}),
    )
    .expect("set editor mode should succeed");
    assert_eq!(mode_changed["snapshot"]["editor_mode"], "view");

    with_fake_parity_state(|state| {
        state.editor_dirty = true;
    });

    let blocked = handle_registered_ui_automation_tool_call(
        "dispatch_ui_action",
        json!({"action": "clear_selection"}),
    )
    .expect("clear selection should serialize");
    assert_eq!(blocked["result_code"], "dirty_guard_blocked");

    clear_registration();
}

#[test]
fn parity_harness_covers_settings_and_graph_actions_via_jsonrpc() {
    clear_registration();
    reset_fake_parity_state();
    register_protocol_api(parity_protocol_api());

    let settings_response = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "tools/call",
        "params": {
            "name": "dispatch_ui_action",
            "arguments": {
                "action": "open_settings_section",
                "section": "plugins"
            }
        }
    }));
    let depth_response = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "tools/call",
        "params": {
            "name": "dispatch_ui_action",
            "arguments": {
                "action": "set_graph_depth",
                "depth": 2
            }
        }
    }));
    let scope_response = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "tools/call",
        "params": {
            "name": "dispatch_ui_action",
            "arguments": {
                "action": "set_graph_scope",
                "scope": "neighborhood"
            }
        }
    }));
    let reset_response = handle_registered_ui_automation_jsonrpc(json!({
        "jsonrpc": "2.0",
        "id": 24,
        "method": "tools/call",
        "params": {
            "name": "dispatch_ui_action",
            "arguments": {
                "action": "reset_graph"
            }
        }
    }));

    let settings_payload: serde_json::Value = serde_json::from_str(
        settings_response["result"]["content"][0]["text"]
            .as_str()
            .expect("settings response text"),
    )
    .expect("settings payload");
    let depth_payload: serde_json::Value = serde_json::from_str(
        depth_response["result"]["content"][0]["text"]
            .as_str()
            .expect("depth response text"),
    )
    .expect("depth payload");
    let scope_payload: serde_json::Value = serde_json::from_str(
        scope_response["result"]["content"][0]["text"]
            .as_str()
            .expect("scope response text"),
    )
    .expect("scope payload");
    let reset_payload: serde_json::Value = serde_json::from_str(
        reset_response["result"]["content"][0]["text"]
            .as_str()
            .expect("reset response text"),
    )
    .expect("reset payload");

    assert_eq!(settings_response["id"], 21);
    assert_eq!(settings_payload["snapshot"]["settings_section"], "plugins");
    assert_eq!(settings_payload["snapshot"]["active_tool"], "settings");

    assert_eq!(depth_response["id"], 22);
    assert_eq!(depth_payload["snapshot"]["graph_depth"], 2);
    assert_eq!(depth_payload["snapshot"]["active_tool"], "graph");

    assert_eq!(scope_response["id"], 23);
    assert_eq!(scope_payload["snapshot"]["graph_scope"], "neighborhood");
    assert_eq!(
        scope_payload["snapshot"]["properties"]["graph.scope"],
        "neighborhood"
    );

    assert_eq!(reset_response["id"], 24);
    assert_eq!(reset_payload["snapshot"]["graph_scope"], "vault");
    assert_eq!(reset_payload["snapshot"]["graph_depth"], 1);

    clear_registration();
}
