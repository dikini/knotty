use gtk::prelude::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::client::{KnotdClient, MaintenanceResult, VaultPluginInfo, VaultSettings};
use crate::config::knotty_config::{
    default_context_panel_width, default_inspector_width, knotty_config_path, load_knotty_config,
    save_knotty_config, ColorSchemePreference, KnottyConfig,
};
use crate::ui::async_bridge;

type PreferencesChangedCallback = Rc<RefCell<Option<Box<dyn Fn(KnottyConfig)>>>>;
type RefreshCallback = Rc<dyn Fn()>;

#[derive(Clone)]
struct VaultWidgets {
    file_visibility_entry: gtk::Entry,
    font_size_spin: gtk::SpinButton,
    tab_size_spin: gtk::SpinButton,
    status_label: gtk::Label,
}

#[derive(Clone)]
struct PluginWidgets {
    plugins_enabled_switch: gtk::Switch,
    status_label: gtk::Label,
}

#[derive(Clone)]
struct AppearanceWidgets {
    color_scheme_dropdown: gtk::DropDown,
    context_width_spin: gtk::SpinButton,
    inspector_width_spin: gtk::SpinButton,
    status_label: gtk::Label,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VaultSettingsForm {
    file_visibility: String,
    font_size: i32,
    tab_size: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PluginSettingsForm {
    plugins_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Appearance,
    Controls,
    Vault,
    Plugins,
    Maintenance,
}

impl SettingsSection {
    pub fn all() -> &'static [SettingsSection] {
        &[
            SettingsSection::General,
            SettingsSection::Appearance,
            SettingsSection::Controls,
            SettingsSection::Vault,
            SettingsSection::Plugins,
            SettingsSection::Maintenance,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            SettingsSection::General => "General",
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Controls => "Controls",
            SettingsSection::Vault => "Vault",
            SettingsSection::Plugins => "Plugins",
            SettingsSection::Maintenance => "Maintenance",
        }
    }

    pub fn stack_name(self) -> &'static str {
        match self {
            SettingsSection::General => "general",
            SettingsSection::Appearance => "appearance",
            SettingsSection::Controls => "controls",
            SettingsSection::Vault => "vault",
            SettingsSection::Plugins => "plugins",
            SettingsSection::Maintenance => "maintenance",
        }
    }
}

pub struct SettingsView {
    widget: gtk::Box,
    stack: gtk::Stack,
    selected_section: Cell<SettingsSection>,
    on_preferences_changed: PreferencesChangedCallback,
    refresh_all: RefreshCallback,
}

impl SettingsView {
    pub fn new(client: Rc<KnotdClient>, initial_config: KnottyConfig) -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();
        let stack = gtk::Stack::builder().hexpand(true).vexpand(true).build();
        let on_preferences_changed: PreferencesChangedCallback = Rc::new(RefCell::new(None));
        let current_config = Rc::new(RefCell::new(initial_config.clone()));

        let general_page = section_page(SettingsSection::General);
        general_page.append(
            &gtk::Label::builder()
                .label("Application preferences are stored locally and apply across vaults.")
                .wrap(true)
                .xalign(0.0)
                .build(),
        );
        general_page.append(
            &gtk::Label::builder()
                .label(match knotty_config_path() {
                    Ok(path) => format!("Config file: {}", path.display()),
                    Err(error) => format!("Config file unavailable: {}", error),
                })
                .css_classes(vec!["dim-label".to_string()])
                .wrap(true)
                .xalign(0.0)
                .selectable(true)
                .build(),
        );

