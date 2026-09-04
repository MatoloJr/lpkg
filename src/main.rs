use anyhow::{Context, Result};
use chrono::Utc;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use lpkg::backends;
use lpkg::backends::all_backends;
use lpkg::cli::{Cli, Commands, OutputFormat, PinCommands};
use lpkg::config::{enabled_backends, ensure_dirs, is_backend_enabled, load_config, Config};
use lpkg::core::orchestrator::UpgradeOrchestrator;
use lpkg::core::registry::InstallRegistry;
use lpkg::core::resolver::PackageResolver;
use lpkg::models::{ExportManifest, ManifestPackage, RegistryEntry};
use lpkg::output::print_candidates;
use std::fs;
use std::io;
use std::path::Path;
use std::process;

fn main() -> Result<()> {
    ensure_dirs()?;
    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    let config = load_config()?;
    let backends = all_backends(&config);

    match cli.command {
        Commands::Search { queries, source } => {
            cmd_search(&backends, &config, &queries, source.as_deref())
        }
        Commands::Install {
            ids,
            all,
            source,
            version,
            auto,
        } => cmd_install(
            &backends,
            &config,
            &ids,
            all.as_deref(),
            source.as_deref(),
            version.as_deref(),
            auto,
        ),
        Commands::List {
            outdated,
            duplicates,
            all,
            format,
            source: _,
        } => cmd_list(&backends, &config, outdated, duplicates, !all, format),
        Commands::Upgrade {
            id,
            all,
            auto,
            dry_run,
            source,
        } => cmd_upgrade(
            &backends,
            &config,
            id.as_deref(),
            all,
            auto,
            dry_run,
            source.as_deref(),
        ),
        Commands::Uninstall { id, purge, source } => {
            cmd_uninstall(&backends, &config, &id, purge, source.as_deref())
        }
        Commands::Cleanup { source } => cmd_cleanup(&backends, &config, source.as_deref()),
        Commands::Info { id } => cmd_info(&backends, &config, &id),
        Commands::Which { id } => cmd_which(&backends, &config, &id),
        Commands::Scan => cmd_scan(&backends, &config),
        Commands::Doctor => cmd_doctor(&backends, &config),
        Commands::Pin { action } => cmd_pin(&backends, &config, action),
        Commands::Export { output } => cmd_export(&backends, &config, &output),
        Commands::Import { file, dry_run } => cmd_import(&backends, &config, &file, dry_run),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "lpkg", &mut io::stdout());
            Ok(())
        }
    }
}

fn cmd_search(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    queries: &[String],
    sources: Option<&[String]>,
) -> Result<()> {
    if queries.is_empty() {
        anyhow::bail!("provide at least one search term, e.g. lpkg search firefox");
    }

    let resolver = PackageResolver::new(backends, config)?;
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for query in queries {
        for c in resolver.search(query, sources)? {
            let key = format!("{}:{}", c.backend, c.backend_id);
            if seen.insert(key) {
                results.push(c);
            }
        }
    }

    if results.is_empty() {
        let joined = queries.join("', '");
        println!("No packages found for '{joined}'");
    } else {
        print_candidates(&results);
    }
    Ok(())
}

fn cmd_install(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    ids: &[String],
    all_manifest: Option<&Path>,
    source: Option<&str>,
    version: Option<&str>,
    auto: bool,
) -> Result<()> {
    if let Some(manifest_path) = all_manifest {
        if !ids.is_empty() {
            anyhow::bail!("do not pass package ids together with --all");
        }
        if version.is_some() {
            anyhow::bail!("--version cannot be used with --all");
        }
        return cmd_import(backends, config, manifest_path, false).or_else(|e| {
            if !manifest_path.exists() {
                anyhow::bail!(
                    "manifest not found at {} (pass a path: lpkg install --all path/to/packages.json)",
                    manifest_path.display()
                );
            }
            Err(e)
        });
    }

    if ids.is_empty() {
        anyhow::bail!("provide at least one package id, or use --all [manifest]");
    }

    if ids.len() > 1 && version.is_some() {
        anyhow::bail!("--version can only be used when installing a single package");
    }

    for id in ids {
        install_one(backends, config, id, source, version, auto)?;
    }
    Ok(())
}

