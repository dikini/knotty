#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

search_paths=(Cargo.toml)
for path in src tests fuzz fuzz_targets hfuzz_target; do
  if [[ -e "$path" ]]; then
    search_paths+=("$path")
  fi
done

usage() {
  cat <<'USAGE'
Usage:
  scripts/check-fuzz.sh
  scripts/check-fuzz.sh --smoke
USAGE
}

mode="default"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --smoke)
      mode="smoke"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "check-fuzz: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

backend=""
target=""

if [[ -d fuzz ]]; then
  if [[ -f fuzz/Cargo.toml ]] || rg -n 'cargo-fuzz|libfuzzer_sys' "${search_paths[@]}" -g '!target' >/dev/null 2>&1; then
    backend="cargo-fuzz"
  fi
fi

if [[ -z "$backend" && -d fuzz_targets ]]; then
  backend="cargo-fuzz"
fi

if [[ -z "$backend" ]] && rg -n 'honggfuzz' "${search_paths[@]}" -g '!target' >/dev/null 2>&1; then
  backend="honggfuzz"
fi

if [[ -z "$backend" ]] && rg -n 'bolero' "${search_paths[@]}" -g '!target' >/dev/null 2>&1; then
  backend="bolero"
fi

if [[ -z "$backend" ]]; then
  echo "fuzz: no fuzz harness detected, skipping"
  exit 0
fi

case "$backend" in
  cargo-fuzz)
    if ! cargo fuzz --help >/dev/null 2>&1; then
      echo "check-fuzz: cargo-fuzz harness detected but cargo-fuzz is not installed" >&2
      exit 1
    fi

    if ! targets_output="$(cargo fuzz list 2>&1)"; then
      echo "check-fuzz: cargo-fuzz harness detected but no runnable target list could be produced" >&2
      echo "$targets_output" >&2
      exit 1
    fi

    target="$(printf '%s\n' "$targets_output" | sed '/^$/d' | head -n 1 | awk '{print $1}')"
    if [[ -z "$target" ]]; then
      echo "check-fuzz: cargo-fuzz harness detected but no runnable fuzz target was found" >&2
      exit 1
    fi

    echo "fuzz: backend cargo-fuzz"
    echo "fuzz: selected target $target"
    if [[ "$mode" == "smoke" ]]; then
      echo "fuzz: running cargo fuzz run $target -- -max_total_time=1"
      cargo fuzz run "$target" -- -max_total_time=1
    else
      echo "fuzz: running cargo fuzz run $target"
      cargo fuzz run "$target"
    fi
    ;;
  honggfuzz)
    if ! cargo hfuzz --help >/dev/null 2>&1; then
      echo "check-fuzz: honggfuzz harness detected but cargo-hfuzz is not installed" >&2
      exit 1
    fi

    if [[ ! -d hfuzz_target ]]; then
      echo "check-fuzz: honggfuzz markers detected but hfuzz_target/ is missing" >&2
      exit 1
    fi

    target="$(find hfuzz_target -maxdepth 1 -type f -name '*.rs' -print | sort | head -n 1 | xargs -r -n 1 basename | sed 's/\.rs$//')"
    if [[ -z "$target" ]]; then
      echo "check-fuzz: honggfuzz harness detected but no runnable target was found" >&2
      exit 1
    fi

    echo "fuzz: backend honggfuzz"
    echo "fuzz: selected target $target"
    if [[ "$mode" == "smoke" ]]; then
      echo "fuzz: running cargo hfuzz run $target -- -n 100"
      cargo hfuzz run "$target" -- -n 100
    else
      echo "fuzz: running cargo hfuzz run $target"
      cargo hfuzz run "$target"
    fi
    ;;
  bolero)
    if ! cargo bolero --help >/dev/null 2>&1; then
      echo "check-fuzz: bolero harness detected but cargo-bolero is not installed" >&2
      exit 1
    fi

    echo "fuzz: backend bolero"
    if [[ "$mode" == "smoke" ]]; then
      echo "fuzz: running cargo bolero test --engine libfuzzer --sanitizer none --runs 100"
      cargo bolero test --engine libfuzzer --sanitizer none --runs 100
    else
      echo "fuzz: running cargo bolero test --engine libfuzzer --sanitizer none"
      cargo bolero test --engine libfuzzer --sanitizer none
    fi
    ;;
  *)
    echo "check-fuzz: fuzz harness detected for backend '$backend' but no runnable command is configured" >&2
    exit 1
    ;;
esac
