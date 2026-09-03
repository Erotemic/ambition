"""The doc-link ratchet must not advise banking a rise it did not show you.

`check_doc_link_ratchet.py` runs `cargo doc` over nine crates and compares
broken-intra-doc-link counts against a baseline. Until 2026-09-03 its two
messages were asymmetric, and the asymmetry pointed one way:

    "⭐ … improved — run --update to bank it"   printed ALWAYS
    "⛔ N crate(s) gained broken doc links"      printed only under --check

⇒ A plain run showed "ROSE" in its table, said nothing further, exited 0, and
advised `--update` — which rewrites EVERY count and would bank the regressions
as the new normal. That is how `ambition_characters` 24→26 and
`ambition_platformer2d_core` 34→35 sat unread: this guard is also CI-only
(`.github/workflows/test.yml`), so nothing local ran it either.

⚠ `measure()` is monkeypatched throughout. A test that really ran `cargo doc`
over nine crates would take minutes and would be testing rustdoc, not the
reporting this file is about.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_doc_link_ratchet.py"


def load():
    spec = importlib.util.spec_from_file_location("ratchet", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture
def rig(tmp_path, monkeypatch):
    """The module with a fake `measure` and a baseline in tmp_path."""
    module = load()
    baseline = tmp_path / "baseline.json"
    monkeypatch.setattr(module, "BASELINE", str(baseline))
    monkeypatch.setattr(module, "CRATES", ["alpha", "beta"])

    def configure(counts: dict[str, int], recorded: dict[str, int]):
        baseline.write_text(json.dumps({"crates": recorded}))
        monkeypatch.setattr(
            module,
            "measure",
            lambda crate: (counts[crate], "Documenting x\nFinished"),
        )
    return module, configure, baseline


def run(module, argv):
    monkey = sys.argv
    sys.argv = ["check_doc_link_ratchet.py", *argv]
    try:
        return module.main()
    finally:
        sys.argv = monkey


def test_a_rise_is_reported_without_check(rig, capsys):
    """⭐ THE FIX. A plain run must name the regression."""
    module, configure, _ = rig
    configure({"alpha": 5, "beta": 1}, {"alpha": 3, "beta": 1})
    run(module, [])
    out = capsys.readouterr().out
    assert "ROSE from 3" in out
    assert "gained broken doc links" in out, (
        "a run that shows ROSE in its table and then says nothing about it is "
        "why two regressions went unread"
    )


def test_a_rise_and_a_fall_together_withhold_the_update_advice(rig, capsys):
    """⛔ THE DANGEROUS CASE. `--update` rewrites EVERY count, so advising it
    while a rise is outstanding banks the rise."""
    module, configure, _ = rig
    configure({"alpha": 5, "beta": 0}, {"alpha": 3, "beta": 1})
    run(module, [])
    out = capsys.readouterr().out
    assert "DO NOT --update YET" in out
    assert "run --update to bank it" not in out


def test_a_clean_fall_still_advises_banking(rig, capsys):
    """⛔ THE PREMISE for the test above: with no rise, the advice is correct
    and must survive. Otherwise the fix could be 'never advise anything'."""
    module, configure, _ = rig
    configure({"alpha": 2, "beta": 1}, {"alpha": 3, "beta": 1})
    run(module, [])
    out = capsys.readouterr().out
    assert "run --update to bank it" in out
    assert "DO NOT --update YET" not in out


def test_check_exits_nonzero_on_a_rise(rig):
    module, configure, _ = rig
    configure({"alpha": 5, "beta": 1}, {"alpha": 3, "beta": 1})
    assert run(module, ["--check"]) == 1


def test_check_exits_zero_when_nothing_rose(rig):
    module, configure, _ = rig
    configure({"alpha": 3, "beta": 1}, {"alpha": 3, "beta": 1})
    assert run(module, ["--check"]) == 0


def test_a_crate_that_produced_no_rustdoc_output_is_not_scored_zero(rig, capsys):
    """⛔ The module's own guard: a doc build that FAILED emits no warnings, and
    zero warnings from a build that did not happen is not a score."""
    module, configure, _ = rig
    configure({"alpha": 0, "beta": 0}, {"alpha": 3, "beta": 1})
    # no "Documenting"/"Finished" in the output: the build did not happen
    object.__setattr__(module, "measure", lambda crate: (0, ""))
    assert run(module, []) == 1
    assert "produced no rustdoc output at all" in capsys.readouterr().out


def test_the_tracked_crates_all_exist(rig):
    """⛔ A crate renamed out from under this list scores 0 forever, and 0 is
    the best possible score."""
    module, _, _ = rig
    real = load()
    declared = {
        line.split('"')[1]
        for tom in subprocess.run(
            ["git", "ls-files", "*Cargo.toml"], cwd=REPO, capture_output=True, text=True
        ).stdout.split()
        for line in (REPO / tom).read_text(errors="replace").splitlines()
        if line.startswith("name = ")
    }
    missing = [c for c in real.CRATES if c not in declared]
    assert not missing, f"tracked crates that no longer exist: {missing}"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
