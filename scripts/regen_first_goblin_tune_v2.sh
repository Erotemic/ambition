#!/usr/bin/env bash
# Render and install the first_goblin_tune_v2 adaptive cue.
#
# Usage:
#   ./scripts/regen_first_goblin_tune_v2.sh               # render full mixes if stale + install
#   ./scripts/regen_first_goblin_tune_v2.sh --skip-render # only re-install from existing render
#   ./scripts/regen_first_goblin_tune_v2.sh --force       # force render + install
#   ./scripts/regen_first_goblin_tune_v2.sh --with-stems  # also render/install per-stem OGGs
#   ./scripts/regen_first_goblin_tune_v2.sh --keep-debug-stems  # keep scratch .npy files
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#
# Output staging / preview:
#   tools/ambition_music_renderer/generated/first_goblin_tune_v2/
#
# Installed assets:
#   crates/ambition_actors/assets/audio/music/generated/first_goblin_tune_v2/
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"
spec="$renderer_dir/scores/active/first_goblin_tune_v2.music.yaml"
staging="$renderer_dir/generated/first_goblin_tune_v2"
backend="${AMBITION_MUSIC_BACKEND:-pretty-midi}"

select_python() {
    if [ -n "${PYTHON:-}" ]; then
        printf '%s\n' "$PYTHON"
    elif [ -n "${VIRTUAL_ENV:-}" ] && [ -x "$VIRTUAL_ENV/bin/python" ]; then
        printf '%s\n' "$VIRTUAL_ENV/bin/python"
    elif [ -x "$repo_root/.venv/bin/python" ]; then
        printf '%s\n' "$repo_root/.venv/bin/python"
    elif [ -x "$renderer_dir/.venv/bin/python" ]; then
        printf '%s\n' "$renderer_dir/.venv/bin/python"
    else
        printf '%s\n' python
    fi
}

print_help() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

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
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ "$skip_render" -eq 1 ] && [ "$force_render" -eq 1 ]; then
    echo "--skip-render and --force cannot be combined" >&2
    exit 2
fi

if [ ! -f "$spec" ]; then
    echo "spec not found: $spec" >&2
    exit 1
fi

python_bin="$(select_python)"
if ! command -v "$python_bin" >/dev/null 2>&1; then
    echo "python executable not found: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate a venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

if ! "$python_bin" -c 'import ambition_music_renderer' >/dev/null 2>&1; then
    echo "ambition_music_renderer is not installed in: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate the configured venv, or set PYTHON=/path/to/python" >&2
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
        "$python_bin" -m ambition_music_renderer.render.isolated "${render_args[@]}"
    )
fi
if [ ! -d "$staging/adaptive" ]; then
    echo "render output missing: $staging/adaptive" >&2
    exit 1
fi

echo "==> audit generated cue balance"
"$python_bin" -m ambition_music_renderer audit cue_balance "$staging" || true

echo "==> installing into crates/ambition_actors/assets/audio/music/generated/first_goblin_tune_v2"
install_args=(--src "$staging" --clean)
if [ "$with_stems" -eq 1 ]; then
    install_args+=(--with-stems)
fi
"$python_bin" -m ambition_music_renderer legacy install_first_goblin_tune_v2 "${install_args[@]}"

echo "==> previews:"
find "$staging/preview" -maxdepth 1 -type f -name '*.ogg' -print | sort

echo "==> done"
