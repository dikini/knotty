#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-rust-clippy.sh
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "check-rust-clippy: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

echo "check-rust-clippy: running cargo clippy --workspace --all-targets --all-features"
cargo clippy --workspace --all-targets --all-features
