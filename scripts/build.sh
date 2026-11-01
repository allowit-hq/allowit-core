#!/usr/bin/env bash
# Build allowit-core.
#
#   scripts/build.sh            build every target and the Wasm module
#   scripts/build.sh --check    check the cross-repository pins
#   scripts/build.sh --update   update stale pins, then build
#
# Core depends on no other Allowit repository, so it has no pins: --check
# always passes and --update only builds. The flags match the build script
# of every Allowit repository.
set -euo pipefail
cd "$(dirname "$0")/.."

case "${1-}" in
  "" | --update) ;;
  --check) exit 0 ;;
  *) echo "usage: scripts/build.sh [--check | --update]" >&2; exit 2 ;;
esac

cargo build --all-targets --locked
cargo rustc --locked --release --lib --target wasm32-unknown-unknown --crate-type cdylib
