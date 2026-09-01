use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_backend_priority")]
    pub backend_priority: Vec<String>,
    #[serde(default = "default_true")]
    pub apps_only: bool,
    #[serde(default)]
    pub backends: BackendConfig,
    #[serde(default)]
    pub appimage_paths: Vec<String>,
    #[serde(default = "default_cache_ttl_hours")]
    pub cache_ttl_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    #[serde(default = "default_true")]
    pub apt: bool,
    #[serde(default = "default_true")]
    pub snap: bool,
    #[serde(default = "default_true")]
    pub flatpak: bool,
    #[serde(default = "default_true")]
    pub brew: bool,
    #[serde(default = "default_true")]
    pub pipx: bool,
    #[serde(default = "default_true")]
    pub cargo: bool,
    #[serde(default = "default_true")]
    pub appimage: bool,
    #[serde(default = "default_true")]
    pub direct_deb: bool,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            apt: true,
            snap: true,
            flatpak: true,
            brew: true,
            pipx: true,
            cargo: true,
            appimage: true,
            direct_deb: true,
        }
    }
}

fn default_backend_priority() -> Vec<String> {
    vec![
        "flatpak".into(),
        "apt".into(),
        "snap".into(),
        "brew".into(),
        "pipx".into(),
        "cargo".into(),
        "direct_deb".into(),
        "appimage".into(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_cache_ttl_hours() -> u64 {
    24
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend_priority: default_backend_priority(),
            apps_only: true,
            backends: BackendConfig::default(),
            appimage_paths: default_appimage_paths(),
            cache_ttl_hours: default_cache_ttl_hours(),
        }
    }
}

fn default_appimage_paths() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    vec![
        format!("{home}/Applications"),
        format!("{home}/bin"),
        format!("{home}/Desktop"),
        format!("{home}/.local/bin"),
    ]
}

pub fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "lpkg").context("could not determine project directories")
}

pub fn config_path() -> Result<PathBuf> {
    let dirs = project_dirs()?;
    Ok(dirs.config_dir().join("config.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    let dirs = project_dirs()?;
    let path = dirs.data_dir().to_path_buf();
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn registry_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("registry.db"))
}

pub fn aliases_path() -> PathBuf {
    // Prefer user override, fall back to bundled data
    if let Ok(dirs) = project_dirs() {
        let user_aliases = dirs.config_dir().join("aliases.toml");
        if user_aliases.exists() {
            return user_aliases;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/aliases.toml")
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("failed to parse config at {}", path.display()))?;
        // Recover from corrupted initial configs where all backends were disabled
        if !config.backends.apt
            && !config.backends.snap
            && !config.backends.flatpak
            && !config.backends.brew
            && !config.backends.pipx
            && !config.backends.cargo
            && !config.backends.appimage
            && !config.backends.direct_deb
        {
            config.backends = BackendConfig::default();
            save_config(&config)?;
        }
        Ok(config)
    } else {
        let config = Config::default();
        save_config(&config)?;
        Ok(config)
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content)?;
    Ok(())
}

pub fn is_backend_enabled(config: &Config, backend: &str) -> bool {
    match backend {
        "apt" => config.backends.apt,
        "snap" => config.backends.snap,
        "flatpak" => config.backends.flatpak,
        "brew" => config.backends.brew,
        "pipx" => config.backends.pipx,
        "cargo" => config.backends.cargo,
        "appimage" => config.backends.appimage,
        "direct_deb" => config.backends.direct_deb,
        _ => false,
    }
}

pub fn enabled_backends(config: &Config) -> Vec<String> {
    config
        .backend_priority
        .iter()
        .filter(|b| is_backend_enabled(config, b))
        .cloned()
        .collect()
}

pub fn ensure_dirs() -> Result<()> {
    let _ = data_dir()?;
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
