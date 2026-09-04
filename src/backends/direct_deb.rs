use crate::backends::{command_exists, normalize_id, run_command_sudo, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct DebPackageEntry {
    name: String,
    #[serde(default)]
    package: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    description: String,
}

pub struct DirectDebBackend {
    index: Vec<DebPackageEntry>,
}

impl DirectDebBackend {
    pub fn new() -> Self {
        let index_path = crate::config::direct_deb_index_path();
        let index = if index_path.exists() {
            let content = fs::read_to_string(&index_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            default_index()
        };
        Self { index }
    }
}

fn default_index() -> Vec<DebPackageEntry> {
    vec![
        DebPackageEntry {
            name: "cursor".into(),
            package: "cursor".into(),
            url: "https://api2.cursor.sh/updates/download/golden/linux-x64-deb/cursor/latest".into(),
            description: "Cursor AI IDE".into(),
        },
        DebPackageEntry {
            name: "google-chrome".into(),
            package: "google-chrome-stable".into(),
            url: "https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb".into(),
            description: "Google Chrome browser".into(),
        },
    ]
}

impl Default for DirectDebBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for DirectDebBackend {
    fn id(&self) -> &str {
        "direct_deb"
    }

    fn available(&self) -> bool {
        command_exists("dpkg") && command_exists("wget")
    }

    fn list_installed(&self, _apps_only: bool) -> Result<Vec<Package>> {
        // Direct deb packages are tracked via registry; dpkg won't distinguish them
        Ok(Vec::new())
    }

    fn search(&self, query: &str) -> Result<Vec<Candidate>> {
        let query_lower = query.to_lowercase();
        let candidates = self
            .index
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&query_lower)
                    || e.package.to_lowercase().contains(&query_lower)
                    || e.description.to_lowercase().contains(&query_lower)
            })
            .map(|e| Candidate {
                canonical_id: normalize_id(&e.name),
                name: e.name.clone(),
                version: None,
                backend: "direct_deb".into(),
                backend_id: e.name.clone(),
                description: Some(e.description.clone()),
                origin: Some("vendor".into()),
            })
            .collect();
        Ok(candidates)
    }

    fn install(&self, id: &str, _version: Option<&str>) -> Result<()> {
        let entry = self
            .index
            .iter()
            .find(|e| e.name == id || e.package == id)
            .with_context(|| format!("package {id} not found in direct-deb index"))?;

        let tmp_dir = std::env::temp_dir();
        let deb_path = tmp_dir.join(format!("{}.deb", entry.package));

        run_command_sudo(
            "wget",
            &["-q", "-O", deb_path.to_str().unwrap(), &entry.url],
        )?;
        run_command_sudo("dpkg", &["-i", deb_path.to_str().unwrap()])?;
        // Fix dependencies if needed
        let _ = run_command_sudo("apt-get", &["install", "-f", "-y"]);
        let _ = fs::remove_file(&deb_path);

        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        self.install(id, None)
    }

    fn upgrade_all(&self) -> Result<()> {
        Ok(())
    }

    fn uninstall(&self, id: &str, purge: bool) -> Result<()> {
        let entry = self
            .index
            .iter()
            .find(|e| e.name == id || e.package == id)
            .with_context(|| format!("package {id} not found in direct-deb index"))?;

        if purge {
            run_command_sudo("apt-get", &["purge", "-y", &entry.package])?;
        } else {
            run_command_sudo("apt-get", &["remove", "-y", &entry.package])?;
        }
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        Ok(CleanupReport {
            removed: Vec::new(),
            freed_description: "No direct-deb cleanup".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        Ok(Vec::new())
    }

    fn pin_add(&self, _id: &str, _reason: Option<&str>) -> Result<()> {
        bail!("direct_deb pinning not supported; use apt pin for installed debs")
    }

    fn pin_remove(&self, _id: &str) -> Result<()> {
        bail!("direct_deb pinning not supported")
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        Ok(Vec::new())
    }
}
