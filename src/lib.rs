#[cfg(test)]
mod tests {
    use crate::backends::normalize_id;
    use crate::core::identity::IdentityGraph;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn normalize_id_lowercases_and_sanitizes() {
        assert_eq!(normalize_id("Firefox"), "firefox");
        assert_eq!(normalize_id("Visual Studio Code"), "visual-studio-code");
        assert_eq!(normalize_id("org.mozilla.firefox"), "org.mozilla.firefox");
    }

    #[test]
    fn identity_graph_resolves_aliases() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/aliases.toml");
        let graph = IdentityGraph::load(&path).unwrap();
        let canonical = graph.resolve_canonical("firefox", "flatpak", "org.mozilla.firefox");
        assert_eq!(canonical, "firefox");
        assert_eq!(
            graph.backend_id_for("firefox", "apt"),
            Some("firefox".to_string())
        );
    }

    #[test]
    fn default_config_has_expected_backends() {
        let config = Config::default();
        assert!(config.backends.apt);
        assert!(config.backends.flatpak);
        assert_eq!(config.backend_priority[0], "flatpak");
    }

    #[test]
    fn registry_roundtrip() {
        use crate::core::registry::InstallRegistry;
        use crate::models::RegistryEntry;
        use chrono::Utc;

        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_DATA_HOME", dir.path());

        let registry = InstallRegistry::open().unwrap();
        let entry = RegistryEntry {
            canonical_id: "test-app".into(),
            backend: "apt".into(),
            backend_id: "test-app".into(),
            version: "1.0".into(),
            installed_at: Utc::now(),
            binary_paths: vec!["/usr/bin/test-app".into()],
            desktop_file: None,
        };
        registry.record_install(&entry).unwrap();
        let all = registry.list_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].canonical_id, "test-app");
    }
}
