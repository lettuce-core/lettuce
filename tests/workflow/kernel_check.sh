#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

rustup target add --toolchain stable x86_64-unknown-none

RUSTC_BIN="$(rustup which --toolchain stable rustc)"
env -u RUSTDOC -u RUSTFLAGS -u CARGO_BUILD_TARGET \
  RUSTC="${RUSTC_BIN}" \
  rustup run stable cargo check -p lettuce-kernel --target x86_64-unknown-none
