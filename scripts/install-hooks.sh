#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

git config core.hooksPath .githooks

if [[ -d .githooks ]]; then
  chmod +x .githooks/*
fi

if [[ -d scripts ]]; then
  find scripts -maxdepth 1 -type f -name '*.sh' -exec chmod +x {} +
fi

echo "hooks installed: core.hooksPath=.githooks"
