#!/usr/bin/env bash
set -euo pipefail

# usage:
#   scripts/build_iso.sh            ; default: bios
#   scripts/build_iso.sh bios
#   scripts/build_iso.sh uefi
MODE="${1:-bios}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO_ROOT="${ROOT_DIR}/build/isodir"
ISO_PATH="${ROOT_DIR}/build/lettuce.iso"

BOOT_MODE_FILE="${ROOT_DIR}/build/boot_mode"
KERNEL_ELF="${ROOT_DIR}/target/x86_64-unknown-none/debug/lettuce-kernel"
RUSTC_BIN="$(rustup which --toolchain stable rustc)"

case "${MODE}" in
  bios)
    GRUB_MKRESCUE_BIN="i686-elf-grub-mkrescue"
    ;;
  uefi)
    GRUB_MKRESCUE_BIN="x86_64-elf-grub-mkrescue"
    ;;
  *)
    echo "error: mode must be 'bios' or 'uefi'"
    exit 1
    ;;
esac

command -v "${GRUB_MKRESCUE_BIN}" >/dev/null 2>&1 || {
  echo "error: ${GRUB_MKRESCUE_BIN} not found"
  exit 1
}

command -v xorriso >/dev/null 2>&1 || {
  echo "error: xorriso not found"
  exit 1
}

command -v mformat >/dev/null 2>&1 || {
  echo "error: mformat not found (install mtools)"
  exit 1
}

mkdir -p "${ISO_ROOT}/boot/grub"
RUSTC="${RUSTC_BIN}" rustup run stable cargo build -p lettuce-kernel --target x86_64-unknown-none

cp "${KERNEL_ELF}" "${ISO_ROOT}/boot/lettuce-kernel"
cp "${ROOT_DIR}/boot/grub/grub.cfg" "${ISO_ROOT}/boot/grub/grub.cfg"

"${GRUB_MKRESCUE_BIN}" -o "${ISO_PATH}" "${ISO_ROOT}"

echo "${MODE}" > "${BOOT_MODE_FILE}"
echo "iso created (boot mode: ${MODE}): ${ISO_PATH}"
