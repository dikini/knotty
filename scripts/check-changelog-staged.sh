#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-changelog-staged.sh
USAGE
}

if [[ $# -gt 0 ]]; then
  echo "changelog-staged: unexpected arguments" >&2
  usage
  exit 2
fi

mapfile -t staged_files < <(git diff --cached --name-only --diff-filter=ACMR | sed '/^$/d')

requires_changelog=()
has_changelog=false

for path in "${staged_files[@]}"; do
  case "$path" in
    CHANGELOG.md)
      has_changelog=true
      ;;
    src/*|scripts/*|.githooks/*|Cargo.toml|Cargo.lock|build.rs|justfile|README.md|AGENTS.md|docs/*)
      requires_changelog+=("$path")
      ;;
  esac
done

if [[ "${#requires_changelog[@]}" -gt 0 && "$has_changelog" != true ]]; then
  echo "changelog-staged: staged implementation or policy changes require a staged CHANGELOG.md update" >&2
  echo "changelog-staged: triggering files:" >&2
  printf '  %s\n' "${requires_changelog[@]}" >&2
  echo "changelog-staged: update CHANGELOG.md using Common Changelog sections:" >&2
  echo "  Added" >&2
  echo "  Changed" >&2
  echo "  Fixed" >&2
  echo "  Removed" >&2
  echo "changelog-staged: reference https://common-changelog.org/" >&2
  exit 1
fi

echo "changelog-staged: OK"
