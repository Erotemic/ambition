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

want_audio_libraries=1
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
    # shellcheck source=../lib/tool_python.sh
    . "$repo_root/scripts/lib/tool_python.sh"
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
# play them.
#
# ⛔ THE RENDERER IS NOT BROKEN WITHOUT THESE. `render/group.py` warns per
# instrument and falls back to General MIDI, so a default checkout still renders
# every cue — the GM soundfonts are in the required package list precisely so it
# can. What the libraries buy is quality, at the cost of gigabytes, which is why
# they are opt-in rather than part of the fast path.
#
# The downloader writes `$root/env.sh`, which is what `scripts/regen/music.sh` and
# `render_music.sh` source to expose the SFZ/LV2/VST3/CLAP search paths.
ensure_audio_libraries() {
    local root renderer
    root="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}"
    renderer="$repo_root/tools/ambition_music_renderer"

    if [ "$want_audio_libraries" -eq 0 ]; then
        if [ -f "$root/env.sh" ]; then
            log "sampled instrument libraries already present at $root"
        else
            log "sampled instruments not requested (--audio-libraries adds them);"
            log "   music renders through the General-MIDI fallback until then"
        fi
        return 0
    fi

    if [ ! -x "$renderer/setup.sh" ]; then
        warn "$renderer/setup.sh is missing; skipping the audio toolchain"
        return 0
    fi

    # sfizz first: without a player, a downloaded SFZ library is inert.
    #
    # INSTALL_SFIZZ_OBS defaults to 0 in the renderer's setup because it adds a
    # third-party apt source, and Ubuntu does not package sfizz at all — so with
    # the default the SFZ libraries below install and then nothing can play
    # them. Asking for --audio-libraries IS asking for the SFZ path, so opt in
    # here rather than leaving the flag half-effective.
    log "installing the native audio toolchain (sfizz, LV2/VST3 hosts)"
    INSTALL_SFIZZ_OBS=1 "$renderer/setup.sh" \
        || warn "audio toolchain setup reported a failure; continuing"

    # /data is root-owned on a fresh box and the downloader does not escalate,
    # so its first write would fail on a path the developer never chose.
    #
    # Test writability, NOT `mkdir -p`: mkdir -p SUCCEEDS on a directory that
    # already exists no matter who owns it, so gating the escalation on its exit
    # status skipped the chown on exactly the boxes that needed it.
    if [ ! -w "$root" ] && have sudo; then
        log "making $root writable for $(whoami)"
        sudo mkdir -p "$root" && sudo chown "$(id -u):$(id -g)" "$root" || true
    fi
    if [ ! -w "$root" ]; then
        warn "$root is not writable; skipping the library download"
        warn "set AMBITION_AUDIO_TOOLS_ROOT to a path you own and rerun"
        return 0
    fi

    log "downloading sampled instrument libraries into $root (this is large)"
    "$renderer/download_ambition_audio_tools.sh" "$root" \
        || warn "instrument library download reported a failure; cues will use the fallback"
}

ensure_audio_libraries
report_status
