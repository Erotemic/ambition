#!/usr/bin/env bash
# Shared plumbing for the setup phases under `scripts/setup/`.
#
# ⭐ EACH PHASE IS A SCRIPT YOU CAN RUN ON ITS OWN, and `run_developer_setup.sh`
# is the orchestrator that runs them in order. It CALLS them rather than
# carrying its own copy, so the two cannot drift — which is the whole reason the
# phases were lifted out of it. A contributor who wants only the instruments, or
# only the Python environments, runs that one script.
#
# ⚠ Sourced, never executed. `AMBITION_SETUP_LABEL` names the phase in its
# output so a line is attributable when the orchestrator runs them all.

AMBITION_SETUP_LABEL="${AMBITION_SETUP_LABEL:-setup}"

log() {
    printf '[%s] %s\n' "$AMBITION_SETUP_LABEL" "$*"
}

warn() {
    printf '[%s] warning: %s\n' "$AMBITION_SETUP_LABEL" "$*" >&2
}

fatal() {
    printf '[%s] error: %s\n' "$AMBITION_SETUP_LABEL" "$*" >&2
    exit 1
}

have() {
    command -v "$1" >/dev/null 2>&1
}

setup_limit_jobs() {
    local cpus memory_jobs
    if [ -z "${AMBITION_SETUP_JOBS:-}" ]; then
        cpus="$(getconf _NPROCESSORS_ONLN 2>/dev/null || printf '1')"
        memory_jobs=1
        if [ -r /proc/meminfo ]; then
            memory_jobs="$(awk '/^MemAvailable:/ { jobs = int($2 / 2097152); print (jobs > 0 ? jobs : 1); exit }' /proc/meminfo)"
        fi
        AMBITION_SETUP_JOBS=$(( cpus < memory_jobs ? cpus : memory_jobs ))
    fi
    [[ "$AMBITION_SETUP_JOBS" =~ ^[1-9][0-9]*$ ]] || fatal "AMBITION_SETUP_JOBS must be a positive integer"
    export AMBITION_SETUP_JOBS
}

# The usage block is the file's own header comment, so `--help` cannot describe
# a script that has since changed underneath it.
setup_usage() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$1"
}

# ⛔ RUSTUP EDITS A PROFILE THE RUNNING SHELL HAS ALREADY READ. A phase that
# needs cargo in the same session that installed it has to source this itself.
setup_load_cargo_env() {
    if ! have cargo && [ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]; then
        # shellcheck disable=SC1091
        . "${CARGO_HOME:-$HOME/.cargo}/env"
    fi
}
