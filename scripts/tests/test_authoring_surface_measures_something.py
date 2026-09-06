"""`scripts/authoring_surface.py` still finds the techniques it claims to measure.

The campaign page quotes this script's output as the answer to Jon's acceptance
test — *"EASE OF AUTHORING is the acceptance test"* — so the number crosses a
document boundary and has to keep being re-derivable.

⛔⛔ THE FAILURE THIS GUARDS IS SILENT AND IT LOOKS LIKE GOOD NEWS. The script
finds params structs by REGEX over `crates/ambition_characters/src/smash_*.rs`.
Rename the files, move the techniques, or change the struct suffix, and it finds
NOTHING — and a census of nothing reports a small, tidy, entirely wrong answer
rather than an error. ⇒ The floor below is not a style rule; it is the difference
between "authoring got cheaper" and "the instrument went blind".

⚠ DELIBERATELY NOT PINNED TO AN EXACT COUNT. A guard on the precise number goes
red every time somebody adds a technique, which is the opposite of what this
campaign wants to encourage — and a stale count guard is a chore that teaches
people to edit numbers until tests pass. The page cites the figure with its date
and its command; this only holds that the instrument is still looking at
something.
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/authoring_surface.py"


def test_the_authoring_surface_census_still_finds_the_techniques():
    assert SCRIPT.exists(), f"the measurement script is gone: {SCRIPT}"
    run = subprocess.run(
        [sys.executable, str(SCRIPT)],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    assert run.returncode == 0, f"the script failed:\n{run.stdout}\n{run.stderr}"

    out = run.stdout
    # ⛔ THE SPECIFIC DIAGNOSIS FIRST. Ordered the other way round, the generic
    # "no summary line" assertion fires on a blind census and this one is
    # unreachable — a decorative arm that can never run, which is exactly the
    # defect a poison is supposed to expose. Verified by poisoning the corpus and
    # reading which message came back.
    assert "no techniques found" not in out, (
        "the census found NOTHING and said so. A regex over "
        "`smash_*.rs` went blind — the techniques moved, were renamed, or their "
        "params structs no longer end in `Params`."
    )
    assert "authored technique params" in out, (
        "the script no longer prints its summary line, so the campaign page "
        f"quotes a shape that no longer exists:\n{out}"
    )

    # A floor, not a target: the roster shipped well past this during the
    # 2026-09-05 campaign, so falling under it means the instrument broke rather
    # than that authoring got simpler.
    count = int(out.split("authored technique params")[0].strip().split("\n")[-1].strip())
    assert count >= 10, (
        f"only {count} technique params found. The campaign measured 19 on "
        "2026-09-05; a number this small means the scan lost its corpus, not "
        "that the roster shrank."
    )


def test_the_census_also_finds_the_authoring_verbs():
    """⛔⛔ THE OTHER HALF, WHICH WENT UNGUARDED WHEN IT WAS ADDED.

    A technique count is not an authoring surface: most of what a move IS gets
    written with verbs that are not techniques — `multihit`, `gust`, `tipper`,
    `invuln`, `committed_tail`. The script grew that half; without this it could
    print "0 verbs" after a rename and the summary line would still read like a
    measurement.

    ⚠ IT FLOORS THE COUNT AND NAMES THREE, deliberately. A floor alone survives
    a regex that matches the wrong thing; naming verbs that exist for different
    reasons — a shape (`strike`), a decorator (`committed_tail`) and the newest
    one (`tipper`) — fails if the scan drifts toward one shape of signature.
    """
    result = subprocess.run(
        [sys.executable, str(SCRIPT)], cwd=REPO, capture_output=True, text=True
    )
    assert result.returncode == 0, f"the script failed:\n{result.stdout}\n{result.stderr}"
    assert "AUTHORING VERBS" in result.stdout, (
        "the verb half printed nothing at all:\n" + result.stdout
    )
    assert "measuring nothing" not in result.stdout, (
        "the script says its verb half found none:\n" + result.stdout
    )
    for verb in ("strike", "committed_tail", "tipper"):
        assert f"  {verb} " in result.stdout, (
            f"`{verb}` is missing from the verb census — a scan that loses one "
            "shape of signature keeps counting and reports a smaller vocabulary "
            "as a fact:\n" + result.stdout
        )
    counted = int(
        [line for line in result.stdout.splitlines() if "verbs beside" in line][0]
        .strip()
        .split()[0]
    )
    assert counted >= 15, (
        f"only {counted} authoring verbs found; the module had 22 when this was "
        "written, so a drop this size is the scan breaking rather than the "
        "vocabulary shrinking"
    )
