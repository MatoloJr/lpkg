use crate::backends::normalize_id;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct IdentityGraph {
    aliases: HashMap<String, HashMap<String, String>>,
}

impl IdentityGraph {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let parsed: toml::Value = toml::from_str(&content)?;
        let mut aliases = HashMap::new();

        if let Some(table) = parsed.as_table() {
            for (canonical, backends) in table {
                if let Some(backend_map) = backends.as_table() {
                    let mut map = HashMap::new();
                    for (backend, id) in backend_map {
                        if let Some(id_str) = id.as_str() {
                            map.insert(backend.clone(), id_str.to_string());
                        }
                    }
                    aliases.insert(canonical.clone(), map);
                }
            }
        }

        Ok(Self { aliases })
    }

    pub fn resolve_canonical(&self, hint: &str, backend: &str, backend_id: &str) -> String {
        // Check if backend_id maps to a known canonical
        for (canonical, backends) in &self.aliases {
            if let Some(id) = backends.get(backend) {
                if id == backend_id || normalize_id(id) == normalize_id(backend_id) {
                    return canonical.clone();
                }
            }
        }

        // Check if hint matches a canonical
        let hint_norm = normalize_id(hint);
        if self.aliases.contains_key(&hint_norm) {
            return hint_norm;
        }

        for canonical in self.aliases.keys() {
            if normalize_id(canonical) == hint_norm {
                return canonical.clone();
            }
        }

        hint_norm
    }

    pub fn backend_id_for(&self, canonical: &str, backend: &str) -> Option<String> {
        self.aliases
            .get(canonical)
            .and_then(|m| m.get(backend))
            .cloned()
            .or_else(|| {
                self.aliases
                    .get(&normalize_id(canonical))
                    .and_then(|m| m.get(backend))
                    .cloned()
            })
    }

    pub fn find_duplicates_by_binary(&self, packages: &[crate::models::Package]) -> Vec<Vec<String>> {
        let mut binary_map: HashMap<String, Vec<String>> = HashMap::new();

        for pkg in packages {
            let key = pkg.canonical_id.clone();
            binary_map.entry(key).or_default().push(format!(
                "{}:{} ({})",
                pkg.backend, pkg.backend_id, pkg.version
            ));
        }

        binary_map
            .into_values()
            .filter(|v| v.len() > 1)
            .collect()
    }
}
