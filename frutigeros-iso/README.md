# FrutigerOS ISO

Built with [archiso](https://wiki.archlinux.org/title/Archiso).

The profile structure is complete and statically validated, but a *desktop*
ISO still requires the FrutigerOS packages (`aerowm`, `aeroshell`,
`aero-installer`, wallpapers/themes) to be built and published to a
`[frutigeros]` repository first. Until then the live environment boots Arch
Linux with the FrutigerOS branding and configuration.

## Validate the profile

```bash
./build.sh --check
```

This checks for required files, shell syntax, and the `%ARCHISO_LABEL%`
placeholder used by the boot entries. It does not need root.

## Build the ISO

```bash
# Arch Linux only
sudo pacman -S archiso

# Validate, then build (mkarchiso needs root)
sudo ./build.sh

# Optional: choose output/work directories
sudo ./build.sh --out /var/tmp/frutigeros-out --work /var/tmp/frutigeros-work
```

The ISO is written to `./out/frutigeros-<date>-x86_64.iso`.

## Desktop packages

Once the Rust components are packaged:

1. Build `.pkg.tar.zst` packages for `aerowm`, `aeroshell`, `aero-installer`
   and the asset packages.
2. Serve them from a local repository, e.g. `repo/x86_64`.
3. Uncomment the `[frutigeros]` section in `pacman.conf` and adjust the
   `Server` path.
4. Uncomment the corresponding entries in `packages.x86_64`.
5. Re-run `sudo ./build.sh`.

## Directory structure

```
frutigeros-iso/
├── build.sh                 # validate/build entry point
├── profiledef.sh            # archiso profile definition
├── pacman.conf              # build-time pacman configuration
├── packages.x86_64          # package list for the live ISO
├── airootfs/                # files copied into the live root
│   ├── etc/
│   │   ├── hostname
│   │   ├── issue
│   │   ├── locale.gen / locale.conf / vconsole.conf
│   │   ├── os-release
│   │   ├── NetworkManager/conf.d/
│   │   ├── systemd/
│   │   │   ├── system/          # live setup unit + getty autologin
│   │   │   └── system/multi-user.target.wants/
│   │   └── usr/share/aerowm/config.toml
│   ├── root/.bash_profile   # starts the session on tty1
│   └── usr/local/bin/
│       ├── frutigeros-session
│       ├── frutigeros-live-setup
│       └── frutigeros-installer
├── efiboot/                 # systemd-boot UEFI entries
│   └── loader/{loader.conf,entries/frutigeros.conf}
└── syslinux/                # BIOS boot entries
```

## Live session behaviour

On `tty1` the root autologin runs `frutigeros-session`, which:

1. prepares `XDG_RUNTIME_DIR`,
2. starts `aerowm` (winit backend),
3. waits for `wayland-0`,
4. starts `aeroshell` and `aero-welcome` if installed.

AeroWM's winit backend needs an existing display server, so the session
currently falls back to a shell when booted on a bare TTY. Native DRM/KMS
startup is tracked in Phase 4 of the roadmap.

## System requirements

| Component | Minimum        | Recommended     |
|-----------|----------------|-----------------|
| CPU       | x86_64, 2 cores | x86_64, 4+ cores |
| RAM       | 2 GB           | 4+ GB           |
| Storage   | 20 GB          | 40+ GB          |
| GPU       | Any with Wayland driver | AMD/Intel |
| Boot      | UEFI or BIOS   | UEFI            |
