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