        let appearance_status = section_status_label("Loaded appearance preferences.");
        let color_scheme_model = gtk::StringList::new(&["Follow system", "Light", "Dark"]);
        let color_scheme_dropdown = gtk::DropDown::builder().model(&color_scheme_model).build();
        let context_width_spin = spin_button(default_context_panel_width(), 220, 480, 4);
        let inspector_width_spin = spin_button(default_inspector_width(), 220, 480, 4);
        apply_config_to_widgets(
            &initial_config,
            &color_scheme_dropdown,
            &context_width_spin,
            &inspector_width_spin,
        );
        let appearance_save_btn = gtk::Button::with_label("Save appearance");
        let appearance_reload_btn = gtk::Button::with_label("Reload from disk");
        let appearance_actions = action_row(&[&appearance_save_btn, &appearance_reload_btn]);
        let appearance_page = section_page(SettingsSection::Appearance);
        appearance_page.append(&labeled_row("Color scheme", &color_scheme_dropdown));
        appearance_page.append(&labeled_row("Context pane width", &context_width_spin));
        appearance_page.append(&labeled_row("Inspector width", &inspector_width_spin));
        appearance_page.append(&appearance_actions);
        appearance_page.append(&appearance_status);

        let controls_page = section_page(SettingsSection::Controls);
        controls_page.append(
            &gtk::Label::builder()
                .label("Controls preferences are planned for a later batch in this slice.")
                .css_classes(vec!["dim-label".to_string()])
                .wrap(true)
                .xalign(0.0)
                .build(),
        );

        let vault_status = section_status_label("Loading vault settings...");
        let file_visibility_entry = gtk::Entry::builder()
            .placeholder_text("e.g. visible")
            .hexpand(true)
            .build();
        let font_size_spin = spin_button(16, 8, 48, 1);
        let tab_size_spin = spin_button(4, 2, 12, 1);
        let vault_refresh_btn = gtk::Button::with_label("Refresh");
        let vault_save_btn = gtk::Button::with_label("Save vault settings");
        let vault_actions = action_row(&[&vault_save_btn, &vault_refresh_btn]);
        let vault_page = section_page(SettingsSection::Vault);
        vault_page.append(&labeled_row("File visibility", &file_visibility_entry));
        vault_page.append(&labeled_row("Editor font size", &font_size_spin));
        vault_page.append(&labeled_row("Editor tab size", &tab_size_spin));
        vault_page.append(&vault_actions);
        vault_page.append(&vault_status);
        let vault_widgets = VaultWidgets {
            file_visibility_entry: file_visibility_entry.clone(),
            font_size_spin: font_size_spin.clone(),
            tab_size_spin: tab_size_spin.clone(),
            status_label: vault_status.clone(),
        };

        let plugins_status = section_status_label("Loading plugins...");
        let plugins_enabled_switch = gtk::Switch::builder().halign(gtk::Align::Start).build();
        let plugins_refresh_btn = gtk::Button::with_label("Refresh plugins");
        let plugins_save_btn = gtk::Button::with_label("Save plugin settings");
        let plugins_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();
        let plugins_page = section_page(SettingsSection::Plugins);
        plugins_page.append(&labeled_row(
            "Plugins enabled for this vault",
            &plugins_enabled_switch,
        ));
        plugins_page.append(&action_row(&[&plugins_save_btn, &plugins_refresh_btn]));
        plugins_page.append(&plugins_status);
        plugins_page.append(&plugins_list);
        let plugin_widgets = PluginWidgets {
            plugins_enabled_switch: plugins_enabled_switch.clone(),
            status_label: plugins_status.clone(),
        };

        let maintenance_status = section_status_label("No maintenance action running.");
        let reindex_btn = gtk::Button::with_label("Reindex vault");
        let maintenance_page = section_page(SettingsSection::Maintenance);
        maintenance_page.append(&reindex_btn);
        maintenance_page.append(&maintenance_status);

        stack.add_titled(
            &general_page,
            Some(SettingsSection::General.stack_name()),
            "General",
        );
        stack.add_titled(
            &appearance_page,
            Some(SettingsSection::Appearance.stack_name()),
            "Appearance",
        );
        stack.add_titled(
            &controls_page,
            Some(SettingsSection::Controls.stack_name()),
            "Controls",
        );
        stack.add_titled(
            &vault_page,
            Some(SettingsSection::Vault.stack_name()),
            "Vault",
        );
        stack.add_titled(
            &plugins_page,
            Some(SettingsSection::Plugins.stack_name()),
            "Plugins",
        );
        stack.add_titled(
            &maintenance_page,
            Some(SettingsSection::Maintenance.stack_name()),
            "Maintenance",
        );
        stack.set_visible_child_name(SettingsSection::General.stack_name());
        widget.append(&stack);

