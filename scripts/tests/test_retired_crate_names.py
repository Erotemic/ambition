"""The retired-name guard has to FIRE on the cases that motivated it.

The live tree is green — that is the point of the guard — so a test that only ran
it would prove nothing. These feed it the exact lines that survived the
`platformer2d` rename's own sweeps, plus the two shapes it must stay quiet on.
"""

from __future__ import annotations

import sys
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
