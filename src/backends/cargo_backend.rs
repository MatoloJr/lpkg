use crate::backends::{command_exists, normalize_id, run_command, run_command_allow_fail, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::Result;

pub struct CargoBackend;

impl Backend for CargoBackend {
    fn id(&self) -> &str {
        "cargo"
    }

    fn available(&self) -> bool {
        command_exists("cargo")
    }

    fn list_installed(&self, _apps_only: bool) -> Result<Vec<Package>> {
        let output = run_command_allow_fail("cargo", &["install", "--list"])?;
        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let version = parts[1].trim_start_matches('v').to_string();
                packages.push(Package {
                    canonical_id: normalize_id(name),
                    name: name.to_string(),
                    version,
                    backend: "cargo".into(),
                    backend_id: name.to_string(),
                    origin: Some("crates.io".into()),
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
        let output = run_command_allow_fail("cargo", &["search", query, "--limit", "10"])?;
        let mut candidates = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split(" = ").collect();
            if parts.len() >= 2 {
                let name = parts[0].trim();
                let version = parts[1].trim().trim_matches('"').to_string();
                candidates.push(Candidate {
                    canonical_id: normalize_id(name),
                    name: name.to_string(),
                    version: Some(version),
                    backend: "cargo".into(),
                    backend_id: name.to_string(),
                    description: None,
                    origin: Some("crates.io".into()),
                });
            }
        }
        Ok(candidates)
    }

    fn install(&self, id: &str, version: Option<&str>) -> Result<()> {
        match version {
            Some(v) => {
                run_command("cargo", &["install", id, "--version", v])?;
            }
            None => {
                run_command("cargo", &["install", id])?;
            }
        }
        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        run_command("cargo", &["install", id, "--force"])?;
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        let packages = self.list_installed(true)?;
        for pkg in packages {
            let _ = run_command("cargo", &["install", &pkg.backend_id, "--force"]);
        }
        Ok(())
    }

    fn uninstall(&self, id: &str, _purge: bool) -> Result<()> {
        run_command("cargo", &["uninstall", id])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        Ok(CleanupReport {
            removed: Vec::new(),
            freed_description: "cargo has no built-in cleanup; use cargo uninstall".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let packages = self.list_installed(true)?;
        let mut updates = Vec::new();

        for pkg in packages {
            let search = run_command_allow_fail("cargo", &["search", &pkg.backend_id, "--limit", "1"])?;
            if let Some(line) = search.lines().next() {
                if let Some((_, latest)) = line.split_once(" = ") {
                    let latest = latest.trim().trim_matches('"');
                    if latest != pkg.version {
                        updates.push(UpdateAvailable {
                            backend_id: pkg.backend_id.clone(),
                            backend: "cargo".into(),
                            current_version: pkg.version,
                            latest_version: latest.to_string(),
                        });
                    }
                }
            }
        }
        Ok(updates)
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
