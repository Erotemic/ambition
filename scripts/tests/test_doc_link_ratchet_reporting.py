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

    def links(crate: str, n: int, offset: int = 0) -> list[str]:
        """`n` distinct broken-link identities for `crate`, in the real shape."""
        return sorted(
            f"crates/{crate}/src/lib.rs: unresolved `Item{i + offset}`"
            for i in range(n)
        )

    def configure(counts: dict[str, int], recorded: dict[str, int]):
        """Counts in, identity lists out — the baseline and the measurement agree
        on WHICH links are broken, so a plain count change is the same story it
        always was."""
        baseline.write_text(
            json.dumps({"crates": {c: links(c, n) for c, n in recorded.items()}})
        )
        monkeypatch.setattr(
            module,
            "measure",
            # ⛔ THE THIRD MEMBER IS CARGO'S EXIT STATUS, and it is part of the
            # measurement: see `measure`'s own comment. The evidence line has to
            # name the CRATE now, because the acceptance is per-crate.
            lambda crate: (
                links(crate, counts[crate]),
                f"Documenting {crate}\nFinished",
                0,
            ),
        )

    configure.links = links
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
    assert "NEW (was 3, now 5)" in out
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
    # nothing naming this crate in the output: the build did not happen
    object.__setattr__(module, "measure", lambda crate: ([], "", 0))
    assert run(module, []) == 1
    assert "produced no rustdoc output at all" in capsys.readouterr().out


def test_finished_without_this_crate_is_not_evidence(rig, capsys):
    """⛔⛤ **THE ARM THAT WAS TOO WEAK, MEASURED 2026-09-18.** The acceptance
    was `"Documenting" in output or "Finished" in output`, over ONE crate's
    output. `Finished` prints whether or not anything was documented, so a run
    that did nothing for this crate still cleared the arm — which is how thirteen
    crates read zero and printed *"repaired"*."""
    module, configure, _ = rig
    configure({"alpha": 0, "beta": 0}, {"alpha": 3, "beta": 1})
    object.__setattr__(
        module, "measure", lambda crate: ([], "    Finished `dev` profile", 0)
    )
    assert run(module, []) == 1
    assert "produced no rustdoc output at all" in capsys.readouterr().out


def test_a_warm_run_that_only_replays_is_still_a_measurement(rig, capsys):
    """⭐ AND THE CONTROL, because the arm above must not refuse a real run.
    A fresh doc unit prints no `Documenting` line and cargo replays its cached
    diagnostics under `Generated .../doc/<crate>/index.html` — measured, warm,
    on `ambition_body_seed`: one warning, no `Documenting`."""
    module, configure, _ = rig
    configure({"alpha": 3, "beta": 1}, {"alpha": 3, "beta": 1})
    recorded = {
        crate: configure.links(crate, n) for crate, n in {"alpha": 3, "beta": 1}.items()
    }
    object.__setattr__(
        module,
        "measure",
        lambda crate: (
            recorded[crate],
            f"   Generated /repo/target/doc/{crate}/index.html",
            0,
        ),
    )
    assert run(module, ["--check"]) == 0
    assert "produced no rustdoc output" not in capsys.readouterr().out


def test_a_cargo_doc_that_failed_is_refused_rather_than_scored(rig, capsys):
    """⛔ A NON-ZERO EXIT IS NOT AN EMPTY WARNING LIST. The status was discarded,
    so a failed build scored zero and read as a total repair."""
    module, configure, _ = rig
    configure({"alpha": 0, "beta": 0}, {"alpha": 3, "beta": 1})
    object.__setattr__(
        module,
        "measure",
        lambda crate: ([], "error: could not compile `alpha`", 101),
    )
    assert run(module, []) == 1
    out = capsys.readouterr().out
    assert "`cargo doc` FAILED" in out
    # and it must NAME the failure, because the operator has to act on it
    assert "could not compile" in out


def test_every_crate_at_zero_against_a_banked_baseline_is_refused(rig, capsys):
    """⛔⛔ THE SHAPE OF 2026-09-18: `TOTAL 0`, a *"repaired"* mark on every row,
    and exit 0. Even with cargo exiting 0 and naming each crate, a simultaneous
    repair of every tracked crate is an instrument failure, not a landing."""
    module, configure, _ = rig
    configure({"alpha": 0, "beta": 0}, {"alpha": 3, "beta": 1})
    object.__setattr__(
        module, "measure", lambda crate: ([], f"Documenting {crate}\nFinished", 0)
    )
    assert run(module, []) == 1
    assert "every tracked crate measured ZERO" in capsys.readouterr().out