        wire_appearance_actions(
            Rc::clone(&on_preferences_changed),
            Rc::clone(&current_config),
            AppearanceWidgets {
                color_scheme_dropdown: color_scheme_dropdown.clone(),
                context_width_spin: context_width_spin.clone(),
                inspector_width_spin: inspector_width_spin.clone(),
                status_label: appearance_status.clone(),
            },
            appearance_save_btn,
            appearance_reload_btn,
        );
        let refresh_vault = Rc::new({
            let client = Rc::clone(&client);
            let vault_widgets = vault_widgets.clone();
            let plugin_widgets = plugin_widgets.clone();
            move || {
                refresh_vault_settings(
                    Rc::clone(&client),
                    vault_widgets.clone(),
                    plugin_widgets.clone(),
                )
            }
        });
        wire_vault_actions(
            Rc::clone(&client),
            vault_widgets.clone(),
            plugin_widgets.clone(),
            vault_save_btn,
            vault_refresh_btn,
        );
        wire_plugins_actions(
            Rc::clone(&client),
            plugin_widgets.clone(),
            plugins_list.clone(),
            plugins_save_btn,
            plugins_refresh_btn,
        );
        wire_maintenance_actions(Rc::clone(&client), maintenance_status.clone(), reindex_btn);

        let refresh_plugins_cb = Rc::new({
            let client = Rc::clone(&client);
            let plugin_widgets = plugin_widgets.clone();
            let plugins_list = plugins_list.clone();
            move || {
                refresh_plugins(
                    Rc::clone(&client),
                    plugin_widgets.clone(),
                    plugins_list.clone(),
                )
            }
        });
        let refresh_all: RefreshCallback = Rc::new({
            let refresh_vault = Rc::clone(&refresh_vault);
            let refresh_plugins = Rc::clone(&refresh_plugins_cb);
            move || {
                refresh_vault();
                refresh_plugins();
            }
        });
        refresh_all();

        Self {
            widget,
            stack,
            selected_section: Cell::new(SettingsSection::General),
            on_preferences_changed,
            refresh_all,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }

    pub fn set_section(&self, section: SettingsSection) {
        self.selected_section.set(section);
        self.stack.set_visible_child_name(section.stack_name());
    }

    pub fn selected_section(&self) -> SettingsSection {
        self.selected_section.get()
    }

    pub fn refresh(&self) {
        (self.refresh_all)();
    }

    pub fn connect_preferences_changed<F>(&self, f: F)
    where
        F: Fn(KnottyConfig) + 'static,
    {
        *self.on_preferences_changed.borrow_mut() = Some(Box::new(f));
    }
}

fn section_page(section: SettingsSection) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .hexpand(true)
        .vexpand(true)
        .build();
    page.append(
        &gtk::Label::builder()
            .label(section.title())
            .css_classes(vec!["title-3".to_string()])
            .xalign(0.0)
            .build(),
    );
    page
}

fn labeled_row(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .build();
    row.append(
        &gtk::Label::builder()
            .label(label)
            .xalign(0.0)
            .css_classes(vec!["caption-heading".to_string()])
            .build(),
    );
    row.append(widget);
    row
}

fn action_row(buttons: &[&gtk::Button]) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();
    for button in buttons {
        row.append(*button);
    }
    row
}

fn section_status_label(initial: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(initial)
        .css_classes(vec!["dim-label".to_string()])
        .wrap(true)
        .xalign(0.0)
        .build()
}

