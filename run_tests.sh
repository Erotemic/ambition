#!/usr/bin/env bash
# Ambition test suite -- pytest-like front door. Runs repo-coupled validation
# headlessly by default; detached developer-tool, periodic maintenance, and
# heavy/diagnostic tests are opt-in.
#
#   ./run_tests.sh              BACKBONE: python suites + cargo test --workspace
#   ./run_tests.sh --rust       Rust/Cargo lane only; no Python checkers
#   ./run_tests.sh --tool-tests detached developer-tool tests only
#   ./run_tests.sh --maintenance periodic repository-hygiene audits only
#   ./run_tests.sh -p <crate>   only that crate's job (repeatable)
#   ./run_tests.sh -k <substr>  only tests whose name contains <substr>
#   ./run_tests.sh --list       print the job plan, run nothing
#   ./run_tests.sh -- --nocapture   args after `--` go to libtest
#   ./run_tests.sh --run-everything-you-probably-dont-need-this
#                               the 33-job exhaustive plan (~1 HOUR)
#   ./run_tests.sh --heavy      ALSO #[ignore]d tests + app acceptance cycles;
#                               implies the exhaustive plan
#
# The default is deliberately not exhaustive (Jon, 2026-08-02): the exhaustive
# plan spends ~63 minutes to execute ~4 minutes of tests, and being the default
# made it the thing an agent ran instead of the focused test that would have
# answered the question. Every non-exhaustive run prints what it did not cover.
#
# Waiting for a run: read target/run_tests_status.json ("running"/"done"/
# "crashed"), NOT `pgrep -f run_tests.py` -- that matches the polling shell
# itself and hangs forever. See the module docstring in scripts/run_tests.py.
#
# The job plan (which crates run with which features) is computed from the
# Cargo manifests in scripts/run_tests.py, so it can't drift as features change.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$repo_root/scripts/run_tests.py" "$@"
