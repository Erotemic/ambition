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
#   ./run_developer_setup.sh [--profile] [--full] [--skip-update]
#       [--skip-system-packages] [--skip-rust] [--skip-submodules]
#       [--skip-tally] [--skip-python] [--skip-audio-libraries]
#       [--assets-only] [--skip-assets] [--skip-cargo-build]
#
#   --assets-only       prepare the asset tools and update runtime assets
#                       without building the desktop game
#   --skip-update       use the current superproject revision
#
# Environment:
#   AMBITION_SETUP_JOBS=1
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

setup_arguments=("$@")
skip_update=0
skip_system_packages=0
skip_rust=0
skip_submodules=0
skip_python=0
skip_audio_libraries=0
skip_assets=0
skip_cargo_build=0
assets_only=0
skip_tally=0
want_profiling=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-system-packages) skip_system_packages=1 ;;
        --skip-update) skip_update=1 ;;
        --skip-rust) skip_rust=1 ;;
        --skip-submodules) skip_submodules=1 ;;
        --skip-python) skip_python=1 ;;
        --skip-audio-libraries) skip_audio_libraries=1 ;;
        --skip-assets) skip_assets=1 ;;
        --skip-cargo-build|--skip-cargo-check) skip_cargo_build=1 ;;
        --assets-only) assets_only=1; skip_cargo_build=1 ;;
        --skip-tally) skip_tally=1 ;;
        --profile|--full) want_profiling=1 ;;
        # Accepted so an existing invocation does not break; it is the default.
        --audio-libraries) skip_audio_libraries=0 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done
if [ "$assets_only" -eq 1 ] && [ "$skip_assets" -eq 1 ]; then
    fatal "--assets-only requires asset generation"
fi

update_checkout() {
    local upstream remote before
    [ "$(git symbolic-ref --quiet --short HEAD)" = main ] || return 0
    if ! upstream="$(git rev-parse --symbolic-full-name '@{upstream}' 2>/dev/null)"; then
        log "main has no upstream. Using the current revision."
        return 0
    fi
    remote="$(git config --get branch.main.remote)"
    log "updating main from ${upstream#refs/remotes/}"
    git fetch --no-recurse-submodules "$remote"
    before="$(git rev-parse HEAD)"
    git -c submodule.recurse=false merge --ff-only --no-autostash "$upstream" \
        || fatal "main could not be fast-forwarded. Resolve the Git state or use --skip-update."
    if [ "$before" != "$(git rev-parse HEAD)" ]; then
        # Read the setup script from the new revision.
        AMBITION_SETUP_ALREADY_UPDATED=1 exec "$repo_root/run_developer_setup.sh" "$@"
    fi
}

if [ "$skip_update" -eq 0 ] && [ "${AMBITION_SETUP_ALREADY_UPDATED:-0}" -eq 0 ]; then
    update_checkout "${setup_arguments[@]}"
fi
unset AMBITION_SETUP_ALREADY_UPDATED
setup_limit_jobs
log "using up to $AMBITION_SETUP_JOBS worker(s) per build phase"

# Run phases in order. apt takes a machine-wide lock, and the asset pipeline
# uses the available CPU capacity.
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
setup_load_cargo_env
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$AMBITION_SETUP_JOBS}"
if [ "$skip_cargo_build" -eq 0 ] || [ "$want_profiling" -eq 1 ]; then
    "$setup/native_linker.sh" --install
    "$setup/target_bindmount.sh"
fi
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
phase "$skip_cargo_build" desktop_check.sh

echo
if [ "$skip_assets" -eq 0 ] && [ "$skip_cargo_build" -eq 0 ]; then
    log "developer setup complete"
    log "run the desktop build with: ./run_game.sh dev"
elif [ "$assets_only" -eq 1 ]; then
    log "runtime assets are current"
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