fn spin_button(initial: i32, min: i32, max: i32, step: i32) -> gtk::SpinButton {
    gtk::SpinButton::with_range(min as f64, max as f64, step as f64).tap(|spin| {
        spin.set_value(initial as f64);
        spin.set_hexpand(false);
        spin.set_halign(gtk::Align::Start);
    })
}

trait WidgetExtTap {
    fn tap<F: FnOnce(&Self)>(self, f: F) -> Self
    where
        Self: Sized;
}

impl<T> WidgetExtTap for T {
    fn tap<F: FnOnce(&Self)>(self, f: F) -> Self
    where
        Self: Sized,
    {
        f(&self);
        self
    }
}

fn apply_config_to_widgets(
    config: &KnottyConfig,
    color_scheme_dropdown: &gtk::DropDown,
    context_width_spin: &gtk::SpinButton,
    inspector_width_spin: &gtk::SpinButton,
) {
    color_scheme_dropdown.set_selected(match config.appearance.color_scheme {
        ColorSchemePreference::System => 0,
        ColorSchemePreference::Light => 1,
        ColorSchemePreference::Dark => 2,
    });
    context_width_spin.set_value(config.appearance.context_panel_width as f64);
    inspector_width_spin.set_value(config.appearance.inspector_width as f64);
}

fn gather_config_from_widgets(
    existing_config: &KnottyConfig,
    color_scheme_dropdown: &gtk::DropDown,
    context_width_spin: &gtk::SpinButton,
    inspector_width_spin: &gtk::SpinButton,
) -> KnottyConfig {
    let color_scheme = match color_scheme_dropdown.selected() {
        1 => ColorSchemePreference::Light,
        2 => ColorSchemePreference::Dark,
        _ => ColorSchemePreference::System,
    };
    let mut config = existing_config.clone();
    config.appearance = crate::config::knotty_config::AppearancePreferences {
        context_panel_width: context_width_spin.value_as_int(),
        inspector_width: inspector_width_spin.value_as_int(),
        color_scheme,
    };
    config
}

fn apply_vault_settings_form(settings: &VaultSettings, widgets: &VaultWidgets) {
    widgets
        .file_visibility_entry
        .set_text(&settings.file_visibility);
    widgets
        .font_size_spin
        .set_value(settings.editor.font_size as f64);
    widgets
        .tab_size_spin
        .set_value(settings.editor.tab_size as f64);
}

fn current_vault_settings_form(widgets: &VaultWidgets) -> VaultSettingsForm {
    VaultSettingsForm {
        file_visibility: widgets.file_visibility_entry.text().to_string(),
        font_size: widgets.font_size_spin.value_as_int(),
        tab_size: widgets.tab_size_spin.value_as_int(),
    }
}

fn current_plugin_settings_form(widgets: &PluginWidgets) -> PluginSettingsForm {
    PluginSettingsForm {
        plugins_enabled: widgets.plugins_enabled_switch.is_active(),
    }
}

fn build_plugin_rows(plugins: &[VaultPluginInfo], plugins_list: &gtk::ListBox) {
    while let Some(child) = plugins_list.first_child() {
        plugins_list.remove(&child);
    }

    if plugins.is_empty() {
        plugins_list.append(&plugin_info_row(
            "No plugins reported by knotd",
            Some("The vault currently has no visible plugin state."),
        ));
        return;
    }

    for plugin in plugins {
        let subtitle = match plugin.effective_enabled {
            Some(effective) if effective != plugin.enabled => Some(format!(
                "configured: {}, effective: {}",
                yes_no(plugin.enabled),
                yes_no(effective)
            )),
            _ => Some(format!("enabled: {}", yes_no(plugin.enabled))),
        };
        plugins_list.append(&plugin_info_row(
            if plugin.title.is_empty() {
                &plugin.id
            } else {
                &plugin.title
            },
            subtitle.as_deref(),
        ));
    }
}

