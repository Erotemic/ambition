#!/usr/bin/env bash
# Ambition test-suite front door. Repo-coupled validation runs headlessly by default; detached
# developer tools, maintenance audits, ignored tests, and exhaustive feature jobs are opt-in.
#
# Common lanes: `--rust`, `--tool-tests`, `--maintenance`, `-p <crate>`, `-k <substr>`, `--list`.
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

exec python3 "$repo_root/scripts/run_tests.py" "$@"