fn install_one(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    id: &str,
    source: Option<&str>,
    version: Option<&str>,
    auto: bool,
) -> Result<()> {
    if auto {
        let resolver = PackageResolver::new(backends, config)?;
        let mut existing = resolver.find_installed(id, true)?;
        if let Some(src) = source {
            existing.retain(|p| p.backend == src);
        }
        if !existing.is_empty() {
            println!("--auto: removing existing install(s) of {id}...");
            let registry = InstallRegistry::open()?;
            for pkg in existing {
                let backend = backends::get_backend(backends, &pkg.backend)
                    .ok_or_else(|| anyhow::anyhow!("backend {} not found", pkg.backend))?;
                println!("  uninstalling {} via {}...", pkg.name, pkg.backend);
                backend.uninstall(&pkg.backend_id, false)?;
                registry.remove_install(&pkg.canonical_id, &pkg.backend, &pkg.backend_id)?;
            }
        }
    }

    let resolver = PackageResolver::new(backends, config)?;
    let (backend, backend_id) = resolver.resolve_backend_for_install(id, source)?;

    println!("Installing {id} via {} as {backend_id}...", backend.id());
    backend.install(&backend_id, version)?;

    let registry = InstallRegistry::open()?;
    let canonical = backends::normalize_id(id);
    registry.record_install(&RegistryEntry {
        canonical_id: canonical,
        backend: backend.id().to_string(),
        backend_id: backend_id.clone(),
        version: version.unwrap_or("latest").to_string(),
        installed_at: Utc::now(),
        binary_paths: Vec::new(),
        desktop_file: None,
    })?;

    println!("Successfully installed {id}");
    Ok(())
}

fn cmd_list(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    outdated: bool,
    duplicates: bool,
    apps_only: bool,
    format: OutputFormat,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;

    if duplicates {
        let dups = resolver.find_duplicates(apps_only)?;
        if dups.is_empty() {
            println!("No duplicate installations found.");
            return Ok(());
        }
        for (canonical, pkgs) in &dups {
            println!("{canonical}:");
            for pkg in pkgs {
                println!("  {} {} ({})", pkg.backend, pkg.backend_id, pkg.version);
            }
        }
        return Ok(());
    }

    let packages = if outdated {
        resolver.check_outdated(apps_only)?
    } else {
        resolver.list_installed(apps_only)?
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&packages)?);
        }
        OutputFormat::Table => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["ID", "Name", "Version", "Backend", "Outdated"]);

            for pkg in &packages {
                table.add_row(vec![
                    Cell::new(&pkg.canonical_id),
                    Cell::new(&pkg.name),
                    Cell::new(&pkg.version),
                    Cell::new(&pkg.backend),
                    Cell::new(if pkg.outdated { "yes" } else { "" }),
                ]);
            }
            if packages.is_empty() {
                println!("No packages found.");
            } else {
                println!("{table}");
                println!("{} package(s)", packages.len());
            }
        }
    }
    Ok(())
}