fn plugin_info_row(title: &str, subtitle: Option<&str>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&gtk::Label::builder().label(title).xalign(0.0).build());
    if let Some(subtitle) = subtitle {
        content.append(
            &gtk::Label::builder()
                .label(subtitle)
                .css_classes(vec!["dim-label".to_string()])
                .wrap(true)
                .xalign(0.0)
                .build(),
        );
    }
    row.set_child(Some(&content));
    row
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn build_vault_settings_patch(form: &VaultSettingsForm) -> Value {
    json!({
        "file_visibility": form.file_visibility,
        "editor": {
            "font_size": form.font_size,
            "tab_size": form.tab_size,
        }
    })
}

fn build_plugin_settings_patch(form: &PluginSettingsForm) -> Value {
    json!({
        "plugins_enabled": form.plugins_enabled,
    })
}

fn format_maintenance_result(result: &MaintenanceResult) -> String {
    match result {
        MaintenanceResult::Message(message) => message.clone(),
        MaintenanceResult::Count(count) => format!("Reindexed {} items", count),
        MaintenanceResult::Object {
            message: Some(message),
            ..
        } => message.clone(),
        MaintenanceResult::Object {
            count: Some(count), ..
        }
        | MaintenanceResult::Object {
            reindexed: Some(count),
            ..
        } => format!("Reindexed {} items", count),
        MaintenanceResult::Object { .. } => "Maintenance complete".to_string(),
    }
}

fn wire_appearance_actions(
    on_preferences_changed: PreferencesChangedCallback,
    current_config: Rc<RefCell<KnottyConfig>>,
    widgets: AppearanceWidgets,
    save_button: gtk::Button,
    reload_button: gtk::Button,
) {
    let save_widgets = widgets.clone();
    let save_preferences_callback = Rc::clone(&on_preferences_changed);
    let save_current_config = Rc::clone(&current_config);
    save_button.connect_clicked(move |_| {
        save_widgets
            .status_label
            .set_label("Saving appearance preferences...");
        let config = gather_config_from_widgets(
            &save_current_config.borrow(),
            &save_widgets.color_scheme_dropdown,
            &save_widgets.context_width_spin,
            &save_widgets.inspector_width_spin,
        );
        let status_label = save_widgets.status_label.clone();
        let on_preferences_changed = Rc::clone(&save_preferences_callback);
        let save_current_config = Rc::clone(&save_current_config);
        async_bridge::run_background(move || {
            save_knotty_config(&config)?;
            Ok::<_, String>(config)
        })
        .attach_local(move |result| match result {
            Ok(config) => {
                status_label.set_label("Saved appearance preferences.");
                *save_current_config.borrow_mut() = config.clone();
                if let Some(callback) = &*on_preferences_changed.borrow() {
                    callback(config);
                }
            }
            Err(error) => {
                status_label
                    .set_label(&format!("Failed to save appearance preferences: {}", error));
            }
        });
    });

    let reload_widgets = widgets.clone();
    let reload_preferences_callback = Rc::clone(&on_preferences_changed);
    let reload_current_config = Rc::clone(&current_config);
    reload_button.connect_clicked(move |_| {
        reload_widgets
            .status_label
            .set_label("Reloading appearance preferences...");
        let status_label = reload_widgets.status_label.clone();
        let color_scheme_dropdown = reload_widgets.color_scheme_dropdown.clone();
        let context_width_spin = reload_widgets.context_width_spin.clone();
        let inspector_width_spin = reload_widgets.inspector_width_spin.clone();
        let on_preferences_changed = Rc::clone(&reload_preferences_callback);
        let reload_current_config = Rc::clone(&reload_current_config);
        async_bridge::run_background(load_knotty_config).attach_local(move |result| match result {
            Ok(config) => {
                apply_config_to_widgets(
                    &config,
                    &color_scheme_dropdown,
                    &context_width_spin,
                    &inspector_width_spin,
                );
                *reload_current_config.borrow_mut() = config.clone();
                status_label.set_label("Reloaded appearance preferences.");
                if let Some(callback) = &*on_preferences_changed.borrow() {
                    callback(config);
                }
            }
            Err(error) => {
                status_label.set_label(&format!(
                    "Failed to reload appearance preferences: {}",
                    error
                ));
            }
        });
    });
}

