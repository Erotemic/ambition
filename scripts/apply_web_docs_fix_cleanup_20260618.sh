#!/usr/bin/env bash
set -euo pipefail

# Cleanup companion for ambition-web-docs-fix overlay 2026-06-18.
# Overlay ZIPs replace/create files but do not delete stale paths, so remove
# the old pre-Stage-20 web bootstrap location after the new app-owned web
# bootstrap has been overlaid.

if [[ ! -f game/ambition_app/web/index.html ]]; then
    echo "expected game/ambition_app/web/index.html to exist before cleanup" >&2
    exit 1
fi

rm -rf crates/ambition_actors/web

echo "removed stale crates/ambition_actors/web bootstrap; web assets now live under game/ambition_app/web"
