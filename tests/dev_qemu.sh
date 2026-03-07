#!/usr/bin/env bash
set -euo pipefail

# usage:
#   tests/dev_qemu.sh            ; default: bios 512M
#   tests/dev_qemu.sh bios
#   tests/dev_qemu.sh uefi 1G
MODE="${1:-bios}"
MEMORY="${2:-512M}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

bash tests/build_iso.sh "${MODE}"
exec bash tests/run_qemu.sh "${MODE}" "${MEMORY}"
