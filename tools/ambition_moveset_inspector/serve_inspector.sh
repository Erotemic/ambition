#!/usr/bin/env bash
# Serve the moveset inspector. Re-exports the bundle first unless --no-export.
#
#   --with-renderer   also build `capture_scene`, the GPU binary the Engine Takes
#                     view uses to draw a fighter's REAL animation. Without it the
#                     view falls back to CPU-derived sprites and says so.
#
# PyYAML is supplied ephemerally through `uv --with`, the same shape
# `review_music.sh` uses, so looking at frame data does not require installing
# anything into the repo's environment.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

export_bundle=1
build_renderer=0
args=()
for arg in "$@"; do
  case "$arg" in
    --no-export) export_bundle=0 ;;
    # ⭐ THE GPU RENDERER IS OPT-IN, because it is the one part of this tool that
    # needs a GPU and the whole point of the fallback is that the rest does not.
    # Building it here rather than making somebody find the incantation is what
    # turns "the animation is unavailable" into a one-flag fix.
    --with-renderer) build_renderer=1 ;;
    *) args+=("$arg") ;;
  esac
done

if [[ "$build_renderer" == 1 ]]; then
  ( cd "$repo" && cargo build --quiet -p ambition_app_tools --bin capture_scene )
fi

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
