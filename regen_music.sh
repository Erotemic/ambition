#!/usr/bin/env bash
# Re-render and republish all in-game music cues.
#
# Covers every in-game cue through ONE path:
#   - All radio cues: scores/active/* (auto-discovered) plus the curated
#     EXTRA_RADIO_CUES list (example-tree cues we expose on the radio).
#     Driven via `ambition_music_renderer radio render-publish`. Adaptive cues
#     (e.g. first_goblin_tune_v2) are detected and published per-section by the
#     SAME path — the renderer publishes each adaptive/<section>/<section>.full.ogg
#     into crates/ambition_actors/assets/audio/music/generated/. No dedicated installer.
#
# Usage:
#   ./regen_music.sh                    # render + install everything (default)
#   ./regen_music.sh --skip-render      # only republish from existing renders
#   ./regen_music.sh --force            # force re-render where supported
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#                                   # forwarded to the cue renderer
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"

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
radio_args=()
for arg in "$@"; do
    case "$arg" in
        --skip-render)
            skip_render=1
            ;;
        --force|--force-render)
            force_render=1
            radio_args+=(--force-render)
            ;;
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ "$skip_render" -eq 1 ] && [ "$force_render" -eq 1 ]; then
    echo "--skip-render and --force cannot be combined" >&2
    exit 2
fi

renderer_py="$(select_python)"
if ! command -v "$renderer_py" >/dev/null 2>&1; then
    echo "python executable not found: $renderer_py" >&2
    echo "run ./run_developer_setup.sh, activate a venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

if ! "$renderer_py" -c 'import ambition_music_renderer' >/dev/null 2>&1; then
    echo "ambition_music_renderer is not installed in: $renderer_py" >&2
    echo "run ./run_developer_setup.sh, activate the configured venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

echo "==> radio cues (scores/active/* + EXTRA_RADIO_CUES; adaptive cues per-section)"
if [ "$skip_render" -eq 1 ]; then
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer radio publish)
else
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer radio render-publish "${radio_args[@]}")
fi

# Project every published OGG into the in-game music registry so newly
# rendered cues are registered automatically (stdlib-only; no venv needed).
echo "==> music registry (music_registry.ron)"
"$renderer_py" "$repo_root/scripts/regen_music_registry.py"

echo "==> done"
