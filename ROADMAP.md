# FrutigerOS Roadmap

> Status key: ✅ Done · 🔨 In Progress · 📋 Planned · ❌ Blocked

---

## Phase 1 — Foundation ✅

- [x] Repository structure
- [x] README, LICENSE, CONTRIBUTING
- [x] Code of Conduct, Security policy
- [x] GitHub issue/PR templates
- [x] GitHub Actions CI workflow
- [x] AeroWM Rust project skeleton
- [x] Design tokens (TOML)

---

## Phase 2 — AeroWM Core ✅

- [x] Smithay 0.3 integration (winit backend)
- [x] Wayland display socket
- [x] XDG shell (NewToplevel → map window)
- [x] SHM buffer import and rendering
- [x] Keyboard input → action dispatch
- [x] Workspace switching (Super+1–9)
- [x] Window focus cycling (Alt+Tab)
- [x] Floating window mode
- [x] Config file (TOML, load/save)
- [x] Pointer hit-testing, click-to-focus and pointer event forwarding
- [x] 23 unit tests
- [ ] Window move / resize (interactive drag) — Phase 4
- [ ] Rounded corners (shader) — Phase 4
- [ ] Blur / glass shader pass — Phase 4
- [ ] udev/DRM backend (real hardware) — Phase 4
- [ ] XWayland support — Phase 5

---

## Phase 3 — AeroShell ✅

- [x] Panel layout engine (items, geometry, screen-width layout)
- [x] AeroStart: XDG .desktop file discovery + search + launch
- [x] Notification manager (queue, urgency, expiry, max_visible)
- [x] AeroGlass widget system (Color, GlassSurface, ButtonStyle, TextStyle)
- [x] Shell configuration (TOML)
- [x] Real Wayland panel surface (xdg toplevel, `frutigeros-aeropanel`)
- [x] Panel pixel rendering with tiny-skia + built-in bitmap font
- [x] 23 unit tests
- [ ] AeroStart popup surface — Phase 4
- [ ] Notification toast surfaces — Phase 4
- [ ] D-Bus notification listener — Phase 5
- [ ] Panel ↔ compositor IPC (workspace state, clock) — Phase 4

---

## Phase 4 — Visual Identity 🔨 (next)

- [ ] Aero Glass blur effect (GLSL shader in AeroWM)
- [ ] Translucent window surfaces
- [ ] Gradient window borders
- [ ] Window shadow pre-pass
- [ ] Smooth open/close animations (spring curve applied)
- [x] Panel surface with tiny-skia rendering (xdg toplevel pinned by AeroWM)
- [ ] AeroStart launcher popup
- [ ] Notification toasts
- [ ] Nature-inspired wallpaper set (original SVG art)
- [ ] Icon theme (original)
- [ ] Rounded corners via clip shader
- [ ] Dark Aero mode
- [ ] Light mode
- [ ] Pointer hit-testing + interactive move/resize (hit-testing done; drag pending)

---

## Phase 5 — Installer and ISO 🔨

- [x] archiso profile definition (`profiledef.sh`, `pacman.conf`, package list, BIOS/UEFI boot entries, `build.sh --check` passes)
- [ ] Live desktop session (autologin, session script, service units added; not boot-tested)
- [x] Installer wizard state machine (CLI/dry-run, 10 tests)
- [ ] Disk partitioning execution (Btrfs / ext4) — planning only, not wired
- [ ] UEFI bootloader setup
- [ ] User creation flow (state machine only)
- [ ] Timezone / locale selection (state machine only)
- [ ] Post-install first boot experience
- [ ] Virtual machine testing (QEMU)

---

## Phase 6 — Native Applications 🔨

Backends and CLI front ends exist for all nine apps (**51 tests passing**).
Interactive Aero Glass UIs are still being built, so the apps are not yet marked
complete.

- [ ] AeroFiles — list, recursive search, mkdir, copy, move, delete (10 tests)
- [ ] AeroTerminal — PTY config, CSI parser (cursor/colour/erase) (10 tests)
- [ ] AeroSettings — 12 settings sections (2 tests)
- [ ] AeroSoftware — pacman query/search/install/remove with `--yes` guard (2 tests)
- [ ] AeroMonitor — CPU, memory (MemAvailable fallback), disk stats (1 test)
- [ ] AeroWelcome — 6-step first-boot wizard (2 tests)
- [ ] AeroScreenshot — grim/slurp/wl-copy/notify-send capture (6 tests)
- [ ] AeroMusic — recursive scan, search, queue/repeat/shuffle (13 tests)
- [ ] AeroNotes — Markdown store, create/show/search/pin/delete (5 tests)

---

## Phase 7 — Website and Documentation 🔨

- [x] Official website (HTML + CSS + JS, Frutiger Aero design)
- [x] Features section, AeroWM section, roadmap, download page
- [x] Documentation portal (static HTML/CSS/JS, 9 pages + original SVG art)
- [ ] Screenshots / gallery
- [ ] Download page (real checksums, real ISO)
- [x] Online contributing guide (documentation portal page)

---

## Phase 8 — Public Alpha

- [ ] ISO tested on real hardware
- [ ] ISO tested in VMs (QEMU, VirtualBox, VMware)
- [ ] Community testing period
- [ ] Bug tracker open
- [ ] First public announcement

---

## Not Planned (explicitly out of scope for v1)

- Android app support
- Proprietary software store
- Paid features
