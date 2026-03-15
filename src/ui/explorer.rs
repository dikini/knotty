//! Explorer tree rendering and mutation dispatch.
//!
//! The current slice keeps `TreeView`/`TreeStore` temporarily even though GTK marks
//! them deprecated. Replacing the widget family cleanly would spill across shell
//! and mutation flows, so this module isolates the deprecated usage while locking
//! explorer behavior behind deterministic adapters and tests.

use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use tracing as log;

use crate::client::{ExplorerFolderNode, ExplorerNoteNode, ExplorerTree, KnotdClient};
use crate::ui::async_bridge;

const COL_ICON: u32 = 0;
const COL_DISPLAY_NAME: u32 = 1;
const COL_PATH: u32 = 2;
const COL_IS_FOLDER: u32 = 3;
const COL_EXPANDED: u32 = 4;
const COL_BADGE: u32 = 5;

type NoteSelectedCallback = Rc<RefCell<Option<Box<dyn Fn(&str)>>>>;
type FolderToggledCallback = Rc<RefCell<Option<Box<dyn Fn(&str, bool)>>>>;
type SelectionChangedCallback = Rc<RefCell<Option<Box<dyn Fn(Option<ExplorerSelection>)>>>>;
type SelectionClearedCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
type StatusChangedCallback = Rc<RefCell<Option<Box<dyn Fn(&str, bool)>>>>;
type NoteSwitchGuard = Rc<RefCell<Option<Box<dyn Fn(&str) -> NoteSwitchDecision>>>>;
type FolderPersistenceHandler = Rc<dyn Fn(&str, bool)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorerSelection {
    Folder { path: String },
    Note { path: String },
}

