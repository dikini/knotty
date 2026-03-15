use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::client::{GraphLayout, KnotdClient, NoteData};
use crate::config::knotty_config::{load_knotty_config, ColorSchemePreference, KnottyConfig};
use crate::ui::async_bridge;
use crate::ui::automation_state::{
    UiAutomationAction, UiAutomationActionDescription, UiAutomationActionResult,
    UiAutomationDescription, UiAutomationSnapshot,
};
use crate::ui::context_panel::{ContextPanel, GraphPanelEvent, GraphPanelState};
use crate::ui::editor::{EditorMode, NoteEditor};
use crate::ui::explorer::NoteSwitchDecision;
use crate::ui::graph_view::{
    graph_context_details, normalize_neighborhood_layout, normalize_vault_layout, GraphScene,
    GraphScope, GraphView,
};
use crate::ui::inspector_rail::InspectorRail;
use crate::ui::request_state::RequestState;
use crate::ui::search_view::SearchView;
use crate::ui::settings_view::{SettingsSection, SettingsView};
use crate::ui::shell_state::{InspectorMode, ShellState};
use crate::ui::tool_rail::{ToolMode, ToolRail};
use crate::{AUTOMATION_RUNTIME_ENABLED, AUTOMATION_RUNTIME_TOKEN};

type NoteLoadState = RequestState<NoteData, String>;
type NoteLoadResult = Result<NoteData, String>;
type NoteLoadDispatcher = Rc<dyn Fn(NoteLoadOrigin, &str)>;
type GraphLoadResult = Result<(GraphScene, Option<GraphLayout>), String>;
#[cfg(test)]
type DeferredUiCallback = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteLoadOrigin {
    Context,
    Search,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirtyNoteSwitchResponse {
    Cancel,
    Discard,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupAction {
    RetryDaemon,
    OpenVault,
    CreateVault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StartupState {
    DaemonUnavailable { message: String },
    NoVault,
    VaultOpen { name: Option<String> },
}

pub struct KnotWindow {
    window: libadwaita::ApplicationWindow,
    client: Rc<KnotdClient>,
    tool_rail: ToolRail,
    context_panel: Rc<ContextPanel>,
    inspector_rail: InspectorRail,
    startup_state: Rc<RefCell<StartupState>>,
    automation_indicator: gtk::Label,
    vault_label: gtk::Label,
    daemon_detail_label: gtk::Label,
    retry_startup_btn: gtk::Button,
    open_vault_btn: gtk::Button,
    create_vault_btn: gtk::Button,
    editor: Rc<NoteEditor>,
    search_view: Rc<SearchView>,
    graph_view: Rc<GraphView>,
    settings_view: Rc<SettingsView>,
    content_stack: gtk::Stack,
    current_note: Rc<RefCell<Option<NoteData>>>,
    note_load_state: Rc<RefCell<NoteLoadState>>,
    note_load_generation: Rc<RefCell<u64>>,
    shell_state: Rc<RefCell<ShellState>>,
    graph_scope: Rc<RefCell<GraphScope>>,
    graph_depth: Rc<Cell<u32>>,
    graph_selected_path: Rc<RefCell<Option<String>>>,
    graph_vault_layout: Rc<RefCell<Option<GraphLayout>>>,
    graph_current_scene: Rc<RefCell<GraphScene>>,
    graph_request_generation: Rc<Cell<u64>>,
}

#[derive(Clone)]
struct ShellUiHandles {
    shell_state: Rc<RefCell<ShellState>>,
    tool_rail: ToolRail,
    context_panel: Rc<ContextPanel>,
    content_stack: gtk::Stack,
    inspector_rail: InspectorRail,
    search_view: Rc<SearchView>,
}

#[derive(Clone)]
struct StartupRefreshHandles {
    client: Rc<KnotdClient>,
    startup_state: Rc<RefCell<StartupState>>,
    vault_label: gtk::Label,
    daemon_detail_label: gtk::Label,
    retry_startup_btn: gtk::Button,
    open_vault_btn: gtk::Button,
    create_vault_btn: gtk::Button,
    shell_ui: ShellUiHandles,
}

#[derive(Clone)]
struct NoteSessionHandles {
    window: libadwaita::ApplicationWindow,
    editor: Rc<NoteEditor>,
    current_note: Rc<RefCell<Option<NoteData>>>,
    note_load_state: Rc<RefCell<NoteLoadState>>,
    note_load_generation: Rc<RefCell<u64>>,
    shell_ui: ShellUiHandles,
}

#[derive(Clone)]
struct GraphSessionHandles {
    client: Rc<KnotdClient>,
    graph_view: Rc<GraphView>,
    context_panel: Rc<ContextPanel>,
    scope: Rc<RefCell<GraphScope>>,
    depth: Rc<Cell<u32>>,
    selected_path: Rc<RefCell<Option<String>>>,
    vault_layout: Rc<RefCell<Option<GraphLayout>>>,
    current_scene: Rc<RefCell<GraphScene>>,
    request_generation: Rc<Cell<u64>>,
}

fn update_note_load_state(state: &Rc<RefCell<NoteLoadState>>, result: &NoteLoadResult) {
    *state.borrow_mut() = match result {
        Ok(note) => RequestState::success(note.clone()),
        Err(error) => RequestState::error(error.clone()),
    };
}

fn determine_startup_state(client: &KnotdClient) -> StartupState {
    match client.is_vault_open() {
        Ok(true) => match client.get_vault_info() {
            Ok(info) => StartupState::VaultOpen {
                name: Some(info.name),
            },
            Err(error) => {
                log::error!(
                    "Failed to load vault info while daemon is reachable: {}",
                    error
                );
                StartupState::VaultOpen { name: None }
            }
        },
        Ok(false) => StartupState::NoVault,
        Err(error) => StartupState::DaemonUnavailable {
            message: error.to_string(),
        },
    }
}

fn startup_header_text(state: &StartupState) -> String {
    match state {
        StartupState::DaemonUnavailable { .. } => "knotd unavailable".to_string(),
        StartupState::NoVault => "No vault open".to_string(),
        StartupState::VaultOpen { name: Some(name) } => format!("Connected to vault: {name}"),
        StartupState::VaultOpen { name: None } => "Connected to vault".to_string(),
    }
}

fn startup_detail_text(state: &StartupState) -> String {
    match state {
        StartupState::DaemonUnavailable { message } => message.clone(),
        StartupState::NoVault => "Open or create a vault to start browsing notes.".to_string(),
        StartupState::VaultOpen { .. } => String::new(),
    }
}

fn startup_content_child_name(state: &StartupState) -> &'static str {
    match state {
        StartupState::DaemonUnavailable { .. } => "daemon-unavailable",
        StartupState::NoVault => "no-vault",
        StartupState::VaultOpen { .. } => "empty",
    }
}

fn startup_shell_chrome_visible(state: &StartupState) -> bool {
    matches!(state, StartupState::VaultOpen { .. })
}

fn startup_action_specs(state: &StartupState) -> &'static [StartupAction] {
    match state {
        StartupState::DaemonUnavailable { .. } => &[StartupAction::RetryDaemon],
        StartupState::NoVault => &[StartupAction::OpenVault, StartupAction::CreateVault],
        StartupState::VaultOpen { .. } => &[],
    }
}

fn startup_action_label(action: StartupAction) -> &'static str {
    match action {
        StartupAction::RetryDaemon => "Retry after starting knotd",
        StartupAction::OpenVault => "Open vault",
        StartupAction::CreateVault => "Create vault",
    }
}

#[cfg(test)]
fn content_stack_child_names() -> &'static [&'static str] {
    &[
        "empty",
        "daemon-unavailable",
        "no-vault",
        "editor",
        "search",
        "graph",
        "settings",
    ]
}

fn content_child_name_for_shell(shell_state: &ShellState) -> &'static str {
    match shell_state.content_mode() {
        crate::ui::shell_state::ContentMode::Welcome => "empty",
        crate::ui::shell_state::ContentMode::Note => "editor",
        crate::ui::shell_state::ContentMode::Search => "search",
        crate::ui::shell_state::ContentMode::Graph => "graph",
        crate::ui::shell_state::ContentMode::Settings => "settings",
    }
}

fn apply_shell_state(
    shell_state: &ShellState,
    tool_rail: &ToolRail,
    context_panel: &ContextPanel,
    content_stack: &gtk::Stack,
    inspector_rail: &InspectorRail,
    search_view: &SearchView,
) {
    tool_rail.set_active_mode(shell_state.tool_mode());
    context_panel.set_mode(shell_state.tool_mode());
    content_stack.set_visible_child_name(content_child_name_for_shell(shell_state));
    if matches!(shell_state.tool_mode(), ToolMode::Search) {
        search_view.grab_focus();
    }
    match shell_state.inspector_mode() {
        InspectorMode::Hidden => inspector_rail.set_open(false),
        InspectorMode::Details => {
            inspector_rail.set_open(true);
            inspector_rail.set_mode("details");
        }
    }
}

fn apply_knotty_config(
    config: &KnottyConfig,
    context_panel: &ContextPanel,
    inspector_rail: &InspectorRail,
) {
    context_panel
        .widget()
        .set_width_request(config.appearance.context_panel_width);
    inspector_rail
        .widget()
        .set_width_request(config.appearance.inspector_width);

    let color_scheme = match config.appearance.color_scheme {
        ColorSchemePreference::System => libadwaita::ColorScheme::Default,
        ColorSchemePreference::Light => libadwaita::ColorScheme::ForceLight,
        ColorSchemePreference::Dark => libadwaita::ColorScheme::ForceDark,
    };
    libadwaita::StyleManager::default().set_color_scheme(color_scheme);
}

