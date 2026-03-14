#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-doc-work-evidence.sh
USAGE
}

if [[ $# -gt 0 ]]; then
  echo "doc-work-evidence: unexpected arguments" >&2
  usage
  exit 2
fi

mapfile -t staged_files < <(git diff --cached --name-only --diff-filter=ACMR | sed '/^$/d')

code_paths=()
doc_paths=()

for path in "${staged_files[@]}"; do
  case "$path" in
    docs/*|README.md)
      doc_paths+=("$path")
      ;;
    src/*|Cargo.toml|Cargo.lock|build.rs|justfile|.githooks/*|scripts/*)
      code_paths+=("$path")
      ;;
  esac
done

if [[ "${#code_paths[@]}" -gt 0 && "${#doc_paths[@]}" -eq 0 ]]; then
  echo "doc-work-evidence: staged code changes require staged docs updates" >&2
  echo "doc-work-evidence: staged code files:" >&2
  printf '  %s\n' "${code_paths[@]}" >&2
  echo "doc-work-evidence: stage at least one docs path under docs/ or README.md" >&2
  exit 1
fi

echo "doc-work-evidence: OK"
