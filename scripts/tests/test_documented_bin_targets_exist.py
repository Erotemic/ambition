"""⛔ EVERY `--bin <name>` IN PROSE MUST NAME A BIN CARGO ACTUALLY BUILDS.

⚠ **THIS CLASS IS INVISIBLE TO EVERY OTHER CHECK WE HAVE.**
`check_planning_citations.py` resolves cited SYMBOLS and paths; a fenced shell
command is prose to it. So `cargo run -p X --bin ladder_rig` keeps resolving
forever after `ladder_rig` stops existing — the page stays green and the command
stops working, which is the exact shape of
`docs/recipes/checks-that-did-not-run.md`.

⭐ **WRITTEN FOR A MIGRATION THAT WOULD HAVE CAUSED IT.** 2026-09-03 nine bins in
`ambition_demo_smash_app` collapsed into one modal `smash_tool`; three documented
invocations named the old bins. They were updated in the same commit, and this
exists so the NEXT rename cannot quietly leave them behind.

ⓘ Names only. Whether the SUBCOMMAND after `--bin smash_tool --` is real is a
different claim needing the built binary's `--help`, and this deliberately does
not pretend to check it.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from lib.cargo_bin import cargo_binary  # noqa: E402

#: `--bin <name>`, the form every documented invocation in this repo uses.
BIN_FLAG = re.compile(r"--bin\s+([A-Za-z_][A-Za-z0-9_-]*)")

#: ⛔⛔ THE FORM THE `--bin` CHECK CANNOT SEE: a bare tool name run as a command.
#:
#: ⚠ Found 2026-09-03, hours after this guard was written and by a sweep that had
#: nothing to do with it. `engine/fighter-brain.md:68` read
#: ``Run: `ladder_rig --sweep-below --seeds 45` `` — an invocation of a binary
#: deleted that same day, in a page whose OTHER invocation had been retargeted,
#: because the retarget matched `--bin ladder_rig` and this one has no `--bin`.
#: A second was in `maintainer-decisions.md`. ⇒ Two of the nine, missed by a
#: guard written for exactly this migration.
#:
#: ⭐ Kept as an explicit list rather than inferred: a guard cannot know a name
#: WAS a target once it stops being one, and matching every bare word that looks
#: like a command would fire on prose forever. When a bin is retired, add it
#: here; that is the whole maintenance burden and it is one line.
RETIRED_BINS = (
    "capture_probe", "ladder_probe", "ladder_rig", "match_diagram",
    "match_report", "match_shots", "roll_probe", "select_walkthrough",
    "stage_diagram",
)

#: A retired name being DISCUSSED is fine; being RUN is not. This matches the
#: invocation shape — the name followed by a flag or a `--` separator.
RETIRED_RUN = re.compile(
    r"(?<![\w./-])(" + "|".join(RETIRED_BINS) + r")\s+(?:--?\w|--\s)"
)

SEARCHED = ("docs", "scripts", ".github")

#: ⛔ THIS FILE MATCHES ITS OWN PATTERN. Every `--bin NAME` in the docstring and
#: the assertion message above is a hit, so without this the guard fails on
#: itself — the same self-match that stranded shells with `pgrep -f <script>`
#: twice in this repo. The pattern must never be allowed to see the matcher.
SELF = Path(__file__).resolve()

#: ⚠ `docs/archive/` is EXCLUDED BY GENRE, not by convenience. An archived page
#: records what a command was on the day it was archived; a currency check
#: against a deliberately historical corpus produces work with no yield — the
#: same reason `--vanished` is not aimed at `docs/adr/`.
#: ⓘ It is not empty: `procedural-tune-authoring.md` names `--bin tune_preview`
#: twice, and that bin no longer exists. Correct as archive, reportable if the
#: page is ever un-archived.
EXCLUDED = (Path("docs") / "archive",)


def documented_bins() -> dict[str, list[str]]:
    """Every `--bin NAME` under the searched trees, name -> where it was seen."""
    found: dict[str, list[str]] = {}
    for root in SEARCHED:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in base.rglob("*"):
            if not path.is_file() or path.suffix not in {".md", ".py", ".sh", ".yml", ".yaml"}:
                continue
            if path.resolve() == SELF:
                continue
            rel = path.relative_to(REPO)
            if any(rel.is_relative_to(skip) for skip in EXCLUDED):
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for name in BIN_FLAG.findall(text):
                found.setdefault(name, []).append(str(path.relative_to(REPO)))
    return found


def real_bins() -> set[str]:
    """Every bin target cargo knows about, from `cargo metadata`."""
    # ⚠ `cargo` is not on PATH in the tool venv; the repo already solved that
    # once and this uses the same answer rather than a second one.
    try:
        out = subprocess.run(
            [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
            cwd=REPO, capture_output=True, text=True, timeout=120,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        pytest.skip(f"cargo metadata could not run: {exc}")
    if out.returncode != 0:
        pytest.skip(f"cargo metadata unavailable: {out.stderr.strip()[:200]}")
    meta = json.loads(out.stdout)
    return {
        target["name"]
        for package in meta["packages"]
        for target in package["targets"]
        if "bin" in target["kind"]
    }


def test_there_are_documented_bins_to_check():
    """⛔ THE PREMISE. With no matches the test below asserts nothing and passes."""
    assert documented_bins(), (
        f"no `--bin NAME` found anywhere under {SEARCHED} — has the invocation "
        "form changed? This guard is checking nothing."
    )


def test_no_retired_bin_is_still_shown_as_a_command():
    """⛔ A RETIRED TOOL NAME, RUN RATHER THAN DISCUSSED.

    The `--bin` check above cannot see `ladder_rig --sweep-below`: there is no
    `--bin`, just a name and its flags. That is how two invocations of deleted
    binaries survived the migration that deleted them.
    ⓘ Mentions are allowed — a page may name `ladder_rig` in prose or record it
    in a dated row. Only the invocation shape is refused.
    """
    offenders: dict[str, list[str]] = {}
    for root in SEARCHED:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in base.rglob("*"):
            if not path.is_file() or path.suffix not in {".md", ".py", ".sh", ".yml", ".yaml"}:
                continue
            if path.resolve() == SELF:
                continue
            rel = path.relative_to(REPO)
            if any(rel.is_relative_to(skip) for skip in EXCLUDED):
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for name in RETIRED_RUN.findall(text):
                offenders.setdefault(name, []).append(str(rel))
    assert not offenders, (
        "retired binaries still shown as runnable commands:\n"
        + "\n".join(f"  {n} — in {', '.join(sorted(set(w)))}" for n, w in sorted(offenders.items()))
        + "\n⇒ These no longer exist. Retarget them at `smash_tool <subcommand>`."
    )


def test_every_documented_bin_is_a_real_target():
    documented = documented_bins()
    real = real_bins()
    missing = {name: where for name, where in documented.items() if name not in real}
    assert not missing, (
        "documented `--bin` targets that cargo does not build:\n"
        + "\n".join(f"  {name} — named in {', '.join(sorted(set(w)))}" for name, w in sorted(missing.items()))
        + "\n⇒ The command in those files cannot run. Repoint them at the bin "
        "that replaced it, or remove the invocation."
    )
