use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::client::{KnotdClient, NoteData};
use crate::ui::async_bridge;
use crate::ui::context_panel::ContextPanel;
use crate::ui::editor::NoteEditor;
use crate::ui::inspector_rail::InspectorRail;
use crate::ui::request_state::RequestState;
use crate::ui::search_view::SearchView;
use crate::ui::shell_state::{InspectorMode, ShellState};
use crate::ui::tool_rail::{ToolMode, ToolRail};

type NoteLoadState = RequestState<NoteData, String>;
type NoteLoadResult = Result<NoteData, String>;

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
    context_panel: Rc<RefCell<ContextPanel>>,
    inspector_rail: InspectorRail,
    vault_label: gtk::Label,
    retry_startup_btn: gtk::Button,
    open_vault_btn: gtk::Button,
    create_vault_btn: gtk::Button,
    editor: Rc<NoteEditor>,
    search_view: Rc<SearchView>,
    content_stack: gtk::Stack,
    current_note: Rc<RefCell<Option<NoteData>>>,
    note_load_state: Rc<RefCell<NoteLoadState>>,
    note_load_generation: Rc<RefCell<u64>>,
    shell_state: Rc<RefCell<ShellState>>,
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
        crate::ui::shell_state::ContentMode::Error => "daemon-unavailable",
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

fn apply_startup_state(
    state: &StartupState,
    shell_state: &ShellState,
    vault_label: &gtk::Label,
    retry_startup_btn: &gtk::Button,
    open_vault_btn: &gtk::Button,
    create_vault_btn: &gtk::Button,
    tool_rail: &ToolRail,
    context_panel: &ContextPanel,
    content_stack: &gtk::Stack,
    inspector_rail: &InspectorRail,
    search_view: &SearchView,
) {
    vault_label.set_label(&startup_header_text(state));
    let startup_actions = startup_action_specs(state);
    retry_startup_btn.set_visible(startup_actions.contains(&StartupAction::RetryDaemon));
    open_vault_btn.set_visible(startup_actions.contains(&StartupAction::OpenVault));
    create_vault_btn.set_visible(startup_actions.contains(&StartupAction::CreateVault));

    let shell_chrome_visible = startup_shell_chrome_visible(state);
    tool_rail.widget().set_visible(shell_chrome_visible);
    context_panel.widget().set_visible(shell_chrome_visible);
    inspector_rail.widget().set_visible(shell_chrome_visible);

    if shell_chrome_visible {
        apply_shell_state(
            shell_state,
            tool_rail,
            context_panel,
            content_stack,
            inspector_rail,
            search_view,
        );
    } else {
        content_stack.set_visible_child_name(startup_content_child_name(state));
        inspector_rail.set_open(false);
    }
}

