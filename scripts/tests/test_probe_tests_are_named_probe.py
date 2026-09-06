"""Every print-only or panicking `#[ignore]` test must be named `probe_*`.

⛔ WHY THIS GUARD EXISTS. `./run_tests.sh --heavy` runs
`cargo test --workspace -- --include-ignored --skip probe_`. The `--skip` is the
only thing keeping the lane from being RED BY DESIGN: `#[ignore]` in this repo
means three unrelated things, and two of them are not "slow".

* PROBES panic or print instead of asserting. `d71_transaction_census`'s two end
  in `panic!` to report a census — verified 2026-09-03 by running one with
  `--ignored`: `test result: FAILED. 0 passed; 1 failed`. Before the `--skip`,
  the plan whose job is to run everything could not pass, so its red carried no
  information and a real regression inside it was indistinguishable from a probe
  doing its job.
* Some tests are VALID ONLY ALONE. Those are handled by `--test-threads=1` in
  the same job, not by this rule.

⇒ The `--skip probe_` filter is a STRING. Nothing else stops a new
"print-only" test from being called `list_…` or `dump_…` and quietly turning
the lane red again. This test is what makes the naming a rule rather than a
habit.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# An `#[ignore = "..."]` whose REASON says the test prints or panics rather than
# asserting. Deliberately matched on the author's own words: the reason is where
# the intent is written down.
PROBE_WORDS = ("panic", "print-only", "print only", "read it", "audit listing", "probe")

IGNORE_THEN_FN = re.compile(
    r'#\[ignore\s*=\s*"((?:[^"\\]|\\.)*)"\]\s*(?:\n\s*)*(?:pub\s+)?fn\s+([a-z_0-9]+)',
    re.S,
)


def _rust_sources() -> list[Path]:
    out: list[Path] = []
    for root in ("crates", "game"):
        out.extend((REPO / root).rglob("*.rs"))
    return out


def probe_ignores() -> list[tuple[Path, str, str]]:
    """(file, test name, reason) for every ignore whose reason reads probe-ish."""
    found: list[tuple[Path, str, str]] = []
    for path in _rust_sources():
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in IGNORE_THEN_FN.finditer(text):
            reason = " ".join(match.group(1).split())
            if any(word in reason.lower() for word in PROBE_WORDS):
                found.append((path, match.group(2), reason))
    return found


def test_the_repo_still_has_probe_tests_to_check():
    """Premise. A zero-length list would make the assertion below vacuous."""
    found = probe_ignores()
    assert len(found) >= 5, (
        "expected several print-only/panicking #[ignore] tests; found "
        f"{len(found)}. If they are genuinely gone, delete this guard and the "
        "`--skip probe_` in run_tests.py's heavy job together — a filter with "
        "nothing to filter is how the next one slips back in."
    )


def test_every_printing_or_panicking_ignored_test_is_named_probe():
    offenders = [
        (path.relative_to(REPO), name, reason)
        for path, name, reason in probe_ignores()
        if not name.startswith("probe_")
    ]
    assert not offenders, (
        "these #[ignore]d tests say they print or panic rather than assert, but "
        "are not named `probe_*`, so `--heavy`'s `--skip probe_` will RUN them "
        "and the lane goes red for a test doing its job:\n"
        + "\n".join(f"  {p}::{n} — {r}" for p, n, r in offenders)
    )


def test_the_heavy_plan_still_skips_probes():
    """The naming only helps while the plan actually filters on it."""
    plan = (REPO / "scripts" / "run_tests.py").read_text(encoding="utf-8")
    assert '"--skip", "probe_"' in plan, (
        "run_tests.py's heavy job no longer passes `--skip probe_`; the probes "
        "will run and panic. See this file's module docstring."
    )
    assert '"--test-threads=1"' in plan, (
        "run_tests.py's heavy job no longer forces --test-threads=1; the "
        "isolation-required ignored tests (parallax_theme_retires_on_walk, "
        "hall_redecode_census) will pass VACUOUSLY beside their siblings."
    )