fn apply_startup_state(state: &StartupState, handles: &StartupRefreshHandles) {
    *handles.startup_state.borrow_mut() = state.clone();
    handles.vault_label.set_label(&startup_header_text(state));
    handles
        .daemon_detail_label
        .set_label(&startup_detail_text(state));
    let startup_actions = startup_action_specs(state);
    handles
        .retry_startup_btn
        .set_visible(startup_actions.contains(&StartupAction::RetryDaemon));
    handles
        .open_vault_btn
        .set_visible(startup_actions.contains(&StartupAction::OpenVault));
    handles
        .create_vault_btn
        .set_visible(startup_actions.contains(&StartupAction::CreateVault));

    let shell_chrome_visible = startup_shell_chrome_visible(state);
    handles
        .shell_ui
        .tool_rail
        .widget()
        .set_visible(shell_chrome_visible);
    handles
        .shell_ui
        .context_panel
        .widget()
        .set_visible(shell_chrome_visible);
    handles
        .shell_ui
        .inspector_rail
        .widget()
        .set_visible(shell_chrome_visible);

    if shell_chrome_visible {
        let shell_state = handles.shell_ui.shell_state.borrow();
        apply_shell_state(
            &shell_state,
            &handles.shell_ui.tool_rail,
            handles.shell_ui.context_panel.as_ref(),
            &handles.shell_ui.content_stack,
            &handles.shell_ui.inspector_rail,
            handles.shell_ui.search_view.as_ref(),
        );
    } else {
        handles
            .shell_ui
            .content_stack
            .set_visible_child_name(startup_content_child_name(state));
        handles.shell_ui.inspector_rail.set_open(false);
    }
}

fn refresh_startup_shell(handles: &StartupRefreshHandles) {
    let startup_state = determine_startup_state(handles.client.as_ref());
    apply_startup_state(&startup_state, handles);
}

fn choose_vault_directory<F>(
    window: &libadwaita::ApplicationWindow,
    title: &str,
    accept_label: &str,
    on_selected: F,
) where
    F: FnOnce(String) + 'static,
{
    let dialog = gtk::FileDialog::builder().title(title).modal(true).build();

    let action_label = accept_label.to_string();
    let on_selected = Rc::new(RefCell::new(Some(on_selected)));
    dialog.select_folder(
        Some(window),
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(folder) => {
                if let Some(path) = folder.path() {
                    if let Some(callback) = on_selected.borrow_mut().take() {
                        callback(path.to_string_lossy().into_owned());
                    }
                }
            }
            Err(error) => {
                log::info!(
                    "Folder selection cancelled or failed for {}: {}",
                    action_label,
                    error
                );
            }
        },
    );
}

fn begin_note_load_with_dispatch<Dispatch, OnResult>(
    client: KnotdClient,
    path: String,
    state: Rc<RefCell<NoteLoadState>>,
    generation: u64,
    active_generation: Rc<RefCell<u64>>,
    dispatch: Dispatch,
    on_result: OnResult,
) where
    Dispatch: FnOnce(Box<dyn FnOnce() -> NoteLoadResult + Send>, Box<dyn FnOnce(NoteLoadResult)>),
    OnResult: FnOnce(NoteLoadResult) + 'static,
{
    *state.borrow_mut() = RequestState::loading();

    let state_for_ui = Rc::clone(&state);
    dispatch(
        Box::new(move || client.get_note(&path).map_err(|error| error.to_string())),
        Box::new(move |result| {
            if *active_generation.borrow() != generation {
                return;
            }
            update_note_load_state(&state_for_ui, &result);
            on_result(result);
        }),
    );
}

fn build_note_load_dispatcher(
    client: Rc<KnotdClient>,
    session: NoteSessionHandles,
) -> NoteLoadDispatcher {
    Rc::new(move |origin, path| {
        log::info!("Loading note: {}", path);
        session.window.set_title(Some("Loading note... — Knot"));
        let load_path = path.to_string();
        let log_path = load_path.clone();
        let generation = {
            let mut current = session.note_load_generation.borrow_mut();
            *current += 1;
            *current
        };

        begin_note_load_with_dispatch(
            client.as_ref().clone(),
            load_path,
            Rc::clone(&session.note_load_state),
            generation,
            Rc::clone(&session.note_load_generation),
            |work, ui| {
                async_bridge::run_background(work).attach_local(move |result| {
                    ui(result);
                });
            },
            {
                let session = session.clone();
                move |result| match result {
                    Ok(note) => {
                        session.editor.load_note(&note);
                        let title = session.editor.current_title();
                        sync_window_title(&session.window, Some(&title), false);
                        *session.current_note.borrow_mut() = Some(note);
                        let mut shell_state = session.shell_ui.shell_state.borrow_mut();
                        shell_state.set_note_selected(true);
                        if should_route_loaded_note_to_notes(origin, shell_state.tool_mode()) {
                            shell_state.select_tool(ToolMode::Notes);
                            apply_shell_state(
                                &shell_state,
                                &session.shell_ui.tool_rail,
                                session.shell_ui.context_panel.as_ref(),
                                &session.shell_ui.content_stack,
                                &session.shell_ui.inspector_rail,
                                session.shell_ui.search_view.as_ref(),
                            );
                        }
                    }
                    Err(error) => {
                        session.window.set_title(Some("Failed to load note — Knot"));
                        log::error!("Failed to load note {}: {}", log_path, error);
                    }
                }
            },
        );
    })
}

#[derive(Clone)]
struct NoteSwitchPromptHandles {
    window: libadwaita::ApplicationWindow,
    editor: Rc<NoteEditor>,
    prompt_open: Rc<Cell<bool>>,
    dispatch_note_load: NoteLoadDispatcher,
}

fn present_dirty_note_switch_prompt(
    handles: &NoteSwitchPromptHandles,
    origin: NoteLoadOrigin,
    path: &str,
) {
    if handles.prompt_open.replace(true) {
        return;
    }

    let dialog = libadwaita::AlertDialog::new(
        Some("Unsaved changes"),
        Some("Save the current note before switching, discard your changes, or keep editing."),
    );
    dialog.add_responses(&[
        ("cancel", "Keep editing"),
        ("discard", "Discard changes"),
        ("save", "Save and switch"),
    ]);
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("save"));
    dialog.set_response_appearance("discard", libadwaita::ResponseAppearance::Destructive);
    dialog.set_response_appearance("save", libadwaita::ResponseAppearance::Suggested);

    let prompt_open = Rc::clone(&handles.prompt_open);
    let editor = Rc::clone(&handles.editor);
    let dispatch_note_load = Rc::clone(&handles.dispatch_note_load);
    let path = path.to_string();
    dialog.choose(
        &handles.window,
        None::<&gio::Cancellable>,
        move |response| {
            prompt_open.set(false);
            match dirty_note_switch_response(response.as_str()) {
                DirtyNoteSwitchResponse::Cancel => {}
                DirtyNoteSwitchResponse::Discard => {
                    editor.discard_changes();
                    dispatch_note_load(origin, &path);
                }
                DirtyNoteSwitchResponse::Save => {
                    if let Err(error) = editor.save() {
                        log::error!("Failed to save note before switching: {}", error);
                        return;
                    }
                    dispatch_note_load(origin, &path);
                }
            }
        },
    );
}

fn should_route_loaded_note_to_notes(origin: NoteLoadOrigin, tool_mode: ToolMode) -> bool {
    matches!(tool_mode, ToolMode::Notes)
        || matches!(origin, NoteLoadOrigin::Graph)
        || (matches!(origin, NoteLoadOrigin::Search) && matches!(tool_mode, ToolMode::Search))
}

fn dirty_note_switch_response(response: &str) -> DirtyNoteSwitchResponse {
    match response {
        "discard" => DirtyNoteSwitchResponse::Discard,
        "save" => DirtyNoteSwitchResponse::Save,
        _ => DirtyNoteSwitchResponse::Cancel,
    }
}

fn focus_search_shell_state(shell_state: &mut ShellState) {
    shell_state.select_tool(ToolMode::Search);
}

fn focus_search_shell(
    shell_state: &mut ShellState,
    tool_rail: &ToolRail,
    context_panel: &ContextPanel,
    content_stack: &gtk::Stack,
    inspector_rail: &InspectorRail,
    search_view: &SearchView,
) {
    focus_search_shell_state(shell_state);
    apply_shell_state(
        shell_state,
        tool_rail,
        context_panel,
        content_stack,
        inspector_rail,
        search_view,
    );
}

fn note_window_title(note_title: Option<&str>, modified: bool) -> String {
    match note_title {
        Some(title) if modified => format!("*{} — Knot", title),
        Some(title) => format!("{} — Knot", title),
        None => "Knot".to_string(),
    }
}

fn sync_window_title(
    window: &libadwaita::ApplicationWindow,
    note_title: Option<&str>,
    modified: bool,
) {
    let title = note_window_title(note_title, modified);
    window.set_title(Some(&title));
}

fn clear_active_note(session: &NoteSessionHandles) {
    cancel_note_load(&session.note_load_state, &session.note_load_generation);
    sync_window_title(&session.window, None, false);
    session.editor.clear();
    *session.current_note.borrow_mut() = None;

    let mut shell_state = session.shell_ui.shell_state.borrow_mut();
    shell_state.set_note_selected(false);
    apply_shell_state(
        &shell_state,
        &session.shell_ui.tool_rail,
        session.shell_ui.context_panel.as_ref(),
        &session.shell_ui.content_stack,
        &session.shell_ui.inspector_rail,
        session.shell_ui.search_view.as_ref(),
    );
}

fn graph_panel_state(scope: GraphScope, depth: u32, scene: &GraphScene) -> GraphPanelState {
    GraphPanelState {
        scope,
        depth,
        load_state: scene.load_state.clone(),
        details: graph_context_details(scene),
    }
}

fn sync_graph_ui(session: &GraphSessionHandles, scene: GraphScene) {
    let scope = *session.scope.borrow();
    let depth = session.depth.get();
    session.graph_view.set_scene(scope, scene.clone());
    session
        .context_panel
        .set_graph_state(&graph_panel_state(scope, depth, &scene));
    *session.current_scene.borrow_mut() = scene;
}

fn set_graph_selection(session: &GraphSessionHandles, path: Option<String>) {
    *session.selected_path.borrow_mut() = path.clone();
    let mut scene = session.current_scene.borrow().clone();
    scene.selected_path = path;
    sync_graph_ui(session, scene);
}

