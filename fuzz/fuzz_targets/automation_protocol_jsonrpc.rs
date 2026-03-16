#![no_main]

use knot_gtk::ui::automation_controller::{clear_registration, register_protocol_api, UiAutomationApi};
use knot_gtk::ui::automation_protocol::handle_registered_ui_automation_jsonrpc;
use knot_gtk::ui::automation_state::{
    UiAutomationAction, UiAutomationActionResult, UiAutomationDescription, UiAutomationSnapshot,
};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Once;

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
        result_codes: vec![
            "ok".to_string(),
            "automation_disabled".to_string(),
            "invalid_request".to_string(),
        ],
    })
}

fn snapshot() -> Option<UiAutomationSnapshot> {
    Some(UiAutomationSnapshot {
        active_tool: "notes".to_string(),
        active_content: "editor".to_string(),
        startup_state: "vault_open".to_string(),
        inspector_visible: true,
        active_note_path: Some("notes/example.md".to_string()),
        editor_mode: Some("view".to_string()),
        editor_dirty: false,
        search_query: None,
        graph_scope: Some("vault".to_string()),
        graph_depth: Some(1),
        graph_selected_path: None,
        settings_section: None,
        automation_active: true,
        properties: BTreeMap::from([("tool.active".to_string(), "notes".to_string())]),
    })
}

fn dispatch(action: UiAutomationAction) -> Option<UiAutomationActionResult> {
    Some(UiAutomationActionResult {
        action_id: match action {
            UiAutomationAction::SwitchTool { .. } => "switch_tool".to_string(),
            UiAutomationAction::FocusSearch => "focus_search".to_string(),
            UiAutomationAction::SelectNote { .. } => "select_note".to_string(),
            UiAutomationAction::ClearSelection => "clear_selection".to_string(),
            UiAutomationAction::SetEditorMode { .. } => "set_editor_mode".to_string(),
            UiAutomationAction::OpenSettingsSection { .. } => "open_settings_section".to_string(),
            UiAutomationAction::SetGraphScope { .. } => "set_graph_scope".to_string(),
            UiAutomationAction::SetGraphDepth { .. } => "set_graph_depth".to_string(),
            UiAutomationAction::ResetGraph => "reset_graph".to_string(),
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

fn ensure_registered() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        clear_registration();
        register_protocol_api(protocol_api());
    });
}

fuzz_target!(|data: &[u8]| {
    ensure_registered();

    let Ok(request) = serde_json::from_slice::<Value>(data) else {
        return;
    };

    let response = handle_registered_ui_automation_jsonrpc(request);

    let Some(jsonrpc) = response.get("jsonrpc").and_then(Value::as_str) else {
        panic!("missing jsonrpc field in response");
    };
    assert_eq!(jsonrpc, "2.0");

    if let Some(text) = response
        .get("result")
        .and_then(|result| result.get("content"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
    {
        let _: Value = serde_json::from_str(text).expect("result payload should remain valid JSON");
    } else {
        assert!(response.get("error").is_some(), "response must contain result or error");
        assert!(
            response
                .get("error")
                .and_then(|error| error.get("data"))
                .and_then(|data| data.get("code"))
                .and_then(Value::as_str)
                .is_some(),
            "error response must include a stable error code"
        );
    }
});