impl ExplorerSelection {
    pub fn path(&self) -> &str {
        match self {
            Self::Folder { path } | Self::Note { path } => path,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSwitchDecision {
    Allow,
    Deny,
    SaveThenAllow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExplorerRowKind {
    Folder { expanded: bool },
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExplorerRowData {
    icon_name: String,
    display_name: String,
    path: String,
    badge: String,
    kind: ExplorerRowKind,
}

impl ExplorerRowData {
    fn from_folder(folder: &ExplorerFolderNode) -> Self {
        Self {
            icon_name: "folder".to_string(),
            display_name: folder.name.clone(),
            path: folder.path.clone(),
            badge: String::new(),
            kind: ExplorerRowKind::Folder {
                expanded: folder.expanded,
            },
        }
    }

    fn from_note(note: &ExplorerNoteNode) -> Self {
        let (icon_name, badge) = note_type_indicator(note.type_badge.as_deref());
        let display_name = if badge.is_empty() {
            note.display_title.clone()
        } else {
            format!("{}  [{}]", note.display_title, badge)
        };

        Self {
            icon_name,
            display_name,
            path: note.path.clone(),
            badge,
            kind: ExplorerRowKind::Note,
        }
    }

    fn selection(&self) -> ExplorerSelection {
        match self.kind {
            ExplorerRowKind::Folder { .. } => ExplorerSelection::Folder {
                path: self.path.clone(),
            },
            ExplorerRowKind::Note => ExplorerSelection::Note {
                path: self.path.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefreshFollowUp {
    PreserveCurrent(Option<ExplorerSelection>),
    Focus(ExplorerSelection),
    ActivateNote(String),
    ClearSelection,
    FocusAndClear(ExplorerSelection),
}

#[derive(Clone)]
struct ExplorerHandles {
    tree_view: gtk::TreeView,
    store: gtk::TreeStore,
    client: Rc<KnotdClient>,
    path_index: Rc<RefCell<HashMap<String, gtk::TreePath>>>,
    on_note_selected: NoteSelectedCallback,
    on_folder_toggled: FolderToggledCallback,
    on_selection_changed: SelectionChangedCallback,
    on_selection_cleared: SelectionClearedCallback,
    on_status_changed: StatusChangedCallback,
    note_switch_guard: NoteSwitchGuard,
    selected_item: Rc<RefCell<Option<ExplorerSelection>>>,
    refresh_generation: Rc<Cell<u64>>,
    suppress_folder_persistence_depth: Rc<Cell<u32>>,
    suppress_note_activation: Rc<Cell<bool>>,
}

pub struct ExplorerView {
    widget: gtk::ScrolledWindow,
    handles: ExplorerHandles,
}

fn note_type_indicator(type_badge: Option<&str>) -> (String, String) {
    let badge_lower = type_badge.map(str::to_lowercase);
    match badge_lower.as_deref() {
        Some("youtube") => ("video-x-generic".to_string(), "YT".to_string()),
        Some("pdf") => ("application-pdf".to_string(), "PDF".to_string()),
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("svg") => {
            ("image-x-generic".to_string(), String::new())
        }
        Some("image") => ("image-x-generic".to_string(), String::new()),
        Some(other) => ("text-x-generic".to_string(), other.to_uppercase()),
        None => ("text-x-markdown".to_string(), String::new()),
    }
}

fn note_selection_request(row: &ExplorerRowData) -> Option<String> {
    match row.kind {
        ExplorerRowKind::Note => Some(row.path.clone()),
        ExplorerRowKind::Folder { .. } => None,
    }
}

fn folder_toggle_request(row: &ExplorerRowData, expanded: bool) -> Option<(String, bool)> {
    match row.kind {
        ExplorerRowKind::Folder { .. } => Some((row.path.clone(), expanded)),
        ExplorerRowKind::Note => None,
    }
}

fn read_row_data(store: &gtk::TreeStore, iter: &gtk::TreeIter) -> ExplorerRowData {
    let icon_name = store
        .get_value(iter, COL_ICON as i32)
        .get()
        .unwrap_or_default();
    let display_name = store
        .get_value(iter, COL_DISPLAY_NAME as i32)
        .get()
        .unwrap_or_default();
    let path = store
        .get_value(iter, COL_PATH as i32)
        .get()
        .unwrap_or_default();
    let is_folder = store
        .get_value(iter, COL_IS_FOLDER as i32)
        .get()
        .unwrap_or(false);
    let expanded = store
        .get_value(iter, COL_EXPANDED as i32)
        .get()
        .unwrap_or(false);
    let badge = store
        .get_value(iter, COL_BADGE as i32)
        .get()
        .unwrap_or_default();

    let kind = if is_folder {
        ExplorerRowKind::Folder { expanded }
    } else {
        ExplorerRowKind::Note
    };

    ExplorerRowData {
        icon_name,
        display_name,
        path,
        badge,
        kind,
    }
}

fn append_row(
    store: &gtk::TreeStore,
    path_index: &Rc<RefCell<HashMap<String, gtk::TreePath>>>,
    parent: Option<&gtk::TreeIter>,
    row: &ExplorerRowData,
) -> gtk::TreeIter {
    let iter = store.insert(parent, -1);
    let (is_folder, expanded) = match row.kind {
        ExplorerRowKind::Folder { expanded } => (true, expanded),
        ExplorerRowKind::Note => (false, false),
    };

    store.set(
        &iter,
        &[
            (COL_ICON, &row.icon_name),
            (COL_DISPLAY_NAME, &row.display_name),
            (COL_PATH, &row.path),
            (COL_IS_FOLDER, &is_folder),
            (COL_EXPANDED, &expanded),
            (COL_BADGE, &row.badge),
        ],
    );
    path_index
        .borrow_mut()
        .insert(row.path.clone(), store.path(&iter));
    iter
}

fn emit_note_selection_request(on_note_selected: &NoteSelectedCallback, path: &str) {
    if let Some(ref cb) = *on_note_selected.borrow() {
        cb(path);
    }
}

fn emit_folder_toggle_request(
    persistence_handler: &FolderPersistenceHandler,
    on_folder_toggled: &FolderToggledCallback,
    row: &ExplorerRowData,
    expanded: bool,
) {
    if let Some((path, expanded)) = folder_toggle_request(row, expanded) {
        persistence_handler(path.as_str(), expanded);
        if let Some(ref cb) = *on_folder_toggled.borrow() {
            cb(&path, expanded);
        }
    }
}

fn emit_selection_changed(
    on_selection_changed: &SelectionChangedCallback,
    selection: Option<ExplorerSelection>,
) {
    if let Some(ref cb) = *on_selection_changed.borrow() {
        cb(selection);
    }
}

fn emit_selection_cleared(on_selection_cleared: &SelectionClearedCallback) {
    if let Some(ref cb) = *on_selection_cleared.borrow() {
        cb();
    }
}

fn emit_status(on_status_changed: &StatusChangedCallback, message: &str, is_error: bool) {
    if let Some(ref cb) = *on_status_changed.borrow() {
        cb(message, is_error);
    }
}

fn note_switch_decision(guard: &NoteSwitchGuard, path: &str) -> NoteSwitchDecision {
    guard
        .borrow()
        .as_ref()
        .map(|cb| cb(path))
        .unwrap_or(NoteSwitchDecision::Allow)
}

fn dispatch_note_selection_request(
    on_note_selected: &NoteSelectedCallback,
    note_switch_guard: &NoteSwitchGuard,
    path: &str,
) -> NoteSwitchDecision {
    let decision = note_switch_decision(note_switch_guard, path);
    if matches!(decision, NoteSwitchDecision::Allow) {
        emit_note_selection_request(on_note_selected, path);
    }
    decision
}

fn guard_block_message(decision: NoteSwitchDecision, action: &str) -> Option<String> {
    match decision {
        NoteSwitchDecision::Allow => None,
        NoteSwitchDecision::Deny => Some(format!("{action} is blocked by unsaved note changes.")),
        NoteSwitchDecision::SaveThenAllow => Some(format!(
            "{action} requires save-then-switch, which is not implemented yet."
        )),
    }
}

fn normalized_input_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err("Name cannot be empty.".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn parent_directory(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

fn join_container_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}/{child}")
    }
}

fn normalize_note_name(name: &str) -> Result<String, String> {
    let trimmed = normalized_input_name(name)?;
    let has_extension = Path::new(&trimmed).extension().is_some();
    if has_extension {
        Ok(trimmed)
    } else {
        Ok(format!("{trimmed}.md"))
    }
}

fn selected_container_path(selection: Option<&ExplorerSelection>) -> String {
    match selection {
        Some(ExplorerSelection::Folder { path }) => path.clone(),
        Some(ExplorerSelection::Note { path }) => parent_directory(path),
        None => String::new(),
    }
}

fn note_creation_target(
    selection: Option<&ExplorerSelection>,
    requested_name: &str,
) -> Result<String, String> {
    let name = normalize_note_name(requested_name)?;
    Ok(join_container_path(
        &selected_container_path(selection),
        &name,
    ))
}

fn directory_creation_target(
    selection: Option<&ExplorerSelection>,
    requested_name: &str,
) -> Result<String, String> {
    let name = normalized_input_name(requested_name)?;
    Ok(join_container_path(
        &selected_container_path(selection),
        &name,
    ))
}

fn rename_target(selection: &ExplorerSelection, requested_name: &str) -> Result<String, String> {
    let trimmed = requested_name.trim();
    if trimmed.contains('/') {
        return match selection {
            ExplorerSelection::Folder { .. } => normalized_input_name(trimmed),
            ExplorerSelection::Note { .. } => normalize_note_name(trimmed),
        };
    }

    let new_leaf = match selection {
        ExplorerSelection::Folder { .. } => normalized_input_name(trimmed)?,
        ExplorerSelection::Note { .. } => normalize_note_name(trimmed)?,
    };
    Ok(join_container_path(
        &parent_directory(selection.path()),
        &new_leaf,
    ))
}

fn can_rename_selection(selection: &ExplorerSelection) -> bool {
    !selection.path().trim().is_empty()
}

fn set_silent_note_activation(suppress_note_activation: &Rc<Cell<bool>>) {
    suppress_note_activation.set(true);
}

fn clear_selection_internal(handles: &ExplorerHandles, notify: bool) {
    handles.suppress_note_activation.set(true);
    handles.tree_view.selection().unselect_all();
    *handles.selected_item.borrow_mut() = None;
    if notify {
        emit_selection_changed(&handles.on_selection_changed, None);
    }
}

fn reset_empty_selection_state(
    suppress_note_activation: &Rc<Cell<bool>>,
    selected_item: &Rc<RefCell<Option<ExplorerSelection>>>,
    on_selection_changed: &SelectionChangedCallback,
) {
    suppress_note_activation.set(false);
    *selected_item.borrow_mut() = None;
    emit_selection_changed(on_selection_changed, None);
}

fn handle_empty_selection(handles: &ExplorerHandles) {
    reset_empty_selection_state(
        &handles.suppress_note_activation,
        &handles.selected_item,
        &handles.on_selection_changed,
    );
}

fn select_tree_item(handles: &ExplorerHandles, selection: &ExplorerSelection) -> bool {
    let Some(tree_path) = handles.path_index.borrow().get(selection.path()).cloned() else {
        return false;
    };

    let current = handles.selected_item.borrow().clone();
    let selection_changed = current.as_ref() != Some(selection);
    if selection_changed {
        set_silent_note_activation(&handles.suppress_note_activation);
    }

    handles.tree_view.selection().select_path(&tree_path);
    gtk::prelude::TreeViewExt::set_cursor(
        &handles.tree_view,
        &tree_path,
        None::<&gtk::TreeViewColumn>,
        false,
    );

    if !selection_changed {
        *handles.selected_item.borrow_mut() = Some(selection.clone());
        emit_selection_changed(&handles.on_selection_changed, Some(selection.clone()));
    }

    true
}

fn apply_refresh_follow_up(handles: &ExplorerHandles, follow_up: RefreshFollowUp) {
    match follow_up {
        RefreshFollowUp::PreserveCurrent(Some(selection)) | RefreshFollowUp::Focus(selection) => {
            if !select_tree_item(handles, &selection) {
                clear_selection_internal(handles, true);
            }
        }
        RefreshFollowUp::PreserveCurrent(None) => {
            clear_selection_internal(handles, true);
        }
        RefreshFollowUp::ClearSelection => {
            clear_selection_internal(handles, true);
            emit_selection_cleared(&handles.on_selection_cleared);
        }
        RefreshFollowUp::ActivateNote(path) => {
            request_note_selection_internal(handles, &path);
        }
        RefreshFollowUp::FocusAndClear(selection) => {
            if !select_tree_item(handles, &selection) {
                clear_selection_internal(handles, true);
            }
            emit_selection_cleared(&handles.on_selection_cleared);
        }
    }
}

fn add_folder_node(
    handles: &ExplorerHandles,
    parent: Option<&gtk::TreeIter>,
    folder: &ExplorerFolderNode,
) {
    let row = ExplorerRowData::from_folder(folder);
    let iter = append_row(&handles.store, &handles.path_index, parent, &row);

    for subfolder in &folder.folders {
        add_folder_node(handles, Some(&iter), subfolder);
    }

    for note in &folder.notes {
        add_note_node(handles, &iter, note);
    }

    if folder.expanded {
        let path = handles.store.path(&iter);
        let depth = handles.suppress_folder_persistence_depth.get();
        handles
            .suppress_folder_persistence_depth
            .set(depth.saturating_add(1));
        handles.tree_view.expand_row(&path, false);
        handles.suppress_folder_persistence_depth.set(depth);
    }
}

fn add_note_node(handles: &ExplorerHandles, parent: &gtk::TreeIter, note: &ExplorerNoteNode) {
    let row = ExplorerRowData::from_note(note);
    append_row(&handles.store, &handles.path_index, Some(parent), &row);
}

fn load_explorer_tree(handles: &ExplorerHandles, tree: &ExplorerTree) {
    handles.store.clear();
    handles.path_index.borrow_mut().clear();
    add_folder_node(handles, None, &tree.root);
}

fn refresh_with_follow_up(handles: ExplorerHandles, follow_up: RefreshFollowUp) {
    let generation = handles.refresh_generation.get().saturating_add(1);
    handles.refresh_generation.set(generation);
    let client = handles.client.as_ref().clone();

    async_bridge::run_background(move || {
        client
            .get_explorer_tree()
            .map_err(|error| error.to_string())
    })
    .attach_local(move |result| {
        if handles.refresh_generation.get() != generation {
            return;
        }
        match result {
            Ok(tree) => {
                load_explorer_tree(&handles, &tree);
                apply_refresh_follow_up(&handles, follow_up);
            }
            Err(error) => {
                log::error!("Failed to load explorer tree: {}", error);
                emit_status(&handles.on_status_changed, &error, true);
            }
        }
    });
}

fn run_mutation_job<T, Work, OnSuccess>(
    handles: ExplorerHandles,
    work: Work,
    on_success: OnSuccess,
    error_prefix: &'static str,
) where
    T: Send + 'static,
    Work: FnOnce(KnotdClient) -> Result<T, String> + Send + 'static,
    OnSuccess: FnOnce(ExplorerHandles, T) + 'static,
{
    let client = handles.client.as_ref().clone();
    async_bridge::run_background(move || work(client)).attach_local(move |result| match result {
        Ok(value) => on_success(handles, value),
        Err(error) => {
            log::error!("{}: {}", error_prefix, error);
            emit_status(&handles.on_status_changed, &error, true);
        }
    });
}

fn request_note_selection_internal(handles: &ExplorerHandles, path: &str) {
    let selection = ExplorerSelection::Note {
        path: path.to_string(),
    };
    let _ = select_tree_item(handles, &selection);
    let decision = dispatch_note_selection_request(
        &handles.on_note_selected,
        &handles.note_switch_guard,
        path,
    );
    if let Some(message) = guard_block_message(decision, "Switching notes") {
        emit_status(&handles.on_status_changed, &message, true);
        clear_selection_internal(handles, true);
    }
}

fn fallback_selection_after_removal(selection: &ExplorerSelection) -> Option<ExplorerSelection> {
    let parent = parent_directory(selection.path());
    if parent.is_empty() {
        None
    } else {
        Some(ExplorerSelection::Folder { path: parent })
    }
}

fn folder_removal_follow_up(path: &str) -> RefreshFollowUp {
    fallback_selection_after_removal(&ExplorerSelection::Folder {
        path: path.to_string(),
    })
    .map(RefreshFollowUp::FocusAndClear)
    .unwrap_or(RefreshFollowUp::ClearSelection)
}

fn mutation_guard_allows(
    handles: &ExplorerHandles,
    path: &str,
    action: &str,
) -> Result<(), String> {
    let decision = note_switch_decision(&handles.note_switch_guard, path);
    if let Some(message) = guard_block_message(decision, action) {
        emit_status(&handles.on_status_changed, &message, true);
        Err(message)
    } else {
        Ok(())
    }
}

impl ExplorerView {
    pub fn new(client: Rc<KnotdClient>) -> Self {
        let persistence_handler: FolderPersistenceHandler = {
            let client = client.as_ref().clone();
            Rc::new(move |path, expanded| {
                let client = client.clone();
                let path = path.to_string();
                let rpc_path = path.clone();
                async_bridge::run_background(move || {
                    client
                        .set_folder_expanded(&rpc_path, expanded)
                        .map_err(|error| error.to_string())
                })
                .attach_local(move |result| {
                    if let Err(error) = result {
                        log::error!(
                            "Failed to persist folder expansion {} -> {}: {}",
                            path,
                            expanded,
                            error
                        );
                    }
                });
            })
        };
        Self::with_persistence_handler(client, persistence_handler)
    }

    fn with_persistence_handler(
        client: Rc<KnotdClient>,
        persistence_handler: FolderPersistenceHandler,
    ) -> Self {
        let store = gtk::TreeStore::new(&[
            String::static_type(),
            String::static_type(),
            String::static_type(),
            bool::static_type(),
            bool::static_type(),
            String::static_type(),
        ]);

        let tree_view = gtk::TreeView::builder()
            .model(&store)
            .headers_visible(false)
            .build();

        let icon_renderer = gtk::CellRendererPixbuf::new();
        let icon_column = gtk::TreeViewColumn::builder().title("").build();
        icon_column.pack_start(&icon_renderer, false);
        icon_column.add_attribute(&icon_renderer, "icon-name", 0);
        tree_view.append_column(&icon_column);

        let name_column = gtk::TreeViewColumn::builder()
            .title("Name")
            .expand(true)
            .build();

        let text_renderer = gtk::CellRendererText::new();
        text_renderer.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name_column.pack_start(&text_renderer, true);
        name_column.add_attribute(&text_renderer, "text", 1);
        tree_view.append_column(&name_column);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&tree_view)
            .build();

        let handles = ExplorerHandles {
            tree_view: tree_view.clone(),
            store: store.clone(),
            client,
            path_index: Rc::new(RefCell::new(HashMap::new())),
            on_note_selected: Rc::new(RefCell::new(None)),
            on_folder_toggled: Rc::new(RefCell::new(None)),
            on_selection_changed: Rc::new(RefCell::new(None)),
            on_selection_cleared: Rc::new(RefCell::new(None)),
            on_status_changed: Rc::new(RefCell::new(None)),
            note_switch_guard: Rc::new(RefCell::new(None)),
            selected_item: Rc::new(RefCell::new(None)),
            refresh_generation: Rc::new(Cell::new(0)),
            suppress_folder_persistence_depth: Rc::new(Cell::new(0)),
            suppress_note_activation: Rc::new(Cell::new(false)),
        };

        let store_for_expand = store.clone();
        let persist_for_expand = Rc::clone(&persistence_handler);
        let handles_for_expand = handles.clone();
        tree_view.connect_row_expanded(move |_, iter, _| {
            if handles_for_expand.suppress_folder_persistence_depth.get() > 0 {
                return;
            }
            let row = read_row_data(&store_for_expand, iter);
            emit_folder_toggle_request(
                &persist_for_expand,
                &handles_for_expand.on_folder_toggled,
                &row,
                true,
            );
        });

        let store_for_collapse = store.clone();
        let persist_for_collapse = Rc::clone(&persistence_handler);
        let handles_for_collapse = handles.clone();
        tree_view.connect_row_collapsed(move |_, iter, _| {
            if handles_for_collapse.suppress_folder_persistence_depth.get() > 0 {
                return;
            }
            let row = read_row_data(&store_for_collapse, iter);
            emit_folder_toggle_request(
                &persist_for_collapse,
                &handles_for_collapse.on_folder_toggled,
                &row,
                false,
            );
        });

        let store_for_selection = store.clone();
        let handles_for_selection = handles.clone();
        tree_view.connect_cursor_changed(move |view| {
            let Some((model, iter)) = view.selection().selected() else {
                handle_empty_selection(&handles_for_selection);
                return;
            };

            let store = model
                .downcast_ref::<gtk::TreeStore>()
                .unwrap_or(&store_for_selection);
            let row = read_row_data(store, &iter);
            let previous_selection = handles_for_selection.selected_item.borrow().clone();
            let selection = row.selection();
            *handles_for_selection.selected_item.borrow_mut() = Some(selection.clone());
            emit_selection_changed(
                &handles_for_selection.on_selection_changed,
                Some(selection.clone()),
            );

            if handles_for_selection
                .suppress_note_activation
                .replace(false)
            {
                return;
            }

            if let Some(path) = note_selection_request(&row) {
                let decision = dispatch_note_selection_request(
                    &handles_for_selection.on_note_selected,
                    &handles_for_selection.note_switch_guard,
                    &path,
                );
                if let Some(message) = guard_block_message(decision, "Switching notes") {
                    emit_status(&handles_for_selection.on_status_changed, &message, true);
                    match previous_selection {
                        Some(previous_selection) => {
                            if !select_tree_item(&handles_for_selection, &previous_selection) {
                                clear_selection_internal(&handles_for_selection, true);
                            }
                        }
                        None => clear_selection_internal(&handles_for_selection, true),
                    }
                }
            }
        });

        Self {
            widget: scrolled,
            handles,
        }
    }

    pub fn load_explorer_tree(&self, tree: &ExplorerTree) {
        load_explorer_tree(&self.handles, tree);
    }

    pub fn refresh(&self) {
        refresh_with_follow_up(
            self.handles.clone(),
            RefreshFollowUp::PreserveCurrent(self.selected_item()),
        );
    }

    pub fn request_note_selection(&self, path: &str) {
        request_note_selection_internal(&self.handles, path);
    }

    pub fn selected_item(&self) -> Option<ExplorerSelection> {
        self.handles.selected_item.borrow().clone()
    }

    pub fn create_note_in_selected_container(&self, requested_name: &str) -> Result<(), String> {
        let target_path = note_creation_target(self.selected_item().as_ref(), requested_name)?;
        mutation_guard_allows(&self.handles, &target_path, "Creating a note")?;
        let handles = self.handles.clone();
        run_mutation_job(
            handles,
            move |client| {
                client
                    .create_note(&target_path, None)
                    .map_err(|error| error.to_string())
            },
            move |handles, note| {
                emit_status(&handles.on_status_changed, "Note created.", false);
                refresh_with_follow_up(handles, RefreshFollowUp::ActivateNote(note.path));
            },
            "Failed to create note",
        );
        Ok(())
    }

    pub fn create_directory_in_selected_container(
        &self,
        requested_name: &str,
    ) -> Result<(), String> {
        let target_path = directory_creation_target(self.selected_item().as_ref(), requested_name)?;
        mutation_guard_allows(&self.handles, &target_path, "Creating a folder")?;
        let handles = self.handles.clone();
        let refresh_target = target_path.clone();
        run_mutation_job(
            handles,
            move |client| {
                client
                    .create_directory(&target_path)
                    .map_err(|error| error.to_string())
            },
            move |handles, ()| {
                emit_status(&handles.on_status_changed, "Folder created.", false);
                refresh_with_follow_up(
                    handles,
                    RefreshFollowUp::Focus(ExplorerSelection::Folder {
                        path: refresh_target,
                    }),
                );
            },
            "Failed to create directory",
        );
        Ok(())
    }

    pub fn rename_selected(&self, requested_name: &str) -> Result<(), String> {
        let selection = self
            .selected_item()
            .ok_or_else(|| "Select a note or folder first.".to_string())?;
        if !can_rename_selection(&selection) {
            return Err("The root folder cannot be renamed.".to_string());
        }
        let target_path = rename_target(&selection, requested_name)?;
        mutation_guard_allows(&self.handles, selection.path(), "Renaming")?;
        let handles = self.handles.clone();
        match selection {
            ExplorerSelection::Note { path } => {
                let refresh_target = target_path.clone();
                run_mutation_job(
                    handles,
                    move |client| {
                        client
                            .rename_note(&path, &target_path)
                            .map_err(|error| error.to_string())
                    },
                    move |handles, ()| {
                        emit_status(&handles.on_status_changed, "Note renamed.", false);
                        refresh_with_follow_up(
                            handles,
                            RefreshFollowUp::ActivateNote(refresh_target),
                        );
                    },
                    "Failed to rename note",
                );
            }
            ExplorerSelection::Folder { path } => {
                let refresh_target = target_path.clone();
                run_mutation_job(
                    handles,
                    move |client| {
                        client
                            .rename_directory(&path, &target_path)
                            .map_err(|error| error.to_string())
                    },
                    move |handles, ()| {
                        emit_status(&handles.on_status_changed, "Folder renamed.", false);
                        refresh_with_follow_up(
                            handles,
                            RefreshFollowUp::Focus(ExplorerSelection::Folder {
                                path: refresh_target,
                            }),
                        );
                    },
                    "Failed to rename folder",
                );
            }
        }
        Ok(())
    }

    pub fn delete_selected(&self) -> Result<(), String> {
        let selection = self
            .selected_item()
            .ok_or_else(|| "Select a note or folder first.".to_string())?;
        match &selection {
            ExplorerSelection::Folder { path } if path.trim().is_empty() => {
                return Err("The root folder cannot be removed.".to_string());
            }
            _ => {}
        }
        mutation_guard_allows(&self.handles, selection.path(), "Deleting")?;

        let handles = self.handles.clone();
        match selection {
            ExplorerSelection::Note { path } => {
                let follow_up = fallback_selection_after_removal(&ExplorerSelection::Note {
                    path: path.clone(),
                })
                .map(RefreshFollowUp::FocusAndClear)
                .unwrap_or(RefreshFollowUp::ClearSelection);
                run_mutation_job(
                    handles,
                    move |client| client.delete_note(&path).map_err(|error| error.to_string()),
                    move |handles, ()| {
                        emit_status(&handles.on_status_changed, "Note deleted.", false);
                        refresh_with_follow_up(handles, follow_up);
                    },
                    "Failed to delete note",
                );
            }
            ExplorerSelection::Folder { path } => {
                let follow_up = folder_removal_follow_up(&path);
                run_mutation_job(
                    handles,
                    move |client| {
                        client
                            .remove_directory(&path, true)
                            .map_err(|error| error.to_string())
                    },
                    move |handles, ()| {
                        emit_status(&handles.on_status_changed, "Folder removed.", false);
                        refresh_with_follow_up(handles, follow_up);
                    },
                    "Failed to remove folder",
                );
            }
        }
        Ok(())
    }

    pub fn connect_note_selected<F>(&self, f: F)
    where
        F: Fn(&str) + 'static,
    {
        *self.handles.on_note_selected.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_folder_toggled<F>(&self, f: F)
    where
        F: Fn(&str, bool) + 'static,
    {
        *self.handles.on_folder_toggled.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_selection_changed<F>(&self, f: F)
    where
        F: Fn(Option<ExplorerSelection>) + 'static,
    {
        *self.handles.on_selection_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_selection_cleared<F>(&self, f: F)
    where
        F: Fn() + 'static,
    {
        *self.handles.on_selection_cleared.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_status_changed<F>(&self, f: F)
    where
        F: Fn(&str, bool) + 'static,
    {
        *self.handles.on_status_changed.borrow_mut() = Some(Box::new(f));
    }

    pub fn connect_note_switch_guard<F>(&self, f: F)
    where
        F: Fn(&str) -> NoteSwitchDecision + 'static,
    {
        *self.handles.note_switch_guard.borrow_mut() = Some(Box::new(f));
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.widget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_rows_keep_name_path_and_expanded_state() {
        let folder = ExplorerFolderNode {
            path: "notes/projects".to_string(),
            name: "Projects".to_string(),
            expanded: true,
            folders: Vec::new(),
            notes: Vec::new(),
        };

        let row = ExplorerRowData::from_folder(&folder);

        assert_eq!(
            row,
            ExplorerRowData {
                icon_name: "folder".to_string(),
                display_name: "Projects".to_string(),
                path: "notes/projects".to_string(),
                badge: String::new(),
                kind: ExplorerRowKind::Folder { expanded: true },
            }
        );
    }

    #[test]
    fn note_rows_render_badges_and_icons_deterministically() {
        let note = ExplorerNoteNode {
            path: "notes/guide.pdf".to_string(),
            title: "Guide".to_string(),
            display_title: "Guide".to_string(),
            modified_at: 0,
            word_count: 42,
            type_badge: Some("pdf".to_string()),
            is_dimmed: false,
        };

        let row = ExplorerRowData::from_note(&note);

        assert_eq!(row.icon_name, "application-pdf");
        assert_eq!(row.display_name, "Guide  [PDF]");
        assert_eq!(row.badge, "PDF");
        assert_eq!(row.kind, ExplorerRowKind::Note);
    }

    #[test]
    fn folder_toggle_requests_only_emit_for_folder_rows() {
        let folder_row = ExplorerRowData::from_folder(&ExplorerFolderNode {
            path: "notes/projects".to_string(),
            name: "Projects".to_string(),
            expanded: false,
            folders: Vec::new(),
            notes: Vec::new(),
        });
        let note_row = ExplorerRowData::from_note(&ExplorerNoteNode {
            path: "notes/guide.pdf".to_string(),
            title: "Guide".to_string(),
            display_title: "Guide".to_string(),
            modified_at: 0,
            word_count: 42,
            type_badge: Some("pdf".to_string()),
            is_dimmed: false,
        });

        assert_eq!(
            folder_toggle_request(&folder_row, true),
            Some(("notes/projects".to_string(), true))
        );
        assert_eq!(folder_toggle_request(&note_row, true), None);
    }

    #[test]
    fn note_selection_requests_only_emit_for_note_rows() {
        let folder_row = ExplorerRowData::from_folder(&ExplorerFolderNode {
            path: "notes/projects".to_string(),
            name: "Projects".to_string(),
            expanded: true,
            folders: Vec::new(),
            notes: Vec::new(),
        });
        let note_row = ExplorerRowData::from_note(&ExplorerNoteNode {
            path: "notes/guide.pdf".to_string(),
            title: "Guide".to_string(),
            display_title: "Guide".to_string(),
            modified_at: 0,
            word_count: 42,
            type_badge: Some("pdf".to_string()),
            is_dimmed: false,
        });

        assert_eq!(note_selection_request(&folder_row), None);
        assert_eq!(
            note_selection_request(&note_row),
            Some("notes/guide.pdf".to_string())
        );
    }

    #[test]
    fn emit_note_selection_request_uses_shared_callback_path() {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let on_note_selected: NoteSelectedCallback = Rc::new(RefCell::new(None));
        *on_note_selected.borrow_mut() = Some(Box::new({
            let requests = Rc::clone(&requests);
            move |path| requests.borrow_mut().push(path.to_string())
        }));

        emit_note_selection_request(&on_note_selected, "notes/example.md");

        assert_eq!(*requests.borrow(), vec!["notes/example.md".to_string()]);
    }

    #[test]
    fn dispatch_note_selection_request_respects_guard_decision() {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let on_note_selected: NoteSelectedCallback = Rc::new(RefCell::new(None));
        *on_note_selected.borrow_mut() = Some(Box::new({
            let requests = Rc::clone(&requests);
            move |path| requests.borrow_mut().push(path.to_string())
        }));
        let note_switch_guard: NoteSwitchGuard =
            Rc::new(RefCell::new(Some(Box::new(|_| NoteSwitchDecision::Deny))));

        let decision =
            dispatch_note_selection_request(&on_note_selected, &note_switch_guard, "notes/a.md");

        assert_eq!(decision, NoteSwitchDecision::Deny);
        assert!(requests.borrow().is_empty());
    }

    #[test]
    fn dispatch_note_selection_request_does_not_emit_for_save_then_allow() {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let on_note_selected: NoteSelectedCallback = Rc::new(RefCell::new(None));
        *on_note_selected.borrow_mut() = Some(Box::new({
            let requests = Rc::clone(&requests);
            move |path| requests.borrow_mut().push(path.to_string())
        }));
        let note_switch_guard: NoteSwitchGuard = Rc::new(RefCell::new(Some(Box::new(|_| {
            NoteSwitchDecision::SaveThenAllow
        }))));

        let decision =
            dispatch_note_selection_request(&on_note_selected, &note_switch_guard, "notes/a.md");

        assert_eq!(decision, NoteSwitchDecision::SaveThenAllow);
        assert!(requests.borrow().is_empty());
    }

    #[test]
    fn emit_folder_toggle_request_persists_and_notifies_once() {
        let persisted = Rc::new(RefCell::new(Vec::new()));
        let notified = Rc::new(RefCell::new(Vec::new()));
        let persistence_handler: FolderPersistenceHandler = Rc::new({
            let persisted = Rc::clone(&persisted);
            move |path, expanded| persisted.borrow_mut().push((path.to_string(), expanded))
        });
        let on_folder_toggled: FolderToggledCallback = Rc::new(RefCell::new(None));
        *on_folder_toggled.borrow_mut() = Some(Box::new({
            let notified = Rc::clone(&notified);
            move |path, expanded| notified.borrow_mut().push((path.to_string(), expanded))
        }));
        let row = ExplorerRowData::from_folder(&ExplorerFolderNode {
            path: "notes/projects".to_string(),
            name: "Projects".to_string(),
            expanded: false,
            folders: Vec::new(),
            notes: Vec::new(),
        });

        emit_folder_toggle_request(&persistence_handler, &on_folder_toggled, &row, true);

        assert_eq!(
            *persisted.borrow(),
            vec![("notes/projects".to_string(), true)]
        );
        assert_eq!(
            *notified.borrow(),
            vec![("notes/projects".to_string(), true)]
        );
    }

    #[test]
    fn create_note_target_uses_selected_folder_or_note_parent() {
        let folder_selection = ExplorerSelection::Folder {
            path: "notes/projects".to_string(),
        };
        let note_selection = ExplorerSelection::Note {
            path: "notes/projects/guide.md".to_string(),
        };

        assert_eq!(
            note_creation_target(Some(&folder_selection), "Draft").unwrap(),
            "notes/projects/Draft.md"
        );
        assert_eq!(
            note_creation_target(Some(&note_selection), "Draft.md").unwrap(),
            "notes/projects/Draft.md"
        );
        assert_eq!(note_creation_target(None, "Inbox").unwrap(), "Inbox.md");
    }

    #[test]
    fn create_directory_target_uses_selected_container() {
        let note_selection = ExplorerSelection::Note {
            path: "notes/projects/guide.md".to_string(),
        };

        assert_eq!(
            directory_creation_target(Some(&note_selection), "Archive").unwrap(),
            "notes/projects/Archive"
        );
        assert_eq!(
            directory_creation_target(None, "Projects").unwrap(),
            "Projects"
        );
    }

    #[test]
    fn rename_target_keeps_existing_parent_directory() {
        let note_selection = ExplorerSelection::Note {
            path: "notes/projects/guide.md".to_string(),
        };
        let folder_selection = ExplorerSelection::Folder {
            path: "notes/projects".to_string(),
        };

        assert_eq!(
            rename_target(&note_selection, "updated").unwrap(),
            "notes/projects/updated.md"
        );
        assert_eq!(
            rename_target(&folder_selection, "archive").unwrap(),
            "notes/archive"
        );
    }

    #[test]
    fn rename_target_accepts_full_target_paths_for_move_flows() {
        let note_selection = ExplorerSelection::Note {
            path: "notes/projects/guide.md".to_string(),
        };
        let folder_selection = ExplorerSelection::Folder {
            path: "notes/projects".to_string(),
        };

        assert_eq!(
            rename_target(&note_selection, "archive/updated").unwrap(),
            "archive/updated.md"
        );
        assert_eq!(
            rename_target(&folder_selection, "archive/projects").unwrap(),
            "archive/projects"
        );
    }

    #[test]
    fn root_folder_cannot_be_renamed() {
        let root_selection = ExplorerSelection::Folder {
            path: String::new(),
        };

        assert!(!can_rename_selection(&root_selection));
    }

    #[test]
    fn guard_block_message_covers_deny_and_save_then_allow() {
        assert_eq!(
            guard_block_message(NoteSwitchDecision::Deny, "Deleting"),
            Some("Deleting is blocked by unsaved note changes.".to_string())
        );
        assert_eq!(
            guard_block_message(NoteSwitchDecision::SaveThenAllow, "Deleting"),
            Some("Deleting requires save-then-switch, which is not implemented yet.".to_string())
        );
        assert_eq!(
            guard_block_message(NoteSwitchDecision::Allow, "Deleting"),
            None
        );
    }

    #[test]
    fn fallback_selection_after_note_removal_prefers_parent_folder() {
        let note_selection = ExplorerSelection::Note {
            path: "notes/projects/guide.md".to_string(),
        };

        assert_eq!(
            fallback_selection_after_removal(&note_selection),
            Some(ExplorerSelection::Folder {
                path: "notes/projects".to_string(),
            })
        );
        assert_eq!(
            fallback_selection_after_removal(&ExplorerSelection::Note {
                path: "guide.md".to_string(),
            }),
            None
        );
    }

    #[test]
    fn handle_empty_selection_clears_suppression_and_selection() {
        let suppress_note_activation = Rc::new(Cell::new(true));
        let selected_item = Rc::new(RefCell::new(Some(ExplorerSelection::Note {
            path: "notes/example.md".to_string(),
        })));
        let selection_events = Rc::new(RefCell::new(Vec::new()));
        let on_selection_changed: SelectionChangedCallback =
            Rc::new(RefCell::new(Some(Box::new({
                let selection_events = Rc::clone(&selection_events);
                move |selection| selection_events.borrow_mut().push(selection)
            }))));

        reset_empty_selection_state(
            &suppress_note_activation,
            &selected_item,
            &on_selection_changed,
        );

        assert!(!suppress_note_activation.get());
        assert_eq!(*selected_item.borrow(), None);
        assert_eq!(*selection_events.borrow(), vec![None]);
    }

    #[test]
    fn folder_removal_focuses_parent_and_clears_active_note() {
        assert_eq!(
            folder_removal_follow_up("notes/projects"),
            RefreshFollowUp::FocusAndClear(ExplorerSelection::Folder {
                path: "notes".to_string(),
            })
        );
        assert_eq!(
            folder_removal_follow_up("projects"),
            RefreshFollowUp::ClearSelection
        );
    }
}