fn request_graph_scene(
    client: &KnotdClient,
    scope: GraphScope,
    selected_path: Option<String>,
    depth: u32,
    vault_layout: Option<GraphLayout>,
) -> GraphLoadResult {
    match scope {
        GraphScope::Vault => {
            let mut scene = normalize_vault_layout(
                client
                    .get_graph_layout(1200.0, 800.0)
                    .map_err(|error| error.to_string())?,
            );
            scene.selected_path = selected_path;
            let layout = GraphLayout {
                nodes: scene.nodes.clone(),
                edges: scene.edges.clone(),
            };
            Ok((scene, Some(layout)))
        }
        GraphScope::Neighborhood => {
            let Some(path) = selected_path else {
                return Ok((
                    GraphScene::error("Select a node to focus the graph", None),
                    None,
                ));
            };
            let neighborhood = client
                .graph_neighbors(&path, Some(depth as usize))
                .map_err(|error| error.to_string())?;
            let scene =
                normalize_neighborhood_layout(neighborhood, vault_layout.as_ref(), Some(&path));
            Ok((scene, None))
        }
    }
}

fn load_graph_with_dispatch<Dispatch>(session: GraphSessionHandles, dispatch: Dispatch)
where
    Dispatch: FnOnce(Box<dyn FnOnce() -> GraphLoadResult + Send>, Box<dyn FnOnce(GraphLoadResult)>),
{
    let scope = *session.scope.borrow();
    let depth = session.depth.get();
    let selected_path = session.selected_path.borrow().clone();
    let loading_scene = GraphScene::loading(selected_path.clone());
    sync_graph_ui(&session, loading_scene);

    let generation = session.request_generation.get() + 1;
    session.request_generation.set(generation);
    let client = session.client.as_ref().clone();
    let vault_layout = session.vault_layout.borrow().clone();

    dispatch(
        Box::new(move || request_graph_scene(&client, scope, selected_path, depth, vault_layout)),
        Box::new(move |result| {
            if session.request_generation.get() != generation {
                return;
            }
            match result {
                Ok((scene, new_vault_layout)) => {
                    if let Some(layout) = new_vault_layout {
                        *session.vault_layout.borrow_mut() = Some(layout);
                    }
                    sync_graph_ui(&session, scene);
                }
                Err(error) => {
                    sync_graph_ui(
                        &session,
                        GraphScene::error(error, session.selected_path.borrow().clone()),
                    );
                }
            }
        }),
    );
}

fn load_graph(session: GraphSessionHandles) {
    load_graph_with_dispatch(session, |work, ui| {
        async_bridge::run_background(work).attach_local(move |result| {
            ui(result);
        });
    });
}

fn ensure_graph_loaded(session: GraphSessionHandles) {
    let state = session.current_scene.borrow().load_state.clone();
    if matches!(state, crate::ui::graph_view::GraphLoadState::Idle) {
        load_graph(session);
    } else {
        let scene = session.current_scene.borrow().clone();
        sync_graph_ui(&session, scene);
    }
}

#[cfg(test)]
fn resolve_note_switch_decision<Save>(
    is_modified: bool,
    requested: NoteSwitchDecision,
    save_current_note: Save,
) -> NoteSwitchDecision
where
    Save: FnOnce() -> Result<(), String>,
{
    if !is_modified {
        return NoteSwitchDecision::Allow;
    }

    match requested {
        NoteSwitchDecision::Allow => NoteSwitchDecision::Allow,
        NoteSwitchDecision::Deny => NoteSwitchDecision::Deny,
        NoteSwitchDecision::SaveThenAllow => match save_current_note() {
            Ok(()) => NoteSwitchDecision::Allow,
            Err(error) => {
                log::error!("Failed to save note before switching: {}", error);
                NoteSwitchDecision::Deny
            }
        },
        NoteSwitchDecision::Prompt => NoteSwitchDecision::Prompt,
    }
}

fn cancel_note_load(
    note_load_state: &Rc<RefCell<NoteLoadState>>,
    note_load_generation: &Rc<RefCell<u64>>,
) {
    *note_load_generation.borrow_mut() += 1;
    *note_load_state.borrow_mut() = RequestState::idle();
}

const UI_AUTOMATION_PROTOCOL_VERSION: u32 = 1;
const UI_AUTOMATION_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const UI_AUTOMATION_ACTION_CATALOG_VERSION: u32 = 1;

fn startup_state_name(state: &StartupState) -> &'static str {
    match state {
        StartupState::DaemonUnavailable { .. } => "daemon_unavailable",
        StartupState::NoVault => "no_vault",
        StartupState::VaultOpen { .. } => "vault_open",
    }
}

fn tool_mode_name(mode: ToolMode) -> &'static str {
    match mode {
        ToolMode::Notes => "notes",
        ToolMode::Search => "search",
        ToolMode::Graph => "graph",
        ToolMode::Settings => "settings",
    }
}

fn editor_mode_name(mode: EditorMode) -> &'static str {
    match mode {
        EditorMode::Meta => "meta",
        EditorMode::Source => "source",
        EditorMode::Edit => "edit",
        EditorMode::View => "view",
    }
}

fn graph_scope_name(scope: GraphScope) -> &'static str {
    match scope {
        GraphScope::Vault => "vault",
        GraphScope::Neighborhood => "neighborhood",
    }
}

fn settings_section_name(section: SettingsSection) -> &'static str {
    match section {
        SettingsSection::General => "general",
        SettingsSection::Appearance => "appearance",
        SettingsSection::Controls => "controls",
        SettingsSection::Vault => "vault",
        SettingsSection::Plugins => "plugins",
        SettingsSection::Maintenance => "maintenance",
    }
}

fn parse_tool_mode(value: &str) -> Option<ToolMode> {
    match value {
        "notes" => Some(ToolMode::Notes),
        "search" => Some(ToolMode::Search),
        "graph" => Some(ToolMode::Graph),
        "settings" => Some(ToolMode::Settings),
        _ => None,
    }
}

fn parse_editor_mode(value: &str) -> Option<EditorMode> {
    match value {
        "meta" => Some(EditorMode::Meta),
        "source" => Some(EditorMode::Source),
        "edit" => Some(EditorMode::Edit),
        "view" => Some(EditorMode::View),
        _ => None,
    }
}

fn parse_graph_scope(value: &str) -> Option<GraphScope> {
    match value {
        "vault" => Some(GraphScope::Vault),
        "neighborhood" => Some(GraphScope::Neighborhood),
        _ => None,
    }
}

fn parse_settings_section(value: &str) -> Option<SettingsSection> {
    match value {
        "general" => Some(SettingsSection::General),
        "appearance" => Some(SettingsSection::Appearance),
        "controls" => Some(SettingsSection::Controls),
        "vault" => Some(SettingsSection::Vault),
        "plugins" => Some(SettingsSection::Plugins),
        "maintenance" => Some(SettingsSection::Maintenance),
        _ => None,
    }
}

fn automation_gate_state(
    config_enabled: bool,
    runtime_enabled: bool,
    runtime_token: Option<&str>,
) -> (bool, Option<&'static str>) {
    if !config_enabled {
        return (false, Some("config_opt_in_required"));
    }
    if !runtime_enabled || runtime_token.is_none() {
        return (false, Some("runtime_token_required"));
    }
    (true, None)
}

fn automation_runtime_enabled() -> bool {
    AUTOMATION_RUNTIME_ENABLED.get().copied().unwrap_or(false)
}

fn automation_runtime_token() -> Option<&'static str> {
    AUTOMATION_RUNTIME_TOKEN
        .get()
        .and_then(|token| token.as_deref())
}

fn ui_automation_result_codes() -> Vec<String> {
    [
        "ok",
        "automation_disabled",
        "invalid_token",
        "startup_blocked",
        "dirty_guard_blocked",
        "unsupported_context",
        "not_found",
        "invalid_arguments",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn ui_automation_action_catalog() -> Vec<UiAutomationActionDescription> {
    vec![
        UiAutomationActionDescription {
            action_id: "switch_tool".to_string(),
            title: "Switch Tool".to_string(),
            description: "Switch the active shell tool.".to_string(),
            argument_schema: serde_json::json!({
                "type": "object",
                "required": ["tool"],
                "properties": {
                    "tool": {
                        "type": "string",
                        "enum": ["notes", "search", "graph", "settings"]
                    }
                }
            }),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "invalid_arguments".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "focus_search".to_string(),
            title: "Focus Search".to_string(),
            description: "Route to search and focus the query entry.".to_string(),
            argument_schema: serde_json::json!({"type": "object", "properties": {}}),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "select_note".to_string(),
            title: "Select Note".to_string(),
            description: "Load the note at the given path and route to notes.".to_string(),
            argument_schema: serde_json::json!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {"type": "string"}
                }
            }),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "dirty_guard_blocked".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "clear_selection".to_string(),
            title: "Clear Selection".to_string(),
            description: "Clear the active note selection.".to_string(),
            argument_schema: serde_json::json!({"type": "object", "properties": {}}),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "dirty_guard_blocked".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "set_editor_mode".to_string(),
            title: "Set Editor Mode".to_string(),
            description: "Switch the note editor to an available mode.".to_string(),
            argument_schema: serde_json::json!({
                "type": "object",
                "required": ["mode"],
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["meta", "source", "edit", "view"]
                    }
                }
            }),
            preconditions: vec![
                "startup.state == vault_open".to_string(),
                "active_note_path != null".to_string(),
            ],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "unsupported_context".to_string(),
                "invalid_arguments".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "open_settings_section".to_string(),
            title: "Open Settings Section".to_string(),
            description: "Switch to settings and show the requested section.".to_string(),
            argument_schema: serde_json::json!({
                "type": "object",
                "required": ["section"],
                "properties": {
                    "section": {
                        "type": "string",
                        "enum": ["general", "appearance", "controls", "vault", "plugins", "maintenance"]
                    }
                }
            }),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "invalid_arguments".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "set_graph_scope".to_string(),
            title: "Set Graph Scope".to_string(),
            description: "Set the graph scope and refresh the graph surface.".to_string(),
            argument_schema: serde_json::json!({
                "type": "object",
                "required": ["scope"],
                "properties": {
                    "scope": {
                        "type": "string",
                        "enum": ["vault", "neighborhood"]
                    }
                }
            }),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "invalid_arguments".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "set_graph_depth".to_string(),
            title: "Set Graph Depth".to_string(),
            description: "Set neighborhood graph depth and refresh when needed.".to_string(),
            argument_schema: serde_json::json!({
                "type": "object",
                "required": ["depth"],
                "properties": {
                    "depth": {"type": "integer", "minimum": 1, "maximum": 10}
                }
            }),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
                "invalid_arguments".to_string(),
            ],
        },
        UiAutomationActionDescription {
            action_id: "reset_graph".to_string(),
            title: "Reset Graph".to_string(),
            description: "Reset graph scope, depth, and selection to vault defaults.".to_string(),
            argument_schema: serde_json::json!({"type": "object", "properties": {}}),
            preconditions: vec!["startup.state == vault_open".to_string()],
            result_codes: vec![
                "ok".to_string(),
                "automation_disabled".to_string(),
                "startup_blocked".to_string(),
            ],
        },
    ]
}