def test_a_deliberate_universal_repair_can_still_be_banked(rig, capsys):
    """⭐ AND ITS ESCAPE HATCH, so the guard cannot forbid its own remedy — the
    lesson the stale-baseline arm above already learned. `--update` is a human
    act and is exempt."""
    module, configure, baseline = rig
    configure({"alpha": 0, "beta": 0}, {"alpha": 3, "beta": 1})
    object.__setattr__(
        module, "measure", lambda crate: ([], f"Documenting {crate}\nFinished", 0)
    )
    assert run(module, ["--update"]) == 0
    import json as _json

    assert _json.loads(baseline.read_text())["crates"] == {"alpha": [], "beta": []}


def test_a_repair_and_a_new_break_in_one_crate_is_a_RISE(rig, capsys):
    """⛔⛔ THE DEFECT A COUNT BASELINE CANNOT SEE, and the reason this ratchet
    stores names.

    Repair one broken link and break another in the same crate and the TOTAL is
    unchanged. Under a count baseline that reads as "no change", the new break is
    banked silently, and a regression can be paid off with an unrelated repair.
    That is not hypothetical: on 2026-09-07 this ratchet went red at +7 across
    three crates, could not say WHICH seven, and the repair that turned it green
    fixed seven OTHER links. The number came back; nothing forced the actual
    regression to.
    """
    module, configure, baseline = rig
    links = configure.links
    baseline.write_text(json.dumps({"crates": {
        "alpha": links("alpha", 3),          # Item0, Item1, Item2
        "beta": links("beta", 1),
    }}))
    swapped = links("alpha", 2) + links("alpha", 1, offset=9)  # Item2 -> Item9
    object.__setattr__(
        module, "measure",
        lambda crate: ((swapped if crate == "alpha" else links("beta", 1)),
                       f"Documenting {crate}\nFinished", 0),
    )
    assert len(swapped) == 3, "the swap must not change the count, or it proves nothing"

    assert run(module, ["--check"]) == 1
    out = capsys.readouterr().out
    assert "Item9" in out, "the report must NAME the link that appeared"
    assert "gained broken doc links" in out


def test_the_report_names_the_link_not_only_the_crate(rig, capsys):
    """A red that says "three crates rose" sends the reader to `cargo doc` to
    re-derive what the guard already measured. It cost 20 minutes on 2026-09-07."""
    module, configure, _ = rig
    configure({"alpha": 5, "beta": 1}, {"alpha": 3, "beta": 1})
    run(module, ["--check"])
    out = capsys.readouterr().out
    assert "Item3" in out and "Item4" in out, (
        f"the two links that appeared must be named; got:\n{out}"
    )


def test_a_count_shaped_baseline_refuses_rather_than_reading_ints_as_empty(rig, capsys):
    """⛔ INDETERMINATE IS NOT A PASS. An int where a list belongs is the OLD
    baseline shape; treating it as "no links recorded" would report every
    existing break as new. Refuse and name the migration."""
    module, _, baseline = rig
    baseline.write_text(json.dumps({"crates": {"alpha": 3, "beta": 1}}))
    assert run(module, ["--check"]) == 1
    out = capsys.readouterr().out
    assert "carry a COUNT, not a link list" in out
    assert "--update" in out


def test_update_can_actually_perform_the_migration_it_recommends(rig, capsys):
    """⛔ A GUARD THAT FORBIDS ITS OWN REMEDY has no road out of the state it
    detects. The refusal above names `--update`; the first version of that block
    refused `--update` too, so the baseline it existed to migrate could not be."""
    module, configure, baseline = rig
    links = configure.links
    baseline.write_text(json.dumps({"crates": {"alpha": 3, "beta": 1}}))
    object.__setattr__(
        module, "measure",
        lambda crate: (links(crate, 2), f"Documenting {crate}\nFinished", 0),
    )
    assert run(module, ["--update"]) == 0, capsys.readouterr().out
    written = json.loads(baseline.read_text())["crates"]
    assert all(isinstance(v, list) for v in written.values()), written
    # and the migrated file passes the check it previously failed
    assert run(module, ["--check"]) == 0


