#!/usr/bin/env bash
# Ambition test suite -- pytest-like front door. Runs everything that can run
# headlessly by default; heavy/diagnostic tests are #[ignore]d and opt-in.
#
#   ./run_tests.sh              full headless suite (excludes #[ignore])
#   ./run_tests.sh --heavy      ALSO run #[ignore]d tests + app acceptance cycles
#   ./run_tests.sh --list       print the job plan, run nothing
#   ./run_tests.sh -k <substr>  only tests whose name contains <substr>
#   ./run_tests.sh -p <crate>   only that crate's job (repeatable)
#   ./run_tests.sh --fast       backbone only: cargo test --workspace
#   ./run_tests.sh -- --nocapture   args after `--` go to libtest
#
# The job plan (which crates run with which features) is computed from the
# Cargo manifests in scripts/run_tests.py, so it can't drift as features change.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$repo_root/scripts/run_tests.py" "$@"
