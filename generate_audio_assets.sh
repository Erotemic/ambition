#!/usr/bin/env bash
# Render and install the first_goblin_tune_v2 adaptive cue.
#
# Usage:
#   ./generate_audio_assets.sh               # render full mixes if stale + install (default)
#   ./generate_audio_assets.sh --skip-render # only re-install from existing render
#   ./generate_audio_assets.sh --force       # force render + install
#   ./generate_audio_assets.sh --with-stems  # also render/install per-stem OGGs
#   ./generate_audio_assets.sh --keep-debug-stems  # keep scratch .npy files
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#
# Output staging / preview:
#   tools/ambition_music_renderer/generated/first_goblin_tune_v2/
#
# Installed assets:
#   crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"
spec="$renderer_dir/scores/active/first_goblin_tune_v2.music.yaml"
staging="$renderer_dir/generated/first_goblin_tune_v2"
installer="$renderer_dir/install_first_goblin_tune_v2.py"
auditor="$renderer_dir/audit_cue_balance.py"
backend="${AMBITION_MUSIC_BACKEND:-pretty-midi}"

skip_render=0
force_render=0
with_stems=0
keep_debug_stems=0
for arg in "$@"; do
    case "$arg" in
        --skip-render) skip_render=1 ;;
        --force|--force-render) force_render=1 ;;
        --with-stems) with_stems=1 ;;
        --keep-debug-stems) keep_debug_stems=1 ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ ! -f "$spec" ]; then
    echo "spec not found: $spec" >&2
    exit 1
fi

if [ "$skip_render" -eq 0 ]; then
    echo "==> rendering first_goblin_tune_v2 if stale"
    echo "    backend=$backend"
    if [ "$with_stems" -eq 0 ]; then
        echo "    mode=full-mix-only (skips per-stem OGG encodes)"
    else
        echo "    mode=full mixes + per-stem OGGs"
    fi
    mkdir -p "$staging"
    render_args=(
        "$spec"
        --outdir "$staging"
        --backend "$backend"
    )
    if [ "$with_stems" -eq 0 ]; then
        render_args+=(--full-mix-only)
    fi
    if [ "$keep_debug_stems" -eq 1 ]; then
        render_args+=(--keep-debug-stems)
    fi
    if [ "$force_render" -eq 1 ]; then
        echo "    force=true"
        rm -rf "$staging"
        mkdir -p "$staging"
        render_args+=(--force)
    fi
    (
        cd "$renderer_dir"
        python -m ambition_music_renderer.render_isolated "${render_args[@]}"
    )
fi
if [ ! -d "$staging/adaptive" ]; then
    echo "render output missing: $staging/adaptive" >&2
    exit 1
fi

echo "==> audit generated cue balance"
python "$auditor" "$staging" || true

echo "==> installing into crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2"
install_args=(--src "$staging" --clean)
if [ "$with_stems" -eq 1 ]; then
    install_args+=(--with-stems)
fi
python "$installer" "${install_args[@]}"

echo "==> previews:"
find "$staging/preview" -maxdepth 1 -type f -name '*.ogg' -print | sort

echo "==> done"