def test_identities_are_parsed_out_of_real_rustdoc_output():
    """⛔ THE PARSER IS THE WHOLE INSTRUMENT, so it is exercised on rustdoc's own
    text rather than on the fixture's synthetic shape.

    Both warning forms, the location line, and the form rustdoc emits with NO
    `-->` at all (8 of 88 measured 2026-09-07) — that last one must still produce
    an identity, or a link would silently leave the population when it changed
    shape.
    """
    module = load()
    output = (
        "warning: unresolved link to `HitboxLifetime`\n"
        "  --> crates/ambition_combat/src/clank.rs:88:46\n"
        "   |\n"
        "warning: public documentation for `body_mode` links to private item `mechanics`\n"
        " --> crates/ambition_platformer2d_actor_monolith/src/body_mode/mod.rs:3:7\n"
        "   |\n"
        "warning: unresolved link to `super::blink::blink_target`\n"
        "  |\n"
        "  = note: the link appears in this line:\n"
    )
    found = module.identities(output)
    assert found == sorted([
        "crates/ambition_combat/src/clank.rs: unresolved `HitboxLifetime`",
        "crates/ambition_platformer2d_actor_monolith/src/body_mode/mod.rs: "
        "private `body_mode -> mechanics`",
        "<no location>: unresolved `super::blink::blink_target`",
    ]), found

    # ⚠ ANTI-VACUITY: an unrelated rustdoc warning must NOT become an identity,
    # or the parser is counting the wrong population.
    assert module.identities("warning: unused variable: `x`\n") == []


def test_a_coloured_rustdoc_stream_is_still_counted():
    """⛔⛔ **THE BYTES BELOW ARE WHY THIS GUARD READ 0/13 INSIDE `--maintenance`
    ON 2026-09-18.** Every pattern in the parser is line-anchored on
    `^warning:`, and `scripts/run_tests.py` exports `CARGO_TERM_COLOR=always`
    to every child job — so the anchor sat behind `ESC[1mESC[33m` and matched
    nothing. Thirteen crates scored zero, the table printed *"⭐ 42 repaired"*,
    and the advice was `--update`, which would have banked an empty baseline.

    ⚠ COPIED FROM A REAL RUN, not composed: `cargo doc -p ambition_characters
    --no-deps` under the variable, 2026-09-18. A hand-written escape is a guess
    about which codes rustdoc picks and where it puts the reset.

    The assertion is EQUALITY WITH THE PLAIN FORM, not merely non-empty: a
    parser that counted the coloured lines but lost the `-->` path would still
    be scoring the wrong identities.
    """
    module = load()
    plain = (
        "warning: unresolved link to `resolve_worn_control`\n"
        "  --> crates/ambition_characters/src/action_scheme.rs:60:46\n"
        "   |\n"
        "warning: public documentation for `derive_action_scheme` links to "
        "private item `combat_actions`\n"
        "   --> crates/ambition_characters/src/action_scheme.rs:449:9\n"
        "    |\n"
    )
    coloured = (
        "\x1b[1m\x1b[33mwarning\x1b[0m\x1b[1m: unresolved link to "
        "`resolve_worn_control`\x1b[0m\n"
        "  \x1b[1m\x1b[94m--> \x1b[0mcrates/ambition_characters/src/action_scheme.rs:60:46\n"
        "   \x1b[1m\x1b[94m|\x1b[0m\n"
        "\x1b[1m\x1b[33mwarning\x1b[0m\x1b[1m: public documentation for "
        "`derive_action_scheme` links to private item `combat_actions`\x1b[0m\n"
        "   \x1b[1m\x1b[94m--> \x1b[0mcrates/ambition_characters/src/action_scheme.rs:449:9\n"
        "    \x1b[1m\x1b[94m|\x1b[0m\n"
    )
    assert module.identities(plain) == module.identities(coloured) != []


def test_the_measurement_asks_cargo_for_plain_output(monkeypatch):
    """⭐ THE FIRST LINE OF DEFENCE, AND IT IS SEPARATELY POISONABLE.

    Stripping in the parser fixes the reading; asking cargo not to colour fixes
    the stream. Both, because each covers a source the other does not — an
    ambient `CARGO_TERM_COLOR` for the flag, a `RUSTDOCFLAGS=--color=always` for
    the strip.
    """
    module = load()
    seen = {}

    class Result:
        stdout = ""
        stderr = ""
        returncode = 0

    def fake_run(argv, **kwargs):
        seen["argv"] = list(argv)
        seen["env"] = kwargs.get("env")
        return Result()

    monkeypatch.setattr(module.subprocess, "run", fake_run)
    monkeypatch.setenv("CARGO_TERM_COLOR", "always")
    module.measure("ambition_characters")

    assert seen["argv"][-2:] == ["--color", "never"], seen["argv"]
    assert seen["env"] is not None, "the child inherited the caller's environment"
    assert seen["env"]["CARGO_TERM_COLOR"] == "never", seen["env"]["CARGO_TERM_COLOR"]


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
