#!/usr/bin/env bash
# Build the FrutigerOS live ISO, or validate the profile without building.
#
# Usage:
#   ./build.sh              # validate, then build with mkarchiso (needs root)
#   ./build.sh --check      # validate the profile only (no root required)
#   ./build.sh --out DIR    # choose the output directory
#   ./build.sh --work DIR   # choose the work directory
#
# Environment:
#   FRUTIGEROS_ISO_OUT   default: ./out
#   FRUTIGEROS_ISO_WORK  default: /tmp/frutigeros-archiso-work
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
out_dir="${FRUTIGEROS_ISO_OUT:-$script_dir/out}"
work_dir="${FRUTIGEROS_ISO_WORK:-/tmp/frutigeros-archiso-work}"
check_only=0

usage() {
  sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
  case "$1" in
    --check) check_only=1 ;;
    --out) out_dir="${2:?--out needs a directory}"; shift ;;
    --work) work_dir="${2:?--work needs a directory}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

fail=0
note() { printf '  %s\n' "$*"; }
check_file() {
  if [ ! -e "$script_dir/$1" ]; then
    printf 'MISSING: %s\n' "$1" >&2
    fail=1
  else
    note "ok  $1"
  fi
}

echo "Validating archiso profile in $script_dir"
check_file profiledef.sh
check_file pacman.conf
check_file packages.x86_64
check_file efiboot/loader/loader.conf
check_file efiboot/loader/entries/frutigeros.conf
check_file syslinux/syslinux.cfg
check_file syslinux/archiso_sys.cfg
check_file airootfs/etc/os-release
check_file airootfs/etc/locale.gen
check_file airootfs/usr/local/bin/frutigeros-session
check_file airootfs/usr/local/bin/frutigeros-live-setup
check_file airootfs/usr/local/bin/frutigeros-installer

echo "Checking shell syntax"
for f in profiledef.sh \
         airootfs/usr/local/bin/frutigeros-session \
         airootfs/usr/local/bin/frutigeros-live-setup \
         airootfs/usr/local/bin/frutigeros-installer \
         airootfs/root/.bash_profile; do
  if bash -n "$script_dir/$f"; then
    note "ok  $f"
  else
    printf 'SYNTAX ERROR: %s\n' "$f" >&2
    fail=1
  fi
done

echo "Checking boot-label placeholders"
if grep -q '%ARCHISO_LABEL%' "$script_dir/efiboot/loader/entries/frutigeros.conf" &&
   grep -q '%ARCHISO_LABEL%' "$script_dir/syslinux/archiso_sys.cfg"; then
  note "ok  archisolabel placeholder present"
else
  echo "Boot entries must use %ARCHISO_LABEL% so mkarchiso can substitute it." >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "Profile validation failed." >&2
  exit 1
fi
echo "Profile validation passed."

if [ "$check_only" -eq 1 ]; then
  exit 0
fi

if ! command -v mkarchiso >/dev/null 2>&1; then
  cat >&2 <<'EOF'
mkarchiso was not found.

Install the archiso package on Arch Linux:
  sudo pacman -S archiso
EOF
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "mkarchiso must run as root (it mounts filesystems)." >&2
  echo "Re-run with sudo, or use --check for validation only." >&2
  exit 1
fi

mkdir -p "$out_dir"
echo "Building ISO:"
echo "  profile: $script_dir"
echo "  work:    $work_dir"
echo "  output:  $out_dir"

exec mkarchiso -v -w "$work_dir" -o "$out_dir" "$script_dir"
