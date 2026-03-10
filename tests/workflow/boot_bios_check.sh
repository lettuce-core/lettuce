#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

bash tests/build_iso.sh bios

LOG_FILE="${ROOT_DIR}/build/qemu-boot-bios.log"
rm -f "${LOG_FILE}"

qemu-system-x86_64 \
  -cdrom "${ROOT_DIR}/build/lettuce.iso" \
  -m 512M \
  -display none \
  -serial stdio >"${LOG_FILE}" 2>&1 &
QEMU_PID=$!

cleanup() {
  kill "${QEMU_PID}" >/dev/null 2>&1 || true
}

trap cleanup EXIT

for _ in $(seq 1 30); do
  if grep -q "boot source: grub multiboot2" "${LOG_FILE}"; then
    echo "bios boot check passed"
    exit 0
  fi

  sleep 1
done

echo "bios boot check failed"
cat "${LOG_FILE}"
exit 1
