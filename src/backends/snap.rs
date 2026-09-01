use crate::backends::{command_exists, normalize_id, run_command_allow_fail, run_command_sudo, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::Result;

pub struct SnapBackend;

impl Backend for SnapBackend {
    fn id(&self) -> &str {
        "snap"
    }

    fn available(&self) -> bool {
        command_exists("snap")
    }

    fn list_installed(&self, apps_only: bool) -> Result<Vec<Package>> {
        let _ = apps_only;
        let output = run_command_allow_fail("snap", &["list"])?;
        let mut packages = Vec::new();

        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let version = parts[1];
                if name == "Name" {
                    continue;
                }
                packages.push(Package {
                    canonical_id: normalize_id(name),
                    name: name.to_string(),
                    version: version.to_string(),
                    backend: "snap".into(),
                    backend_id: name.to_string(),
                    origin: parts.get(2).map(|s| s.to_string()),
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
        let output = run_command_allow_fail("snap", &["find", query])?;
        let mut candidates = Vec::new();

        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[0];
                let version = parts[1];
                let publisher = parts[2];
                candidates.push(Candidate {
                    canonical_id: normalize_id(name),
                    name: name.to_string(),
                    version: Some(version.to_string()),
                    backend: "snap".into(),
                    backend_id: name.to_string(),
                    description: None,
                    origin: Some(publisher.to_string()),
                });
            }
        }
        Ok(candidates)
    }

    fn install(&self, id: &str, version: Option<&str>) -> Result<()> {
        match version {
            Some(v) => {
                run_command_sudo("snap", &["install", id, "--channel", v])?;
            }
            None => {
                run_command_sudo("snap", &["install", id])?;
            }
        }
        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        run_command_sudo("snap", &["refresh", id])?;
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        run_command_sudo("snap", &["refresh"])?;
        Ok(())
    }

    fn uninstall(&self, id: &str, _purge: bool) -> Result<()> {
        run_command_sudo("snap", &["remove", id])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        let output = run_command_allow_fail("snap", &["list", "--all"])?;
        let mut removed = Vec::new();

        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                let name = parts[0];
                let rev = parts[2];
                let notes = parts[6];
                if notes == "disabled" {
                    let _ = run_command_sudo("snap", &["remove", name, "--revision", rev]);
                    removed.push(format!("{name} rev {rev}"));
                }
            }
        }

        Ok(CleanupReport {
            removed,
            freed_description: "Old snap revisions removed".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let output = run_command_allow_fail("snap", &["refresh", "--list"])?;
        let mut updates = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                updates.push(UpdateAvailable {
                    backend_id: parts[0].to_string(),
                    backend: "snap".into(),
                    current_version: parts.get(1).unwrap_or(&"?").to_string(),
                    latest_version: parts.get(2).unwrap_or(&"?").to_string(),
                });
            }
        }
        Ok(updates)
    }

    fn pin_add(&self, id: &str, reason: Option<&str>) -> Result<()> {
        let _ = reason;
        // Hold current revision
        run_command_sudo("snap", &["refresh", "--hold", id])?;
        Ok(())
    }

    fn pin_remove(&self, id: &str) -> Result<()> {
        run_command_sudo("snap", &["refresh", "--unhold", id])?;
        Ok(())
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        // snap doesn't have a simple list-holds; return empty for now
        let _ = run_command_allow_fail("snap", &["refresh", "--list"]);
        Ok(Vec::new())
    }
}
