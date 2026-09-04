---
name: lpkg docs index.html
overview: Add a single self-contained `index.html` in the repo root as a polished docs landing page covering purpose, install, customization, commands, and how to uninstall lpkg itself (vs packages).
todos:
  - id: write-index-html
    content: "Create root index.html with embedded CSS/JS: hero, purpose, install, customize, commands, uninstall tool"
    status: pending
  - id: readme-link
    content: Link index.html from README.md as web docs entry point
    status: pending
isProject: false
---

# lpkg documentation index.html

## Deliverable

Create one file: [`index.html`](index.html) at the repo root with **embedded CSS and JS** (no external frameworks). Link it from [`README.md`](README.md) under Install/Docs as “Web docs: open `index.html` or view on GitHub Pages if enabled.”

## Design direction (concrete)

Avoid default AI looks (no purple gradients, no cream+serif terracotta, no broadsheet). Direction:

- **Brand:** deep ink background (`#0b1220`) with a soft teal→cyan accent (`#2dd4bf` / `#67e8f9`) and warm off-white text
- **Type:** Google Fonts via `<link>` — **Syne** (display/brand) + **IBM Plex Sans** (body) + **IBM Plex Mono** (commands)
- **Hero (first viewport):** full-bleed atmospheric gradient/mesh background; large **lpkg** wordmark; one headline (“One CLI for every Linux package source”); one short sentence; one CTA group (`Install`, `Commands`, `Uninstall tool`) — no cards in the hero, no stat strips
- **Motion:** 2–3 intentional effects — hero fade/slide-in, sticky nav link underline, copy-button flash / section reveal on scroll (`IntersectionObserver`)
- **Layout:** single-column readable max-width (~720–780px) for prose; command blocks as mono code with a one-click **Copy** button (JS)
- **Responsive:** stacks cleanly on mobile; nav collapses to horizontal scroll chips

## Page sections (one job each)

1. **Hero** — what it is: winget-like orchestrator for Ubuntu/Debian across apt, snap, flatpak, brew, pipx, cargo, appimage, direct_deb
2. **Why** — problem/solution in 2–3 short paragraphs (fragmented package managers → one search/install/upgrade surface + registry)
3. **Install** — one-liner, clone+`make install`, `.deb`, `cargo install` (from README)
4. **Customize** — dedicated section elaborating:
   - `~/.config/lpkg/config.toml`: `backend_priority`, per-backend toggles, `apps_only`, `appimage_paths`, `cache_ttl_hours`
   - User aliases override: `~/.config/lpkg/aliases.toml` over system `/usr/local/share/lpkg/aliases.toml`
   - Install prefix: `PREFIX` / `BINDIR` / `DATADIR` for `make install`
   - Manifest workflow: `export` / `import` / `install --all`
5. **Commands** — compact reference of the CLI (search/install/`--auto`/upgrade/list/info/which/…); copyable examples
6. **Uninstall packages vs uninstall lpkg** — clear split:
   - **Remove a managed package:** `lpkg uninstall <id>` (optional `--purge`, `--source`)
   - **Remove the lpkg tool itself** (elaborate):
     - From clone/`make install`: `sudo make uninstall` (removes `$PREFIX/bin/lpkg` and `$PREFIX/share/lpkg`; default `PREFIX=/usr/local`)
     - From `.deb`: `sudo dpkg -r lpkg` (or `sudo apt remove lpkg`)
     - From cargo: `cargo uninstall lpkg`
     - From one-liner/manual binary: `sudo rm -f /usr/local/bin/lpkg && sudo rm -rf /usr/local/share/lpkg`
     - Note that config/registry under `~/.config/lpkg` and `~/.local/share/lpkg` are **not** deleted by `make uninstall`; optional cleanup commands documented
7. **Footer** — MIT, GitHub link `https://github.com/MatoloJr/lpkg`, version note `0.2.0`

## Implementation notes

- Single HTML file; CSS in `<style>`, JS in `<script>` at bottom
- Fonts: one `<link rel="preconnect">` + stylesheet for Google Fonts (acceptable for a docs page; no other CDN UI kits)
- Accessible: skip link, focus-visible styles, semantic landmarks (`header`/`nav`/`main`/`section`/`footer`), sufficient contrast
- Copy buttons use `navigator.clipboard` with fallback
- Do **not** edit the plan file; do **not** change CLI code unless needed (none expected)

## README touch

Add a short line near the top or Install section pointing to `index.html` for the visual docs.