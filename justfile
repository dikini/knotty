set shell := ["bash", "-euo", "pipefail", "-c"]

verify:
    scripts/check-pre-commit-gate.sh

pre-commit-gate:
    scripts/check-pre-commit-gate.sh

full-gate:
    scripts/check-full-gate.sh

install-hooks:
    scripts/install-hooks.sh
