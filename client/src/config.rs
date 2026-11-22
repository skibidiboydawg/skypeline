//! Local, non-secret desktop preferences. Stored in `config.json`, which is
//! git-ignored. Never store tokens or passwords here.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DEFAULT_SERVER_URL: &str = "https://rezixyzz-co.hf.space";
pub const LOCAL_SERVER_URL: &str = "http://127.0.0.1:7860";
const CONFIG_PATH: &str = "config.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default = "default_dark_mode")]
    pub dark_mode: bool,
}

fn default_server_url() -> String {
    DEFAULT_SERVER_URL.to_string()
}

fn default_dark_mode() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            dark_mode: default_dark_mode(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if !Path::new(CONFIG_PATH).exists() {
            return Self::default();
        }
        match fs::read_to_string(CONFIG_PATH)
            .ok()
            .and_then(|s| serde_json::from_str::<Config>(&s).ok())
        {
            Some(mut cfg) => {
                if cfg.server_url.trim().is_empty() {
                    cfg.server_url = default_server_url();
                }
                cfg
            }
            None => Self::default(),
        }
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(CONFIG_PATH, json);
        }
    }

    /// Trim trailing slashes so URL joining is predictable.
    pub fn base_url(&self) -> String {
        self.server_url.trim().trim_end_matches('/').to_string()
    }
}