fn wire_vault_actions(
    client: Rc<KnotdClient>,
    widgets: VaultWidgets,
    plugin_widgets: PluginWidgets,
    save_button: gtk::Button,
    refresh_button: gtk::Button,
) {
    let save_client = Rc::clone(&client);
    let save_widgets = widgets.clone();
    let save_plugin_widgets = plugin_widgets.clone();
    save_button.connect_clicked(move |_| {
        save_widgets
            .status_label
            .set_label("Saving vault settings...");
        let form = current_vault_settings_form(&save_widgets);
        let patch = build_vault_settings_patch(&form);
        let widgets = save_widgets.clone();
        let plugin_widgets = save_plugin_widgets.clone();
        let client = save_client.as_ref().clone();
        async_bridge::run_background(move || {
            client
                .update_vault_settings(patch)
                .map_err(|e| e.to_string())
        })
        .attach_local(move |result| match result {
            Ok(settings) => {
                apply_vault_settings_form(&settings, &widgets);
                plugin_widgets
                    .plugins_enabled_switch
                    .set_active(settings.plugins_enabled);
                widgets.status_label.set_label("Saved vault settings.");
            }
            Err(error) => {
                widgets
                    .status_label
                    .set_label(&format!("Failed to save vault settings: {}", error));
            }
        });
    });

    let refresh_client = Rc::clone(&client);
    let refresh_widgets = widgets.clone();
    let refresh_plugin_widgets = plugin_widgets.clone();
    refresh_button.connect_clicked(move |_| {
        refresh_vault_settings(
            Rc::clone(&refresh_client),
            refresh_widgets.clone(),
            refresh_plugin_widgets.clone(),
        );
    });
}

fn refresh_vault_settings(
    client: Rc<KnotdClient>,
    widgets: VaultWidgets,
    plugin_widgets: PluginWidgets,
) {
    widgets.status_label.set_label("Loading vault settings...");
    let client = client.as_ref().clone();
    async_bridge::run_background(move || client.get_vault_settings().map_err(|e| e.to_string()))
        .attach_local(move |result| match result {
            Ok(settings) => {
                apply_vault_settings_form(&settings, &widgets);
                plugin_widgets
                    .plugins_enabled_switch
                    .set_active(settings.plugins_enabled);
                widgets.status_label.set_label("Loaded vault settings.");
            }
            Err(error) => {
                widgets
                    .status_label
                    .set_label(&format!("Failed to load vault settings: {}", error));
                plugin_widgets
                    .status_label
                    .set_label(&format!("Failed to load plugin settings: {}", error));
            }
        });
}

fn wire_plugins_actions(
    client: Rc<KnotdClient>,
    widgets: PluginWidgets,
    plugins_list: gtk::ListBox,
    save_button: gtk::Button,
    refresh_button: gtk::Button,
) {
    let save_client = Rc::clone(&client);
    let save_widgets = widgets.clone();
    save_button.connect_clicked(move |_| {
        save_widgets
            .status_label
            .set_label("Saving plugin settings...");
        let form = current_plugin_settings_form(&save_widgets);
        let patch = build_plugin_settings_patch(&form);
        let widgets = save_widgets.clone();
        let client = save_client.as_ref().clone();
        async_bridge::run_background(move || {
            client
                .update_vault_settings(patch)
                .map_err(|e| e.to_string())
        })
        .attach_local(move |result| match result {
            Ok(settings) => {
                widgets
                    .plugins_enabled_switch
                    .set_active(settings.plugins_enabled);
                widgets.status_label.set_label("Saved plugin settings.");
            }
            Err(error) => {
                widgets
                    .status_label
                    .set_label(&format!("Failed to save plugin settings: {}", error));
            }
        });
    });

    let refresh_widgets = widgets.clone();
    refresh_button.connect_clicked(move |_| {
        refresh_plugins(
            Rc::clone(&client),
            refresh_widgets.clone(),
            plugins_list.clone(),
        );
    });
}

