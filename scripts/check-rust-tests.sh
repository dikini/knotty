#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-rust-tests.sh --workspace
  scripts/check-rust-tests.sh --args <cargo-test-args...>
USAGE
}

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

if [[ "$1" == "--workspace" ]]; then
  shift
  args=(--workspace "$@")
elif [[ "$1" == "--args" ]]; then
  shift
  args=("$@")
else
  echo "check-rust-tests: unknown argument '$1'" >&2
  usage
  exit 2
fi

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if cargo nextest --version >/dev/null 2>&1; then
  echo "check-rust-tests: repo root $ROOT"
  echo "check-rust-tests: using cargo nextest"
  echo "check-rust-tests: running cargo nextest run ${args[*]}"
  cargo nextest run "${args[@]}"
else
  echo "check-rust-tests: repo root $ROOT"
  echo "check-rust-tests: cargo nextest not found, falling back to cargo test"
  echo "check-rust-tests: running cargo test ${args[*]}"
  cargo test "${args[@]}"
fi
