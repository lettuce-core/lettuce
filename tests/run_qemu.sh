#!/usr/bin/env bash
set -euo pipefail

# usage:
#   tests/run_qemu.sh             ; auto-detect last built mode, 512M
#   tests/run_qemu.sh bios
#   tests/run_qemu.sh uefi
#   tests/run_qemu.sh uefi 1G
#   tests/run_qemu.sh bios 256M

MODE="${1:-auto}"
MEMORY="${2:-512M}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BOOT_MODE_FILE="${ROOT_DIR}/build/boot_mode"
OVMF_CODE="${OVMF_CODE:-/opt/homebrew/share/qemu/edk2-x86_64-code.fd}"

ISO_PATH="${ROOT_DIR}/build/lettuce.iso"
[[ -f "${ISO_PATH}" ]] || {
    echo "error: iso not found: ${ISO_PATH}";
    exit 1;
}

if [[ "${MODE}" == "auto" ]]; then
  if [[ -f "${BOOT_MODE_FILE}" ]]; then
    MODE="$(cat "${BOOT_MODE_FILE}")"
  else
    MODE="bios"
  fi
fi

case "${MODE}" in
  bios)
    echo "running qemu in bios mode (memory=${MEMORY})"
    exec qemu-system-x86_64 \
      -cdrom "${ISO_PATH}" \
      -m "${MEMORY}" \
      -serial stdio
    ;;
  uefi)
    [[ -f "${OVMF_CODE}" ]] || {
        echo "error: OVMF firmware not found: ${OVMF_CODE}";
        exit 1;
    }

    echo "running qemu in uefi mode (memory=${MEMORY})"
    exec qemu-system-x86_64 \
      -machine q35 \
      -drive if=pflash,format=raw,readonly=on,file="${OVMF_CODE}" \
      -cdrom "${ISO_PATH}" \
      -m "${MEMORY}" \
      -serial stdio
    ;;
  *)
    echo "error: mode must be 'bios', 'uefi', or 'auto'"
    exit 1
    ;;
esac
