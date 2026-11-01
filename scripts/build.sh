#!/usr/bin/env bash
# Build allowit-core.
#
#   scripts/build.sh            build every target and the Wasm module
#   scripts/build.sh --check    check the cross-repository pins
#   scripts/build.sh --update   update stale pins, then build
#   scripts/build.sh --update <repo>...
#                               update only the pins of the named repositories
#
# Core depends on no other Allowit repository, so it has no pins: --check
# always passes, --update only builds and a named repository is an error.
# The flags match the build script of every Allowit repository.
set -euo pipefail
cd "$(dirname "$0")/.."

usage="usage: scripts/build.sh [--check | --update [repo...]]"
case "${1-}" in
  "") [ $# -le 1 ] || { echo "$usage" >&2; exit 2; } ;;
  --update)
    if [ $# -gt 1 ]; then echo "$2: no git dependency on this repository" >&2; exit 2; fi ;;
  --check) [ $# -le 1 ] || { echo "$usage" >&2; exit 2; }; exit 0 ;;
  *) echo "$usage" >&2; exit 2 ;;
esac

cargo build --all-targets --locked
cargo rustc --locked --release --lib --target wasm32-unknown-unknown --crate-type cdylib
