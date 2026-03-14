#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
source "$ROOT/scripts/gate-helpers.sh"

write_marker=true
git_dir="$(git rev-parse --git-dir)"
marker_file="$git_dir/.full-gate.ok"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-full-gate.sh
  scripts/check-full-gate.sh --no-marker
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-marker)
      write_marker=false
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "full-gate: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

bash scripts/check-pre-commit-gate.sh --no-marker
bash scripts/check-rust-tests.sh --workspace
bash scripts/check-property-tests.sh
bash scripts/check-fuzz.sh

if [[ "$write_marker" == true ]]; then
  gate_write_marker "$marker_file"
  echo "full-gate: marker updated at $marker_file"
fi

echo "full-gate: OK"
