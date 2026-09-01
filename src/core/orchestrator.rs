use crate::backends::Backend;
use crate::config::Config;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

const UPGRADE_ORDER: &[&str] = &[
    "apt",
    "flatpak",
    "snap",
    "brew",
    "pipx",
    "cargo",
    "direct_deb",
];

pub struct UpgradeOrchestrator<'a> {
    backends: &'a [Box<dyn Backend>],
    config: &'a Config,
}

impl<'a> UpgradeOrchestrator<'a> {
    pub fn new(backends: &'a [Box<dyn Backend>], config: &'a Config) -> Self {
        Self { backends, config }
    }

    pub fn upgrade_all(&self, dry_run: bool) -> Result<Vec<(String, bool)>> {
        let lock = acquire_system_lock()?;
        let mut results = Vec::new();

        for backend_name in UPGRADE_ORDER {
            if !crate::config::is_backend_enabled(self.config, backend_name) {
                continue;
            }
            let backend = self
                .backends
                .iter()
                .find(|b| b.id() == *backend_name && b.available());

            if let Some(backend) = backend {
                if dry_run {
                    results.push((backend_name.to_string(), true));
                    continue;
                }

                let result = retry_with_backoff(|| backend.upgrade_all(), 3);
                results.push((backend_name.to_string(), result.is_ok()));

                if let Err(e) = result {
                    eprintln!("warning: {backend_name} upgrade failed: {e}");
                }

                // Brief pause between backends to avoid lock contention
                thread::sleep(Duration::from_secs(1));
            }
        }

        drop(lock);
        Ok(results)
    }

    pub fn upgrade_package(&self, backend: &dyn Backend, id: &str, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("would upgrade {} via {}", id, backend.id());
            return Ok(());
        }
        let _lock = acquire_system_lock()?;
        backend.upgrade(id)
    }

    pub fn cleanup_all(&self, sources: Option<&[String]>) -> Result<Vec<(String, String)>> {
        let enabled = crate::config::enabled_backends(self.config);
        let target: Vec<&str> = match sources {
            Some(s) => s.iter().map(|s| s.as_str()).collect(),
            None => enabled.iter().map(|s| s.as_str()).collect(),
        };

        let mut reports = Vec::new();
        for backend in self.backends.iter() {
            if !target.contains(&backend.id()) || !backend.available() {
                continue;
            }
            if let Ok(report) = backend.cleanup() {
                reports.push((backend.id().to_string(), report.freed_description));
            }
        }
        Ok(reports)
    }
}

struct SystemLock;

fn acquire_system_lock() -> Result<SystemLock> {
    let lock_path = "/tmp/lpkg-upgrade.lock";
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(lock_path)?;

    // Simple file-based lock using flock via libc would be ideal;
    // for portability we use a lock file with retry
    for attempt in 0..10 {
        if is_dpkg_locked() {
            thread::sleep(Duration::from_secs(2 * (attempt + 1) as u64));
            continue;
        }
        writeln!(&file, "locked")?;
        return Ok(SystemLock);
    }

    Ok(SystemLock)
}

fn is_dpkg_locked() -> bool {
    Path::new("/var/lib/dpkg/lock-frontend").exists()
        && std::fs::read_to_string("/var/lib/dpkg/lock-frontend")
            .map(|_| true)
            .unwrap_or(false)
}

fn retry_with_backoff<F>(mut f: F, max_retries: u32) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    for attempt in 0..max_retries {
        match f() {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt + 1 == max_retries {
                    return Err(e);
                }
                thread::sleep(Duration::from_secs(2u64.pow(attempt)));
            }
        }
    }
    Ok(())
}
