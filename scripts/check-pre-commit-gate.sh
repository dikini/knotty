#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
source "$ROOT/scripts/gate-helpers.sh"

write_marker=true
git_dir="$(git rev-parse --git-dir)"
marker_file="$git_dir/.pre-commit-gate.ok"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-pre-commit-gate.sh
  scripts/check-pre-commit-gate.sh --no-marker
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
      echo "pre-commit-gate: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

bash scripts/check-doc-work-evidence.sh
bash scripts/check-rust-format.sh
echo "pre-commit-gate: running cargo check"
cargo check
bash scripts/check-rust-tests.sh --workspace
bash scripts/check-property-tests.sh
bash scripts/check-fuzz.sh --smoke

if [[ "$write_marker" == true ]]; then
  gate_write_marker "$marker_file"
  echo "pre-commit-gate: marker updated at $marker_file"
fi

echo "pre-commit-gate: OK"
