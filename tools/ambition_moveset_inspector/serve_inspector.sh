#!/usr/bin/env bash
# Serve the moveset inspector.
#
# ⛔⛔ THIS SCRIPT NEVER INVOKES CARGO. It used to `cargo run` the exporter on
# every start, which takes the cargo build lock — so looking at frame data could
# block, or be blocked by, an agent building on another branch. Jon, 2026-08-27:
# *"I don't want it to force a build if an agent is working on the main branch."*
# It runs binaries that already exist and tells you the command when one does not.
#
#   --no-export   serve the bundle already on disk, without re-exporting
#
# ⚠ THE COST OF NEVER BUILDING is that an existing binary can be older than the
# source it was built from. The bundle records the cast generation it came from,
# and the age of each binary is printed below, so a stale answer is at least a
# visible one.
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

# The same lookup the server uses for the renderer, and the same convention
# `scripts/profile_desktop.sh` uses: honour CARGO_TARGET_DIR, prefer release.
target_root="${CARGO_TARGET_DIR:-$repo/target}"
find_bin() {
  for profile in release debug; do
    if [[ -x "$target_root/$profile/$1" ]]; then
      printf '%s' "$target_root/$profile/$1"
      return 0
    fi
  done
  return 1
}

say_missing() {
  echo "[inspector] $1 is not built — $2" >&2
  echo "[inspector]   cargo build -p ambition_app_tools --bin $1" >&2
}

if [[ "$export_bundle" == 1 ]]; then
  if exporter="$(find_bin moveset_export)"; then
    echo "[inspector] exporting with $exporter ($(date -r "$exporter" '+%Y-%m-%d %H:%M'))"
    "$exporter" --out "$here/data/moveset_bundle.json" >/dev/null
  else
    say_missing moveset_export "serving whatever bundle is already on disk"
  fi
fi

# ⭐ NAMED, NOT BUILT. The renderer is the only GPU-dependent piece; the view
# falls back to CPU-derived sprites without it and says which it is showing.
if ! find_bin capture_scene >/dev/null; then
  say_missing capture_scene "Engine Takes will use CPU-derived sprites"
fi
if ! find_bin moveset_takes >/dev/null; then
  say_missing moveset_takes "there will be no recorded takes to look at"
fi

cd "$here"
if command -v uv >/dev/null 2>&1; then
  exec uv run --with pyyaml python -m ambition_moveset_inspector.server "${args[@]}"
else
  exec python3 -m ambition_moveset_inspector.server "${args[@]}"
fi
