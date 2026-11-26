#!/usr/bin/env bash
# Install the sampled instrument libraries and the sfizz/LV2 hosts that play
# them, into $AMBITION_AUDIO_TOOLS_ROOT (default /data/audio-tools).
#
# Usage:
#   scripts/setup/audio_libraries.sh
#   scripts/setup/audio_libraries.sh --status   # what is installed here
#   scripts/setup/audio_libraries.sh --help
#
# Environment:
#   AMBITION_AUDIO_TOOLS_ROOT=/data/audio-tools
#   MODE=starter|pro|all        # forwarded to the downloader; `pro` is default
#   PLUGINS=0                   # skip the CLAP/VST3/LV2 plugin bundles
#
# ⛔⛔ THIS IS NOT AN EXTRA. Without these, every sampled instrument renders as a
# General-MIDI stand-in that is indistinguishable downstream — same .ogg, same
# registry entry, same playback — so a machine missing them ships the wrong
# music and reports success. The shipped cues name 54 distinct library
# references across a dozen families, plus `kind: lv2` plugin backends, so
# `MODE=starter` is NOT sufficient for the catalogue: it omits the Salamander
# Grand that twenty cue instruments resolve to.
#
# ⚠ It is large — tens of GB — and slow on a spinning disk. That is the price of
# the real instruments; `run_developer_setup.sh --skip-audio-libraries` is for a
# machine that only needs to compile the game, and such a machine cannot
# regenerate music.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=audio-libraries
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"

# shellcheck source=../lib/tool_python.sh
. "$repo_root/scripts/lib/tool_python.sh"

show_status=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --status) show_status=1 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

# What this machine actually has, without installing anything. The renderer's
# own resolver answers it, because "is there an .sfz tree" is its question.
report_status() {
    local root="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}"
    log "root      : $root"
    log "env.sh    : $([ -f "$root/env.sh" ] && echo present || echo MISSING)"
    log "sfizz     : $(command -v sfizz_render || echo MISSING)"
    local py
    py="$(ambition_select_tool_python "$repo_root/tools/ambition_music_renderer" AMBITION_MUSIC_PYTHON)"
    if ambition_python_exists "$py"; then
        ( cd "$repo_root/tools/ambition_music_renderer" && "$py" -c "
from ambition_music_renderer.instrument_libraries import discover_sfz_files
print(f'[audio-libraries] sfz files : {len(discover_sfz_files())}')
" 2>/dev/null ) || log "sfz files : (renderer not importable)"
    else
        log "sfz files : (no music-renderer interpreter)"
    fi
    # ⛔ A COUNT IS NOT COVERAGE, and saying so here is the point. A box can hold
    # thousands of `.sfz` files and still be missing a family the cues NAME —
    # and then only those cues render through General MIDI, while every other
    # cue is correct, so the run looks healthy. The number above cannot see
    # that; resolving the catalogue's references can, and the renderer's own
    # bulk preflight does it before any cue is written.
    log "note      : a file COUNT is not catalogue coverage — a render preflight"
    log "            resolves every library the cues name and refuses on a miss"
}

if [ "$show_status" -eq 1 ]; then
    report_status
    exit 0
fi

# The sampled instrument libraries, and the sfizz/LV2 hosts that can actually
# play them. These are dependencies of the shipped music, not a quality extra:
# the renderer refuses to render a cue whose named library is missing (see the
# header above), so a machine without them cannot regenerate the catalogue.
#
# The downloader writes `$root/env.sh`, which is what `scripts/regen/music.sh` and
# `render_music.sh` source to expose the SFZ/LV2/VST3/CLAP search paths.
ensure_audio_libraries() {
    local root renderer
    root="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}"
    renderer="$repo_root/tools/ambition_music_renderer"

    [ -x "$renderer/setup.sh" ] || fatal "music renderer is missing. Run scripts/setup/submodules.sh."

    if [ ! -d "$root" ] && ! mkdir -p "$root" 2>/dev/null; then
        if [ "$root" = /data/audio-tools ] && have sudo; then
            sudo mkdir -p "$root" && sudo chown "$(id -u):$(id -g)" "$root"
        fi
    fi
    [ -w "$root" ] || fatal "$root is not writable. Set AMBITION_AUDIO_TOOLS_ROOT to a directory you can write to."

    log "installing the native audio toolchain (sfizz, LV2/VST3 hosts)"
    setup_limit_jobs
    CMAKE_BUILD_PARALLEL_LEVEL="$AMBITION_SETUP_JOBS" "$renderer/setup.sh" --native-only \
        || fatal "native audio setup failed"

    log "downloading sampled instrument libraries into $root (this is large)"
    "$renderer/download_ambition_audio_tools.sh" "$root" \
        || fatal "instrument library setup failed. See $root/MANUAL_DOWNLOADS.md and the download log above."
}

ensure_audio_libraries
report_status
