use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorSchemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearancePreferences {
    #[serde(default = "default_context_panel_width")]
    pub context_panel_width: i32,
    #[serde(default = "default_inspector_width")]
    pub inspector_width: i32,
    #[serde(default)]
    pub color_scheme: ColorSchemePreference,
}

impl Default for AppearancePreferences {
    fn default() -> Self {
        Self {
            context_panel_width: default_context_panel_width(),
            inspector_width: default_inspector_width(),
            color_scheme: ColorSchemePreference::System,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AutomationConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KnottyConfig {
    #[serde(default)]
    pub appearance: AppearancePreferences,
    #[serde(default)]
    pub automation: AutomationConfig,
}

pub fn default_context_panel_width() -> i32 {
    280
}

pub fn default_inspector_width() -> i32 {
    280
}

pub fn knotty_config_path() -> Result<PathBuf, String> {
    knotty_config_path_with_env(
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

fn config_root_with_env(
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(path) = xdg_config_home {
        return Ok(PathBuf::from(path));
    }
    let home = home.ok_or_else(|| {
        "Cannot resolve knotty config path without XDG_CONFIG_HOME or HOME".to_string()
    })?;
    Ok(PathBuf::from(home).join(".config"))
}

pub fn knotty_config_path_with_env(
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf, String> {
    Ok(config_root_with_env(xdg_config_home, home)?
        .join("knot")
        .join("knotty.toml"))
}

pub fn load_knotty_config() -> Result<KnottyConfig, String> {
    let path = knotty_config_path()?;
    load_knotty_config_from_path(&path)
}

pub fn load_knotty_config_from_path(path: &Path) -> Result<KnottyConfig, String> {
    match fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content)
            .map_err(|error| format!("Failed to parse {}: {}", path.display(), error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(KnottyConfig::default()),
        Err(error) => Err(format!("Failed to read {}: {}", path.display(), error)),
    }
}

pub fn save_knotty_config(config: &KnottyConfig) -> Result<(), String> {
    let path = knotty_config_path()?;
    save_knotty_config_to_path(config, &path)
}

pub fn save_knotty_config_to_path(config: &KnottyConfig, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {}", parent.display(), error))?;
    }
    let content = toml::to_string_pretty(config)
        .map_err(|error| format!("Failed to encode config: {}", error))?;
    fs::write(path, content)
        .map_err(|error| format!("Failed to write {}: {}", path.display(), error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knotty_config_path_prefers_xdg_config_home() {
        let path = knotty_config_path_with_env(Some("/tmp/config"), Some("/tmp/home"))
            .expect("path should resolve");
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/config/knot/knotty.toml")
        );
    }

    #[test]
    fn knotty_config_path_falls_back_to_home_config_dir() {
        let path =
            knotty_config_path_with_env(None, Some("/tmp/home")).expect("path should resolve");
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/home/.config/knot/knotty.toml")
        );
    }

    #[test]
    fn knotty_config_round_trips_automation_opt_in() {
        let path = std::env::temp_dir().join(format!(
            "knotty-config-automation-{}.toml",
            std::process::id()
        ));
        let config = KnottyConfig {
            appearance: AppearancePreferences::default(),
            automation: AutomationConfig { enabled: true },
        };

        save_knotty_config_to_path(&config, &path).expect("config should save");
        let loaded = load_knotty_config_from_path(&path).expect("config should load");

        assert!(loaded.automation.enabled);
        let _ = std::fs::remove_file(path);
    }
}
