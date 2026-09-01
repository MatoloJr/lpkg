# lpkg

A winget-like package orchestrator for Ubuntu and Debian-based Linux distributions.

`lpkg` unifies install, upgrade, inventory, cleanup, pinning, and export across multiple package managers:

- **apt** — system packages
- **snap** — Ubuntu snaps
- **flatpak** — sandboxed apps (Flathub)
- **brew** — Homebrew on Linux
- **pipx** — isolated Python CLIs
- **cargo** — Rust binaries
- **appimage** — portable AppImages (discovery)
- **direct_deb** — vendor `.deb` downloads

## Install

```bash
cargo build --release
sudo cp target/release/lpkg /usr/local/bin/
```

## Usage

```bash
# Search across all backends
lpkg search firefox

# Install (auto-picks best backend by priority)
lpkg install firefox
lpkg install code --source apt

# List installed apps
lpkg list
lpkg list --outdated
lpkg list --duplicates
lpkg list --format json

# Upgrade
lpkg upgrade --all
lpkg upgrade firefox
lpkg upgrade --dry-run

# Uninstall
lpkg uninstall firefox --purge

# Cleanup old versions
lpkg cleanup
lpkg cleanup --source snap

# Pin packages
lpkg pin add firefox --reason "distro version"
lpkg pin list

# Export / import manifest
lpkg export -o packages.json
lpkg import packages.json --dry-run

# Maintenance
lpkg scan
lpkg doctor

# Shell completions
lpkg completions bash > /etc/bash_completion.d/lpkg
```

## Configuration

Config file: `~/.config/lpkg/config.toml`

```toml
backend_priority = ["flatpak", "apt", "snap", "brew", "pipx", "cargo", "direct_deb", "appimage"]

[backends]
apt = true
snap = true
flatpak = true
brew = true
pipx = true
cargo = true
appimage = true
direct_deb = true
```

Aliases for cross-backend identity: `data/aliases.toml` (override at `~/.config/lpkg/aliases.toml`).

## Registry

Install provenance is tracked in `~/.local/share/lpkg/registry.db` (SQLite).

## License

MIT
