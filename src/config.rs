//! Configuration: paths + `config.toml`.
//!
//! Paths follow the XDG Base Directory Specification:
//!   data dir  = $XDG_DATA_HOME/buckland   (default ~/.local/share/buckland)
//!   config    = $XDG_CONFIG_HOME/buckland/config.toml
//!
//! The file is optional. Missing file means "default config, no shortcut token."

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub shortcut: ShortcutConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub tray: TrayConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShortcutConfig {
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default = "default_icons")]
    pub icons: String,
    #[serde(default = "default_accent")]
    pub accent_color: String,
}

fn default_icons() -> String {
    "unicode".into()
}

fn default_accent() -> String {
    "cyan".into()
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            icons: default_icons(),
            accent_color: default_accent(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrayConfig {
    #[serde(default = "default_poll")]
    pub poll_seconds: u64,
}

fn default_poll() -> u64 {
    30
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            poll_seconds: default_poll(),
        }
    }
}

/// Data file location (the SQLite database).
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("buckland")
}

pub fn db_path() -> PathBuf {
    data_dir().join("buckland.db")
}

/// Config file location (`config.toml`).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("buckland")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Load config from `path`. Missing file returns default.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

/// Save config to `path`. Creates parent directories. Writes with mode 0600
/// on Unix so the token stays private.
pub fn save(path: &Path, cfg: &Config) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(cfg)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(text.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = load(&path).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.shortcut.token = Some("abc123".into());
        cfg.tray.poll_seconds = 45;
        save(&path, &cfg).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn defaults_fill_missing_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[shortcut]\ntoken = \"xyz\"\n").unwrap();
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.shortcut.token.as_deref(), Some("xyz"));
        assert_eq!(cfg.ui.icons, "unicode");
        assert_eq!(cfg.tray.poll_seconds, 30);
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_has_user_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        save(&path, &Config::default()).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600, got {mode:o}");
    }
}
