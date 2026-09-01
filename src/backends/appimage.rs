use crate::backends::{normalize_id, Backend};
use crate::config::Config;
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct AppImageBackend {
    paths: Vec<String>,
}

impl AppImageBackend {
    pub fn new(config: &Config) -> Self {
        Self {
            paths: config.appimage_paths.clone(),
        }
    }
}

impl Backend for AppImageBackend {
    fn id(&self) -> &str {
        "appimage"
    }

    fn available(&self) -> bool {
        true
    }

    fn list_installed(&self, _apps_only: bool) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        for base in &self.paths {
            let path = Path::new(base);
            if !path.exists() {
                continue;
            }

            let walker = if path.is_dir() {
                WalkDir::new(path).max_depth(2).into_iter()
            } else {
                continue;
            };

            for entry in walker.filter_map(|e| e.ok()) {
                let file_path = entry.path();
                if file_path.extension().and_then(|e| e.to_str()) == Some("AppImage")
                    || file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.ends_with(".appimage"))
                        .unwrap_or(false)
                {
                    let name = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let version = fs::metadata(file_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "?".into());

                    packages.push(Package {
                        canonical_id: normalize_id(&name),
                        name: name.clone(),
                        version,
                        backend: "appimage".into(),
                        backend_id: file_path.to_string_lossy().to_string(),
                        origin: Some("local".into()),
                        path: Some(file_path.to_string_lossy().to_string()),
                        latest_version: None,
                        outdated: false,
                        is_app: true,
                    });
                }
            }
        }
        Ok(packages)
    }

    fn search(&self, _query: &str) -> Result<Vec<Candidate>> {
        Ok(Vec::new()) // AppImages must be downloaded manually or via direct_deb
    }

    fn install(&self, _id: &str, _version: Option<&str>) -> Result<()> {
        bail!("AppImage install requires a file path; use direct_deb or download manually")
    }

    fn upgrade(&self, _id: &str) -> Result<()> {
        bail!("AppImage upgrade requires manual download or appimageupdatetool")
    }

    fn upgrade_all(&self) -> Result<()> {
        Ok(()) // No automatic upgrade for AppImages
    }

    fn uninstall(&self, id: &str, purge: bool) -> Result<()> {
        let path = Path::new(id);
        if path.exists() {
            fs::remove_file(path)?;
            if purge {
                let desktop = format!(
                    "{}/.local/share/applications/{}.desktop",
                    std::env::var("HOME").unwrap_or_default(),
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("app")
                );
                let _ = fs::remove_file(desktop);
            }
        }
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        Ok(CleanupReport {
            removed: Vec::new(),
            freed_description: "No AppImage cleanup available".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        Ok(Vec::new())
    }

    fn pin_add(&self, _id: &str, _reason: Option<&str>) -> Result<()> {
        Ok(())
    }

    fn pin_remove(&self, _id: &str) -> Result<()> {
        Ok(())
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        Ok(Vec::new())
    }
}
