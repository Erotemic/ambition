#!/usr/bin/env bash

set -Eeuo pipefail

if ! command -v git >/dev/null 2>&1; then
    echo "error: git is required" >&2
    exit 1
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

if cargo modules --help >/dev/null 2>&1; then
    cargo modules structure --package ambition_sandbox --lib \
        > "$REPO_ROOT"/.agent/reports/module-tree-ambition_sandbox.md

    cargo modules dependencies --package ambition_sandbox --lib \
        > .agent/reports/module-dependencies-ambition_sandbox.md

fi


cargo check --workspace --all-targets --message-format=json
.agent/reports/cargo-check-warnings.md


python tools/ecs_inventory.py \
  --json .agent/ecs_inventory.json \
  --markdown .agent/ecs_inventory.md
