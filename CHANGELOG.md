# Changelog

All notable changes to FrutigerOS are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added — Phase 1: Foundation ✅
- Project repository structure (all component directories)
- README, LICENSE (MIT), CONTRIBUTING, ROADMAP, CHANGELOG
- Code of Conduct and Security policy
- GitHub issue templates (Bug Report, Feature Request)
- GitHub PR template
- GitHub Actions CI workflow for AeroWM (fmt + clippy + build + test + audit)
- Design tokens file (colors, typography, spacing, animation)
- archiso base configuration README for FrutigerOS ISO

### Added — Phase 2: AeroWM Core ✅
- AeroWM Rust compositor (Smithay 0.3, Cargo.toml)
- `config.rs` — full TOML config with serde, load/save, unit tests
- `workspace.rs` — WorkspaceManager, window focus cycling, layout enum
- `input.rs` — key→action mapping (Super+keys, Alt+Tab, workspace switch)
- `render.rs` — Color, GlassMaterial, Shadow, spring animation curve
- `compositor.rs` — Smithay winit backend, Wayland display socket, XDG shell,
  SHM buffer import, 60fps render loop, keyboard input dispatch
- **15 unit tests passing**
- AeroWM binary builds cleanly with `cargo build`

### Added — Phase 3: AeroShell ✅
- AeroShell Rust project (wayland-client, tokio, serde, chrono)
- `config.rs` — panel, launcher, notification configuration
- `panel.rs` — layout engine (panel items, geometry, screen-width layout)
- `launcher.rs` — XDG .desktop discovery, search, exec cleanup, launch
- `notifications.rs` — notification queue, urgency, tick/expiry, max_visible
- `widgets.rs` — AeroGlass widget primitives (Color, GlassSurface, ButtonStyle, TextStyle)
- **15 unit tests passing**
- AeroShell binary builds cleanly

### Added — Phase 4: Visual Identity 🔨 (shaders + animation system)
- `aerowm/src/shaders/glass.vert` — Aero Glass vertex shader
- `aerowm/src/shaders/glass.frag` — Glass fragment shader (9-tap blur, rounded corners, gloss, border)
- `aerowm/src/shaders/shadow.frag` — Drop shadow fragment shader
- `aerowm/src/effects/glass.rs` — GlassEffect + GlassParams (active/inactive, time-driven)
- `aerowm/src/effects/shadow.rs` — ShadowEffect + ShadowParams
- `aerowm/src/effects/animation.rs` — WindowAnimation with spring easing, open/close phases
- `aerowm/assets/wallpapers/manifest.toml` — 5 original wallpaper descriptions (art pending)
- **8 new unit tests in AeroWM (23 total, all passing)**
- Shader sources are compiled-in and unit-tested, but not yet wired into the
  Smithay render path (blur/rounded corners are still pending)

### Added — AeroPanel ↔ AeroWM integration
- `aeroshell/src/panel.rs` now creates a real Wayland surface: binds
  `wl_compositor`, `wl_shm` and `xdg_wm_base`, creates an xdg toplevel with
  `app_id = "frutigeros-aeropanel"`, acks configures, and presents
  ARGB8888 SHM buffers with release tracking
- `aeroshell/src/glyphs.rs` — dependency-free 5×7 bitmap font for panel text
- `aeroshell/src/panel_render.rs` — deep-ocean glass panel rendering via
  `tiny-skia` (gradient, start orb, workspace dots, clock/date, tray, quick settings)
- AeroWM detects the panel by `app_id`, pins it full-width at the bottom,
  excludes it from workspace/focus handling and draws it above normal windows
- AeroWM now forwards pointer motion/buttons, does click-to-focus hit-testing,
  tracks `ToplevelSurface` metadata, removes dead/unmapped surfaces, manages
  keyboard focus, and implements `Super+W` close and fullscreen toggling
- `AppearanceConfig.panel_height` (default 48) added to AeroWM config
- **AeroShell now has 23 unit tests (8 new), all passing**

### Added — Phase 5: Installer & ISO ✅
- `aero-installer/` — full Rust project (tokio + slog + serde)
- `steps/mod.rs` — 11-step installation state machine with forward/back transitions
- `steps/config.rs` — InstallConfig (base packages, target mount)
- `disk/mod.rs` — DiskInfo, Filesystem, PartitionPlan, auto_partition_plan()
  - probe_disks() via lsblk (read-only)
  - apply_partition_plan() gated behind confirmed=true (DESTRUCTIVE, not wired yet)
- `ui/mod.rs` — CLI progress display (GTK4 UI is Phase 7)
- **10 unit tests passing**
- `frutigeros-iso/profiledef.sh` — archiso profile definition
- `frutigeros-iso/packages.x86_64` — full package list (base + desktop + accessibility)
- `frutigeros-iso/airootfs/etc/hostname` — "frutigeros"
- `frutigeros-iso/airootfs/usr/share/aerowm/config.toml` — default AeroWM config for live env
- `frutigeros-iso/efiboot/loader/entries/frutigeros.conf` — UEFI boot entry

