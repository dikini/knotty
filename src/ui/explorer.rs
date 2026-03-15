//! Explorer tree rendering and mutation dispatch.

use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use tracing as log;

use crate::client::{ExplorerFolderNode, ExplorerNoteNode, ExplorerTree, KnotdClient};
use crate::ui::async_bridge;
use crate::ui::note_types::note_type_indicator;

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
    #[allow(dead_code)]
    Deny,
    #[allow(dead_code)]
    SaveThenAllow,
    Prompt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExplorerRowKind {
    Folder,
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
            kind: ExplorerRowKind::Folder,
        }
    }

    fn from_note(note: &ExplorerNoteNode) -> Self {
        let indicator = note_type_indicator(note.type_badge.as_deref());
        let display_name = if indicator.badge.is_empty() {
            note.display_title.clone()
        } else {
            format!("{}  [{}]", note.display_title, indicator.badge)
        };

        Self {
            icon_name: indicator.icon_name,
            display_name,
            path: note.path.clone(),
            badge: indicator.badge,
            kind: ExplorerRowKind::Note,
        }
    }

    fn selection(&self) -> ExplorerSelection {
        match self.kind {
            ExplorerRowKind::Folder => ExplorerSelection::Folder {
                path: self.path.clone(),
            },
            ExplorerRowKind::Note => ExplorerSelection::Note {
                path: self.path.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExplorerItem {
    row: ExplorerRowData,
    children: Vec<ExplorerItem>,
}

impl ExplorerItem {
    fn from_folder(folder: &ExplorerFolderNode) -> Self {
        let mut children = Vec::with_capacity(folder.folders.len() + folder.notes.len());
        children.extend(folder.folders.iter().map(Self::from_folder));
        children.extend(folder.notes.iter().map(Self::from_note));

        Self {
            row: ExplorerRowData::from_folder(folder),
            children,
        }
    }

    fn from_note(note: &ExplorerNoteNode) -> Self {
        Self {
            row: ExplorerRowData::from_note(note),
            children: Vec::new(),
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
    selection_model: gtk::SingleSelection,
    tree_model: Rc<RefCell<Option<gtk::TreeListModel>>>,
    client: Rc<KnotdClient>,
    path_index: Rc<RefCell<HashMap<String, u32>>>,
    observed_rows: Rc<RefCell<HashSet<usize>>>,
    folder_persistence_handler: FolderPersistenceHandler,
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

fn note_selection_request(row: &ExplorerRowData) -> Option<String> {
    match row.kind {
        ExplorerRowKind::Note => Some(row.path.clone()),
        ExplorerRowKind::Folder => None,
    }
}

fn folder_toggle_request(row: &ExplorerRowData, expanded: bool) -> Option<(String, bool)> {
    match row.kind {
        ExplorerRowKind::Folder => Some((row.path.clone(), expanded)),
        ExplorerRowKind::Note => None,
    }
}

fn emit_note_selection_request(on_note_selected: &NoteSelectedCallback, path: &str) {
    if let Some(ref cb) = *on_note_selected.borrow() {
        cb(path);
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
            "{action} requires saving the current note before proceeding."
        )),
        NoteSwitchDecision::Prompt => None,
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

fn expanded_folder_paths(folder: &ExplorerFolderNode) -> Vec<String> {
    fn collect(folder: &ExplorerFolderNode, output: &mut Vec<String>) {
        if folder.expanded {
            output.push(folder.path.clone());
        }
        for child in &folder.folders {
            collect(child, output);
        }
    }

    let mut output = Vec::new();
    collect(folder, &mut output);
    output
}

fn selection_expansion_paths(selection: &ExplorerSelection) -> Vec<String> {
    if selection.path().trim().is_empty() {
        return Vec::new();
    }

    let target_parent = match selection {
        ExplorerSelection::Note { path } | ExplorerSelection::Folder { path } => {
            parent_directory(path)
        }
    };

    let mut paths = vec![String::new()];
    if target_parent.is_empty() {
        return paths;
    }

    let mut current = String::new();
    for segment in target_parent
        .split('/')
        .filter(|segment| !segment.is_empty())
    {
        current = join_container_path(&current, segment);
        paths.push(current.clone());
    }
    paths
}

fn build_list_store(items: &[ExplorerItem]) -> gio::ListStore {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    for item in items {
        store.append(&glib::BoxedAnyObject::new(item.clone()));
    }
    store
}

fn tree_root_items(tree: &ExplorerTree) -> Vec<ExplorerItem> {
    vec![ExplorerItem::from_folder(&tree.root)]
}

fn child_model_for_object(item: &glib::Object) -> Option<gio::ListModel> {
    let boxed = item.clone().downcast::<glib::BoxedAnyObject>().ok()?;
    let children = {
        let borrowed = boxed.borrow::<ExplorerItem>();
        borrowed.children.clone()
    };

    if children.is_empty() {
        None
    } else {
        Some(build_list_store(&children).upcast())
    }
}

fn row_data_from_tree_list_row(row: &gtk::TreeListRow) -> Option<ExplorerRowData> {
    let item = row.item()?;
    let boxed = item.downcast::<glib::BoxedAnyObject>().ok()?;
    let data = boxed.borrow::<ExplorerItem>();
    Some(data.row.clone())
}

fn rebuild_path_index(handles: &ExplorerHandles) {
    let mut path_index = HashMap::new();
    let Some(model) = handles.tree_model.borrow().clone() else {
        *handles.path_index.borrow_mut() = path_index;
        return;
    };

    let count = model.n_items();
    for position in 0..count {
        let Some(row) = model.row(position) else {
            continue;
        };
        let Some(data) = row_data_from_tree_list_row(&row) else {
            continue;
        };
        path_index.insert(data.path.clone(), position);
    }

    *handles.path_index.borrow_mut() = path_index;
}

fn set_folder_expanded_internal(
    handles: &ExplorerHandles,
    path: &str,
    expanded: bool,
    persist: bool,
) -> bool {
    rebuild_path_index(handles);
    let Some(position) = handles.path_index.borrow().get(path).copied() else {
        return false;
    };
    let Some(model) = handles.tree_model.borrow().clone() else {
        return false;
    };
    let Some(row) = model.row(position) else {
        return false;
    };
    let Some(data) = row_data_from_tree_list_row(&row) else {
        return false;
    };
    if !matches!(data.kind, ExplorerRowKind::Folder) {
        return false;
    }
    if row.is_expanded() == expanded {
        return true;
    }

    if !persist {
        let depth = handles.suppress_folder_persistence_depth.get();
        handles
            .suppress_folder_persistence_depth
            .set(depth.saturating_add(1));
        row.set_expanded(expanded);
        handles.suppress_folder_persistence_depth.set(depth);
    } else {
        row.set_expanded(expanded);
    }
    true
}

fn observe_visible_rows(handles: &ExplorerHandles) {
    let Some(model) = handles.tree_model.borrow().clone() else {
        return;
    };

    for position in 0..model.n_items() {
        let Some(row) = model.row(position) else {
            continue;
        };
        let Some(data) = row_data_from_tree_list_row(&row) else {
            continue;
        };
        if !matches!(data.kind, ExplorerRowKind::Folder) {
            continue;
        }

        let key = row.as_ptr() as usize;
        if !handles.observed_rows.borrow_mut().insert(key) {
            continue;
        }

        let handles_for_row = handles.clone();
        row.connect_expanded_notify(move |row| {
            if handles_for_row.suppress_folder_persistence_depth.get() > 0 {
                return;
            }
            let Some(row_data) = row_data_from_tree_list_row(row) else {
                return;
            };
            if let Some((path, expanded)) = folder_toggle_request(&row_data, row.is_expanded()) {
                (handles_for_row.folder_persistence_handler)(path.as_str(), expanded);
                if let Some(ref cb) = *handles_for_row.on_folder_toggled.borrow() {
                    cb(&path, expanded);
                }
            }
        });
    }
}

fn clear_selection_internal(handles: &ExplorerHandles, notify: bool) {
    let had_selection = handles.selection_model.selected() != gtk::INVALID_LIST_POSITION;
    handles.suppress_note_activation.set(true);
    handles
        .selection_model
        .set_selected(gtk::INVALID_LIST_POSITION);
    if !had_selection {
        *handles.selected_item.borrow_mut() = None;
        if notify {
            emit_selection_changed(&handles.on_selection_changed, None);
        }
        handles.suppress_note_activation.set(false);
    }
}

fn should_restore_tree_selection(
    current_selection: Option<&ExplorerSelection>,
    current_position: u32,
    target_position: u32,
    target_selection: &ExplorerSelection,
) -> bool {
    current_selection != Some(target_selection) || current_position != target_position
}

fn select_tree_item(handles: &ExplorerHandles, selection: &ExplorerSelection) -> bool {
    for path in selection_expansion_paths(selection) {
        let _ = set_folder_expanded_internal(handles, &path, true, false);
    }

    rebuild_path_index(handles);
    let Some(position) = handles.path_index.borrow().get(selection.path()).copied() else {
        return false;
    };

    let current = handles.selected_item.borrow().clone();
    let current_position = handles.selection_model.selected();
    if !should_restore_tree_selection(current.as_ref(), current_position, position, selection) {
        *handles.selected_item.borrow_mut() = Some(selection.clone());
        emit_selection_changed(&handles.on_selection_changed, Some(selection.clone()));
        return true;
    }

    set_silent_note_activation(&handles.suppress_note_activation);
    handles.selection_model.set_selected(position);
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

fn load_explorer_tree(handles: &ExplorerHandles, tree: &ExplorerTree) {
    let root_store = build_list_store(&tree_root_items(tree));
    let tree_model = gtk::TreeListModel::new(root_store, false, false, child_model_for_object);

    handles.observed_rows.borrow_mut().clear();
    *handles.tree_model.borrow_mut() = Some(tree_model.clone());
    handles
        .selection_model
        .set_selected(gtk::INVALID_LIST_POSITION);
    handles.selection_model.set_model(Some(&tree_model));

    let handles_for_items = handles.clone();
    tree_model.connect_items_changed(move |_, _, _, _| {
        rebuild_path_index(&handles_for_items);
        observe_visible_rows(&handles_for_items);
    });

    rebuild_path_index(handles);
    observe_visible_rows(handles);

    for path in expanded_folder_paths(&tree.root) {
        let _ = set_folder_expanded_internal(handles, &path, true, false);
    }

    rebuild_path_index(handles);
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

fn bind_factory_row(list_item: &gtk::ListItem) {
    let Some(expander_widget) = list_item.child() else {
        return;
    };
    let Ok(expander) = expander_widget.downcast::<gtk::TreeExpander>() else {
        return;
    };
    let Some(row_object) = list_item.item() else {
        return;
    };
    let Ok(row) = row_object.downcast::<gtk::TreeListRow>() else {
        return;
    };
    expander.set_list_row(Some(&row));

    let Some(container_widget) = expander.child() else {
        return;
    };
    let Ok(container) = container_widget.downcast::<gtk::Box>() else {
        return;
    };
    let Some(icon_widget) = container.first_child() else {
        return;
    };
    let Ok(icon) = icon_widget.downcast::<gtk::Image>() else {
        return;
    };
    let Some(label_widget) = icon.next_sibling() else {
        return;
    };
    let Ok(label) = label_widget.downcast::<gtk::Label>() else {
        return;
    };

    let Some(data) = row_data_from_tree_list_row(&row) else {
        return;
    };
    icon.set_icon_name(Some(&data.icon_name));
    label.set_label(&data.display_name);
    expander.set_hide_expander(!matches!(data.kind, ExplorerRowKind::Folder));
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
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, list_item| {
            let Ok(list_item) = list_item.clone().downcast::<gtk::ListItem>() else {
                return;
            };
            let expander = gtk::TreeExpander::new();
            let row_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(6)
                .build();
            let icon = gtk::Image::new();
            let label = gtk::Label::builder().xalign(0.0).hexpand(true).build();
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            row_box.append(&icon);
            row_box.append(&label);
            expander.set_child(Some(&row_box));
            list_item.set_child(Some(&expander));
        });
        factory.connect_bind(|_, list_item| {
            let Ok(list_item) = list_item.clone().downcast::<gtk::ListItem>() else {
                return;
            };
            bind_factory_row(&list_item);
        });

        let selection_model = gtk::SingleSelection::new(None::<gio::ListModel>);
        selection_model.set_autoselect(false);
        selection_model.set_can_unselect(true);

        let list_view = gtk::ListView::new(Some(selection_model.clone()), Some(factory));
        list_view.set_vexpand(true);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list_view)
            .build();

        let handles = ExplorerHandles {
            selection_model: selection_model.clone(),
            tree_model: Rc::new(RefCell::new(None)),
            client,
            path_index: Rc::new(RefCell::new(HashMap::new())),
            observed_rows: Rc::new(RefCell::new(HashSet::new())),
            folder_persistence_handler: persistence_handler,
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

        let handles_for_selection = handles.clone();
        selection_model.connect_selected_notify(move |selection_model| {
            if selection_model.selected() == gtk::INVALID_LIST_POSITION {
                handle_empty_selection(&handles_for_selection);
                return;
            }

            let Some(row_object) = selection_model.selected_item() else {
                handle_empty_selection(&handles_for_selection);
                return;
            };
            let Ok(row) = row_object.downcast::<gtk::TreeListRow>() else {
                handle_empty_selection(&handles_for_selection);
                return;
            };
            let Some(row_data) = row_data_from_tree_list_row(&row) else {
                handle_empty_selection(&handles_for_selection);
                return;
            };

            let previous_selection = handles_for_selection.selected_item.borrow().clone();
            let selection = row_data.selection();
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

            if let Some(path) = note_selection_request(&row_data) {
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

    pub fn refresh(&self) {
        refresh_with_follow_up(
            self.handles.clone(),
            RefreshFollowUp::PreserveCurrent(self.selected_item()),
        );
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
                kind: ExplorerRowKind::Folder,
            }
        );
    }

    #[test]
    fn unchanged_selection_still_reselects_after_tree_reload() {
        let selection = ExplorerSelection::Note {
            path: "notes/example.md".to_string(),
        };

        assert!(should_restore_tree_selection(
            Some(&selection),
            gtk::INVALID_LIST_POSITION,
            3,
            &selection
        ));
        assert!(should_restore_tree_selection(
            Some(&selection),
            1,
            3,
            &selection
        ));
        assert!(!should_restore_tree_selection(
            Some(&selection),
            3,
            3,
            &selection
        ));
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

        let image_row = ExplorerRowData::from_note(&ExplorerNoteNode {
            path: "notes/screenshot.png".to_string(),
            title: "Screenshot".to_string(),
            display_title: "Screenshot".to_string(),
            modified_at: 0,
            word_count: 0,
            type_badge: Some("image".to_string()),
            is_dimmed: false,
        });
        assert_eq!(image_row.icon_name, "image-x-generic");
        assert_eq!(image_row.display_name, "Screenshot");
        assert!(image_row.badge.is_empty());

        let youtube_row = ExplorerRowData::from_note(&ExplorerNoteNode {
            path: "notes/video.md".to_string(),
            title: "Video".to_string(),
            display_title: "Video".to_string(),
            modified_at: 0,
            word_count: 0,
            type_badge: Some("youtube".to_string()),
            is_dimmed: false,
        });
        assert_eq!(youtube_row.icon_name, "video-x-generic");
        assert_eq!(youtube_row.display_name, "Video  [YT]");
        assert_eq!(youtube_row.badge, "YT");
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
            Some("Deleting requires saving the current note before proceeding.".to_string())
        );
        assert_eq!(
            guard_block_message(NoteSwitchDecision::Allow, "Deleting"),
            None
        );
        assert_eq!(
            guard_block_message(NoteSwitchDecision::Prompt, "Deleting"),
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

    #[test]
    fn expanded_folder_paths_follow_preorder_tree_order() {
        let tree = ExplorerTree {
            root: ExplorerFolderNode {
                path: String::new(),
                name: "root".to_string(),
                expanded: true,
                folders: vec![ExplorerFolderNode {
                    path: "notes".to_string(),
                    name: "notes".to_string(),
                    expanded: true,
                    folders: vec![ExplorerFolderNode {
                        path: "notes/projects".to_string(),
                        name: "projects".to_string(),
                        expanded: false,
                        folders: Vec::new(),
                        notes: Vec::new(),
                    }],
                    notes: Vec::new(),
                }],
                notes: Vec::new(),
            },
            hidden_policy: "show".to_string(),
        };

        assert_eq!(
            expanded_folder_paths(&tree.root),
            vec![String::new(), "notes".to_string()]
        );
    }

    #[test]
    fn selection_expansion_paths_expand_folder_ancestors_only() {
        assert_eq!(
            selection_expansion_paths(&ExplorerSelection::Note {
                path: "notes/projects/guide.md".to_string(),
            }),
            vec![
                String::new(),
                "notes".to_string(),
                "notes/projects".to_string()
            ]
        );
        assert_eq!(
            selection_expansion_paths(&ExplorerSelection::Folder {
                path: "notes/projects".to_string(),
            }),
            vec![String::new(), "notes".to_string()]
        );
        assert_eq!(
            selection_expansion_paths(&ExplorerSelection::Folder {
                path: String::new(),
            }),
            Vec::<String>::new()
        );
    }
}
