use crate::config::Config;
use crate::models::{Candidate, CleanupReport, Package, UpdateAvailable};
use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

pub mod apt;
pub mod appimage;
pub mod brew;
pub mod cargo_backend;
pub mod direct_deb;
pub mod flatpak;
pub mod pipx;
pub mod snap;

pub use apt::AptBackend;
pub use appimage::AppImageBackend;
pub use brew::BrewBackend;
pub use cargo_backend::CargoBackend;
pub use direct_deb::DirectDebBackend;
pub use flatpak::FlatpakBackend;
pub use pipx::PipxBackend;
pub use snap::SnapBackend;

pub trait Backend: Send + Sync {
    fn id(&self) -> &str;
    fn available(&self) -> bool;
    fn list_installed(&self, apps_only: bool) -> Result<Vec<Package>>;
    fn search(&self, query: &str) -> Result<Vec<Candidate>>;
    fn install(&self, id: &str, version: Option<&str>) -> Result<()>;
    fn upgrade(&self, id: &str) -> Result<()>;
    fn upgrade_all(&self) -> Result<()>;
    fn uninstall(&self, id: &str, purge: bool) -> Result<()>;
    fn cleanup(&self) -> Result<CleanupReport>;
    fn check_updates(&self) -> Result<Vec<UpdateAvailable>>;
    fn pin_add(&self, id: &str, reason: Option<&str>) -> Result<()>;
    fn pin_remove(&self, id: &str) -> Result<()>;
    fn list_pins(&self) -> Result<Vec<(String, Option<String>)>>;
}

pub fn all_backends(config: &Config) -> Vec<Box<dyn Backend>> {
    vec![
        Box::new(AptBackend),
        Box::new(SnapBackend),
        Box::new(FlatpakBackend),
        Box::new(BrewBackend),
        Box::new(PipxBackend),
        Box::new(CargoBackend),
        Box::new(AppImageBackend::new(config)),
        Box::new(DirectDebBackend::new()),
    ]
}

pub fn get_backend<'a>(backends: &'a [Box<dyn Backend>], id: &str) -> Option<&'a dyn Backend> {
    backends.iter().find(|b| b.id() == id).map(|b| b.as_ref())
}

pub fn enabled_backend_instances<'a>(
    backends: &'a [Box<dyn Backend>],
    enabled: &[String],
) -> Vec<&'a dyn Backend> {
    backends
        .iter()
        .filter(|b| enabled.contains(&b.id().to_string()) && b.available())
        .map(|b| b.as_ref())
        .collect()
}

pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

pub fn run_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {program}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{program} failed: {stderr}");
    }
}

pub fn run_command_allow_fail(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {program}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run_command_sudo(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("sudo")
        .arg("-n")
        .arg(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run sudo {program}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let output = Command::new("sudo")
            .arg(program)
            .args(args)
            .output()
            .with_context(|| format!("failed to run sudo {program}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("sudo {program} failed: {stderr}");
        }
    }
}

pub fn desktop_packages() -> HashSet<String> {
    let mut packages = HashSet::new();
    let desktop_dirs = ["/usr/share/applications", "/var/lib/snapd/desktop/applications"];
    for dir in desktop_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".desktop") {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            for line in content.lines() {
                                if let Some(pkg) = line.strip_prefix("X-AppStream-Package=") {
                                    packages.insert(pkg.trim().to_string());
                                }
                            }
                        }
                        let stem = name.trim_end_matches(".desktop");
                        packages.insert(stem.to_string());
                    }
                }
            }
        }
    }
    packages
}

pub fn manual_apt_packages() -> HashSet<String> {
    run_command_allow_fail("apt-mark", &["showmanual"])
        .map(|out| out.lines().map(|l| l.trim().to_string()).collect())
        .unwrap_or_default()
}

pub fn normalize_id(name: &str) -> String {
    name.to_lowercase()
        .replace([' ', '_'], "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '.')
        .collect()
}

pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}
