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

# ⛔⛔ THE TARGET DIRECTORY MUST BE SHADOWED ONTO LOCAL DISK BEFORE ANY OF THIS
# BUILDS. On a virtiofs checkout an unbound `target/` costs minutes per job and
# grows a second copy of every artifact under the mount point, and NOTHING else
# in the run says so — the suite just gets slower and the disk quietly fills.
# Checked here rather than in a doc because a doc is what got skipped: an agent
# built all day unbound on 2026-08-27, was asked to reclaim the space, and
# deleted 205GB of the live target instead of restoring the mount.
#
# ⛔ Refuse rather than warn. A warning at the top of a job that prints for
# several minutes is a warning nobody reads, and the fix is one command.
"$repo_root/scripts/setup/target_bindmount.sh" --check

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
