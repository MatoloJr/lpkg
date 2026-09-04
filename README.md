# lpkg

A winget-like package orchestrator for Ubuntu and Debian-based Linux distributions.

`lpkg` unifies install, upgrade, inventory, cleanup, pinning and export across multiple package managers:

- **apt** — system packages
- **snap** — Ubuntu snaps
- **flatpak** — sandboxed apps (Flathub)
- **brew** — Homebrew on Linux
- **pipx** — isolated Python CLIs
- **cargo** — Rust binaries
- **appimage** — portable AppImages (discovery)
- **direct_deb** — vendor `.deb` downloads

## Install

### One-liner (release binary, falls back to source build)

```bash
curl -fsSL https://raw.githubusercontent.com/MatoloJr/lpkg/main/scripts/install.sh | bash
```

### Clone and install

```bash
git clone https://github.com/MatoloJr/lpkg.git
cd lpkg
sudo make install
```

Uses `PREFIX=/usr/local` by default (`BINDIR`, `DATADIR` overridable).

### Debian package

```bash
# from a release asset, or build locally:
make deb
sudo dpkg -i dist/lpkg_0.2.0_amd64.deb
```

### Cargo

```bash
cargo install --git https://github.com/MatoloJr/lpkg.git
```

After install, verify with `lpkg --version`.

## Commands

| Command | What it does |
|---|---|
| `lpkg search <query…>` | Search **all** enabled backends for each query term; merge and dedupe results |
| `lpkg install <id…>` | Install each package via the best backend (`backend_priority`), or `--source` |
| `lpkg install --all [file]` | Install every entry from a JSON manifest (default: `packages.json`) |
| `lpkg install … --auto` / `-a` | If already installed, remove the old copy first, then install |
| `lpkg list` | List installed packages (apps by default) |
| `lpkg list --outdated` | Show packages with available updates |
| `lpkg list --duplicates` | Show packages installed via more than one backend |
| `lpkg list --all` | Include non-app packages where backends support it |
| `lpkg list --format json` | Machine-readable inventory |
| `lpkg upgrade` | Upgrade all **outdated** packages |
| `lpkg upgrade --auto` / `-a` | Same as bare `upgrade` (outdated only) |
| `lpkg upgrade --all` | Run each backend’s full `upgrade_all` |
| `lpkg upgrade <id>` | Upgrade one installed package |
| `lpkg upgrade <id> --auto` | Upgrade only if outdated; no-op if current |
| `lpkg upgrade … --dry-run` | Print planned upgrades without applying them |
| `lpkg uninstall <id>` | Remove a package (`--purge` where supported, `--source` to pick backend) |
| `lpkg cleanup` | Clean old revisions / unused data across backends |
| `lpkg info <id>` | Show installed copies and search candidates for a package |
| `lpkg which <id>` | Print which backend(s) currently provide an installed package |
| `lpkg scan` | Refresh inventory cache |
| `lpkg doctor` | Health checks for backends, locks, and duplicates |
| `lpkg pin add\|remove\|list` | Pin packages to skip upgrades |
| `lpkg export -o packages.json` | Export installed packages to a JSON manifest |
| `lpkg import packages.json` | Install from a manifest (`--dry-run` to preview) |
| `lpkg completions <shell>` | Generate shell completions |
| `lpkg --version` | Print lpkg version |

### Examples

```bash
# Search across all backends
lpkg search firefox
lpkg search firefox code

# Install (auto-picks best backend by priority)
lpkg install firefox
lpkg install firefox code
lpkg install code --source apt
lpkg install firefox --auto          # replace existing install

# Bulk install from manifest
lpkg export -o packages.json
lpkg install --all
lpkg install --all my-packages.json

# List
lpkg list
lpkg list --outdated
lpkg list --duplicates
lpkg list --format json

# Upgrade
lpkg upgrade                  # outdated only
lpkg upgrade --auto           # same
lpkg upgrade --all            # full backend upgrades
lpkg upgrade firefox --auto
lpkg upgrade --dry-run

# Inspect
lpkg info firefox
lpkg which firefox

# Uninstall / cleanup
lpkg uninstall firefox --purge
lpkg cleanup
lpkg cleanup --source snap

# Pin
lpkg pin add firefox --reason "distro version"
lpkg pin list

# Export / import
lpkg export -o packages.json
lpkg import packages.json --dry-run

# Maintenance
lpkg scan
lpkg doctor

# Shell completions
lpkg completions bash | sudo tee /etc/bash_completion.d/lpkg
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

Aliases for cross-backend identity: packaged under `/usr/local/share/lpkg/aliases.toml` (or `/usr/share/lpkg/`), overridable at `~/.config/lpkg/aliases.toml`.

## Registry

Install provenance is tracked in `~/.local/share/lpkg/registry.db` (SQLite).

## License

MIT
