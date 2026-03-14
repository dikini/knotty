#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-changelog-policy.sh
USAGE
}

if [[ $# -gt 0 ]]; then
  echo "changelog-policy: unexpected arguments" >&2
  usage
  exit 2
fi

if [[ ! -f CHANGELOG.md ]]; then
  echo "changelog-policy: CHANGELOG.md is required at the repository root" >&2
  exit 1
fi

if ! grep -q '^# Changelog$' CHANGELOG.md; then
  echo "changelog-policy: CHANGELOG.md must start with '# Changelog'" >&2
  exit 1
fi

if ! grep -Eq '^## [^[:space:]].*' CHANGELOG.md; then
  echo "changelog-policy: CHANGELOG.md must contain at least one version section" >&2
  exit 1
fi

mapfile -t invalid_headings < <(grep '^### ' CHANGELOG.md | sed 's/^### //' | grep -Ev '^(Added|Changed|Fixed|Removed)$' || true)
if [[ "${#invalid_headings[@]}" -gt 0 ]]; then
  echo "changelog-policy: invalid section headings found in CHANGELOG.md" >&2
  printf '  %s\n' "${invalid_headings[@]}" >&2
  echo "changelog-policy: only Added, Changed, Fixed, and Removed are allowed" >&2
  exit 1
fi

echo "changelog-policy: OK"
