//! Local desktop preferences and session state.
//!
//! Stored in the user's OS config directory (`%APPDATA%\Wireline\config.json`
//! on Windows, `~/.config/wireline/config.json` on Linux) so shortcuts,
//! installers, and upgrades always preserve your login session.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_SERVER_URL: &str = "https://skypeline-co.onrender.com";
const LEGACY_CONFIG_PATH: &str = "config.json";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SavedSession {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub user_id: i64,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub avatar_hash: Option<String>,
}

impl SavedSession {
    pub fn is_valid(&self) -> bool {
        !self.token.trim().is_empty() && self.user_id > 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_dark_mode")]
    pub dark_mode: bool,
    #[serde(default)]
    pub session: Option<SavedSession>,
}

fn default_dark_mode() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dark_mode: default_dark_mode(),
            session: None,
        }
    }
}

pub fn config_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("Wireline").join("config.json");
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("wireline").join("config.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("wireline")
                .join("config.json");
        }
    }
    PathBuf::from(LEGACY_CONFIG_PATH)
}

impl Config {
    pub fn load() -> Self {
        let path = config_file_path();
        if path.exists() {
            if let Some(cfg) = fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Config>(&s).ok())
            {
                return cfg;
            }
        }
        // Migrate legacy relative config.json if present.
        if Path::new(LEGACY_CONFIG_PATH).exists() {
            if let Some(cfg) = fs::read_to_string(LEGACY_CONFIG_PATH)
                .ok()
                .and_then(|s| serde_json::from_str::<Config>(&s).ok())
            {
                cfg.save();
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = config_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    /// The hosted relay is fixed.
    pub fn base_url(&self) -> String {
        DEFAULT_SERVER_URL.trim_end_matches('/').to_string()
    }
}
