use crate::backends::{normalize_id, Backend};
use crate::config::{aliases_path, Config};
use crate::core::identity::IdentityGraph;
use crate::models::{Candidate, Package};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;

pub struct PackageResolver<'a> {
    backends: &'a [Box<dyn Backend>],
    config: &'a Config,
    identity: IdentityGraph,
}

impl<'a> PackageResolver<'a> {
    pub fn new(backends: &'a [Box<dyn Backend>], config: &'a Config) -> Result<Self> {
        let identity = IdentityGraph::load(&aliases_path())?;
        Ok(Self {
            backends,
            config,
            identity,
        })
    }

    pub fn identity(&self) -> &IdentityGraph {
        &self.identity
    }

    pub fn search(&self, query: &str, sources: Option<&[String]>) -> Result<Vec<Candidate>> {
        let enabled = crate::config::enabled_backends(self.config);
        let target_sources: Vec<&str> = match sources {
            Some(s) => s.iter().map(|s| s.as_str()).collect(),
            None => enabled.iter().map(|s| s.as_str()).collect(),
        };

        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for backend in self.backends.iter() {
            if !target_sources.contains(&backend.id()) || !backend.available() {
                continue;
            }
            if let Ok(candidates) = backend.search(query) {
                for mut c in candidates {
                    c.canonical_id = self.identity.resolve_canonical(&c.canonical_id, backend.id(), &c.backend_id);
                    let key = format!("{}:{}", c.backend, c.backend_id);
                    if seen.insert(key) {
                        results.push(c);
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn list_installed(&self, apps_only: bool) -> Result<Vec<Package>> {
        let enabled = crate::config::enabled_backends(self.config);
        let mut packages = Vec::new();

        for backend in self.backends.iter() {
            if !enabled.contains(&backend.id().to_string()) || !backend.available() {
                continue;
            }
            if let Ok(mut pkgs) = backend.list_installed(apps_only) {
                for pkg in &mut pkgs {
                    pkg.canonical_id = self.identity.resolve_canonical(
                        &pkg.canonical_id,
                        backend.id(),
                        &pkg.backend_id,
                    );
                }
                packages.extend(pkgs);
            }
        }
        Ok(packages)
    }

    pub fn resolve_backend_for_install(
        &self,
        id: &str,
        source: Option<&str>,
    ) -> Result<(&dyn Backend, String)> {
        let canonical = normalize_id(id);

        if let Some(src) = source {
            let backend = self
                .backends
                .iter()
                .find(|b| b.id() == src && b.available())
                .map(|b| b.as_ref())
                .with_context(|| format!("backend {src} not available"))?;

            let backend_id = self
                .identity
                .backend_id_for(&canonical, src)
                .unwrap_or_else(|| id.to_string());
            return Ok((backend, backend_id));
        }

        for backend_name in &self.config.backend_priority {
            if !crate::config::is_backend_enabled(self.config, backend_name) {
                continue;
            }
            let backend = self
                .backends
                .iter()
                .find(|b| b.id() == backend_name.as_str() && b.available());
            if let Some(backend) = backend {
                let backend_id = self
                    .identity
                    .backend_id_for(&canonical, backend_name)
                    .unwrap_or_else(|| id.to_string());

                if let Ok(candidates) = backend.search(&backend_id) {
                    if candidates.iter().any(|c| {
                        c.backend_id == backend_id
                            || c.canonical_id == canonical
                            || c.name.to_lowercase() == id.to_lowercase()
                    }) {
                        return Ok((backend.as_ref(), backend_id));
                    }
                }
                // Also try direct install ID match
                if backend_name == "apt" || backend_name == "snap" || backend_name == "brew" {
                    return Ok((backend.as_ref(), backend_id));
                }
            }
        }

        bail!("could not resolve package {id} on any enabled backend")
    }

    pub fn find_installed(&self, id: &str, apps_only: bool) -> Result<Vec<Package>> {
        let canonical = normalize_id(id);
        let packages = self.list_installed(apps_only)?;
        Ok(packages
            .into_iter()
            .filter(|p| {
                p.canonical_id == canonical
                    || p.backend_id == id
                    || p.name.to_lowercase() == id.to_lowercase()
            })
            .collect())
    }

    pub fn find_duplicates(&self, apps_only: bool) -> Result<HashMap<String, Vec<Package>>> {
        let packages = self.list_installed(apps_only)?;
        let mut groups: HashMap<String, Vec<Package>> = HashMap::new();

        for pkg in packages {
            groups
                .entry(pkg.canonical_id.clone())
                .or_default()
                .push(pkg);
        }

        groups.retain(|_, v| v.len() > 1);
        Ok(groups)
    }

    pub fn check_outdated(&self, apps_only: bool) -> Result<Vec<Package>> {
        let mut packages = self.list_installed(apps_only)?;
        let enabled = crate::config::enabled_backends(self.config);

        let mut update_map: HashMap<String, String> = HashMap::new();
        for backend in self.backends.iter() {
            if !enabled.contains(&backend.id().to_string()) || !backend.available() {
                continue;
            }
            if let Ok(updates) = backend.check_updates() {
                for u in updates {
                    update_map.insert(
                        format!("{}:{}", u.backend, u.backend_id),
                        u.latest_version,
                    );
                }
            }
        }

        for pkg in &mut packages {
            let key = format!("{}:{}", pkg.backend, pkg.backend_id);
            if let Some(latest) = update_map.get(&key) {
                if latest != &pkg.version && latest != "?" {
                    pkg.latest_version = Some(latest.clone());
                    pkg.outdated = true;
                }
            }
        }

        Ok(packages.into_iter().filter(|p| p.outdated).collect())
    }
}
