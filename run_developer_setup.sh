#!/usr/bin/env bash
# Idempotently prepare a fresh checkout for desktop development.
#
# This is an ORCHESTRATOR. Every phase is a script under `scripts/setup/` that
# runs on its own, and this file CALLS them rather than carrying a second copy
# of what they do — so the umbrella and the pieces cannot drift. Run one
# directly when you are fixing one thing:
#
#   scripts/setup/audio_libraries.sh --status   # what instruments this box has
#   scripts/setup/python_tools.sh --verify      # are the tool venvs usable
#   scripts/setup/generated_content.sh          # just re-render the assets
#
# See `scripts/setup/README.md` for the phase table and what each needs first.
#
# ⭐ A DEFAULT RUN IS THE READY-TO-BUILD STATE, AND NOTHING LESS. The only thing
# it leaves out is the profiling/analysis toolchain, which measures the game
# rather than building it:
#
#   --profile           Tracy, cargo-flamegraph, and the cargo analysis tools
#                       (llvm-cov, modules, sweep, mark-sweep, nextest). Costs
#                       ~190 apt packages plus several source builds.
#   --full              the same thing; kept because it reads better.
#
# ⛔ THE SAMPLED INSTRUMENT LIBRARIES ARE NOT OPTIONAL. They were, behind
# `--audio-libraries`, on the theory that without them "every cue still
# renders — a quality difference, not a failure". It is a failure: a default
# clone rendered the ENTIRE catalogue through General MIDI and reported success,
# and nothing downstream can tell that audio from the real thing. Two cues
# (`aether_severance`, `blazingly_fast`) set `render.strict_backends` and could
# not render at all, which is what finally made it visible. Every library is a
# public download. `--skip-audio-libraries` builds a compile-only machine, and
# such a machine cannot regenerate music.
#
# Usage:
#   ./run_developer_setup.sh [--profile] [--full]
#       [--skip-system-packages] [--skip-rust] [--skip-submodules]
#       [--skip-tally] [--skip-python] [--skip-audio-libraries]
#       [--skip-assets] [--skip-cargo-check]
#
# Environment:
#   AMBITION_TOOL_PYTHON=3.12
#   UV_EXCLUDE_NEWER=YYYY-MM-DD
#   AMBITION_AUDIO_TOOLS_ROOT=/data/audio-tools
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

AMBITION_SETUP_LABEL=developer-setup
# shellcheck source=scripts/lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"

setup="$repo_root/scripts/setup"

skip_system_packages=0
skip_rust=0
skip_submodules=0
skip_python=0
skip_audio_libraries=0
skip_assets=0
skip_cargo_check=0
skip_tally=0
want_profiling=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-system-packages) skip_system_packages=1 ;;
        --skip-rust) skip_rust=1 ;;
        --skip-submodules) skip_submodules=1 ;;
        --skip-python) skip_python=1 ;;
        --skip-audio-libraries) skip_audio_libraries=1 ;;
        --skip-assets) skip_assets=1 ;;
        --skip-cargo-check) skip_cargo_check=1 ;;
        --skip-tally) skip_tally=1 ;;
        --profile|--full) want_profiling=1 ;;
        # Accepted so an existing invocation does not break; it is the default.
        --audio-libraries) skip_audio_libraries=0 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

# ⚠ SEQUENTIAL ON PURPOSE. The first four phases are independent of each other,
# but apt takes a machine-wide lock and the asset pipeline already saturates the
# CPU, so running them concurrently buys little and can deadlock on that lock.
# `scripts/setup/README.md` records the dependency order for anyone assembling
# a setup by hand.
phase() {
    local flag="$1" script="$2"
    shift 2
    if [ "$flag" -eq 1 ]; then
        log "skipping ${script%.sh}"
        return 0
    fi
    "$setup/$script" "$@"
}

phase "$skip_system_packages" system_packages.sh
phase "$skip_rust" rust_toolchain.sh
if [ "$want_profiling" -eq 1 ]; then
    "$setup/profiling_tools.sh"
else
    log "profiling toolchain not requested (--profile adds it)"
fi
phase "$skip_tally" resource_tally.sh
phase "$skip_submodules" submodules.sh
phase "$skip_python" python_tools.sh
# Before the assets: `scripts/regen/music.sh` sources this phase's `env.sh` to
# find the instruments, so installing them afterwards would render the fallback.
phase "$skip_audio_libraries" audio_libraries.sh
phase "$skip_assets" generated_content.sh
phase "$skip_cargo_check" desktop_check.sh

echo
if [ "$skip_assets" -eq 0 ] && [ "$skip_cargo_check" -eq 0 ] && [ "$skip_audio_libraries" -eq 0 ]; then
    log "developer setup complete"
    log "the checkout is ready for: ./run_game.sh"
else
    log "selected developer setup phases complete"
    log "rerun without skip flags for the zero-to-runnable setup"
fi
# Plain `if`s, not `[ ... ] && log`: these are the last statements in the
# script, and a short-circuit whose test is false would make a successful setup
# exit non-zero.
if [ "$want_profiling" -eq 0 ]; then
    log "not installed (one argument away):"
    log "   --profile          profiling + cargo analysis toolchain"
fi
if [ "$skip_audio_libraries" -eq 1 ]; then
    warn "sampled instruments were SKIPPED; this machine cannot render music"
    log "   rerun without --skip-audio-libraries before regenerating it"
fi