### Added — Phase 6: Native Applications ✅
- `aero-apps/` — Cargo workspace (9 members)
- **AeroFiles**: directory listing, MIME detection, human-readable sizes,
  safe copy/move/delete, mkdir, and bounded recursive search — 10 tests
- **AeroMonitor**: /proc/stat CPU, /proc/meminfo (MemAvailable with
  free+buffers+cached fallback), df disk usage — 1 live test
- **AeroNotes**: Markdown front-matter store with create/show/search/pin/delete
  CLI, pinned-first ordering — 5 tests
- **AeroTerminal**: PTY config, CSI parser (cursor movement, SGR colour,
  erase, unknown-sequence passthrough), font/glass config — 10 tests
- **AeroSettings**: 12 settings pages with icons/descriptions, no duplicate IDs — 2 tests
- **AeroSoftware**: pacman package query, search, update count, and
  install/remove gated behind `--yes` — 2 tests
- **AeroWelcome**: 6-step first-boot wizard, done-file tracking — 2 tests
- **AeroScreenshot**: grim/slurp/wl-copy/notify-send capture pipeline — 6 tests
- **AeroMusic**: recursive hidden-dir-skipping scan, search, and a playback
  queue with repeat (none/one/all) and shuffle — 13 tests
- **Total app tests: 51 (all passing)**

- `frutigeros-website/index.html` — full single-page site
- `frutigeros-website/src/style.css` — Frutiger Aero design system in CSS
- `frutigeros-website/src/app.js` — live clock, scroll fade, smooth scroll
- Sections: hero with desktop mockup, features, AeroWM, roadmap, download
- Responsive (1-3 column), reduced-motion aware, accessible markup

### Added — Phase 5b: ISO profile and installer backend
- `frutigeros-iso/pacman.conf` — build-time repository configuration, with a
  commented `[frutigeros]` local repository section
- `frutigeros-iso/build.sh` — profile validation (`--check`), dependency and
  root checks, then `mkarchiso`
- Live environment files: `os-release`, `issue`, `locale.gen`, `locale.conf`,
  `vconsole.conf`, NetworkManager and logind drop-ins
- Autologin on tty1, `frutigeros-live-setup` oneshot unit, and
  `frutigeros-session` (starts AeroWM + AeroShell when a host display exists)
- systemd-boot UEFI entries and syslinux BIOS entries using
  `%ARCHISO_LABEL%`; microcode packages added
- `aero-installer/src/install/` — pure command planner for
  sgdisk/mkfs/mount/pacstrap/arch-chroot/GRUB, sanitised user/host/locale/
  timezone inputs, and an executor that defaults to dry-run
- Installer CLI flags `--dry-run` (default), `--apply --yes`, `--help`
- `aero-installer` tests: **18 passing** (8 new)

### Changed — quality and tooling
- All Rust crates (`aerowm`, `aeroshell`, `aero-installer`, `aero-apps`) now pass
  `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
- `.github/workflows/rust-ci.yml` adds fmt/clippy/test jobs for AeroShell,
  AeroInstaller (including a dry-run smoke test), and the Aero Apps workspace
- AeroWM: `--save-config` writes the loaded/default config; workspace switches
  are logged with id, name, and layout; removed a duplicate unused input
  dispatcher; effects/render primitives are explicitly marked as Phase 4b
  wiring rather than silently dead
- AeroShell: `--save-config`; config `Default` derivation; clippy style fixes
  in the launcher and panel renderer
- Native apps gained CLI front ends:
  `aero-notes` (new/show/search/pin/delete), `aero-music`
  (search/play/pause/stop/next/prev/repeat/shuffle), `aero-software`
  (search/install/remove with `--yes`), plus richer `aero-settings` and
  `aero-terminal` startup output
- AeroMusic `RepeatMode::One` now actually repeats the current track

### Added — Phase 7: documentation portal
- `frutigeros-docs/` — static documentation site (9 pages) with responsive
  layout, dark/light themes, navigation filter, copy buttons and scroll reveals
- Original SVG assets: logo, banner and component-map diagram
- Guides for getting started, installation, AeroWM, AeroShell, applications,
  architecture, troubleshooting and contributing
- README banner corrected to point at the new SVG asset

### Not Yet Implemented
- Aero Glass shader / blur wired into AeroWM's render path (Phase 4)
- Interactive move/resize (Phase 4)
- AeroStart popup surface and notification toasts (Phase 4)
- Panel ↔ compositor IPC (Phase 4)
- archiso ISO build and QEMU validation (Phase 5)
- Real partitioning/bootloader execution in the installer (Phase 5)
- Interactive UIs for the native applications (Phase 6)
- Audio playback backend for AeroMusic (Phase 6/7)
