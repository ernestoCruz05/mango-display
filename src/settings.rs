
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub monitors_conf_path: String,
    pub config_conf_path: String,
    #[serde(default)]
    pub auto_append_source: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            monitors_conf_path: "~/.config/mango/monitors.conf".to_string(),
            config_conf_path: "~/.config/mango/config.conf".to_string(),
            auto_append_source: true,
        }
    }
}

impl AppSettings {
    fn settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("mango-display")
            .join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::settings_path();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&contents) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings dir: {}", e))?;
        }
        
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
            
        fs::write(&path, json).map_err(|e| format!("Failed to write settings.json: {}", e))?;
        Ok(())
    }
}
