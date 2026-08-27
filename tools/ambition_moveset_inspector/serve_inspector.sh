#!/usr/bin/env bash
# Serve the moveset inspector. Re-exports the bundle first unless --no-export.
#
# PyYAML is supplied ephemerally through `uv --with`, the same shape
# `review_music.sh` uses, so looking at frame data does not require installing
# anything into the repo's environment.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

export_bundle=1
args=()
for arg in "$@"; do
  if [[ "$arg" == "--no-export" ]]; then export_bundle=0; else args+=("$arg"); fi
done

if [[ "$export_bundle" == 1 ]]; then
  ( cd "$repo" && cargo run --quiet -p ambition_app_tools --bin moveset_export -- \
      --out "$here/data/moveset_bundle.json" >/dev/null )
fi

cd "$here"
if command -v uv >/dev/null 2>&1; then
  exec uv run --with pyyaml python -m ambition_moveset_inspector.server "${args[@]}"
else
  exec python3 -m ambition_moveset_inspector.server "${args[@]}"
fi
