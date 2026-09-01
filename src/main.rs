use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use clap::CommandFactory;
use clap_complete::generate;
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use lpkg::backends::all_backends;
use lpkg::backends;
use lpkg::cli::{Cli, Commands, OutputFormat, PinCommands};
use lpkg::config::{ensure_dirs, load_config, is_backend_enabled, enabled_backends, Config};
use lpkg::core::orchestrator::UpgradeOrchestrator;
use lpkg::core::registry::InstallRegistry;
use lpkg::core::resolver::PackageResolver;
use lpkg::models::{ExportManifest, ManifestPackage, RegistryEntry};
use lpkg::output::print_candidates;
use std::fs;
use std::io;
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
        Commands::Search { query, source } => cmd_search(&backends, &config, &query, source.as_deref()),
        Commands::Install { id, source, version } => {
            cmd_install(&backends, &config, &id, source.as_deref(), version.as_deref())
        }
        Commands::List {
            outdated,
            duplicates,
            all,
            format,
            source: _,
        } => cmd_list(&backends, &config, outdated, duplicates, !all, format),
        Commands::Upgrade { id, all, dry_run, source } => {
            cmd_upgrade(&backends, &config, id.as_deref(), all, dry_run, source.as_deref())
        }
        Commands::Uninstall { id, purge, source } => {
            cmd_uninstall(&backends, &config, &id, purge, source.as_deref())
        }
        Commands::Cleanup { source } => cmd_cleanup(&backends, &config, source.as_deref()),
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
    query: &str,
    sources: Option<&[String]>,
) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;
    let results = resolver.search(query, sources)?;
    if results.is_empty() {
        println!("No packages found for '{query}'");
    } else {
        print_candidates(&results);
    }
    Ok(())
}

fn cmd_install(
    backends: &[Box<dyn backends::Backend>],
    config: &Config,
    id: &str,
    source: Option<&str>,
    version: Option<&str>,
) -> Result<()> {
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
    dry_run: bool,
    source: Option<&str>,
) -> Result<()> {
    let orchestrator = UpgradeOrchestrator::new(backends, config);

    if all {
        println!("Upgrading all packages...");
        let results = orchestrator.upgrade_all(dry_run)?;
        for (backend, ok) in results {
            let status = if ok { "ok" } else { "failed" };
            println!("  {backend}: {status}");
        }
        return Ok(());
    }

    let id = id.ok_or_else(|| anyhow::anyhow!("specify a package id or use --all"))?;
    let resolver = PackageResolver::new(backends, config)?;

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

fn cmd_scan(backends: &[Box<dyn backends::Backend>], config: &Config) -> Result<()> {
    let resolver = PackageResolver::new(backends, config)?;
    let registry = InstallRegistry::open()?;
    let packages = resolver.list_installed(config.apps_only)?;
    let data = serde_json::to_string(&packages)?;
    registry.cache_scan("all", &data)?;
    println!("Scanned {} packages", packages.len());

    // Report per-backend status
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

    // Check backends
    println!("Backends:");
    for backend in backends.iter() {
        let status = if backend.available() { "available" } else { "missing" };
        println!("  {}: {status}", backend.id());
        if !backend.available() && is_backend_enabled(config, backend.id()) {
            issues += 1;
        }
    }

    // Check dpkg lock
    if std::path::Path::new("/var/lib/dpkg/lock-frontend").exists() {
        println!("\nWarning: dpkg lock file exists (another package manager may be running)");
        issues += 1;
    }

    // Check duplicates
    let resolver = PackageResolver::new(backends, config)?;
    let dups = resolver.find_duplicates(true)?;
    if !dups.is_empty() {
        println!("\nDuplicate installations ({}):", dups.len());
        for (canonical, pkgs) in &dups {
            println!("  {canonical}: {} copies", pkgs.len());
            issues += 1;
        }
    }

    // Check registry
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
    let content = fs::read_to_string(file)?;
    let manifest: ExportManifest = serde_json::from_str(&content)?;

    for pkg in &manifest.packages {
        if dry_run {
            println!("would install {} via {} ({})", pkg.id, pkg.backend, pkg.backend_id);
        } else {
            cmd_install(
                backends,
                config,
                &pkg.id,
                Some(&pkg.backend),
                pkg.version.as_deref(),
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
