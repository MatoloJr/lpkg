use crate::backends::{command_exists, normalize_id, run_command, run_command_allow_fail, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::Result;

pub struct PipxBackend;

impl Backend for PipxBackend {
    fn id(&self) -> &str {
        "pipx"
    }

    fn available(&self) -> bool {
        command_exists("pipx")
    }

    fn list_installed(&self, _apps_only: bool) -> Result<Vec<Package>> {
        let output = run_command_allow_fail("pipx", &["list", "--json"])?;
        if output.is_empty() {
            return Ok(Vec::new());
        }

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap_or(serde_json::json!({}));
        let mut packages = Vec::new();

        if let Some(venvs) = parsed.get("venvs").and_then(|v| v.as_object()) {
            for (name, info) in venvs {
                let version = info
                    .get("metadata")
                    .and_then(|m| m.get("main_package"))
                    .and_then(|p| p.get("package_version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                packages.push(Package {
                    canonical_id: normalize_id(name),
                    name: name.clone(),
                    version,
                    backend: "pipx".into(),
                    backend_id: name.clone(),
                    origin: Some("pypi".into()),
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
        // pipx doesn't have search; use pip index
        let output = run_command_allow_fail("pip", &["index", "versions", query])?;
        let version = output
            .lines()
            .find(|l| l.contains("Available versions"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().split(',').next().unwrap_or("").trim().to_string());

        Ok(vec![Candidate {
            canonical_id: normalize_id(query),
            name: query.to_string(),
            version,
            backend: "pipx".into(),
            backend_id: query.to_string(),
            description: Some("Python package (install via pipx)".into()),
            origin: Some("pypi".into()),
        }])
    }

    fn install(&self, id: &str, version: Option<&str>) -> Result<()> {
        match version {
            Some(v) => {
                run_command("pipx", &["install", &format!("{id}=={v}")])?;
            }
            None => {
                run_command("pipx", &["install", id])?;
            }
        }
        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        run_command("pipx", &["upgrade", id])?;
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        run_command("pipx", &["upgrade-all"])?;
        Ok(())
    }

    fn uninstall(&self, id: &str, _purge: bool) -> Result<()> {
        run_command("pipx", &["uninstall", id])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        Ok(CleanupReport {
            removed: Vec::new(),
            freed_description: "pipx has no built-in cleanup".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let packages = self.list_installed(true)?;
        let mut updates = Vec::new();
        for pkg in packages {
            let output = run_command_allow_fail("pipx", &["upgrade", &pkg.backend_id, "--dry-run"])?;
            if output.contains("would upgrade") || output.contains("upgraded") {
                updates.push(UpdateAvailable {
                    backend_id: pkg.backend_id,
                    backend: "pipx".into(),
                    current_version: pkg.version,
                    latest_version: "?".into(),
                });
            }
        }
        Ok(updates)
    }

    fn pin_add(&self, _id: &str, _reason: Option<&str>) -> Result<()> {
        Ok(()) // pipx doesn't support pinning
    }

    fn pin_remove(&self, _id: &str) -> Result<()> {
        Ok(())
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        Ok(Vec::new())
    }
}
