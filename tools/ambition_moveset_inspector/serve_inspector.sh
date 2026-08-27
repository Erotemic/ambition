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

# ⭐⭐ ONE LINE PER BINARY, ALWAYS. Printing the build command only when
# something was MISSING answered "why is this broken" and never "what am I
# looking at" — and with nothing built by this script, which binary it picked and
# how old that binary is IS the provenance of everything on screen. Jon:
# *"It should show what the build command is, and the mtime or other info of the
# binary it will use."*
report_bin() {
  local name="$1" lost="$2" path age
  if path="$(find_bin "$name")"; then
    age="$(( ( $(date +%s) - $(stat -c %Y "$path") ) / 60 ))"
    if (( age < 60 )); then age="${age}m old"
    elif (( age < 2880 )); then age="$(( age / 60 ))h old"
    else age="$(( age / 1440 ))d old"; fi
    printf '[inspector] %-15s %s  (built %s, %s)\n' \
      "$name" "$path" "$(date -r "$path" '+%Y-%m-%d %H:%M')" "$age"
  else
    printf '[inspector] %-15s NOT BUILT — %s\n' "$name" "$lost" >&2
    printf '[inspector] %-15s   cargo build -p ambition_app_tools --bin %s\n' "" "$name" >&2
    return 1
  fi
}

# ⛔ THE BUILD COMMANDS ARE ALWAYS VISIBLE, not only on the failure path. A
# person who wants to REFRESH a binary that already exists needs the same line as
# a person who has none, and only one of those two used to get it.
echo "[inspector] this tool never builds; refresh a binary yourself with:"
echo "[inspector]   cargo build -p ambition_app_tools --bin {moveset_export,moveset_takes,capture_scene}"

report_bin moveset_export "serving whatever bundle is already on disk" || true
report_bin moveset_takes  "there will be no recorded takes to look at"  || true
report_bin capture_scene  "Engine Takes will use CPU-derived sprites"   || true

if [[ "$export_bundle" == 1 ]]; then
  if exporter="$(find_bin moveset_export)"; then
    "$exporter" --out "$here/data/moveset_bundle.json" >/dev/null
  fi
fi

cd "$here"
if command -v uv >/dev/null 2>&1; then
  exec uv run --with pyyaml python -m ambition_moveset_inspector.server "${args[@]}"
else
  exec python3 -m ambition_moveset_inspector.server "${args[@]}"
fi
