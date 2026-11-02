# AeroWM

The Wayland compositor for FrutigerOS. Written in Rust using [Smithay](https://github.com/Smithay/smithay).

## Current Status: Phase 1 (Foundation)

> ⚠️ **Not yet runnable.** The compositor skeleton, module structure, config system,
> workspace manager, input handler, and render types are implemented.
> Smithay backend wiring (actual window rendering) is Phase 2.

### What's implemented

| Module | Status | Description |
|--------|--------|-------------|
| `config.rs` | ✅ Done | TOML config with serde, defaults, load/save, unit tests |
| `workspace.rs` | ✅ Done | WorkspaceManager, window focus cycling, layout enum, unit tests |
| `input.rs` | ✅ Done | Key binding handler, action dispatch, unit tests |
| `render.rs` | ✅ Done | Color, GlassMaterial, Shadow, animation types + spring easing, unit tests |
| `compositor.rs` | 🔨 Stub | Wires modules together; Smithay event loop is Phase 2 |

### What Phase 2 will add

- Smithay CalloopEventLoop
- `winit` backend (for testing in a window)
- `udev` backend (for real hardware via DRM/KMS)
- Actual surface rendering with Aero Glass shader
- XWayland support

## Building

Requires Rust 1.75+ and Wayland development libraries.

```bash
# Arch Linux
sudo pacman -S rust wayland libxkbcommon systemd libinput libdrm libseat

# Ubuntu 24.04
sudo apt-get install rustup libwayland-dev libxkbcommon-dev libudev-dev \
  libinput-dev libgbm-dev libdrm-dev libseat-dev

cargo build
cargo test
```

## Default Keybindings

| Key | Action |
|-----|--------|
| Super+Return | Open terminal |
| Super+Space | Open launcher |
| Super+W | Close window |
| Super+F | Toggle fullscreen |
| Super+V | Toggle floating/tiling |
| Super+1–9 | Switch to workspace N |
| Alt+Tab | Focus next window |
| Alt+Shift+Tab | Focus previous window |
| Super+Shift+Q | Quit AeroWM |

## Configuration

`~/.config/aerowm/config.toml`:

```toml
[general]
backend = "udev"    # or "winit" for testing
xwayland = true

[appearance]
corner_radius = 12
blur_strength = 20
glass_opacity = 0.82
border_active = "#00AEEFcc"
animation_speed = 1.0

[workspaces]
count = 4
default_layout = "floating"   # "floating", "tiling", "hybrid"

[keybindings]
modifier = "super"
```
