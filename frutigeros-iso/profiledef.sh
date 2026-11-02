#!/usr/bin/env bash
# FrutigerOS archiso profile definition.
#
# Build the ISO with:
#   sudo mkarchiso -v -w /tmp/frutigeros-work -o ./out ./
#
# Requires archiso package on Arch Linux.
# Phase 5: AeroWM and AeroShell packages must be in a custom repo
#          before this profile produces a working desktop ISO.

iso_name="frutigeros"
iso_label="FRUTIGEROS_$(date --date="@${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y%m)"
iso_publisher="FrutigerOS Contributors <https://github.com/frutigeros>"
iso_application="FrutigerOS Live/Installation Medium"
iso_version="0.1.0-alpha"
install_dir="arch"
buildmodes=('iso')
bootmodes=('bios.syslinux.mbr' 'bios.syslinux.eltorito'
           'uefi-x64.systemd-boot.esp' 'uefi-x64.systemd-boot.eltorito')
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '15' '-b' '1M')
bootstrap_tarball_compression=('zstd' '-T0' '--fast' '-c')
file_permissions=(
  ["/etc/shadow"]="0:0:400"
  ["/etc/gshadow"]="0:0:400"
  ["/root"]="0:0:750"
  ["/usr/local/bin/frutigeros-live-setup"]="0:0:755"
  ["/usr/local/bin/frutigeros-session"]="0:0:755"
  ["/usr/local/bin/frutigeros-installer"]="0:0:755"
)
