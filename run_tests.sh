#!/usr/bin/env bash
# Ambition test-suite front door. Repo-coupled validation runs headlessly by default; detached
# developer tools, maintenance audits, ignored tests, and exhaustive feature jobs are opt-in.
#
# Common lanes: `--rust`, `--tool-tests`, `--maintenance`, `-p <crate>`, `-k <substr>`, `--list`.
# On a SHARED machine use `-jN` (e.g. `-j5`): it caps cargo build jobs AND test threads, so the
# suite leaves the rest of the cores alone. Uncapped, cargo takes every core.
# Arguments after `--` go to libtest. Use `--heavy` or
# `--run-everything-you-probably-dont-need-this` only when exhaustive coverage is intended.
#
# To observe a running suite, read `target/run_tests_status.json`; do not poll with
# `pgrep -f run_tests.py`, which can match the polling command itself. Job selection is derived from
# Cargo manifests by `scripts/run_tests.py`.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# ⛔ THE BIND CHECK IS NOT HERE ANY MORE, AND THAT IS THE POINT. It used to run
# `target_bindmount.sh --check` on this line, which guarded THIS door only —
# `python3 scripts/run_tests.py` is the door agents use, and it was unguarded.
# The precondition now lives inside `check_disk_headroom.free_gb_on_target()`,
# which every entry point already calls before it decides whether to build, so
# the volume cannot be measured without first being verified. Restoring a copy
# here would put the fact back in two places.

# ⛔ THE SUITE'S PYTHON JOBS NEED THE REPO'S ENVIRONMENT, NOT THE SYSTEM ONE.
# `run_tests.py` launches the goal guard, the absence contracts and the rest as
# `sys.executable -m pytest`, so whatever interpreter starts it decides whether
# those jobs can run at all. A bare `python3` on a fresh clone has no pytest, and
# the runner reports that as two red jobs at 0.0s among twenty green ones rather
# than as a broken environment. `run_developer_setup.sh` provisions this
# environment; the resolver falls back to `python3` when it is absent, which is
# exactly the old behaviour.
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/cargo_env.sh"
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"
scripts_python="$(ambition_select_tool_python "$repo_root" "" 0)"
ambition_python_exists "$scripts_python" || scripts_python=python3

exec "$scripts_python" "$repo_root/scripts/run_tests.py" "$@"
