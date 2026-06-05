use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStore {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub connections: Vec<ConnectionConfig>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[default]
    English,
    Spanish,
    Catalan,
}

impl Language {
    pub fn choices() -> &'static [(Language, &'static str)] {
        &[
            (Language::English, "English"),
            (Language::Spanish, "Español"),
            (Language::Catalan, "Català"),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AuthConfig {
    InteractivePassword,
    StoredPassword { password: String },
    KeyFile { path: PathBuf },
}

impl ConnectionStore {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse config at {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("VPS_SWITCHER_CONFIG") {
        return Ok(PathBuf::from(path));
    }

    let home = env::var("HOME").map_err(|_| anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("proto-vps-switcher")
        .join("connections.toml"))
}