fn cmd_upgrade(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    id: Option<&str>,
    all: bool,
    auto: bool,
    dry_run: bool,
    source: Option<&str>,
) -> Result<()> {
    let orchestrator = UpgradeOrchestrator::new(backends, config);

    if all {
        println!("Upgrading all packages (backend upgrade_all)...");
        let results = orchestrator.upgrade_all(dry_run)?;
        for (backend, ok) in results {
            let status = if ok { "ok" } else { "failed" };
            println!("  {backend}: {status}");
        }
        return Ok(());
    }

    // Bare `upgrade` or `upgrade --auto` with no id: upgrade outdated only
    if id.is_none() {
        return upgrade_outdated(backends, config, &orchestrator, dry_run, source);
    }

    let id = id.unwrap();
    let resolver = PackageResolver::new(backends, config)?;

    if auto {
        let outdated = resolver.check_outdated(true)?;
        let matches: Vec<_> = outdated
            .into_iter()
            .filter(|p| {
                let id_match = p.canonical_id == backends::normalize_id(id)
                    || p.backend_id == id
                    || p.name.eq_ignore_ascii_case(id);
                let src_match = source.map(|s| p.backend == s).unwrap_or(true);
                id_match && src_match
            })
            .collect();

        if matches.is_empty() {
            // Confirm installed but up to date, vs not installed
            let installed = resolver.find_installed(id, true)?;
            if installed.is_empty() {
                anyhow::bail!("package {id} is not installed");
            }
            println!("{id} is already up to date");
            return Ok(());
        }

        for pkg in matches {
            let backend = backends::get_backend(backends, &pkg.backend)
                .ok_or_else(|| anyhow::anyhow!("backend {} not found", pkg.backend))?;
            if dry_run {
                println!(
                    "would upgrade {} via {} ({} -> {})",
                    pkg.name,
                    pkg.backend,
                    pkg.version,
                    pkg.latest_version.as_deref().unwrap_or("?")
                );
            } else {
                orchestrator.upgrade_package(backend, &pkg.backend_id, false)?;
                println!("Upgraded {} via {}", pkg.name, pkg.backend);
            }
        }
        return Ok(());
    }

    if let Some(src) = source {
        let backend = backends::get_backend(backends, src)
            .ok_or_else(|| anyhow::anyhow!("backend {src} not found"))?;
        orchestrator.upgrade_package(backend, id, dry_run)?;
    } else {
        let installed = resolver.find_installed(id, true)?;
        if installed.is_empty() {
            anyhow::bail!("package {id} is not installed");
        }
        for pkg in installed {
            let backend = backends::get_backend(backends, &pkg.backend)
                .ok_or_else(|| anyhow::anyhow!("backend {} not found", pkg.backend))?;
            orchestrator.upgrade_package(backend, &pkg.backend_id, dry_run)?;
            println!("Upgraded {} via {}", pkg.name, pkg.backend);
        }
    }
    Ok(())
}

fn upgrade_outdated(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    orchestrator: &UpgradeOrchestrator<'_>,
    dry_run: bool,
    source: Option<&str>,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;
    let mut outdated = resolver.check_outdated(true)?;
    if let Some(src) = source {
        outdated.retain(|p| p.backend == src);
    }

    if outdated.is_empty() {
        println!("No outdated packages found.");
        return Ok(());
    }

    println!("Upgrading {} outdated package(s)...", outdated.len());
    for pkg in outdated {
        let backend = backends::get_backend(backends, &pkg.backend)
            .ok_or_else(|| anyhow::anyhow!("backend {} not found", pkg.backend))?;
        if dry_run {
            println!(
                "would upgrade {} via {} ({} -> {})",
                pkg.name,
                pkg.backend,
                pkg.version,
                pkg.latest_version.as_deref().unwrap_or("?")
            );
        } else {
            match orchestrator.upgrade_package(backend, &pkg.backend_id, false) {
                Ok(()) => println!("Upgraded {} via {}", pkg.name, pkg.backend),
                Err(e) => eprintln!("warning: failed to upgrade {} via {}: {e}", pkg.name, pkg.backend),
            }
        }
    }
    Ok(())
}

fn cmd_uninstall(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    id: &str,
    purge: bool,
    source: Option<&str>,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;

    let targets = if let Some(src) = source {
        let installed = resolver.find_installed(id, true)?;
        installed
            .into_iter()
            .filter(|p| p.backend == src)
            .collect()
    } else {
        resolver.find_installed(id, true)?
    };

    if targets.is_empty() {
        anyhow::bail!("package {id} is not installed");
    }

    let registry = InstallRegistry::open()?;
    for pkg in targets {
        let backend = backends::get_backend(backends, &pkg.backend)
            .ok_or_else(|| anyhow::anyhow!("backend {} not found", pkg.backend))?;
        println!("Uninstalling {} via {}...", pkg.name, pkg.backend);
        backend.uninstall(&pkg.backend_id, purge)?;
        registry.remove_install(&pkg.canonical_id, &pkg.backend, &pkg.backend_id)?;
    }
    println!("Successfully uninstalled {id}");
    Ok(())
}