struct AutomationSnapshotInput {
    active_tool: String,
    active_content: String,
    startup_state: String,
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

fn build_ui_automation_snapshot(input: AutomationSnapshotInput) -> UiAutomationSnapshot {
    let mut properties = BTreeMap::new();
    properties.insert("tool.active".to_string(), input.active_tool.clone());
    properties.insert("content.active".to_string(), input.active_content.clone());
    properties.insert("startup.state".to_string(), input.startup_state.clone());
    properties.insert(
        "inspector.visible".to_string(),
        input.inspector_visible.to_string(),
    );
    properties.insert("editor.dirty".to_string(), input.editor_dirty.to_string());
    properties.insert(
        "automation.active".to_string(),
        input.automation_active.to_string(),
    );
    if let Some(path) = input.active_note_path.as_deref() {
        properties.insert("note.path".to_string(), path.to_string());
    }
    if let Some(mode) = input.editor_mode.as_deref() {
        properties.insert("editor.mode".to_string(), mode.to_string());
    }
    if let Some(query) = input.search_query.as_deref() {
        properties.insert("search.query".to_string(), query.to_string());
    }
    if let Some(scope) = input.graph_scope.as_deref() {
        properties.insert("graph.scope".to_string(), scope.to_string());
    }
    if let Some(depth) = input.graph_depth {
        properties.insert("graph.depth".to_string(), depth.to_string());
    }
    if let Some(path) = input.graph_selected_path.as_deref() {
        properties.insert("graph.selected_path".to_string(), path.to_string());
    }
    if let Some(section) = input.settings_section.as_deref() {
        properties.insert("settings.section".to_string(), section.to_string());
    }

    UiAutomationSnapshot {
        active_tool: input.active_tool,
        active_content: input.active_content,
        startup_state: input.startup_state,
        inspector_visible: input.inspector_visible,
        active_note_path: input.active_note_path,
        editor_mode: input.editor_mode,
        editor_dirty: input.editor_dirty,
        search_query: input.search_query,
        graph_scope: input.graph_scope,
        graph_depth: input.graph_depth,
        graph_selected_path: input.graph_selected_path,
        settings_section: input.settings_section,
        automation_active: input.automation_active,
        properties,
    }
}

impl KnotWindow {
    fn shell_ui_handles(&self) -> ShellUiHandles {
        ShellUiHandles {
            shell_state: Rc::clone(&self.shell_state),
            tool_rail: self.tool_rail.clone(),
            context_panel: Rc::clone(&self.context_panel),
            content_stack: self.content_stack.clone(),
            inspector_rail: self.inspector_rail.clone(),
            search_view: Rc::clone(&self.search_view),
        }
    }

    fn note_session_handles(&self) -> NoteSessionHandles {
        NoteSessionHandles {
            window: self.window.clone(),
            editor: Rc::clone(&self.editor),
            current_note: Rc::clone(&self.current_note),
            note_load_state: Rc::clone(&self.note_load_state),
            note_load_generation: Rc::clone(&self.note_load_generation),
            shell_ui: self.shell_ui_handles(),
        }
    }

    fn graph_session_handles(&self) -> GraphSessionHandles {
        GraphSessionHandles {
            client: Rc::clone(&self.client),
            graph_view: Rc::clone(&self.graph_view),
            context_panel: Rc::clone(&self.context_panel),
            scope: Rc::clone(&self.graph_scope),
            depth: Rc::clone(&self.graph_depth),
            selected_path: Rc::clone(&self.graph_selected_path),
            vault_layout: Rc::clone(&self.graph_vault_layout),
            current_scene: Rc::clone(&self.graph_current_scene),
            request_generation: Rc::clone(&self.graph_request_generation),
        }
    }

    fn automation_gate(&self) -> (bool, Option<&'static str>) {
        let config_enabled = load_knotty_config()
            .map(|config| config.automation.enabled)
            .unwrap_or(false);
        let runtime_enabled = automation_runtime_enabled();
        let runtime_token = automation_runtime_token();
        automation_gate_state(config_enabled, runtime_enabled, runtime_token)
    }

    fn automation_result(
        &self,
        action_id: &str,
        ok: bool,
        result_code: &str,
        message: Option<String>,
    ) -> UiAutomationActionResult {
        UiAutomationActionResult {
            action_id: action_id.to_string(),
            ok,
            result_code: result_code.to_string(),
            message,
            snapshot: ok.then(|| self.ui_automation_snapshot()),
        }
    }

    pub fn describe_ui_automation(&self) -> UiAutomationDescription {
        let (available, unavailable_reason) = self.automation_gate();
        UiAutomationDescription {
            protocol_version: UI_AUTOMATION_PROTOCOL_VERSION,
            snapshot_schema_version: UI_AUTOMATION_SNAPSHOT_SCHEMA_VERSION,
            action_catalog_version: UI_AUTOMATION_ACTION_CATALOG_VERSION,
            available,
            unavailable_reason: unavailable_reason.map(str::to_string),
            requires_config_opt_in: true,
            requires_runtime_token: true,
            actions: ui_automation_action_catalog(),
            result_codes: ui_automation_result_codes(),
        }
    }

    pub fn ui_automation_snapshot(&self) -> UiAutomationSnapshot {
        let shell_state = self.shell_state.borrow();
        let startup_state = self.startup_state.borrow();
        let active_tool = tool_mode_name(shell_state.tool_mode()).to_string();
        let active_content = content_child_name_for_shell(&shell_state).to_string();
        let startup_state_name = startup_state_name(&startup_state).to_string();
        let active_note_path = self
            .current_note
            .borrow()
            .as_ref()
            .map(|note| note.path.clone());
        let settings_section = matches!(shell_state.tool_mode(), ToolMode::Settings)
            .then(|| settings_section_name(self.settings_view.selected_section()).to_string());
        let graph_scope = Some(graph_scope_name(*self.graph_scope.borrow()).to_string());
        let graph_depth = Some(self.graph_depth.get() as u8);
        let graph_selected_path = self.graph_selected_path.borrow().clone();
        let editor_mode = active_note_path
            .as_ref()
            .map(|_| editor_mode_name(self.editor.current_mode()).to_string());
        let editor_dirty = self.editor.is_modified();
        let search_query = self.search_view.query();
        let automation_active = self.automation_gate().0;
        let inspector_visible = matches!(shell_state.inspector_mode(), InspectorMode::Details)
            && self.inspector_rail.widget().is_visible();

        build_ui_automation_snapshot(AutomationSnapshotInput {
            active_tool,
            active_content,
            startup_state: startup_state_name,
            inspector_visible,
            active_note_path,
            editor_mode,
            editor_dirty,
            search_query,
            graph_scope,
            graph_depth,
            graph_selected_path,
            settings_section,
            automation_active,
        })
    }

