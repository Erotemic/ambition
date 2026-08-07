#!/usr/bin/env bash
# Re-render and republish all in-game music cues.
#
# Covers every in-game cue through ONE path:
#   - All radio cues: scores/active/* (auto-discovered) plus the curated
#     EXTRA_RADIO_CUES list (example-tree cues we expose on the radio).
#     Driven via `ambition_music_renderer radio render-publish`. Adaptive cues
#     (e.g. first_goblin_tune_v2) are detected and published per-section by the
#     SAME path — the renderer publishes each adaptive/<section>/<section>.full.ogg
#     into crates/ambition_platformer2d_actor_monolith/assets/audio/music/generated/. No dedicated installer.
#
# Usage:
#   ./regen_music.sh                    # render + install everything (default)
#   ./regen_music.sh --skip-render      # only republish from existing renders
#   ./regen_music.sh --force            # force re-render where supported
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#                                   # forwarded to the cue renderer
#   AMBITION_MUSIC_PYTHON=/path/to/python
#                                   # overrides the tool-local .venv
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

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

renderer_py="$(ambition_select_tool_python "$renderer_dir" AMBITION_MUSIC_PYTHON)"
ambition_require_python_module \
    "$renderer_py" ambition_music_renderer \
    "run ./run_developer_setup.sh or set AMBITION_MUSIC_PYTHON=/path/to/python"

echo "==> radio cues (scores/active/* + EXTRA_RADIO_CUES; adaptive cues per-section)"
if [ "$skip_render" -eq 1 ]; then
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer radio publish)
else
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer radio render-publish "${radio_args[@]}")
fi

# **DID THE RENDERER PUBLISH WHERE THIS REPO READS?**
#
# The renderer is a SUBMODULE and it hardcodes the consumer crate's name — its
# repo-root probe, its publish target and its audit root all name one directory.
# On 2026-08-07 the submodule was ahead of this repo on the
# `gameplay_core -> actors` rename (its `60bf470`), so `radio render-publish`
# happily created `crates/ambition_actors/assets/...`, wrote 69 cues into it, and
# exited 0 — while `regen_music_registry.py` read the crate this repo actually
# has and reported "wrote registry" with the new tracks silently missing.
#
# Both halves were working correctly and disagreeing, which is the worst kind of
# green. So: say it, rather than leave the next person to notice that a track
# they just rendered is not in the game.
consumed_dir="$repo_root/crates/ambition_platformer2d_actor_monolith/assets/audio/music/generated"
for stray in "$repo_root"/crates/*/assets/audio/music/generated; do
    [ -d "$stray" ] || continue
    [ "$stray" = "$consumed_dir" ] && continue
    echo "error: the renderer published to a directory this repo does not read:" >&2
    echo "         published: $stray" >&2
    echo "         consumed:  $consumed_dir" >&2
    echo "       The music submodule names the consumer crate in four places" >&2
    echo "       (_paths.py, cli.py, render/bundle_base.py, audit/level_report.py)." >&2
    echo "       Either this repo's crate rename has not happened yet, or the" >&2
    echo "       submodule is pinned to a version that expects a different one." >&2
    echo "       ⚠ the cues ARE rendered — nothing was lost. They are in the" >&2
    echo "       directory above, and this only refuses to pretend the game can" >&2
    echo "       see them." >&2
    exit 3
done

# Project every published OGG into the in-game music registry so newly
# rendered cues are registered automatically (stdlib-only; no venv needed).
echo "==> music registry (music_registry.ron)"
"$renderer_py" "$repo_root/scripts/regen_music_registry.py"

echo "==> done"