fn refresh_startup_shell(
    client: &KnotdClient,
    shell_state: &ShellState,
    vault_label: &gtk::Label,
    retry_startup_btn: &gtk::Button,
    open_vault_btn: &gtk::Button,
    create_vault_btn: &gtk::Button,
    tool_rail: &ToolRail,
    context_panel: &ContextPanel,
    content_stack: &gtk::Stack,
    inspector_rail: &InspectorRail,
    search_view: &SearchView,
) {
    let startup_state = determine_startup_state(client);
    apply_startup_state(
        &startup_state,
        shell_state,
        vault_label,
        retry_startup_btn,
        open_vault_btn,
        create_vault_btn,
        tool_rail,
        context_panel,
        content_stack,
        inspector_rail,
        search_view,
    );
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

fn build_note_selection_handler(
    client: Rc<KnotdClient>,
    editor: Rc<NoteEditor>,
    current_note: Rc<RefCell<Option<NoteData>>>,
    note_load_state: Rc<RefCell<NoteLoadState>>,
    note_load_generation: Rc<RefCell<u64>>,
    shell_state: Rc<RefCell<ShellState>>,
    window: libadwaita::ApplicationWindow,
    content_stack: gtk::Stack,
    context_panel: Rc<RefCell<ContextPanel>>,
    inspector_rail: InspectorRail,
    tool_rail: ToolRail,
    search_view: Rc<SearchView>,
) -> Box<dyn Fn(&str)> {
    Box::new(move |path| {
        log::info!("Loading note: {}", path);
        window.set_title(Some("Loading note... — Knot"));
        let load_path = path.to_string();
        let log_path = load_path.clone();
        let generation = {
            let mut current = note_load_generation.borrow_mut();
            *current += 1;
            *current
        };

        begin_note_load_with_dispatch(
            client.as_ref().clone(),
            load_path,
            Rc::clone(&note_load_state),
            generation,
            Rc::clone(&note_load_generation),
            |work, ui| {
                async_bridge::run_background(move || work()).attach_local(move |result| {
                    ui(result);
                });
            },
            {
                let window = window.clone();
                let editor = Rc::clone(&editor);
                let current_note = Rc::clone(&current_note);
                let content_stack = content_stack.clone();
                let shell_state = Rc::clone(&shell_state);
                let context_panel = Rc::clone(&context_panel);
                let inspector_rail = inspector_rail.clone();
                let tool_rail = tool_rail.clone();
                let search_view = Rc::clone(&search_view);
                move |result| match result {
                    Ok(note) => {
                        window.set_title(Some(&format!("{} — Knot", note.title)));
                        editor.load_note(&note);
                        *current_note.borrow_mut() = Some(note);
                        let mut shell_state = shell_state.borrow_mut();
                        shell_state.set_note_selected(true);
                        if should_route_loaded_note_to_notes(shell_state.tool_mode()) {
                            shell_state.select_tool(ToolMode::Notes);
                            apply_shell_state(
                                &shell_state,
                                &tool_rail,
                                &context_panel.borrow(),
                                &content_stack,
                                &inspector_rail,
                                search_view.as_ref(),
                            );
                        }
                    }
                    Err(error) => {
                        window.set_title(Some("Failed to load note — Knot"));
                        log::error!("Failed to load note {}: {}", log_path, error);
                    }
                }
            },
        );
    })
}

fn should_route_loaded_note_to_notes(tool_mode: ToolMode) -> bool {
    matches!(tool_mode, ToolMode::Notes | ToolMode::Search)
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

impl KnotWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        let client = KnotdClient::new();
        Self::with_client(app, client)
    }

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
            .label(&startup_header_text(&startup_state))
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
        let context_panel = Rc::new(RefCell::new(ContextPanel::new(Rc::clone(&client))));
        main_box.append(context_panel.borrow().widget());

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
        let daemon_message = match &startup_state {
            StartupState::DaemonUnavailable { message } => message.as_str(),
            _ => "The daemon could not be reached.",
        };
        daemon_unavailable_view.append(
            &gtk::Label::builder()
                .label(daemon_message)
                .css_classes(vec!["dim-label".to_string()])
                .wrap(true)
                .max_width_chars(48)
                .justify(gtk::Justification::Center)
                .build(),
        );
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

        // Graph view (placeholder)
        let graph_view = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let graph_label = gtk::Label::builder()
            .label("Graph view coming soon")
            .vexpand(true)
            .build();
        graph_view.append(&graph_label);
        content_stack.add_titled(&graph_view, Some("graph"), "Graph");

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
            vault_label,
            retry_startup_btn,
            open_vault_btn,
            create_vault_btn,
            editor,
            search_view,
            content_stack,
            current_note: Rc::new(RefCell::new(None)),
            note_load_state: Rc::new(RefCell::new(RequestState::idle())),
            note_load_generation: Rc::new(RefCell::new(0)),
            shell_state: Rc::new(RefCell::new(ShellState::default())),
        };

        apply_startup_state(
            &startup_state,
            &win.shell_state.borrow(),
            &win.vault_label,
            &win.retry_startup_btn,
            &win.open_vault_btn,
            &win.create_vault_btn,
            &win.tool_rail,
            &win.context_panel.borrow(),
            &win.content_stack,
            &win.inspector_rail,
            &win.search_view,
        );

        win.install_window_actions();
        win.setup_signals();

        win
    }

    fn install_window_actions(&self) {
        let action = gio::SimpleAction::new("focus-search", None);
        let shell_state = Rc::clone(&self.shell_state);
        let tool_rail = self.tool_rail.clone();
        let context_panel = Rc::clone(&self.context_panel);
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        action.connect_activate(move |_action, _param| {
            let mut shell_state = shell_state.borrow_mut();
            focus_search_shell(
                &mut shell_state,
                &tool_rail,
                &context_panel.borrow(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
        });
        self.window.add_action(&action);
    }

    fn setup_signals(&self) {
        // Tool mode changes
        let content_stack = self.content_stack.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let inspector_rail = self.inspector_rail.clone();
        let tool_rail = self.tool_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        let shell_state = Rc::clone(&self.shell_state);
        self.tool_rail.connect_mode_changed(move |mode| {
            let mut shell_state = shell_state.borrow_mut();
            shell_state.select_tool(mode);
            apply_shell_state(
                &shell_state,
                &tool_rail,
                &context_panel_ref.borrow(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
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
                &context_panel_ref.borrow(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
        });

        let context_note_selected = build_note_selection_handler(
            Rc::clone(&self.client),
            Rc::clone(&self.editor),
            Rc::clone(&self.current_note),
            Rc::clone(&self.note_load_state),
            Rc::clone(&self.note_load_generation),
            Rc::clone(&self.shell_state),
            self.window.clone(),
            self.content_stack.clone(),
            Rc::clone(&self.context_panel),
            self.inspector_rail.clone(),
            self.tool_rail.clone(),
            Rc::clone(&self.search_view),
        );
        self.context_panel
            .borrow()
            .connect_note_selected(move |path| context_note_selected(path));

        let search_note_selected = build_note_selection_handler(
            Rc::clone(&self.client),
            Rc::clone(&self.editor),
            Rc::clone(&self.current_note),
            Rc::clone(&self.note_load_state),
            Rc::clone(&self.note_load_generation),
            Rc::clone(&self.shell_state),
            self.window.clone(),
            self.content_stack.clone(),
            Rc::clone(&self.context_panel),
            self.inspector_rail.clone(),
            self.tool_rail.clone(),
            Rc::clone(&self.search_view),
        );
        self.search_view
            .connect_result_selected(move |path| search_note_selected(path));

        let retry_startup_btn = self.retry_startup_btn.clone();
        let open_vault_btn = self.open_vault_btn.clone();
        let create_vault_btn = self.create_vault_btn.clone();
        let client = Rc::clone(&self.client);
        let shell_state = Rc::clone(&self.shell_state);
        let vault_label = self.vault_label.clone();
        let tool_rail = self.tool_rail.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        self.retry_startup_btn.connect_clicked(move |_| {
            refresh_startup_shell(
                client.as_ref(),
                &shell_state.borrow(),
                &vault_label,
                &retry_startup_btn,
                &open_vault_btn,
                &create_vault_btn,
                &tool_rail,
                &context_panel_ref.borrow(),
                &content_stack,
                &inspector_rail,
                search_view.as_ref(),
            );
        });

        let retry_startup_btn = self.retry_startup_btn.clone();
        let open_vault_btn = self.open_vault_btn.clone();
        let create_vault_btn = self.create_vault_btn.clone();
        let client = Rc::clone(&self.client);
        let shell_state = Rc::clone(&self.shell_state);
        let vault_label = self.vault_label.clone();
        let tool_rail = self.tool_rail.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        let window = self.window.clone();
        self.open_vault_btn.connect_clicked(move |_| {
            let client = Rc::clone(&client);
            let shell_state = Rc::clone(&shell_state);
            let vault_label = vault_label.clone();
            let tool_rail = tool_rail.clone();
            let context_panel_ref = Rc::clone(&context_panel_ref);
            let content_stack = content_stack.clone();
            let inspector_rail = inspector_rail.clone();
            let search_view = Rc::clone(&search_view);
            let retry_startup_btn = retry_startup_btn.clone();
            let open_vault_btn = open_vault_btn.clone();
            let create_vault_btn = create_vault_btn.clone();
            choose_vault_directory(&window, "Open vault", "Open", move |path| {
                if let Err(error) = client.open_vault(&path) {
                    log::error!("Failed to open vault {}: {}", path, error);
                }
                refresh_startup_shell(
                    client.as_ref(),
                    &shell_state.borrow(),
                    &vault_label,
                    &retry_startup_btn,
                    &open_vault_btn,
                    &create_vault_btn,
                    &tool_rail,
                    &context_panel_ref.borrow(),
                    &content_stack,
                    &inspector_rail,
                    search_view.as_ref(),
                );
            });
        });

        let retry_startup_btn = self.retry_startup_btn.clone();
        let open_vault_btn = self.open_vault_btn.clone();
        let create_vault_btn = self.create_vault_btn.clone();
        let client = Rc::clone(&self.client);
        let shell_state = Rc::clone(&self.shell_state);
        let vault_label = self.vault_label.clone();
        let tool_rail = self.tool_rail.clone();
        let context_panel_ref = Rc::clone(&self.context_panel);
        let content_stack = self.content_stack.clone();
        let inspector_rail = self.inspector_rail.clone();
        let search_view = Rc::clone(&self.search_view);
        let window = self.window.clone();
        self.create_vault_btn.connect_clicked(move |_| {
            let client = Rc::clone(&client);
            let shell_state = Rc::clone(&shell_state);
            let vault_label = vault_label.clone();
            let tool_rail = tool_rail.clone();
            let context_panel_ref = Rc::clone(&context_panel_ref);
            let content_stack = content_stack.clone();
            let inspector_rail = inspector_rail.clone();
            let search_view = Rc::clone(&search_view);
            let retry_startup_btn = retry_startup_btn.clone();
            let open_vault_btn = open_vault_btn.clone();
            let create_vault_btn = create_vault_btn.clone();
            choose_vault_directory(&window, "Create vault", "Create", move |path| {
                if let Err(error) = client.create_vault(&path) {
                    log::error!("Failed to create vault {}: {}", path, error);
                }
                refresh_startup_shell(
                    client.as_ref(),
                    &shell_state.borrow(),
                    &vault_label,
                    &retry_startup_btn,
                    &open_vault_btn,
                    &create_vault_btn,
                    &tool_rail,
                    &context_panel_ref.borrow(),
                    &content_stack,
                    &inspector_rail,
                    search_view.as_ref(),
                );
            });
        });

        // Inspector close
        let inspector_rail = self.inspector_rail.clone();
        self.inspector_rail.connect_close(move || {
            inspector_rail.set_open(false);
        });
    }

    pub fn widget(&self) -> &libadwaita::ApplicationWindow {
        &self.window
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
    use crate::ui::request_state::RequestState;
    use std::cell::Cell;

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

    #[test]
    fn startup_state_reports_daemon_unavailable_when_client_cannot_connect() {
        let state = determine_startup_state(&KnotdClient::with_socket_path("/tmp/missing.sock"));

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
            KnotdClient::with_socket_path("/tmp/knot.sock"),
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
            KnotdClient::with_socket_path("/tmp/knot.sock"),
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
        let stale_result: Rc<RefCell<Option<Box<dyn FnOnce()>>>> = Rc::new(RefCell::new(None));
        let fresh_result: Rc<RefCell<Option<Box<dyn FnOnce()>>>> = Rc::new(RefCell::new(None));
        let first_note = sample_note();
        let mut second_note = sample_note();
        second_note.title = "Second".to_string();

        begin_note_load_with_dispatch(
            KnotdClient::with_socket_path("/tmp/knot.sock"),
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
            KnotdClient::with_socket_path("/tmp/knot.sock"),
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
    fn note_load_completion_only_routes_to_notes_from_notes_or_search() {
        assert!(should_route_loaded_note_to_notes(ToolMode::Notes));
        assert!(should_route_loaded_note_to_notes(ToolMode::Search));
        assert!(!should_route_loaded_note_to_notes(ToolMode::Graph));
        assert!(!should_route_loaded_note_to_notes(ToolMode::Settings));
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
}
