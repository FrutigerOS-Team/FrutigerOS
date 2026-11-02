# FrutigerOS

> **Tomorrow, Reimagined.**

An Arch Linux-based desktop operating system inspired by the Frutiger Aero aesthetic — the future that people imagined in 2008.

![FrutigerOS Banner](frutigeros-docs/assets/banner.svg)

## What is FrutigerOS?

FrutigerOS is an open-source Linux distribution combining:

- **Arch Linux** as the base — rolling release, full control
- **AeroWM** — a custom Wayland compositor written in Rust
- **AeroShell** — a complete desktop shell with Aero Glass effects
- **Frutiger Aero design system** — glassmorphism, gradients, nature visuals

This is not a themed desktop. FrutigerOS has its own identity.

## Current Status

> ⚠️ **Pre-Alpha** — Foundation and desktop components are in active development.
> No boot-tested ISO exists yet. Contributions welcome.

| Component       | Status                              |
|-----------------|-------------------------------------|
| AeroWM          | 🔨 In progress (23 tests)            |
| AeroShell       | 🔨 In progress (23 tests, real panel)|
| AeroInstaller   | 🔨 In progress (18 tests, dry-run)   |
| Aero Apps       | 🔨 In progress (51 tests, backends)  |
| ISO Build       | 🔨 Profile validates, not booted     |
| Website         | ✅ Static site                       |
| Documentation   | ✅ Static portal                      |

## Repository Structure

```
frutigeros/
├── aerowm/          # Wayland compositor (Rust + Smithay)
├── aeroshell/       # Desktop shell
├── aero-installer/  # Graphical installer
├── aero-apps/       # Native applications
├── frutigeros-iso/  # archiso configuration
├── frutigeros-website/ # Official website
└── frutigeros-docs/ # Documentation
```

## Development Roadmap

- **Phase 1** ✅ Project foundation, structure, docs
- **Phase 2** ✅ AeroWM — basic Wayland compositor
- **Phase 3** ✅ AeroShell — panel, launcher, notifications
- **Phase 4** 🔨 Visual identity — Aero Glass, animations, themes
- **Phase 5** 🔨 Installer and ISO
- **Phase 6** 🔨 Native applications
- **Phase 7** 🔨 Website and documentation (site + docs portal exist)
- **Phase 8** 📋 Public alpha

## Documentation

Open [`frutigeros-docs/index.html`](frutigeros-docs/index.html) directly, or serve it:

```bash
cd frutigeros-docs
python3 -m http.server 8080
```

## Building AeroWM

Requirements:
- Rust (stable, 1.75+)
- Wayland development libraries
- `libwayland-dev`, `libxkbcommon-dev`, `libudev-dev`

```bash
cd aerowm
cargo build

cd ../aeroshell
cargo build
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributors must follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

MIT License — see [LICENSE](LICENSE).

## Design Philosophy

> *"What if the future imagined in 2008 actually happened?"*

FrutigerOS takes the glossy, nature-inspired, optimistic UI language of the late 2000s and builds it properly on a modern Linux foundation. No compromises on aesthetics. No compromises on power.
