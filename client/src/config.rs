//! Local, non-secret desktop preferences. Stored in `config.json`, which is
//! git-ignored. Never store tokens, passwords, or server overrides here.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DEFAULT_SERVER_URL: &str = "https://skypeline-co.onrender.com";
const CONFIG_PATH: &str = "config.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_dark_mode")]
    pub dark_mode: bool,
}

fn default_dark_mode() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dark_mode: default_dark_mode(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if !Path::new(CONFIG_PATH).exists() {
            return Self::default();
        }

        // Older config.json files may contain a server_url field; serde ignores
        // it because this release is hosted-only.
        fs::read_to_string(CONFIG_PATH)
            .ok()
            .and_then(|s| serde_json::from_str::<Config>(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(CONFIG_PATH, json);
        }
    }

    /// The hosted relay is fixed. No local/Hugging Face server switching.
    pub fn base_url(&self) -> String {
        DEFAULT_SERVER_URL.trim_end_matches('/').to_string()
    }
}