fn cmd_cleanup(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    sources: Option<&[String]>,
) -> Result<()> {
    let orchestrator = UpgradeOrchestrator::new(backends, config);
    let reports = orchestrator.cleanup_all(sources)?;
    for (backend, desc) in reports {
        println!("{backend}: {desc}");
    }
    Ok(())
}

fn cmd_info(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    id: &str,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;

    println!("Package: {id}\n");

    let installed = resolver.find_installed(id, true)?;
    if installed.is_empty() {
        println!("Installed: (none)");
    } else {
        println!("Installed:");
        for pkg in &installed {
            let latest = pkg
                .latest_version
                .as_deref()
                .map(|v| format!(" (latest: {v})"))
                .unwrap_or_default();
            let outdated = if pkg.outdated { " [outdated]" } else { "" };
            println!(
                "  {} {} via {}{latest}{outdated}",
                pkg.name, pkg.version, pkg.backend
            );
        }
    }

    // Mark outdated status if available
    let outdated = resolver.check_outdated(true)?;
    let outdated_here: Vec<_> = outdated
        .iter()
        .filter(|p| {
            p.canonical_id == backends::normalize_id(id)
                || p.backend_id == id
                || p.name.eq_ignore_ascii_case(id)
        })
        .collect();
    if !outdated_here.is_empty() && installed.iter().all(|p| !p.outdated) {
        println!("\nUpdates available:");
        for pkg in outdated_here {
            println!(
                "  {} {} -> {} ({})",
                pkg.name,
                pkg.version,
                pkg.latest_version.as_deref().unwrap_or("?"),
                pkg.backend
            );
        }
    }

    println!("\nSearch candidates:");
    let candidates = resolver.search(id, None)?;
    if candidates.is_empty() {
        println!("  (none)");
    } else {
        print_candidates(&candidates);
    }
    Ok(())
}

fn cmd_which(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    id: &str,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;
    let installed = resolver.find_installed(id, true)?;
    if installed.is_empty() {
        anyhow::bail!("package {id} is not installed");
    }
    for pkg in installed {
        println!("{} ({}) {}", pkg.backend, pkg.backend_id, pkg.version);
    }
    Ok(())
}

fn cmd_scan(backends: &[Box<dyn backends::Backend>], config: &Config) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;
    let registry = InstallRegistry::open()?;
    let packages = resolver.list_installed(config.apps_only)?;
    let data = serde_json::to_string(&packages)?;
    registry.cache_scan("all", &data)?;
    println!("Scanned {} packages", packages.len());

    let enabled = enabled_backends(config);
    for backend in backends.iter() {
        if !enabled.contains(&backend.id().to_string()) {
            continue;
        }
        match backend.list_installed(config.apps_only) {
            Ok(pkgs) => eprintln!("  {}: {} package(s)", backend.id(), pkgs.len()),
            Err(e) => eprintln!("  {}: error - {e}", backend.id()),
        }
    }
    Ok(())
}

fn cmd_doctor(backends: &[Box<dyn backends::Backend>], config: &Config) -> Result<()> {
    let mut issues = 0;

    println!("lpkg doctor\n");

    println!("Backends:");
    for backend in backends.iter() {
        let status = if backend.available() { "available" } else { "missing" };
        println!("  {}: {status}", backend.id());
        if !backend.available() && is_backend_enabled(config, backend.id()) {
            issues += 1;
        }
    }

    if std::path::Path::new("/var/lib/dpkg/lock-frontend").exists() {
        println!("\nWarning: dpkg lock file exists (another package manager may be running)");
        issues += 1;
    }

    let resolver = PackageResolver::new(backends, config)?;
    let dups = resolver.find_duplicates(true)?;
    if !dups.is_empty() {
        println!("\nDuplicate installations ({}):", dups.len());
        for (canonical, pkgs) in &dups {
            println!("  {canonical}: {} copies", pkgs.len());
            issues += 1;
        }
    }

    let registry = InstallRegistry::open()?;
    let entries = registry.list_all()?;
    println!("\nRegistry: {} tracked install(s)", entries.len());

    if issues == 0 {
        println!("\nAll checks passed.");
    } else {
        println!("\n{issues} issue(s) found.");
        process::exit(1);
    }
    Ok(())
}

