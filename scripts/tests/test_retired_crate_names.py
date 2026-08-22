"""The retired-name guard has to FIRE on the cases that motivated it.

The live tree is green — that is the point of the guard — so a test that only ran
it would prove nothing. These feed it the exact lines that survived the
`platformer2d` rename's own sweeps, plus the two shapes it must stay quiet on.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_retired_crate_names import (  # noqa: E402
    RETIRED_CRATE_NAMES,
    retired_names_in_line,
)


def names(line: str) -> set[str]:
    return {old for old, _ in retired_names_in_line(line)}


def test_a_plain_reference_is_caught():
    assert names("use ambition_actors::features::Body;") == {"ambition_actors"}


def test_the_tab_escape_case_that_survived_the_rename():
    """⭐ The reason this guard exists, verbatim.

    Two construction-registry assertions compared an owner string written inside
    a tab-delimited literal. The character before the crate name is the `t` of
    `\\t`, so the rename's word-boundary rule skipped them — and so did the grep
    that verified the rename. They failed in the full suite, an hour later, as a
    registry-dump mismatch.
    """
    line = r'dump.contains("relation\tambition.limb\tambition_actors\tlimb-rig\t"),'
    assert names(line) == {"ambition_actors"}


def test_the_regex_escape_case():
    """The same shape one escape over: `\\b` before the name in a regex literal."""
    assert names(r'_MODULE = re.compile(r"\bambition_runtime::([a-z_]+)")') == {
        "ambition_runtime"
    }


def test_a_path_in_a_symlink_or_script_is_caught():
    assert names("../../../crates/ambition_actors/assets/sprites") == {"ambition_actors"}


def test_a_longer_identifier_that_merely_starts_with_a_retired_name_is_not_a_hit():
    """`ambition_world_entity` is Ambition's world entity, not the retired crate.

    The trailing boundary is what separates them, and it is safe to test where a
    LEADING one is not: an escape sequence sits before a name, never after it.
    """
    assert names("let ambition_world_entity = app") == set()
    assert names('run(f"file_{p}_ambition_actors_top50")') == set()


def test_prose_recording_history_is_left_alone():
    """A record of what happened must not be rewritten to say something else."""
    assert names("//! the former `ambition_engine` workspace crate was collapsed") == set()
    assert names("# was: ambition_world_dependency_allowlist_ratchets") == set()
    assert names('referenced by libambition_runtime-<hash>.rlib') == set()


def test_every_retired_name_maps_to_something_that_is_not_itself():
    for old, new in RETIRED_CRATE_NAMES.items():
        assert new and new != old, f"{old} must say what replaced it"


def test_the_live_tree_names_no_retired_crate():
    """The ratchet itself, against the real repository.

    Matches how every other guard here is wired: the per-line tests above prove
    it CAN fire, and this one is what actually holds the line.
    """
    from check_retired_crate_names import offences, tracked_files

    found = offences(tracked_files())
    assert not found, "\n".join(
        f"  {f}:{n}: `{old}` was renamed to `{new}`" for f, n, old, new in found
    )


def test_the_goal_harness_config_is_scanned_even_though_it_is_untracked():
    """⭐ The blind spot that cost two false "not done" reports.

    `.goal/active.json` holds the goal harness's own check COMMANDS and is not in
    git, so a `git ls-files` sweep cannot see it. When the platformer2d rename
    retired `ambition_actors` and `ambition`, two commands kept naming them,
    `cargo` failed on an unknown package, and the harness reported two FINISHED,
    green items as unfinished work.

    `done-*.json` are archived goals and stay exempt like any other record.

    ⚠ **built against a FIXTURE, not against this machine.** It used to read the
    live `.goal/`, so it asserted that a goal was armed right now — green on a
    developer mid-run, red in a fresh clone and red inside a source archive that
    omits the directory. A tracked test must not depend on untracked local
    state; what is being tested is the RULE, and the rule needs a directory, not
    this directory.
    """
    from check_retired_crate_names import extra_paths

    root = Path(tempfile.mkdtemp())
    goal = root / ".goal"
    goal.mkdir()
    (goal / "active.json").write_text("{}")
    (goal / "done-20260101T000000Z.json").write_text("{}")

    paths = extra_paths(root)
    assert ".goal/active.json" in paths, "the live goal config must be scanned"
    assert not any(
        "done-" in p for p in paths
    ), "archived goals are records, not live configuration"
    assert extra_paths(Path(tempfile.mkdtemp())) == [], (
        "a tree with no .goal/ scans nothing extra — a fresh clone and an archive "
        "are both that tree"
    )


def test_the_LIVE_QUEUES_are_not_exempt():
    """Live work queues must not be exempt from retired-name checks.

    They are actionable instructions, not historical records, so stale names in
    them can misdirect future work.
    """
    import check_retired_crate_names as guard

    exempt = [p for p in guard.HISTORICAL_PREFIXES if "queue-" in p]
    assert not exempt, (
        f"the live worklist is exempt from the retired-name check again: {exempt}"
    )


def test_a_retired_TYPE_name_is_tracked_too():
    """The rule is about NAMES a reader will grep for, not about crates.

    A stale crate name breaks a build. A stale type name in a planning doc
    quietly retires a piece of work, which is worse for being silent.
    """
    import check_retired_crate_names as guard

    assert guard.RETIRED_CRATE_NAMES.get("SandboxAction") == (
        "Platformer2dInputActionMonolith"
    )
