//! Command line argument parsing for knot-gtk

use std::path::PathBuf;

const DEFAULT_SOCKET_DIR: &str = "/run/user/1000/knot";
const DEFAULT_SOCKET_NAME: &str = "knot.sock";

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub socket_path: PathBuf,
    pub vault_path: Option<PathBuf>,
}

impl CliArgs {
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut socket_path = None;
        let mut vault_path = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--socket" | "-s" => {
                    if i + 1 < args.len() {
                        socket_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        eprintln!("Error: --socket requires a path argument");
                        std::process::exit(1);
                    }
                }
                "--vault" | "-v" => {
                    if i + 1 < args.len() {
                        vault_path = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        eprintln!("Error: --vault requires a path argument");
                        std::process::exit(1);
                    }
                }
                "--help" | "-h" => {
                    Self::print_help();
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!("knot-gtk {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("Unknown argument: {}", args[i]);
                    Self::print_help();
                    std::process::exit(1);
                }
            }
        }

        // Determine socket path
        let socket_path = socket_path
            .or_else(|| std::env::var("KNOTD_SOCKET_PATH").ok().map(PathBuf::from))
            .unwrap_or_else(Self::default_socket_path);

        Self {
            socket_path,
            vault_path,
        }
    }

    fn default_socket_path() -> PathBuf {
        // Try /run/user/UID/knot first (XDG_RUNTIME_DIR)
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir)
                .join("knot")
                .join(DEFAULT_SOCKET_NAME);
            return path;
        }

        // Fallback to /run/user/1000/knot (common default)
        PathBuf::from(DEFAULT_SOCKET_DIR).join(DEFAULT_SOCKET_NAME)
    }

    fn print_help() {
        println!("knot-gtk - GTK4 frontend for Knot knowledge base");
        println!();
        println!("Usage: knot-gtk [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -s, --socket <PATH>    Path to knotd Unix socket");
        println!("                         [default: $XDG_RUNTIME_DIR/knot/knot.sock or /run/user/1000/knot/knot.sock]");
        println!("  -v, --vault <PATH>     Path to vault (for auto-starting knotd)");
        println!("  -h, --help             Print this help message");
        println!("  -V, --version          Print version information");
        println!();
        println!("Environment Variables:");
        println!("  KNOTD_SOCKET_PATH      Override default socket path");
        println!("  XDG_RUNTIME_DIR        Used to construct default socket path");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_socket_path() {
        // Save original env var
        let original = std::env::var("XDG_RUNTIME_DIR").ok();

        // Test with XDG_RUNTIME_DIR set
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime");
        let path = CliArgs::default_socket_path();
        assert_eq!(path, PathBuf::from("/tmp/test-runtime/knot/knot.sock"));

        // Test without XDG_RUNTIME_DIR
        std::env::remove_var("XDG_RUNTIME_DIR");
        let path = CliArgs::default_socket_path();
        assert_eq!(path, PathBuf::from("/run/user/1000/knot/knot.sock"));

        // Restore original
        match original {
            Some(val) => std::env::set_var("XDG_RUNTIME_DIR", val),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }
}