fn cmd_pin(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    action: PinCommands,
) -> Result<()> {
    let registry = InstallRegistry::open()?;

    match action {
        PinCommands::Add { id, source, reason } => {
            let resolver = PackageResolver::new(backends, config)?;
            let installed = resolver.find_installed(&id, true)?;
            if installed.is_empty() {
                anyhow::bail!("package {id} is not installed");
            }
            for pkg in installed {
                if let Some(src) = source.as_deref() {
                    if pkg.backend != src {
                        continue;
                    }
                }
                let backend = backends::get_backend(backends, &pkg.backend)
                    .ok_or_else(|| anyhow::anyhow!("backend not found"))?;
                backend.pin_add(&pkg.backend_id, reason.as_deref())?;
                registry.add_pin(&pkg.canonical_id, &pkg.backend, &pkg.backend_id, reason.as_deref())?;
                println!("Pinned {} ({})", pkg.name, pkg.backend);
            }
        }
        PinCommands::Remove { id, source } => {
            let resolver = PackageResolver::new(backends, config)?;
            let installed = resolver.find_installed(&id, true)?;
            for pkg in installed {
                if let Some(src) = source.as_deref() {
                    if pkg.backend != src {
                        continue;
                    }
                }
                let backend = backends::get_backend(backends, &pkg.backend)
                    .ok_or_else(|| anyhow::anyhow!("backend not found"))?;
                backend.pin_remove(&pkg.backend_id)?;
                registry.remove_pin(&pkg.canonical_id)?;
                println!("Unpinned {} ({})", pkg.name, pkg.backend);
            }
        }
        PinCommands::List => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(vec!["ID", "Backend", "Backend ID", "Reason"]);

            for backend in backends.iter() {
                if let Ok(pins) = backend.list_pins() {
                    for (backend_id, reason) in pins {
                        table.add_row(vec![
                            Cell::new(&backend_id),
                            Cell::new(backend.id()),
                            Cell::new(&backend_id),
                            Cell::new(reason.unwrap_or_default()),
                        ]);
                    }
                }
            }

            let registry_pins = registry.list_pins()?;
            for (canonical, backend, backend_id, reason) in registry_pins {
                table.add_row(vec![
                    Cell::new(&canonical),
                    Cell::new(&backend),
                    Cell::new(&backend_id),
                    Cell::new(reason.unwrap_or_default()),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

fn cmd_export(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    output: &std::path::Path,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;
    let packages = resolver.list_installed(config.apps_only)?;
    let registry = InstallRegistry::open()?;
    let pins = registry.list_pins()?;

    let pin_map: std::collections::HashMap<(String, String), String> = pins
        .into_iter()
        .map(|(c, b, _, r)| ((c, b), r.unwrap_or_else(|| "pinned".into())))
        .collect();

    let manifest = ExportManifest {
        version: 1,
        packages: packages
            .into_iter()
            .map(|p| {
                let pin = pin_map
                    .get(&(p.canonical_id.clone(), p.backend.clone()))
                    .cloned();
                ManifestPackage {
                    id: p.canonical_id,
                    backend: p.backend,
                    backend_id: p.backend_id,
                    pin,
                    version: Some(p.version),
                }
            })
            .collect(),
    };

    fs::write(output, serde_json::to_string_pretty(&manifest)?)?;
    println!("Exported {} packages to {}", manifest.packages.len(), output.display());
    Ok(())
}

fn cmd_import(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    file: &std::path::Path,
    dry_run: bool,
) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    let manifest: ExportManifest = serde_json::from_str(&content)?;

    for pkg in &manifest.packages {
        if dry_run {
            println!("would install {} via {} ({})", pkg.id, pkg.backend, pkg.backend_id);
        } else {
            install_one(
                backends,
                config,
                &pkg.id,
                Some(&pkg.backend),
                pkg.version.as_deref(),
                false,
            )?;
            if pkg.pin.is_some() {
                cmd_pin(
                    backends,
                    config,
                    PinCommands::Add {
                        id: pkg.id.clone(),
                        source: Some(pkg.backend.clone()),
                        reason: pkg.pin.clone(),
                    },
                )?;
            }
        }
    }
    Ok(())
}
