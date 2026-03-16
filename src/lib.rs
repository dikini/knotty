pub mod cli;
pub mod client;
pub mod config;
pub mod runtime_contract;
pub mod ui;

use std::path::PathBuf;
use std::sync::OnceLock;

pub static SOCKET_PATH: OnceLock<PathBuf> = OnceLock::new();
pub static BACKGROUND_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
pub static AUTOMATION_RUNTIME_ENABLED: OnceLock<bool> = OnceLock::new();
pub static AUTOMATION_RUNTIME_TOKEN: OnceLock<Option<String>> = OnceLock::new();
