use crate::backends::{command_exists, desktop_packages, manual_apt_packages, normalize_id, run_command, run_command_allow_fail, run_command_sudo, Backend};
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::Result;
use std::collections::HashSet;

pub struct AptBackend;

impl Backend for AptBackend {
    fn id(&self) -> &str {
        "apt"
    }

    fn available(&self) -> bool {
        command_exists("apt") && command_exists("dpkg-query")
    }

    fn list_installed(&self, apps_only: bool) -> Result<Vec<Package>> {
        let output = run_command("dpkg-query", &["-W", "-f", "${Package}\t${Version}\t${Status}\n"])?;
        let manual = manual_apt_packages();
        let desktop = desktop_packages();
        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                continue;
            }
            let name = parts[0];
            let version = parts[1];
            let status = parts[2];
            if !status.contains("installed") {
                continue;
            }

            let is_manual = manual.contains(name);
            let has_desktop = desktop.contains(name);
            let is_app = is_manual && (has_desktop || name.starts_with("linux-") == false);

            if apps_only && !is_app {
                // Include common CLI tools installed manually
                if !is_manual {
                    continue;
                }
                // Skip library packages
                if name.starts_with("lib") && !has_desktop {
                    continue;
                }
            }

            packages.push(Package {
                canonical_id: normalize_id(name),
                name: name.to_string(),
                version: version.to_string(),
                backend: "apt".into(),
                backend_id: name.to_string(),
                origin: Some("ubuntu".into()),
                path: Some(format!("/var/lib/dpkg/info/{name}.list")),
                latest_version: None,
                outdated: false,
                is_app,
            });
        }
        Ok(packages)
    }

    fn search(&self, query: &str) -> Result<Vec<Candidate>> {
        let output = run_command_allow_fail("apt-cache", &["search", query])?;
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        for line in output.lines() {
            if let Some((name, rest)) = line.split_once(" - ") {
                if seen.insert(name.to_string()) {
                    candidates.push(Candidate {
                        canonical_id: normalize_id(name),
                        name: name.to_string(),
                        version: None,
                        backend: "apt".into(),
                        backend_id: name.to_string(),
                        description: Some(rest.to_string()),
                        origin: Some("ubuntu".into()),
                    });
                }
            }
        }
        Ok(candidates)
    }

    fn install(&self, id: &str, version: Option<&str>) -> Result<()> {
        let pkg = match version {
            Some(v) => format!("{id}={v}"),
            None => id.to_string(),
        };
        run_command_sudo("apt-get", &["install", "-y", &pkg])?;
        Ok(())
    }

    fn upgrade(&self, id: &str) -> Result<()> {
        run_command_sudo("apt-get", &["install", "--only-upgrade", "-y", id])?;
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        run_command_sudo("apt-get", &["update"])?;
        run_command_sudo("apt-get", &["upgrade", "-y"])?;
        Ok(())
    }

    fn uninstall(&self, id: &str, purge: bool) -> Result<()> {
        if purge {
            run_command_sudo("apt-get", &["purge", "-y", id])?;
        } else {
            run_command_sudo("apt-get", &["remove", "-y", id])?;
        }
        Ok(())
    }

    fn cleanup(&self) -> Result<CleanupReport> {
        run_command_sudo("apt-get", &["autoremove", "-y"])?;
        run_command_sudo("apt-get", &["autoclean", "-y"])?;
        Ok(CleanupReport {
            removed: vec!["apt autoremove/autoclean".into()],
            freed_description: "APT cache and unused packages cleaned".into(),
        })
    }

    fn check_updates(&self) -> Result<Vec<UpdateAvailable>> {
        let _ = run_command_sudo("apt-get", &["update"]);
        let output = run_command_allow_fail("apt", &["list", "--upgradable"])?;
        let mut updates = Vec::new();

        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name_ver = parts[0];
                if let Some((name, current)) = name_ver.split_once('/') {
                    let latest = parts.get(1).unwrap_or(&current).to_string();
                    updates.push(UpdateAvailable {
                        backend_id: name.to_string(),
                        backend: "apt".into(),
                        current_version: current.to_string(),
                        latest_version: latest,
                    });
                }
            }
        }
        Ok(updates)
    }

    fn pin_add(&self, id: &str, reason: Option<&str>) -> Result<()> {
        let _ = reason;
        run_command_sudo("apt-mark", &["hold", id])?;
        Ok(())
    }

    fn pin_remove(&self, id: &str) -> Result<()> {
        run_command_sudo("apt-mark", &["unhold", id])?;
        Ok(())
    }

    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>> {
        let output = run_command_allow_fail("apt-mark", &["showhold"])?;
        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| (l.trim().to_string(), Some("apt hold".into())))
            .collect())
    }
}
