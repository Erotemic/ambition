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
exec python3 "$repo_root/scripts/run_tests.py" "$@"
