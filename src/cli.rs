//! Command line argument parsing for knotty

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub socket_path: PathBuf,
    pub automation_enabled: bool,
    pub automation_token: Option<String>,
}

impl CliArgs {
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        Self::parse_from(args)
    }

    fn parse_from<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let args: Vec<String> = args.into_iter().map(Into::into).collect();
        let mut socket_path = None;
        let mut automation_enabled = false;
        let mut automation_token = None;

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
                "--automation-token" => {
                    if i + 1 < args.len() {
                        automation_token = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("Error: --automation-token requires a token argument");
                        std::process::exit(1);
                    }
                }
                "--enable-automation" => {
                    automation_enabled = true;
                    i += 1;
                }
                "--help" | "-h" => {
                    Self::print_help();
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!("knotty {}", env!("CARGO_PKG_VERSION"));
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
            .or_else(Self::default_socket_path)
            .unwrap_or_else(|| {
                eprintln!(
                    "Error: {}",
                    crate::runtime_contract::missing_socket_path_message()
                );
                std::process::exit(1);
            });

        Self {
            socket_path,
            automation_enabled,
            automation_token,
        }
    }

    fn default_socket_path() -> Option<PathBuf> {
        crate::runtime_contract::default_socket_path()
    }

    fn print_help() {
        println!("knotty - GTK4 frontend for Knot knowledge base");
        println!();
        println!("Usage: knotty [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -s, --socket <PATH>    Path to knotd Unix socket");
        println!("      --enable-automation");
        println!("                         Allow GTK automation for this process");
        println!("      --automation-token <TOKEN>");
        println!("                         Runtime token for gated GTK automation");
        println!(
            "                         [default: {}]",
            crate::runtime_contract::default_socket_help()
        );
        println!("  -h, --help             Print this help message");
        println!("  -V, --version          Print version information");
        println!();
        println!("Environment Variables:");
        println!("  KNOTD_SOCKET_PATH      Override default socket path");
        println!(
            "  XDG_RUNTIME_DIR        Used to construct the default socket path ({})",
            crate::runtime_contract::default_socket_help()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_uses_knotty_branding() {
        assert_eq!(env!("CARGO_PKG_NAME"), "knotty");
    }

    #[test]
    fn test_default_socket_path() {
        // Save original env var
        let original = std::env::var("XDG_RUNTIME_DIR").ok();

        // Test with XDG_RUNTIME_DIR set
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime");
        let path = CliArgs::default_socket_path().expect("XDG runtime path should resolve");
        assert_eq!(path, PathBuf::from("/tmp/test-runtime/knot/knotd.sock"));

        // Test without XDG_RUNTIME_DIR
        std::env::remove_var("XDG_RUNTIME_DIR");
        let path = CliArgs::default_socket_path();
        assert_eq!(path, None);

        // Restore original
        match original {
            Some(val) => std::env::set_var("XDG_RUNTIME_DIR", val),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }

    #[test]
    fn parse_from_reads_automation_token() {
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime");

        let args = CliArgs::parse_from([
            "knotty",
            "--socket",
            "/tmp/test-runtime/knot/knotd.sock",
            "--enable-automation",
            "--automation-token",
            "dev-token",
        ]);

        assert_eq!(
            args.socket_path,
            PathBuf::from("/tmp/test-runtime/knot/knotd.sock")
        );
        assert!(args.automation_enabled);
        assert_eq!(args.automation_token.as_deref(), Some("dev-token"));

        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}
