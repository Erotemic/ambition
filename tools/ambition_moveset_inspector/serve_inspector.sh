#!/usr/bin/env bash
# Serve the moveset inspector.
#
# This script does not build by default. `--build` is the explicit opt-in path
# for refreshing all three binaries the inspector depends on before serving.
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
if ! repo="$(git -C "$here" rev-parse --show-toplevel 2>/dev/null)"; then
  echo "[inspector] error: could not find the Ambition git repository from $here" >&2
  exit 2
fi

export_bundle=1
build_tools=0

# ⛔⛔ AN INFORMATIONAL FLAG DOES NO WORK BEFORE IT ANSWERS. `--help` and
# `--report` used to pay for the whole startup first: the binary provenance
# block, and then `moveset_export`, which BOOTS THE COMPOSED APP — a second of
# LDtk merges, boss sheets, a schedule census and an asset decode — before
# argparse got the chance to print four lines of usage. Jon, 2026-08-29:
# *"The --help for the moveset tool seems to do work before it gets there,
# that's not good."*
#
# ⭐ NEITHER FLAG READS THE BUNDLE. `--help` is argparse, and `--report` reads
# the review bank (`format_report(bank.open_work())`) and returns before the
# bundle is even looked for — so the export was not merely early, it was never
# used at all on that path.
#
# ⛔ AND IT SKIPS THE PROVENANCE BLOCK TOO. Which binary is on disk and how old
# it is is the provenance of what is ON SCREEN; a usage message has nothing on
# screen to explain.
informational=0
for arg in "$@"; do
  case "$arg" in
    -h|--help|--report) informational=1 ;;
  esac
done

args=()
for arg in "$@"; do
  case "$arg" in
    --no-export) export_bundle=0 ;;
    --build)     build_tools=1 ;;
    *)           args+=("$arg") ;;
  esac
done


if [[ "$informational" == 1 ]]; then
  cd "$here"
  if command -v uv >/dev/null 2>&1; then
    exec uv run --with pyyaml python -m ambition_moveset_inspector.server "${args[@]}"
  else
    exec python3 -m ambition_moveset_inspector.server "${args[@]}"
  fi
fi

target_root="${CARGO_TARGET_DIR:-target}"
if [[ "$target_root" != /* ]]; then
  target_root="$repo/$target_root"
fi

if (( build_tools )); then
  echo "[inspector] building required tools..."
  (
    cd "$repo"
    cargo build \
      --manifest-path "$repo/Cargo.toml" \
      -p ambition_app_tools \
      --bin moveset_export \
      --bin moveset_takes \
      --bin moveset_render
  )
  # Cargo's default build above is DEBUG. Pin the server and exporter to the
  # exact files just produced so an older release binary cannot win discovery.
  export AMBITION_MOVESET_EXPORT="$target_root/debug/moveset_export"
  export AMBITION_MOVESET_TAKES="$target_root/debug/moveset_takes"
  export AMBITION_MOVESET_RENDER="$target_root/debug/moveset_render"
fi

# The same freshness rule the Python server uses: explicit overrides first,
# otherwise the newest executable among release/debug.
find_bin() {
  local name="$1" env_name override best="" best_stamp=-1 path stamp profile
  env_name="AMBITION_${name^^}"
  override="${!env_name:-}"
  if [[ -n "$override" && -x "$override" ]]; then
    printf '%s' "$override"
    return 0
  fi
  for profile in release debug; do
    path="$target_root/$profile/$name"
    [[ -x "$path" ]] || continue
    stamp="$(stat -c %Y "$path")"
    if (( stamp > best_stamp )); then
      best="$path"
      best_stamp="$stamp"
    fi
  done
  [[ -n "$best" ]] || return 1
  printf '%s' "$best"
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
# ⛔⛔ AND THE LINE IS COPY-PASTEABLE. It used to print a BRACE EXPANSION —
# `--bin {moveset_export,moveset_takes,capture_scene}` — which the shell expands
# to `--bin moveset_export moveset_takes capture_scene`, and cargo takes one
# value per `--bin`: *"error: unexpected argument 'moveset_takes' found"*. Jon,
# 2026-08-29: *"the line it prints is not executable."* A suggested command that
# does not run is worse than none, because it costs the reader a round trip to
# find out.
if (( build_tools )); then
  echo "[inspector] --build refreshed the required debug binaries."
else
  echo "[inspector] refresh binaries explicitly with --build, or run:"
fi
echo "[inspector]   cargo build -p ambition_app_tools --bin moveset_export --bin moveset_takes --bin moveset_render"

report_bin moveset_export "serving whatever bundle is already on disk" || true
report_bin moveset_takes  "new interactive runtime takes cannot be generated"  || true
report_bin moveset_render "Engine Takes will use CPU-derived sprites"   || true

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
