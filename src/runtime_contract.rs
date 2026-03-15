//! Shared runtime contract values for connecting to `knotd`.

use std::path::PathBuf;

pub const DEFAULT_RUNTIME_SUBDIR: &str = "knot";
pub const DEFAULT_SOCKET_NAME: &str = "knotd.sock";

pub fn default_socket_path() -> Option<PathBuf> {
    runtime_base_dir().map(|runtime_dir| {
        runtime_dir
            .join(DEFAULT_RUNTIME_SUBDIR)
            .join(DEFAULT_SOCKET_NAME)
    })
}

pub const fn default_socket_help() -> &'static str {
    "$XDG_RUNTIME_DIR/knot/knotd.sock"
}

pub fn missing_socket_path_message() -> String {
    format!(
        "No socket path configured. Pass --socket, set KNOTD_SOCKET_PATH, or set XDG_RUNTIME_DIR so knot-gtk can use {}",
        default_socket_help()
    )
}

fn runtime_base_dir() -> Option<PathBuf> {
    std::env::var("XDG_RUNTIME_DIR").ok().map(PathBuf::from)
}
