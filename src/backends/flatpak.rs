use crate::backends::{command_exists, normalize_id, run_command, run_command_allow_fail, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::Result;

pub struct FlatpakBackend;

impl Backend for FlatpakBackend {
    fn id(&self) -> &str {
        "flatpak"
    }

    fn available(&self) -> bool {
        command_exists("flatpak")
    }

    fn list_installed(&self, apps_only: bool) -> Result<Vec<Package>> {
        let args = if apps_only {
            vec!["list", "--app", "--columns=application,version,origin"]
        } else {
            vec!["list", "--columns=application,version,origin"]
        };
        let output = run_command_allow_fail("flatpak", &args)?;
        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let app_id = parts[0];
                let version = parts[1];
                let origin = parts.get(2).map(|s| s.to_string());
                let name = app_id.rsplit('.').next().unwrap_or(app_id);
                packages.push(Package {
                    canonical_id: normalize_id(name),
                    name: app_id.to_string(),
                    version: version.to_string(),
                    backend: "flatpak".into(),
                    backend_id: app_id.to_string(),
                    origin,
                    path: None,
                    latest_version: None,
                    outdated: false,
                    is_app: apps_only || !app_id.contains(".Runtime"),
                });
            }
        }
        Ok(packages)
    }

    fn search(&self, query: &str) -> Result<Vec<Candidate>> {
        let output = run_command_allow_fail("flatpak", &["search", query])?;
        let mut candidates = Vec::new();

        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let app_id = parts[0];
                let name = app_id.rsplit('.').next().unwrap_or(app_id);
                candidates.push(Candidate {
                    canonical_id: normalize_id(name),
                    name: app_id.to_string(),
                    version: parts.get(1).map(|s| s.to_string()),
                    backend: "flatpak".into(),
                    backend_id: app_id.to_string(),
                    description: parts.get(2).map(|s| s.to_string()),
                    origin: parts.get(3).map(|s| s.to_string()),
                });
            }
        }
        Ok(candidates)
    }

    fn install(&self, id: &str, version: Option<&str>) -> Result<()> {
        match version {
            Some(v) => {
                run_command("flatpak", &["install", "-y", "flathub", &format!("{id}//{v}")])?;
            }
            None => {
                run_command("flatpak", &["install", "-y", "flathub", id])?;
            }
        }
        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        run_command("flatpak", &["update", "-y", id])?;
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        run_command("flatpak", &["update", "-y"])?;
        Ok(())
    }

    fn uninstall(&self, id: &str, _purge: bool) -> Result<()> {
        run_command("flatpak", &["uninstall", "-y", id])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        run_command("flatpak", &["uninstall", "--unused", "-y"])?;
        Ok(CleanupReport {
            removed: vec!["unused flatpak runtimes".into()],
            freed_description: "Unused Flatpak runtimes removed".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let output = run_command_allow_fail("flatpak", &["remote-ls", "--updates", "flathub"])?;
        let mut updates = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                updates.push(UpdateAvailable {
                    backend_id: parts[0].to_string(),
                    backend: "flatpak".into(),
                    current_version: "?".into(),
                    latest_version: parts.get(1).unwrap_or(&"?").to_string(),
                });
            }
        }
        Ok(updates)
    }

    fn pin_add(&self, id: &str, reason: Option<&str>) -> Result<()> {
        let _ = reason;
        run_command("flatpak", &["mask", id])?;
        Ok(())
    }

    fn pin_remove(&self, id: &str) -> Result<()> {
        run_command("flatpak", &["mask", "--remove", id])?;
        Ok(())
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        let output = run_command_allow_fail("flatpak", &["mask", "--list"])?;
        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| (l.trim().to_string(), Some("flatpak mask".into())))
            .collect())
    }
}
