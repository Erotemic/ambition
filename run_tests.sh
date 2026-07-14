#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"

run() {
    printf '\n==> '
    printf '%q ' "$@"
    printf '\n'
    "$@"
}

printf 'Ambition full test and headless acceptance suite\n'
printf 'Repository: %s\n' "$repo_root"

# Default-feature coverage for every workspace member.
run cargo test --workspace

# Feature-gated suites that are not covered by the default workspace run.
run cargo test -p ambition_audio --features kira
run cargo test -p ambition_game_shell --features basic_presentation
run cargo test -p ambition_load_presentation --features basic_presentation
run cargo test -p ambition_demo_sanic_app --features visible
run cargo test -p ambition_demo_smb1_app --features visible

# Shared-host shell and lifecycle acceptance suites.
run cargo test -p ambition_app --features rl_sim --test shell_host_startup
run cargo test -p ambition_app --features rl_sim --test shell_host_lifecycle
run cargo test -p ambition_app --features rl_sim --test shell_host_rendered
run cargo test -p ambition_app --test shell_host_headless_entrypoint

# Exercise the shipping entrypoint rather than a test-only app constructor.
run ./run_game.sh -- --headless-acceptance-cycle
run ./run_game.sh -- --headless --headless-ticks 120

printf '\nAll Ambition tests and headless acceptance checks passed.\n'
