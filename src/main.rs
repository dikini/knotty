use gtk::prelude::*;
use libadwaita::prelude::*;
use std::path::PathBuf;

mod cli;
mod client;
mod runtime_contract;
mod ui;

use cli::CliArgs;
use client::KnotdClient;
use ui::window::KnotWindow;

// Global socket path storage
use std::sync::OnceLock;
pub static SOCKET_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static BACKGROUND_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

const APP_ID: &str = "com.example.Knot";
const SEARCH_FOCUS_ACTION: &str = "win.focus-search";
const SEARCH_FOCUS_ACCELS: &[&str] = &["<Control>k"];
const SAVE_NOTE_ACTION: &str = "win.save-note";
const SAVE_NOTE_ACCELS: &[&str] = &["<Control>s"];

fn load_css() {
    let css = include_str!("../data/style.css");
    let provider = gtk::CssProvider::new();
    provider.load_from_string(css);

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args = CliArgs::parse();

    // Store socket path globally
    SOCKET_PATH
        .set(args.socket_path.clone())
        .map_err(|_| anyhow::anyhow!("Failed to set socket path"))?;

    BACKGROUND_RUNTIME
        .set(
            tokio::runtime::Builder::new_multi_thread()
                .thread_name("knot-gtk-bg")
                .enable_all()
                .build()?,
        )
        .map_err(|_| anyhow::anyhow!("Failed to initialize background runtime"))?;

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,knot_gtk=debug")
        .init();

    tracing::info!("Using socket path: {}", args.socket_path.display());

    // Initialize GTK/libadwaita
    gtk::init()?;
    let _ = libadwaita::init();

    // Load CSS
    load_css();

    // Create application
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .build();

    let socket_path = args.socket_path.clone();

    // Set up actions
    app.connect_startup(|app| {
        setup_actions(app);
        setup_shortcuts(app);
    });

    // Activate handler
    app.connect_activate(move |app| {
        let client = KnotdClient::with_socket_path(&socket_path);
        let window = KnotWindow::with_client(app, client);
        window.present();
    });

    // Run application
    let exit_code = app.run();

    if exit_code.value() != 0 {
        anyhow::bail!("Application exited with code {}", exit_code.value());
    }

    Ok(())
}

fn setup_actions(app: &libadwaita::Application) {
    // New Note action
    let new_action = gio::SimpleAction::new("new-note", None);
    new_action.connect_activate(|_action, _param| {
        tracing::info!("New note action triggered");
        // TODO: Trigger new note in active window
    });
    app.add_action(&new_action);

    // Close tab/window action
    let close_action = gio::SimpleAction::new("close", None);
    close_action.connect_activate(|_action, _param| {
        tracing::info!("Close action triggered");
    });
    app.add_action(&close_action);

    // Toggle sidebar action
    let sidebar_action = gio::SimpleAction::new("toggle-sidebar", None);
    sidebar_action.connect_activate(|_action, _param| {
        tracing::info!("Toggle sidebar action triggered");
    });
    app.add_action(&sidebar_action);

    // Preferences action
    let prefs_action = gio::SimpleAction::new("preferences", None);
    prefs_action.connect_activate(|_action, _param| {
        tracing::info!("Preferences action triggered");
        show_preferences();
    });
    app.add_action(&prefs_action);

    // About action
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_action, _param| {
        // Get the active application to show the dialog
        if let Some(app) = gtk::gio::Application::default() {
            if let Some(app) = app.downcast_ref::<libadwaita::Application>() {
                show_about_dialog(app);
            }
        }
    });
    app.add_action(&about_action);

    // Quit action
    let quit_action = gio::SimpleAction::new("quit", None);
    quit_action.connect_activate(move |_action, _param| {
        if let Some(app) = gtk::gio::Application::default() {
            app.quit();
        }
    });
    app.add_action(&quit_action);
}

fn setup_shortcuts(app: &libadwaita::Application) {
    app.set_accels_for_action("app.new-note", &["<Control>n"]);
    app.set_accels_for_action("app.close", &["<Control>w"]);
    app.set_accels_for_action("app.toggle-sidebar", &["F9"]);
    app.set_accels_for_action(SEARCH_FOCUS_ACTION, SEARCH_FOCUS_ACCELS);
    app.set_accels_for_action(SAVE_NOTE_ACTION, SAVE_NOTE_ACCELS);
    app.set_accels_for_action("app.quit", &["<Control>q"]);
}

fn show_preferences() {
    // TODO: Show preferences dialog
}

fn show_about_dialog(app: &libadwaita::Application) {
    let window = app.active_window();

    let dialog = libadwaita::AboutDialog::builder()
        .application_name("Knot")
        .application_icon("text-editor-symbolic")
        .developer_name("Knot Contributors")
        .version("0.1.0")
        .comments("GTK4 frontend for Knot knowledge base")
        .website("https://github.com/yourusername/knot")
        .issue_url("https://github.com/yourusername/knot/issues")
        .license_type(gtk::License::MitX11)
        .copyright("© 2026 Knot Contributors")
        .build();

    if let Some(win) = window {
        dialog.present(Some(&win));
    } else {
        dialog.present(None::<&gtk::Window>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_focus_shortcut_is_bound_to_window_action() {
        assert_eq!(SEARCH_FOCUS_ACTION, "win.focus-search");
        assert_eq!(SEARCH_FOCUS_ACCELS, &["<Control>k"]);
    }

    #[test]
    fn save_shortcut_is_bound_to_window_action() {
        assert_eq!(SAVE_NOTE_ACTION, "win.save-note");
        assert_eq!(SAVE_NOTE_ACCELS, &["<Control>s"]);
    }
}