fn refresh_plugins(client: Rc<KnotdClient>, widgets: PluginWidgets, plugins_list: gtk::ListBox) {
    widgets.status_label.set_label("Loading plugins...");
    let client = client.as_ref().clone();
    async_bridge::run_background(move || client.list_vault_plugins().map_err(|e| e.to_string()))
        .attach_local(move |result| match result {
            Ok(plugins) => {
                build_plugin_rows(&plugins, &plugins_list);
                widgets.status_label.set_label(if plugins.is_empty() {
                    "No plugins reported by knotd."
                } else {
                    "Loaded plugin state."
                });
            }
            Err(error) => {
                build_plugin_rows(&[], &plugins_list);
                widgets
                    .status_label
                    .set_label(&format!("Failed to load plugins: {}", error));
            }
        });
}

fn wire_maintenance_actions(
    client: Rc<KnotdClient>,
    status_label: gtk::Label,
    reindex_button: gtk::Button,
) {
    reindex_button.connect_clicked(move |_| {
        status_label.set_label("Reindexing vault...");
        let status_label = status_label.clone();
        let client = client.as_ref().clone();
        async_bridge::run_background(move || client.reindex_vault().map_err(|e| e.to_string()))
            .attach_local(move |result| match result {
                Ok(result) => status_label.set_label(&format_maintenance_result(&result)),
                Err(error) => {
                    status_label.set_label(&format!("Failed to reindex vault: {}", error));
                }
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_sections_are_stable_and_include_controls() {
        assert_eq!(
            SettingsSection::all(),
            &[
                SettingsSection::General,
                SettingsSection::Appearance,
                SettingsSection::Controls,
                SettingsSection::Vault,
                SettingsSection::Plugins,
                SettingsSection::Maintenance,
            ]
        );
    }

    #[test]
    fn vault_settings_patch_only_includes_editable_fields() {
        let patch = build_vault_settings_patch(&VaultSettingsForm {
            file_visibility: "visible".to_string(),
            font_size: 18,
            tab_size: 4,
        });

        assert_eq!(
            patch,
            json!({
                "file_visibility": "visible",
                "editor": {
                    "font_size": 18,
                    "tab_size": 4
                }
            })
        );
    }

    #[test]
    fn plugin_settings_patch_only_includes_master_toggle() {
        let patch = build_plugin_settings_patch(&PluginSettingsForm {
            plugins_enabled: true,
        });

        assert_eq!(
            patch,
            json!({
                "plugins_enabled": true
            })
        );
    }

    #[test]
    fn maintenance_success_prefers_explicit_message_then_count() {
        assert_eq!(
            format_maintenance_result(&MaintenanceResult::Message("Reindex complete".to_string())),
            "Reindex complete"
        );
        assert_eq!(
            format_maintenance_result(&MaintenanceResult::Count(42)),
            "Reindexed 42 items"
        );
    }

    #[test]
    fn gather_config_from_widgets_preserves_existing_automation_settings() {
        gtk::init().ok();
        let dropdown = gtk::DropDown::from_strings(&["Follow system", "Light", "Dark"]);
        dropdown.set_selected(2);
        let context = spin_button(320, 220, 480, 4);
        let inspector = spin_button(300, 220, 480, 4);
        let existing = KnottyConfig {
            automation: crate::config::knotty_config::AutomationConfig { enabled: true },
            ..KnottyConfig::default()
        };

        let config = gather_config_from_widgets(&existing, &dropdown, &context, &inspector);

        assert!(config.automation.enabled);
        assert_eq!(config.appearance.color_scheme, ColorSchemePreference::Dark);
        assert_eq!(config.appearance.context_panel_width, 320);
        assert_eq!(config.appearance.inspector_width, 300);
    }
}
