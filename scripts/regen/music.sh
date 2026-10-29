#!/usr/bin/env bash
# Re-render and republish all in-game music cues.
#
# Covers every in-game cue through ONE path:
#   - All radio cues: scores/active/* (auto-discovered) plus the curated
#     EXTRA_RADIO_CUES list (example-tree cues we expose on the radio).
#     Driven via `ambition_music_renderer "$music_collection" render-publish`. Adaptive cues
#     (e.g. first_goblin_tune_v2) are detected and published per-section by the
#     SAME path — the renderer publishes each adaptive/<section>/<section>.full.ogg
#     into game/ambition_content_library/generated/runtime/audio/music/generated/. No dedicated installer.
#
# Usage:
# ./scripts/regen/music.sh                    # render + install everything (default)
# ./scripts/regen/music.sh --skip-render      # only republish from existing renders
# ./scripts/regen/music.sh --force            # force re-render where supported
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#                                   # forwarded to the cue renderer
#   AMBITION_MUSIC_PYTHON=/path/to/python
#                                   # overrides the tool-local .venv
#   AMBITION_MUSIC_JOBS=6             # total cue and stem worker budget
set -euo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"

# shellcheck disable=SC1091
if [ "${AMBITION_PRODUCER_WORKER:-}" != "music" ] && [ "${1:-}" != "--help" ] && [ "${1:-}" != "-h" ]; then
    exec python3 "$repo_root/scripts/generate_library_content.py" music "$@"
fi

source "$repo_root/scripts/lib/tool_python.sh"
# WHERE generated music lands, declared once for every tool that touches it.
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/asset_roots.sh"
music_collection="$(ambition_library_producer music collection)"
case "$music_collection" in radio|sandbox) ;; *) echo "unsupported music collection: $music_collection" >&2; exit 2 ;; esac

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
radio_args+=(--backend "${AMBITION_MUSIC_BACKEND:-pretty-midi}")
radio_args+=(--jobs "${AMBITION_MUSIC_JOBS:-6}")
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

renderer_py="$(ambition_select_tool_python "$renderer_dir" AMBITION_MUSIC_PYTHON)"
ambition_require_python_module \
    "$renderer_py" ambition_music_renderer \
    "run ./run_developer_setup.sh or set AMBITION_MUSIC_PYTHON=/path/to/python"

# The sampled instrument libraries. `render_music.sh` — the renderer's own entry
# point — sources this; this script did not, so the REPO-level regen (the path
# run_developer_setup.sh drives) rendered every cue without the SFZ/LV2/VST3/CLAP
# search paths and quietly took the General-MIDI fallback for anything sampled.
# Same one line, so both entry points resolve the same instruments.
audio_tools_env="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}/env.sh"
if [ -f "$audio_tools_env" ]; then
    echo "==> instrument libraries: $audio_tools_env"
    # shellcheck disable=SC1090
    source "$audio_tools_env"
else
    # ⛔ NOT AN FYI. Rendering without these produces General-MIDI stand-ins
    # that are indistinguishable downstream from the real cues — same .ogg, same
    # registry entry, same playback. The renderer refuses for that reason; this
    # says so up front instead of after the first cue fails.
    echo "==> instrument libraries: NONE at ${audio_tools_env%/env.sh}" >&2
    echo "    music cannot be rendered here: every sampled instrument would" >&2
    echo "    silently become a General-MIDI stand-in." >&2
    echo "    install them with:" >&2
    echo "      ./run_developer_setup.sh                                      # installs them by default" >&2
    echo "      tools/ambition_music_renderer/download_ambition_audio_tools.sh /data/audio-tools" >&2
fi

cue_selection="$("$renderer_py" "$repo_root/scripts/regen_music_registry.py" --list-cues)"
radio_args+=(--cue "$cue_selection")
echo "==> selected library music cues"
if [ "$skip_render" -eq 1 ]; then
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer "$music_collection" publish "${radio_args[@]}")
else
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer "$music_collection" render-publish "${radio_args[@]}")
fi

# Project every published OGG into the in-game music registry so newly
# rendered cues are registered automatically (stdlib-only; no venv needed).
echo "==> music registry (music_registry.ron)"
"$renderer_py" "$repo_root/scripts/regen_music_registry.py" \
    --generated-dir "$AMBITION_MUSIC_PUBLISH_ROOT"

"$renderer_py" "$repo_root/scripts/check_regeneration_coverage.py" --family music

echo "==> done"
