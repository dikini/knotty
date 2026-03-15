use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::client::{GraphLayout, KnotdClient, NoteData};
use crate::ui::async_bridge;
use crate::ui::context_panel::{ContextPanel, GraphPanelEvent, GraphPanelState};
use crate::ui::editor::NoteEditor;
use crate::ui::explorer::NoteSwitchDecision;
use crate::ui::graph_view::{
    graph_context_details, normalize_neighborhood_layout, normalize_vault_layout, GraphScene,
    GraphScope, GraphView,
};
use crate::ui::inspector_rail::InspectorRail;
use crate::ui::request_state::RequestState;
use crate::ui::search_view::SearchView;
use crate::ui::shell_state::{InspectorMode, ShellState};
use crate::ui::tool_rail::{ToolMode, ToolRail};

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
    vault_label: gtk::Label,
    daemon_detail_label: gtk::Label,
    retry_startup_btn: gtk::Button,
    open_vault_btn: gtk::Button,
    create_vault_btn: gtk::Button,
    editor: Rc<NoteEditor>,
    search_view: Rc<SearchView>,
    graph_view: Rc<GraphView>,
    content_stack: gtk::Stack,
    current_note: Rc<RefCell<Option<NoteData>>>,
    note_load_state: Rc<RefCell<NoteLoadState>>,
    note_load_generation: Rc<RefCell<u64>>,
    shell_state: Rc<RefCell<ShellState>>,
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
        InspectorMode::Settings => {
            inspector_rail.set_open(true);
            inspector_rail.set_mode("settings");
        }
    }
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

impl KnotWindow {
    pub fn with_client(app: &libadwaita::Application, client: KnotdClient) -> Self {
        let client = Rc::new(client);
        let startup_state = determine_startup_state(client.as_ref());

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

        // ToolRail (left)
        let tool_rail = ToolRail::new();
        main_box.append(tool_rail.widget());

        // ContextPanel (left-center)
        let context_panel = Rc::new(ContextPanel::new(Rc::clone(&client)));
        main_box.append(context_panel.widget());

        // Content area (center)
        let content_stack = gtk::Stack::builder().vexpand(true).hexpand(true).build();

        // Empty state view (shown when no note selected)
        let empty_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(12)
            .build();

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

        // Settings view (placeholder)
        let settings_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let settings_label = gtk::Label::builder()
            .label("Settings view coming soon")
            .vexpand(true)
            .build();
        settings_view.append(&settings_label);
        content_stack.add_titled(&settings_view, Some("settings"), "Settings");

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
            vault_label,
            daemon_detail_label,
            retry_startup_btn,
            open_vault_btn,
            create_vault_btn,
            editor,
            search_view,
            graph_view,
            content_stack,
            current_note: Rc::new(RefCell::new(None)),
            note_load_state: Rc::new(RefCell::new(RequestState::idle())),
            note_load_generation: Rc::new(RefCell::new(0)),
            shell_state: Rc::new(RefCell::new(ShellState::default())),
        };

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
        let graph_scope = Rc::new(RefCell::new(GraphScope::Vault));
        let graph_depth = Rc::new(Cell::new(1_u32));
        let graph_selected_path = Rc::new(RefCell::new(None::<String>));
        let graph_vault_layout = Rc::new(RefCell::new(None::<GraphLayout>));
        let graph_current_scene = Rc::new(RefCell::new(GraphScene::idle()));
        let graph_request_generation = Rc::new(Cell::new(0_u64));

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

        let shell_ui = ShellUiHandles {
            shell_state: Rc::clone(&self.shell_state),
            tool_rail: self.tool_rail.clone(),
            context_panel: Rc::clone(&self.context_panel),
            content_stack: self.content_stack.clone(),
            inspector_rail: self.inspector_rail.clone(),
            search_view: Rc::clone(&self.search_view),
        };
        let note_session = NoteSessionHandles {
            window: self.window.clone(),
            editor: Rc::clone(&self.editor),
            current_note: Rc::clone(&self.current_note),
            note_load_state: Rc::clone(&self.note_load_state),
            note_load_generation: Rc::clone(&self.note_load_generation),
            shell_ui: shell_ui.clone(),
        };
        let dispatch_note_load =
            build_note_load_dispatcher(Rc::clone(&self.client), note_session.clone());

        let graph_session = GraphSessionHandles {
            client: Rc::clone(&self.client),
            graph_view: Rc::clone(&self.graph_view),
            context_panel: Rc::clone(&self.context_panel),
            scope: Rc::clone(&graph_scope),
            depth: Rc::clone(&graph_depth),
            selected_path: Rc::clone(&graph_selected_path),
            vault_layout: Rc::clone(&graph_vault_layout),
            current_scene: Rc::clone(&graph_current_scene),
            request_generation: Rc::clone(&graph_request_generation),
        };

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
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let tool_rail = self.tool_rail.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let search_view = Rc::clone(&self.search_view);
        let shell_state = Rc::clone(&self.shell_state);
        self.tool_rail.connect_settings(move || {
            let mut shell_state = shell_state.borrow_mut();
            shell_state.select_tool(ToolMode::Settings);
            apply_shell_state(
                &shell_state,
                &tool_rail,
                context_panel_ref.as_ref(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
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
        assert_eq!(shell_state.inspector_mode(), InspectorMode::Settings);
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
}
