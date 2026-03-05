# Repack2Linux (R2L)

Portable game factory for Linux: build once, launch anywhere.

<p>
  <img alt="Version" src="https://img.shields.io/badge/version-v1.01-1f6feb">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Linux-0a0a0a">
  <img alt="Status" src="https://img.shields.io/badge/status-stable-1f883d">
  <img alt="Engine" src="https://img.shields.io/badge/engine-Wine%20%2B%20Proton-8b5cf6">
</p>

## What Is R2L?
Repack2Linux is a desktop app that converts Windows game sources into self-contained Linux-ready packages.  
It builds/learns prefix configuration, exports portable artifacts, and ships auto-recovery launchers (`play_auto.sh` + safe fallback).

## Why Use It?
- Zero-manual pipeline from source to playable package.
- Portable-first architecture with local data isolation.
- Auto-learning profiles shared across runs (and optional community sync).
- Reliable launch flow for non-technical users.

## Screenshots
> Add screenshots in `docs/assets/` and keep names below for clean GitHub rendering.

| Factory | Export | Installer |
|---|---|---|
| `docs/assets/factory.png` | `docs/assets/export.png` | `docs/assets/installer.png` |

## Core Features
| Feature | What it gives you |
|---|---|
| Intelligent prefix scanning | Reuses and scores existing Wine/Lutris/Steam prefixes before creating new ones |
| Learned profiles | Automatically stores successful requirements/proton/prefix hints per game |
| Portable export | Produces runnable package with `play.sh`, `play_auto.sh`, `play_safe.sh` |
| Safe fallback | Auto-switches to safe mode when renderer/runtime issues are detected |
| Isolated runtime | Save/runtime data stays inside package (`./pfx`, `./r2p_userdata`) |
| Unified `.sh` installer | Optional self-extracting installer with desktop/menu integration |

## Launch Behavior (User-Friendly)
- `play_auto.sh` is the recommended entrypoint.
- If normal launch fails, auto mode retries with safe mode.
- System checks explain missing dependencies instead of failing silently.

## Project Structure
```text
src/
  main.rs                # App shell, UI state, orchestration
  engine.rs              # Production pipeline
  installer.rs           # Export, launchers, installer generation
  database.rs            # SQLite + learned profiles JSON
  community_sync.rs      # Optional GitHub sync + retry queue
  ui/                    # Iced UI tabs and theme
cloud/
  games.sample.json      # Cloud-style game knowledge sample
presets.json             # Preset knowledge base
```

## Quick Start
```bash
git clone https://github.com/KrystianG06/Repack2Linux.git
cd Repack2Linux
cargo run --bin repack2proton-rs
```

## Build Release Asset (for end users)
```bash
chmod +x build_release.sh
./build_release.sh
```

This creates:
- `dist/Repack2Linux-v1.01-<target>.tar.gz`
- `dist/Repack2Linux-v1.01-<target>.sha256`

After extracting the archive, install app icon/menu entry:
```bash
cd Repack2Linux-<target>
./install_desktop_icon.sh
```

## Build AppImage (single-file distribution)
```bash
chmod +x build_appimage.sh
./build_appimage.sh
```

This creates:
- `dist/Repack2Linux-v1.01-<arch>.AppImage`
- `dist/Repack2Linux-v1.01-<arch>.AppImage.sha256`

> Note: In AppImage/runtime mode, `Unified SFX Installer (.sh)` export is intentionally disabled.
> Use `Portable` export in AppImage, or run developer build from repository to generate SFX installer.

## Typical Workflow
1. Select game source folder/ISO.
2. Let R2L detect and apply best profile/preset.
3. Run production (test launch from project workspace).
4. Export full portable package (or unified installer).
5. Distribute and run with `play_auto.sh`.

## Pipeline (At a Glance)
```text
Source -> Detect -> Learn/Preset Match -> Prefix Build/Reuse
       -> Test Launch -> Export (Portable / .sh)
       -> Auto Launcher (play_auto.sh + safe fallback)
```

## Community Sync (Optional)
R2L can push learned updates to GitHub presets.

Prefer new env names (legacy fallback is supported):
- `R2L_GITHUB_TOKEN`
- `R2L_GITHUB_REPO`
- `R2L_GITHUB_BRANCH`
- `R2L_COMMUNITY_DB_ROOT`

## Documentation
- Progress log: [`PROGRESS.md`](./PROGRESS.md)
- Full project overview: [`PROJECT_OVERVIEW.md`](./PROJECT_OVERVIEW.md)
- Landing page copy: [`LANDING_COPY.md`](./LANDING_COPY.md)

## Roadmap (Short)
- Delta-sync for large game updates.
- Steam Deck focused presets.
- Public release cleanup (repo rename, screenshots, final docs pass).

## FAQ
**Where are save files?**  
Inside the package: `./r2p_userdata` (typically `./r2p_userdata/Local`).

**Which launcher should users run?**  
`./play_auto.sh` (recommended).

**What if a game has black screen?**  
Auto mode retries safe fallback automatically; manual fallback is `./play_safe.sh`.

**Does R2L require cloud account/Steam login to work?**  
No. Core workflow is local/offline-first.

## License
Currently project-internal. Public license selection planned before release.