    pub fn dispatch_ui_automation_action(
        &self,
        action: UiAutomationAction,
    ) -> UiAutomationActionResult {
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

        if !self.automation_gate().0 {
            return self.automation_result(
                action_id,
                false,
                "automation_disabled",
                Some("UI automation is not enabled for this session.".to_string()),
            );
        }
        if !startup_shell_chrome_visible(&self.startup_state.borrow()) {
            return self.automation_result(
                action_id,
                false,
                "startup_blocked",
                Some("A vault must be open before automation can drive the shell.".to_string()),
            );
        }

        match action {
            UiAutomationAction::SwitchTool { tool } => {
                let Some(mode) = parse_tool_mode(&tool) else {
                    return self.automation_result(
                        action_id,
                        false,
                        "invalid_arguments",
                        Some(format!("Unknown tool '{tool}'.")),
                    );
                };
                if matches!(mode, ToolMode::Settings) {
                    self.settings_view.refresh();
                }
                {
                    let mut shell_state = self.shell_state.borrow_mut();
                    shell_state.select_tool(mode);
                    apply_shell_state(
                        &shell_state,
                        &self.tool_rail,
                        self.context_panel.as_ref(),
                        &self.content_stack,
                        &self.inspector_rail,
                        self.search_view.as_ref(),
                    );
                }
                if matches!(mode, ToolMode::Settings) {
                    self.context_panel
                        .set_settings_section(self.settings_view.selected_section());
                } else if matches!(mode, ToolMode::Graph) {
                    ensure_graph_loaded(self.graph_session_handles());
                }
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::FocusSearch => {
                let mut shell_state = self.shell_state.borrow_mut();
                focus_search_shell(
                    &mut shell_state,
                    &self.tool_rail,
                    self.context_panel.as_ref(),
                    &self.content_stack,
                    &self.inspector_rail,
                    self.search_view.as_ref(),
                );
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::SelectNote { path } => {
                if self.editor.is_modified() {
                    return self.automation_result(
                        action_id,
                        false,
                        "dirty_guard_blocked",
                        Some("Dirty note guard blocked note selection.".to_string()),
                    );
                }
                {
                    let mut shell_state = self.shell_state.borrow_mut();
                    shell_state.select_tool(ToolMode::Notes);
                    apply_shell_state(
                        &shell_state,
                        &self.tool_rail,
                        self.context_panel.as_ref(),
                        &self.content_stack,
                        &self.inspector_rail,
                        self.search_view.as_ref(),
                    );
                }
                build_note_load_dispatcher(Rc::clone(&self.client), self.note_session_handles())(
                    NoteLoadOrigin::Context,
                    &path,
                );
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::ClearSelection => {
                if self.editor.is_modified() {
                    return self.automation_result(
                        action_id,
                        false,
                        "dirty_guard_blocked",
                        Some("Dirty note guard blocked selection clear.".to_string()),
                    );
                }
                clear_active_note(&self.note_session_handles());
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::SetEditorMode { mode } => {
                let Some(mode) = parse_editor_mode(&mode) else {
                    return self.automation_result(
                        action_id,
                        false,
                        "invalid_arguments",
                        Some("Unknown editor mode.".to_string()),
                    );
                };
                if self.current_note.borrow().is_none() {
                    return self.automation_result(
                        action_id,
                        false,
                        "unsupported_context",
                        Some("No active note is loaded.".to_string()),
                    );
                }
                if !self.editor.select_mode(mode) {
                    return self.automation_result(
                        action_id,
                        false,
                        "unsupported_context",
                        Some(
                            "Requested editor mode is unavailable for the active note.".to_string(),
                        ),
                    );
                }
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::OpenSettingsSection { section } => {
                let Some(section) = parse_settings_section(&section) else {
                    return self.automation_result(
                        action_id,
                        false,
                        "invalid_arguments",
                        Some("Unknown settings section.".to_string()),
                    );
                };
                self.settings_view.refresh();
                self.settings_view.set_section(section);
                self.context_panel.set_settings_section(section);
                let mut shell_state = self.shell_state.borrow_mut();
                shell_state.select_tool(ToolMode::Settings);
                apply_shell_state(
                    &shell_state,
                    &self.tool_rail,
                    self.context_panel.as_ref(),
                    &self.content_stack,
                    &self.inspector_rail,
                    self.search_view.as_ref(),
                );
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::SetGraphScope { scope } => {
                let Some(scope) = parse_graph_scope(&scope) else {
                    return self.automation_result(
                        action_id,
                        false,
                        "invalid_arguments",
                        Some("Unknown graph scope.".to_string()),
                    );
                };
                {
                    let mut shell_state = self.shell_state.borrow_mut();
                    shell_state.select_tool(ToolMode::Graph);
                    apply_shell_state(
                        &shell_state,
                        &self.tool_rail,
                        self.context_panel.as_ref(),
                        &self.content_stack,
                        &self.inspector_rail,
                        self.search_view.as_ref(),
                    );
                }
                *self.graph_scope.borrow_mut() = scope;
                load_graph(self.graph_session_handles());
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::SetGraphDepth { depth } => {
                if depth == 0 {
                    return self.automation_result(
                        action_id,
                        false,
                        "invalid_arguments",
                        Some("Graph depth must be at least 1.".to_string()),
                    );
                }
                {
                    let mut shell_state = self.shell_state.borrow_mut();
                    shell_state.select_tool(ToolMode::Graph);
                    apply_shell_state(
                        &shell_state,
                        &self.tool_rail,
                        self.context_panel.as_ref(),
                        &self.content_stack,
                        &self.inspector_rail,
                        self.search_view.as_ref(),
                    );
                }
                self.graph_depth.set(u32::from(depth));
                let graph_session = self.graph_session_handles();
                if matches!(*self.graph_scope.borrow(), GraphScope::Neighborhood) {
                    load_graph(graph_session);
                } else {
                    let scene = self.graph_current_scene.borrow().clone();
                    sync_graph_ui(&graph_session, scene);
                }
                self.automation_result(action_id, true, "ok", None)
            }
            UiAutomationAction::ResetGraph => {
                {
                    let mut shell_state = self.shell_state.borrow_mut();
                    shell_state.select_tool(ToolMode::Graph);
                    apply_shell_state(
                        &shell_state,
                        &self.tool_rail,
                        self.context_panel.as_ref(),
                        &self.content_stack,
                        &self.inspector_rail,
                        self.search_view.as_ref(),
                    );
                }
                *self.graph_scope.borrow_mut() = GraphScope::Vault;
                self.graph_depth.set(1);
                *self.graph_selected_path.borrow_mut() = None;
                load_graph(self.graph_session_handles());
                self.automation_result(action_id, true, "ok", None)
            }
        }
    }

    pub fn with_client(app: &libadwaita::Application, client: KnotdClient) -> Self {
        let client = Rc::new(client);
        let startup_state = determine_startup_state(client.as_ref());
        let initial_config = load_knotty_config().unwrap_or_else(|error| {
            log::error!("Failed to load knotty config: {}", error);
            KnottyConfig::default()
        });

        // Create window
        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("Knot")
            .default_width(1400)
            .default_height(900)
            .build();
        log::info!("Startup state: {:?}", startup_state);

        // Create toolbar view
        let toolbar_view = libadwaita::ToolbarView::new();

        // Header bar
        let header = libadwaita::HeaderBar::new();

        // Vault info label in header
        let vault_label = gtk::Label::builder()
            .label(startup_header_text(&startup_state))
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(40)
            .build();
        header.set_title_widget(Some(&vault_label));

        let automation_indicator = gtk::Label::builder()
            .label("Automation active")
            .css_classes(vec!["accent".to_string(), "caption-heading".to_string()])
            .visible(
                automation_gate_state(
                    initial_config.automation.enabled,
                    automation_runtime_enabled(),
                    automation_runtime_token(),
                )
                .0,
            )
            .build();
        automation_indicator.set_widget_name("knot.automation-indicator");
        header.pack_start(&automation_indicator);

        // Menu button
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();
        header.pack_end(&menu_btn);

        toolbar_view.add_top_bar(&header);

        // Main horizontal box: ToolRail | ContextPanel | Content | InspectorRail
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        main_box.set_widget_name("knot.shell");

        // ToolRail (left)
        let tool_rail = ToolRail::new();
        main_box.append(tool_rail.widget());

        // ContextPanel (left-center)
        let context_panel = Rc::new(ContextPanel::new(Rc::clone(&client)));
        main_box.append(context_panel.widget());

        // Content area (center)
        let content_stack = gtk::Stack::builder().vexpand(true).hexpand(true).build();
        content_stack.set_widget_name("knot.content.stack");

        // Empty state view (shown when no note selected)
        let empty_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(12)
            .build();
        empty_view.set_widget_name("knot.content.empty");

        let empty_icon = gtk::Image::builder()
            .icon_name("emblem-documents-symbolic")
            .pixel_size(64)
            .opacity(0.5)
            .build();

        let empty_label = gtk::Label::builder()
            .label("No note selected")
            .css_classes(vec!["title-3".to_string()])
            .build();

        let empty_hint = gtk::Label::builder()
            .label("Select a note from the sidebar to view or edit")
            .css_classes(vec!["dim-label".to_string()])
            .build();

        empty_view.append(&empty_icon);
        empty_view.append(&empty_label);
        empty_view.append(&empty_hint);
        content_stack.add_titled(&empty_view, Some("empty"), "Empty");

        let daemon_unavailable_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(12)
            .build();
        daemon_unavailable_view.set_widget_name("knot.content.daemon-unavailable");
        daemon_unavailable_view.append(
            &gtk::Label::builder()
                .label("knotd is unavailable")
                .css_classes(vec!["title-3".to_string()])
                .build(),
        );
        let daemon_detail_label = gtk::Label::builder()
            .label(startup_detail_text(&startup_state))
            .css_classes(vec!["dim-label".to_string()])
            .wrap(true)
            .max_width_chars(48)
            .justify(gtk::Justification::Center)
            .build();
        daemon_unavailable_view.append(&daemon_detail_label);
        let retry_startup_btn = gtk::Button::builder()
            .label(startup_action_label(StartupAction::RetryDaemon))
            .build();
        daemon_unavailable_view.append(&retry_startup_btn);
        content_stack.add_titled(
            &daemon_unavailable_view,
            Some("daemon-unavailable"),
            "Daemon unavailable",
        );

        let no_vault_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(12)
            .build();
        no_vault_view.set_widget_name("knot.content.no-vault");
        no_vault_view.append(
            &gtk::Label::builder()
                .label("No vault is open")
                .css_classes(vec!["title-3".to_string()])
                .build(),
        );
        no_vault_view.append(
            &gtk::Label::builder()
                .label("Open or create a vault to start browsing notes.")
                .css_classes(vec!["dim-label".to_string()])
                .wrap(true)
                .max_width_chars(48)
                .justify(gtk::Justification::Center)
                .build(),
        );
        let open_vault_btn = gtk::Button::builder()
            .label(startup_action_label(StartupAction::OpenVault))
            .build();
        no_vault_view.append(&open_vault_btn);
        let create_vault_btn = gtk::Button::builder()
            .label(startup_action_label(StartupAction::CreateVault))
            .build();
        no_vault_view.append(&create_vault_btn);
        content_stack.add_titled(&no_vault_view, Some("no-vault"), "No vault");

        // Editor view
        let editor = Rc::new(NoteEditor::new(Rc::clone(&client)));
        content_stack.add_titled(editor.widget(), Some("editor"), "Editor");

        let search_view = Rc::new(SearchView::new(Rc::clone(&client)));
        content_stack.add_titled(search_view.widget(), Some("search"), "Search");

        let graph_view = Rc::new(GraphView::new());
        content_stack.add_titled(graph_view.widget(), Some("graph"), "Graph");

        let settings_view = Rc::new(SettingsView::new(
            Rc::clone(&client),
            initial_config.clone(),
        ));
        content_stack.add_titled(settings_view.widget(), Some("settings"), "Settings");

        main_box.append(&content_stack);

        // InspectorRail (right)
        let inspector_rail = InspectorRail::new();
        inspector_rail.set_open(false);
        main_box.append(inspector_rail.widget());

        toolbar_view.set_content(Some(&main_box));
        window.set_content(Some(&toolbar_view));

        let win = Self {
            window,
            client,
            tool_rail,
            context_panel,
            inspector_rail,
            startup_state: Rc::new(RefCell::new(startup_state.clone())),
            automation_indicator,
            vault_label,
            daemon_detail_label,
            retry_startup_btn,
            open_vault_btn,
            create_vault_btn,
            editor,
            search_view,
            graph_view,
            settings_view,
            content_stack,
            current_note: Rc::new(RefCell::new(None)),
            note_load_state: Rc::new(RefCell::new(RequestState::idle())),
            note_load_generation: Rc::new(RefCell::new(0)),
            shell_state: Rc::new(RefCell::new(ShellState::default())),
            graph_scope: Rc::new(RefCell::new(GraphScope::Vault)),
            graph_depth: Rc::new(Cell::new(1_u32)),
            graph_selected_path: Rc::new(RefCell::new(None)),
            graph_vault_layout: Rc::new(RefCell::new(None)),
            graph_current_scene: Rc::new(RefCell::new(GraphScene::idle())),
            graph_request_generation: Rc::new(Cell::new(0_u64)),
        };

        apply_knotty_config(
            &initial_config,
            win.context_panel.as_ref(),
            &win.inspector_rail,
        );

        apply_startup_state(
            &startup_state,
            &StartupRefreshHandles {
                client: Rc::clone(&win.client),
                startup_state: Rc::clone(&win.startup_state),
                vault_label: win.vault_label.clone(),
                daemon_detail_label: win.daemon_detail_label.clone(),
                retry_startup_btn: win.retry_startup_btn.clone(),
                open_vault_btn: win.open_vault_btn.clone(),
                create_vault_btn: win.create_vault_btn.clone(),
                shell_ui: ShellUiHandles {
                    shell_state: Rc::clone(&win.shell_state),
                    tool_rail: win.tool_rail.clone(),
                    context_panel: Rc::clone(&win.context_panel),
                    content_stack: win.content_stack.clone(),
                    inspector_rail: win.inspector_rail.clone(),
                    search_view: Rc::clone(&win.search_view),
                },
            },
        );

        win.install_window_actions();
        win.setup_signals();

        win
    }

    fn install_window_actions(&self) {
        let action = gio::SimpleAction::new("focus-search", None);
        let startup_state = Rc::clone(&self.startup_state);
        let shell_state = Rc::clone(&self.shell_state);
        let tool_rail = self.tool_rail.clone();
        let context_panel = Rc::clone(&self.context_panel);
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        action.connect_activate(move |_action, _param| {
            if !startup_shell_chrome_visible(&startup_state.borrow()) {
                return;
            }
            let mut shell_state = shell_state.borrow_mut();
            focus_search_shell(
                &mut shell_state,
                &tool_rail,
                context_panel.as_ref(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
        });
        self.window.add_action(&action);

        let action = gio::SimpleAction::new("open-settings", None);
        let startup_state = Rc::clone(&self.startup_state);
        let shell_state = Rc::clone(&self.shell_state);
        let tool_rail = self.tool_rail.clone();
        let context_panel = Rc::clone(&self.context_panel);
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        let settings_view = Rc::clone(&self.settings_view);
        action.connect_activate(move |_action, _param| {
            if !startup_shell_chrome_visible(&startup_state.borrow()) {
                return;
            }
            settings_view.refresh();
            let mut shell_state = shell_state.borrow_mut();
            shell_state.select_tool(ToolMode::Settings);
            apply_shell_state(
                &shell_state,
                &tool_rail,
                context_panel.as_ref(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
            context_panel.set_settings_section(settings_view.selected_section());
        });
        self.window.add_action(&action);

        let action = gio::SimpleAction::new("save-note", None);
        let startup_state = Rc::clone(&self.startup_state);
        let editor = Rc::clone(&self.editor);
        let current_note = Rc::clone(&self.current_note);
        let window = self.window.clone();
        action.connect_activate(move |_action, _param| {
            if !startup_shell_chrome_visible(&startup_state.borrow()) {
                return;
            }
            if let Err(error) = editor.save() {
                log::error!("Failed to save note: {}", error);
            }
            let title = {
                let current_note = current_note.borrow();
                current_note.as_ref().map(|_| editor.current_title())
            };
            sync_window_title(&window, title.as_deref(), editor.is_modified());
        });
        self.window.add_action(&action);
    }

    fn setup_signals(&self) {
        let window = self.window.clone();
        let current_note = Rc::clone(&self.current_note);
        let editor_for_modified = Rc::clone(&self.editor);
        self.editor.connect_modified_changed({
            let editor = Rc::clone(&editor_for_modified);
            move |modified| {
                let title = {
                    let current_note = current_note.borrow();
                    current_note.as_ref().map(|_| editor.current_title())
                };
                sync_window_title(&window, title.as_deref(), modified);
            }
        });

        let shell_ui = self.shell_ui_handles();
        let note_session = self.note_session_handles();
        let dispatch_note_load =
            build_note_load_dispatcher(Rc::clone(&self.client), note_session.clone());

        let graph_session = self.graph_session_handles();

        // Tool mode changes
        let content_stack = self.content_stack.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let inspector_rail = self.inspector_rail.clone();
        let tool_rail = self.tool_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        let shell_state = Rc::clone(&self.shell_state);
        let graph_session_for_mode = graph_session.clone();
        self.tool_rail.connect_mode_changed(move |mode| {
            let mut shell_state = shell_state.borrow_mut();
            shell_state.select_tool(mode);
            apply_shell_state(
                &shell_state,
                &tool_rail,
                context_panel_ref.as_ref(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
            if matches!(mode, ToolMode::Graph) {
                ensure_graph_loaded(graph_session_for_mode.clone());
            }
        });

        // Settings button
        let window = self.window.clone();
        self.tool_rail.connect_settings(move || {
            let _ = gtk::prelude::WidgetExt::activate_action(&window, "win.open-settings", None);
        });

        let context_panel = Rc::clone(&self.context_panel);
        let inspector_rail = self.inspector_rail.clone();
        let automation_indicator = self.automation_indicator.clone();
        self.settings_view
            .connect_preferences_changed(move |config| {
                apply_knotty_config(&config, context_panel.as_ref(), &inspector_rail);
                automation_indicator.set_visible(
                    automation_gate_state(
                        config.automation.enabled,
                        automation_runtime_enabled(),
                        automation_runtime_token(),
                    )
                    .0,
                );
            });

        let note_switch_prompt = NoteSwitchPromptHandles {
            window: self.window.clone(),
            editor: Rc::clone(&self.editor),
            prompt_open: Rc::new(Cell::new(false)),
            dispatch_note_load: Rc::clone(&dispatch_note_load),
        };

        self.context_panel.connect_note_selected({
            let dispatch_note_load = Rc::clone(&dispatch_note_load);
            move |path| dispatch_note_load(NoteLoadOrigin::Context, path)
        });
        self.context_panel.connect_note_switch_guard({
            let editor = Rc::clone(&self.editor);
            let note_switch_prompt = note_switch_prompt.clone();
            move |path| {
                if editor.is_modified() {
                    present_dirty_note_switch_prompt(
                        &note_switch_prompt,
                        NoteLoadOrigin::Context,
                        path,
                    );
                    return NoteSwitchDecision::Prompt;
                }

                NoteSwitchDecision::Allow
            }
        });

        self.search_view.connect_result_selected({
            let editor = Rc::clone(&self.editor);
            let note_switch_prompt = note_switch_prompt.clone();
            let dispatch_note_load = Rc::clone(&dispatch_note_load);
            move |path| {
                if editor.is_modified() {
                    present_dirty_note_switch_prompt(
                        &note_switch_prompt,
                        NoteLoadOrigin::Search,
                        path,
                    );
                    return;
                }

                dispatch_note_load(NoteLoadOrigin::Search, path);
            }
        });

        let note_session_for_clear = note_session.clone();
        self.context_panel.connect_selection_cleared(move || {
            clear_active_note(&note_session_for_clear);
        });

        let settings_view = Rc::clone(&self.settings_view);
        let context_panel = Rc::clone(&self.context_panel);
        self.context_panel
            .connect_settings_section_selected(move |section| {
                settings_view.set_section(section);
                context_panel.set_settings_section(section);
            });

        self.graph_view.connect_node_selected({
            let graph_session = graph_session.clone();
            move |path| {
                set_graph_selection(&graph_session, Some(path.to_string()));
                if matches!(*graph_session.scope.borrow(), GraphScope::Neighborhood) {
                    load_graph(graph_session.clone());
                }
            }
        });

        self.graph_view.connect_node_activated({
            let editor = Rc::clone(&self.editor);
            let note_switch_prompt = note_switch_prompt.clone();
            let dispatch_note_load = Rc::clone(&dispatch_note_load);
            move |path| {
                if editor.is_modified() {
                    present_dirty_note_switch_prompt(
                        &note_switch_prompt,
                        NoteLoadOrigin::Graph,
                        path,
                    );
                } else {
                    dispatch_note_load(NoteLoadOrigin::Graph, path);
                }
            }
        });

        self.context_panel.connect_graph_event({
            let graph_session = graph_session.clone();
            let editor = Rc::clone(&self.editor);
            let note_switch_prompt = note_switch_prompt.clone();
            let dispatch_note_load = Rc::clone(&dispatch_note_load);
            move |event| match event {
                GraphPanelEvent::ScopeChanged(scope) => {
                    *graph_session.scope.borrow_mut() = scope;
                    load_graph(graph_session.clone());
                }
                GraphPanelEvent::DepthChanged(depth) => {
                    graph_session.depth.set(depth.max(1));
                    if matches!(*graph_session.scope.borrow(), GraphScope::Neighborhood) {
                        load_graph(graph_session.clone());
                    } else {
                        let scene = graph_session.current_scene.borrow().clone();
                        sync_graph_ui(&graph_session, scene);
                    }
                }
                GraphPanelEvent::ResetRequested => {
                    *graph_session.scope.borrow_mut() = GraphScope::Vault;
                    graph_session.depth.set(1);
                    *graph_session.selected_path.borrow_mut() = None;
                    load_graph(graph_session.clone());
                }
                GraphPanelEvent::OpenSelected(path) => {
                    if editor.is_modified() {
                        present_dirty_note_switch_prompt(
                            &note_switch_prompt,
                            NoteLoadOrigin::Graph,
                            &path,
                        );
                    } else {
                        dispatch_note_load(NoteLoadOrigin::Graph, &path);
                    }
                }
            }
        });

        let startup_refresh = StartupRefreshHandles {
            client: Rc::clone(&self.client),
            startup_state: Rc::clone(&self.startup_state),
            vault_label: self.vault_label.clone(),
            daemon_detail_label: self.daemon_detail_label.clone(),
            retry_startup_btn: self.retry_startup_btn.clone(),
            open_vault_btn: self.open_vault_btn.clone(),
            create_vault_btn: self.create_vault_btn.clone(),
            shell_ui,
        };
        let startup_refresh_for_retry = startup_refresh.clone();
        self.retry_startup_btn.connect_clicked(move |_| {
            refresh_startup_shell(&startup_refresh_for_retry);
        });

        let window = self.window.clone();
        let startup_refresh_for_open = startup_refresh.clone();
        self.open_vault_btn.connect_clicked(move |_| {
            let startup_refresh = startup_refresh_for_open.clone();
            choose_vault_directory(&window, "Open vault", "Open", move |path| {
                if let Err(error) = startup_refresh.client.open_vault(&path) {
                    log::error!("Failed to open vault {}: {}", path, error);
                }
                refresh_startup_shell(&startup_refresh);
            });
        });

        let window = self.window.clone();
        let startup_refresh_for_create = startup_refresh;
        self.create_vault_btn.connect_clicked(move |_| {
            let startup_refresh = startup_refresh_for_create.clone();
            choose_vault_directory(&window, "Create vault", "Create", move |path| {
                if let Err(error) = startup_refresh.client.create_vault(&path) {
                    log::error!("Failed to create vault {}: {}", path, error);
                }
                refresh_startup_shell(&startup_refresh);
            });
        });

        // Inspector close
        let inspector_rail = self.inspector_rail.clone();
        self.inspector_rail.connect_close(move || {
            inspector_rail.set_open(false);
        });
    }

    pub fn present(&self) {
        self.window.present();
    }
}

// Helper module for logging
mod log {
    pub use tracing::{error, info};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{GraphEdge, GraphNode};
    use crate::ui::graph_view::{GraphLoadState, GraphScope};
    use crate::ui::request_state::RequestState;
    use std::cell::Cell;

    const TEST_SOCKET_PATH: &str = "/tmp/knot/knotd.sock";
    const TEST_MISSING_SOCKET_PATH: &str = "/tmp/knot/missing-knotd.sock";

    fn test_client() -> KnotdClient {
        KnotdClient::with_socket_path(TEST_SOCKET_PATH)
    }

    fn sample_note() -> NoteData {
        NoteData {
            id: "note-1".to_string(),
            path: "notes/example.md".to_string(),
            title: "Example".to_string(),
            content: "# Example".to_string(),
            created_at: 1,
            modified_at: 2,
            word_count: 2,
            headings: Vec::new(),
            backlinks: Vec::new(),
            note_type: Some(crate::client::NoteType::Markdown),
            available_modes: None,
            metadata: None,
            embed: None,
            media: None,
            type_badge: Some("MD".to_string()),
            is_dimmed: false,
        }
    }

    fn sample_graph_scene() -> GraphScene {
        GraphScene::ready(
            vec![
                GraphNode {
                    id: "notes/one.md".to_string(),
                    label: "one".to_string(),
                    x: 0.0,
                    y: 0.0,
                },
                GraphNode {
                    id: "notes/two.md".to_string(),
                    label: "two".to_string(),
                    x: 100.0,
                    y: 0.0,
                },
            ],
            vec![GraphEdge {
                source: "notes/one.md".to_string(),
                target: "notes/two.md".to_string(),
            }],
            Some("notes/one.md".to_string()),
        )
    }

    #[test]
    fn startup_state_reports_daemon_unavailable_when_client_cannot_connect() {
        let state =
            determine_startup_state(&KnotdClient::with_socket_path(TEST_MISSING_SOCKET_PATH));

        assert!(matches!(state, StartupState::DaemonUnavailable { .. }));
        assert_eq!(startup_content_child_name(&state), "daemon-unavailable");
        assert!(!startup_shell_chrome_visible(&state));
    }

    #[test]
    fn startup_state_reports_no_vault_when_daemon_has_no_active_vault() {
        let state = StartupState::NoVault;

        assert_eq!(startup_header_text(&state), "No vault open");
        assert_eq!(startup_content_child_name(&state), "no-vault");
        assert!(!startup_shell_chrome_visible(&state));
    }

    #[test]
    fn startup_state_reports_vault_open_when_vault_info_is_available() {
        let state = StartupState::VaultOpen {
            name: Some("Example".to_string()),
        };

        assert_eq!(startup_header_text(&state), "Connected to vault: Example");
        assert_eq!(startup_content_child_name(&state), "empty");
        assert!(startup_shell_chrome_visible(&state));
    }

    #[test]
    fn shell_routing_maps_search_to_search_content() {
        let mut shell_state = ShellState::default();

        shell_state.select_tool(ToolMode::Search);

        assert_eq!(content_child_name_for_shell(&shell_state), "search");
        assert_eq!(shell_state.inspector_mode(), InspectorMode::Hidden);
    }

    #[test]
    fn shell_routing_maps_settings_to_settings_content() {
        let mut shell_state = ShellState::default();

        shell_state.select_tool(ToolMode::Settings);

        assert_eq!(content_child_name_for_shell(&shell_state), "settings");
        assert_eq!(shell_state.inspector_mode(), InspectorMode::Hidden);
    }

    #[test]
    fn search_shell_content_is_registered_in_window_stack() {
        let mut shell_state = ShellState::default();
        shell_state.select_tool(ToolMode::Search);

        assert!(content_stack_child_names().contains(&content_child_name_for_shell(&shell_state)));
    }

    #[test]
    fn startup_states_expose_required_actions() {
        let daemon_actions = startup_action_specs(&StartupState::DaemonUnavailable {
            message: "offline".to_string(),
        });
        let no_vault_actions = startup_action_specs(&StartupState::NoVault);

        assert_eq!(daemon_actions, &[StartupAction::RetryDaemon]);
        assert_eq!(
            no_vault_actions,
            &[StartupAction::OpenVault, StartupAction::CreateVault]
        );
    }

    #[test]
    fn startup_navigation_only_unlocked_when_vault_is_open() {
        assert!(!startup_shell_chrome_visible(
            &StartupState::DaemonUnavailable {
                message: "offline".to_string()
            }
        ));
        assert!(!startup_shell_chrome_visible(&StartupState::NoVault));
        assert!(startup_shell_chrome_visible(&StartupState::VaultOpen {
            name: Some("Example".to_string())
        }));
    }

    #[test]
    fn startup_detail_text_tracks_daemon_error_message() {
        let state = StartupState::DaemonUnavailable {
            message: "socket timeout".to_string(),
        };

        assert_eq!(startup_detail_text(&state), "socket timeout");
        assert_eq!(
            startup_detail_text(&StartupState::NoVault),
            "Open or create a vault to start browsing notes."
        );
    }

    #[test]
    fn degraded_vault_info_failure_keeps_shell_chrome_available() {
        let state = StartupState::VaultOpen { name: None };

        assert_eq!(startup_header_text(&state), "Connected to vault");
        assert_eq!(startup_content_child_name(&state), "empty");
        assert!(startup_shell_chrome_visible(&state));
        assert!(startup_action_specs(&state).is_empty());
    }

    #[test]
    fn note_load_uses_dispatcher_and_updates_success_state() {
        let state = Rc::new(RefCell::new(RequestState::idle()));
        let generation = Rc::new(RefCell::new(1_u64));
        let dispatched = Rc::new(Cell::new(false));
        let note = sample_note();

        begin_note_load_with_dispatch(
            test_client(),
            "notes/example.md".to_string(),
            Rc::clone(&state),
            1,
            Rc::clone(&generation),
            {
                let dispatched = Rc::clone(&dispatched);
                let note = note.clone();
                move |_work, ui| {
                    dispatched.set(true);
                    ui(Ok(note));
                }
            },
            |_| {},
        );

        assert!(dispatched.get());
        assert_eq!(*state.borrow(), RequestState::Success(note));
    }

    #[test]
    fn note_load_updates_error_state_without_clearing_previous_note() {
        let state = Rc::new(RefCell::new(RequestState::success(sample_note())));
        let generation = Rc::new(RefCell::new(1_u64));

        begin_note_load_with_dispatch(
            test_client(),
            "notes/missing.md".to_string(),
            Rc::clone(&state),
            1,
            Rc::clone(&generation),
            move |_work, ui| {
                ui(Err("daemon request failed".to_string()));
            },
            |_| {},
        );

        assert_eq!(
            *state.borrow(),
            RequestState::Error("daemon request failed".to_string())
        );
    }

    #[test]
    fn stale_note_load_result_is_ignored_when_newer_selection_exists() {
        let first_state = Rc::new(RefCell::new(RequestState::idle()));
        let second_state = Rc::new(RefCell::new(RequestState::idle()));
        let generation = Rc::new(RefCell::new(2_u64));
        let stale_result: DeferredUiCallback = Rc::new(RefCell::new(None));
        let fresh_result: DeferredUiCallback = Rc::new(RefCell::new(None));
        let first_note = sample_note();
        let mut second_note = sample_note();
        second_note.title = "Second".to_string();

        begin_note_load_with_dispatch(
            test_client(),
            "notes/first.md".to_string(),
            Rc::clone(&first_state),
            1,
            Rc::clone(&generation),
            {
                let stale_result = Rc::clone(&stale_result);
                let first_note = first_note.clone();
                move |_work, ui| {
                    *stale_result.borrow_mut() = Some(Box::new(move || ui(Ok(first_note))));
                }
            },
            |_| panic!("stale result should not update the UI"),
        );

        begin_note_load_with_dispatch(
            test_client(),
            "notes/second.md".to_string(),
            Rc::clone(&second_state),
            2,
            Rc::clone(&generation),
            {
                let fresh_result = Rc::clone(&fresh_result);
                let second_note = second_note.clone();
                move |_work, ui| {
                    *fresh_result.borrow_mut() = Some(Box::new(move || ui(Ok(second_note))));
                }
            },
            |_| {},
        );

        fresh_result
            .borrow_mut()
            .take()
            .expect("fresh result should be captured")();
        stale_result
            .borrow_mut()
            .take()
            .expect("stale result should be captured")();

        assert_eq!(*first_state.borrow(), RequestState::Loading);
        assert_eq!(*second_state.borrow(), RequestState::Success(second_note));
    }

    #[test]
    fn note_load_completion_routes_to_notes_only_for_matching_origin() {
        assert!(should_route_loaded_note_to_notes(
            NoteLoadOrigin::Context,
            ToolMode::Notes
        ));
        assert!(!should_route_loaded_note_to_notes(
            NoteLoadOrigin::Context,
            ToolMode::Search
        ));
        assert!(should_route_loaded_note_to_notes(
            NoteLoadOrigin::Search,
            ToolMode::Search
        ));
        assert!(should_route_loaded_note_to_notes(
            NoteLoadOrigin::Graph,
            ToolMode::Graph
        ));
        assert!(!should_route_loaded_note_to_notes(
            NoteLoadOrigin::Context,
            ToolMode::Graph
        ));
        assert!(!should_route_loaded_note_to_notes(
            NoteLoadOrigin::Search,
            ToolMode::Settings
        ));
    }

    #[test]
    fn context_selection_does_not_reuse_search_only_routing() {
        assert!(!should_route_loaded_note_to_notes(
            NoteLoadOrigin::Context,
            ToolMode::Search
        ));
        assert!(should_route_loaded_note_to_notes(
            NoteLoadOrigin::Search,
            ToolMode::Search
        ));
    }

    #[test]
    fn graph_panel_state_exposes_scope_depth_and_selected_details() {
        let scene = sample_graph_scene();

        let state = graph_panel_state(GraphScope::Neighborhood, 2, &scene);

        assert_eq!(state.scope, GraphScope::Neighborhood);
        assert_eq!(state.depth, 2);
        assert_eq!(state.load_state, GraphLoadState::Ready);
        assert_eq!(state.details.selected_path.as_deref(), Some("notes/one.md"));
        assert_eq!(state.details.neighbors, vec!["notes/two.md".to_string()]);
    }

    #[test]
    fn neighborhood_graph_request_without_selection_returns_error_scene() {
        let result = request_graph_scene(&test_client(), GraphScope::Neighborhood, None, 1, None)
            .expect("request should return a scene");

        assert_eq!(result.1, None);
        assert!(matches!(result.0.load_state, GraphLoadState::Error(_)));
    }

    #[test]
    fn focus_search_shell_routes_to_search_surface_and_hides_inspector() {
        let mut shell_state = ShellState::default();

        shell_state.select_tool(ToolMode::Graph);
        focus_search_shell_state(&mut shell_state);

        assert_eq!(shell_state.tool_mode(), ToolMode::Search);
        assert_eq!(content_child_name_for_shell(&shell_state), "search");
        assert_eq!(shell_state.inspector_mode(), InspectorMode::Hidden);
    }

    #[test]
    fn cancel_note_load_resets_state_to_idle_and_bumps_generation() {
        let note_load_state = Rc::new(RefCell::new(RequestState::loading()));
        let note_load_generation = Rc::new(RefCell::new(5_u64));

        cancel_note_load(&note_load_state, &note_load_generation);

        assert_eq!(*note_load_state.borrow(), RequestState::Idle);
        assert_eq!(*note_load_generation.borrow(), 6);
    }

    #[test]
    fn cleared_note_load_result_is_ignored_and_state_stays_idle() {
        let note_load_state = Rc::new(RefCell::new(RequestState::idle()));
        let note_load_generation = Rc::new(RefCell::new(1_u64));
        let deferred: DeferredUiCallback = Rc::new(RefCell::new(None));
        let note = sample_note();

        begin_note_load_with_dispatch(
            test_client(),
            "notes/example.md".to_string(),
            Rc::clone(&note_load_state),
            1,
            Rc::clone(&note_load_generation),
            {
                let deferred = Rc::clone(&deferred);
                let note = note.clone();
                move |_work, ui| {
                    *deferred.borrow_mut() = Some(Box::new(move || ui(Ok(note))));
                }
            },
            |_| panic!("cleared load should not reach the UI callback"),
        );

        cancel_note_load(&note_load_state, &note_load_generation);

        deferred
            .borrow_mut()
            .take()
            .expect("deferred result should be captured")();

        assert_eq!(*note_load_state.borrow(), RequestState::Idle);
    }

    #[test]
    fn note_window_title_marks_dirty_active_note() {
        assert_eq!(note_window_title(Some("Example"), false), "Example — Knot");
        assert_eq!(note_window_title(Some("Example"), true), "*Example — Knot");
        assert_eq!(note_window_title(None, false), "Knot");
    }

    #[test]
    fn clean_note_switch_always_allows() {
        let decision = resolve_note_switch_decision(false, NoteSwitchDecision::Deny, || Ok(()));

        assert_eq!(decision, NoteSwitchDecision::Allow);
    }

    #[test]
    fn dirty_note_switch_can_be_denied() {
        let decision = resolve_note_switch_decision(true, NoteSwitchDecision::Deny, || Ok(()));

        assert_eq!(decision, NoteSwitchDecision::Deny);
    }

    #[test]
    fn dirty_note_switch_can_discard_and_allow() {
        let decision = resolve_note_switch_decision(true, NoteSwitchDecision::Allow, || Ok(()));

        assert_eq!(decision, NoteSwitchDecision::Allow);
    }

    #[test]
    fn dirty_note_switch_can_save_then_allow() {
        let decision =
            resolve_note_switch_decision(true, NoteSwitchDecision::SaveThenAllow, || Ok(()));

        assert_eq!(decision, NoteSwitchDecision::Allow);
    }

    #[test]
    fn failed_save_then_switch_denies_navigation() {
        let decision =
            resolve_note_switch_decision(true, NoteSwitchDecision::SaveThenAllow, || {
                Err("save failed".to_string())
            });

        assert_eq!(decision, NoteSwitchDecision::Deny);
    }

    #[test]
    fn dirty_note_switch_response_maps_known_dialog_ids() {
        assert_eq!(
            dirty_note_switch_response("discard"),
            DirtyNoteSwitchResponse::Discard
        );
        assert_eq!(
            dirty_note_switch_response("save"),
            DirtyNoteSwitchResponse::Save
        );
        assert_eq!(
            dirty_note_switch_response("cancel"),
            DirtyNoteSwitchResponse::Cancel
        );
        assert_eq!(
            dirty_note_switch_response("unexpected"),
            DirtyNoteSwitchResponse::Cancel
        );
    }

    #[test]
    fn automation_gate_requires_config_opt_in_and_runtime_token() {
        assert_eq!(
            automation_gate_state(false, false, None),
            (false, Some("config_opt_in_required"))
        );
        assert_eq!(
            automation_gate_state(true, false, None),
            (false, Some("runtime_token_required"))
        );
        assert_eq!(
            automation_gate_state(true, true, Some("dev-token")),
            (true, None)
        );
    }

    #[test]
    fn automation_description_reports_discoverable_action_catalog() {
        let description = UiAutomationDescription {
            protocol_version: UI_AUTOMATION_PROTOCOL_VERSION,
            snapshot_schema_version: UI_AUTOMATION_SNAPSHOT_SCHEMA_VERSION,
            action_catalog_version: UI_AUTOMATION_ACTION_CATALOG_VERSION,
            available: false,
            unavailable_reason: Some("runtime_token_required".to_string()),
            requires_config_opt_in: true,
            requires_runtime_token: true,
            actions: ui_automation_action_catalog(),
            result_codes: ui_automation_result_codes(),
        };

        assert_eq!(description.protocol_version, 1);
        assert_eq!(description.snapshot_schema_version, 1);
        assert_eq!(description.action_catalog_version, 1);
        assert!(description
            .actions
            .iter()
            .any(|action| action.action_id == "switch_tool"));
        assert!(description
            .result_codes
            .contains(&"automation_disabled".to_string()));
    }

    #[test]
    fn automation_snapshot_builder_projects_shell_editor_graph_and_settings_state() {
        let snapshot = build_ui_automation_snapshot(AutomationSnapshotInput {
            active_tool: "settings".to_string(),
            active_content: "settings".to_string(),
            startup_state: "vault_open".to_string(),
            inspector_visible: false,
            active_note_path: Some("notes/example.md".to_string()),
            editor_mode: Some("edit".to_string()),
            editor_dirty: false,
            search_query: Some("graph".to_string()),
            graph_scope: Some("neighborhood".to_string()),
            graph_depth: Some(2),
            graph_selected_path: Some("notes/one.md".to_string()),
            settings_section: Some("plugins".to_string()),
            automation_active: true,
        });

        assert_eq!(snapshot.active_tool, "settings");
        assert_eq!(snapshot.active_content, "settings");
        assert_eq!(snapshot.startup_state, "vault_open");
        assert_eq!(
            snapshot.active_note_path.as_deref(),
            Some("notes/example.md")
        );
        assert_eq!(snapshot.editor_mode.as_deref(), Some("edit"));
        assert_eq!(snapshot.graph_scope.as_deref(), Some("neighborhood"));
        assert_eq!(snapshot.graph_depth, Some(2));
        assert_eq!(
            snapshot.graph_selected_path.as_deref(),
            Some("notes/one.md")
        );
        assert_eq!(snapshot.settings_section.as_deref(), Some("plugins"));
        assert_eq!(
            snapshot.properties.get("tool.active").map(String::as_str),
            Some("settings")
        );
        assert_eq!(
            snapshot
                .properties
                .get("settings.section")
                .map(String::as_str),
            Some("plugins")
        );
    }

    #[test]
    fn automation_action_result_preserves_stable_disabled_result() {
        let result = UiAutomationActionResult {
            action_id: "focus_search".to_string(),
            ok: false,
            result_code: "automation_disabled".to_string(),
            message: Some("UI automation is not enabled for this session.".to_string()),
            snapshot: None,
        };

        assert!(!result.ok);
        assert_eq!(result.action_id, "focus_search");
        assert_eq!(result.result_code, "automation_disabled");
        assert!(result.snapshot.is_none());
    }

    #[test]
    fn automation_parsers_accept_protocol_strings() {
        assert_eq!(parse_tool_mode("graph"), Some(ToolMode::Graph));
        assert_eq!(parse_editor_mode("view"), Some(EditorMode::View));
        assert_eq!(parse_graph_scope("vault"), Some(GraphScope::Vault));
        assert_eq!(
            parse_settings_section("maintenance"),
            Some(SettingsSection::Maintenance)
        );
        assert_eq!(parse_tool_mode("Graph"), None);
        assert_eq!(parse_editor_mode("VIEW"), None);
    }
}
