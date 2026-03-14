#!/usr/bin/env bash
set -euo pipefail

gate_repo_root() {
  git rev-parse --show-toplevel
}

gate_content_sha() {
  mapfile -t paths < <({
    git diff --name-only
    git diff --cached --name-only
    git ls-files --others --exclude-standard
  } | sed '/^$/d' | sort -u)

  if [[ "${#paths[@]}" -eq 0 ]]; then
    printf '' | sha256sum | awk '{print $1}'
    return
  fi

  {
    for path in "${paths[@]}"; do
      if [[ -e "$path" ]]; then
        hash="$(git hash-object -- "$path" 2>/dev/null || printf '__nonregular__')"
      else
        hash="__deleted__"
      fi
      printf '%s\t%s\n' "$path" "$hash"
    done
  } | sha256sum | awk '{print $1}'
}

gate_head_ref() {
  if git rev-parse --verify HEAD >/dev/null 2>&1; then
    git rev-parse HEAD
  else
    printf 'UNBORN_HEAD'
  fi
}

gate_write_marker() {
  local marker_file="$1"

  {
    echo "timestamp_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "head=$(gate_head_ref)"
    echo "content_sha=$(gate_content_sha)"
  } >"$marker_file"
}
