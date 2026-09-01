use crate::backends::{command_exists, normalize_id, run_command, run_command_allow_fail, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::Result;

pub struct BrewBackend;

impl Backend for BrewBackend {
    fn id(&self) -> &str {
        "brew"
    }

    fn available(&self) -> bool {
        command_exists("brew")
    }

    fn list_installed(&self, _apps_only: bool) -> Result<Vec<Package>> {
        let output = run_command_allow_fail("brew", &["list", "--versions"])?;
        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0];
                let version = parts.get(1).unwrap_or(&"?").to_string();
                packages.push(Package {
                    canonical_id: normalize_id(name),
                    name: name.to_string(),
                    version,
                    backend: "brew".into(),
                    backend_id: name.to_string(),
                    origin: Some("homebrew".into()),
                    path: None,
                    latest_version: None,
                    outdated: false,
                    is_app: true,
                });
            }
        }
        Ok(packages)
    }

    fn search(&self, query: &str) -> Result<Vec<Candidate>> {
        let output = run_command_allow_fail("brew", &["search", query])?;
        let mut candidates = Vec::new();

        for line in output.lines() {
            let name = line.trim();
            if !name.is_empty() && !name.starts_with("==>") {
                candidates.push(Candidate {
                    canonical_id: normalize_id(name),
                    name: name.to_string(),
                    version: None,
                    backend: "brew".into(),
                    backend_id: name.to_string(),
                    description: None,
                    origin: Some("homebrew".into()),
                });
            }
        }
        Ok(candidates)
    }

    fn install(&self, id: &str, version: Option<&str>) -> Result<()> {
        match version {
            Some(v) => {
                run_command("brew", &["install", &format!("{id}@{v}")])?;
            }
            None => {
                run_command("brew", &["install", id])?;
            }
        }
        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        run_command("brew", &["upgrade", id])?;
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        run_command("brew", &["update"])?;
        run_command("brew", &["upgrade"])?;
        Ok(())
    }

    fn uninstall(&self, id: &str, _purge: bool) -> Result<()> {
        run_command("brew", &["uninstall", id])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        run_command("brew", &["cleanup", "-s"])?;
        Ok(CleanupReport {
            removed: vec!["old brew cellar versions".into()],
            freed_description: "Homebrew cleanup completed".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let output = run_command_allow_fail("brew", &["outdated"])?;
        let mut updates = Vec::new();

        for line in output.lines() {
            let name = line.trim();
            if !name.is_empty() {
                updates.push(UpdateAvailable {
                    backend_id: name.to_string(),
                    backend: "brew".into(),
                    current_version: "?".into(),
                    latest_version: "?".into(),
                });
            }
        }
        Ok(updates)
    }

    fn pin_add(&self, id: &str, reason: Option<&str>) -> Result<()> {
        let _ = reason;
        run_command("brew", &["pin", id])?;
        Ok(())
    }

    fn pin_remove(&self, id: &str) -> Result<()> {
        run_command("brew", &["unpin", id])?;
        Ok(())
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        let output = run_command_allow_fail("brew", &["pin"])?;
        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| (l.trim().to_string(), Some("brew pin".into())))
            .collect())
    }
}
