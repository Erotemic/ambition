#!/usr/bin/env bash
# Validate the sandbox world, then step the headless sim shell.
#
# Arguments are passed through to the binary, e.g.:
#   ./run_headless.sh --ticks 600
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

# ⚠ ASK THE RESOLVER, DO NOT SET `PYTHONPATH` AND HOPE. This ran a bare `python`
# with the package directory on the path, which fails two ways on a fresh
# checkout: Ubuntu ships no `python`, and the package's own dependencies
# (`python-ron`) live in the tool venv the path hack bypasses. `run_game.sh`
# validates the same world through the same seam.
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/cargo_env.sh"
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"
python_bin="$(ambition_select_tool_python \
    "$repo_root/tools/ambition_ldtk_tools" AMBITION_LDTK_PYTHON)"
ambition_require_python_module "$python_bin" ambition_ldtk_tools \
    "run ./run_developer_setup.sh to create the authoring environments"

"$python_bin" -m ambition_ldtk_tools validate \
    game/ambition_content/assets/worlds/sandbox.ldtk
# `--` first: everything after it is the BINARY's, not cargo's.
RUST_BACKTRACE=1 cargo run -p ambition_app_tools --bin headless --release -- "$@"
