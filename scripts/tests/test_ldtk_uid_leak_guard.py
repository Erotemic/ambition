"""`check_ldtk_uid_leak` refuses a leaked counter and knows what it may reclaim.

⛔⛤ **THE GUARD'S REFUSAL BRANCHES ARE WHY THIS EXISTS, NOT ITS GREEN PATH.** On
a healthy tree the check prints `ok` for every world and never reaches the code
that decides whether a leaked range may be reclaimed — so "the guard works" would
rest on a branch nobody takes, which is the failure this repository keeps finding
in its own instruments. Each arm below drives one of those branches on a
synthetic project, so none of them depends on a world being broken.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import sys

_SPEC = importlib.util.spec_from_file_location(
    "check_ldtk_uid_leak",
    pathlib.Path(__file__).resolve().parents[1] / "check_ldtk_uid_leak.py",
)
guard = importlib.util.module_from_spec(_SPEC)
sys.modules["check_ldtk_uid_leak"] = guard
_SPEC.loader.exec_module(guard)


def _world(tmp_path: pathlib.Path, next_uid: int, highest: int) -> pathlib.Path:
    """A minimal project whose largest spent uid is `highest`."""
    project = {
        "nextUid": next_uid,
        "defs": {"layers": [{"identifier": "Collision", "uid": highest}]},
        "levels": [],
    }
    path = tmp_path / "probe.ldtk"
    path.write_text(json.dumps(project, indent=2))
    return path


def test_a_counter_one_above_its_contents_is_the_healthy_state():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        path = _world(pathlib.Path(tmp), next_uid=101, highest=100)
        assert guard.leak_of(path) == (101, 100, 0)


def test_a_counter_that_ran_ahead_is_reported_as_a_leak():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        path = _world(pathlib.Path(tmp), next_uid=104, highest=100)
        counter, highest, leak = guard.leak_of(path)
        assert (counter, highest, leak) == (104, 100, 3)


def test_an_iid_suffix_pins_the_counter_as_firmly_as_a_uid_field():
    """⛔ `<Identifier>-NNNN` iids are minted from the SAME counter.

    A scan that only reads `uid` fields would call a project healthy while its
    highest spent value lives in an iid string — and then `--fix` would wind the
    counter back onto a name that is still in use. `area_authoring.allocate_iid`
    is what mints these, so this is the shape the repository actually writes.
    """
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "probe.ldtk"
        path.write_text(
            json.dumps(
                {
                    "nextUid": 5001,
                    "defs": {"layers": [{"identifier": "Collision", "uid": 10}]},
                    "levels": [{"iid": "Sandbox-5000"}],
                },
                indent=2,
            )
        )
        counter, highest, leak = guard.leak_of(path)
        assert highest == 5000, "the iid suffix was not counted as a spent uid"
        assert leak == 0


def test_a_wide_leak_is_refused_rather_than_reclaimed():
    """⛔ `world_init.py` offsets a new world's counter by 100,000 ON PURPOSE.

    Winding that back would undo the thing that keeps merged iids from
    colliding, so a leak wider than `MAX_RECLAIMABLE` is a counter somebody MOVED
    rather than one an allocation leaked.

    ⚠ AND THE BOUND IS ALSO WHY `--fix` TERMINATES. The first version ran one
    `git grep` per candidate value; poisoning this branch with a 100,000-wide
    range is what found that, so the arm that guards correctness also guards
    against the guard hanging.
    """
    assert guard.unreferenced(range(0, guard.MAX_RECLAIMABLE + 1)) == []
    assert guard.unreferenced(range(0, 0)) == []


def test_an_empty_corpus_is_a_failure_rather_than_a_pass(capsys):
    """⛔ FINDING NO WORLDS MEANS THE CHECK WAS AIMED WRONGLY.

    Printing `ok` over an empty corpus is the single most common way a guard in
    this repository certifies nothing, so the exit code says 2 rather than 0.
    """
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        assert guard.main([tmp]) == 2


def test_the_editor_formatter_fallback_announces_itself(capsys, monkeypatch):
    """⛔ A SILENT FALLBACK AROUND THE ONLY WRITER OF AUTHORED CONTENT.

    `write_project` used to wrap its normalize-and-format in a bare
    `except Exception:` and fall through to plain JSON — which REFORMATS AN
    ENTIRE WORLD. Success and failure handed the caller the same thing, so the
    only evidence was a five-thousand-line diff with no recoverable cause.

    ⇒ The fallback is kept (a tiny tool install legitimately has no validator)
    but narrowed to `ImportError` and made to say so. This drives that branch,
    which on a full install is otherwise never taken.
    """
    import sys as _sys
    import tempfile

    sys.path.insert(
        0, str(pathlib.Path(__file__).resolve().parents[2] / "tools" / "ambition_ldtk_tools")
    )
    from ambition_ldtk_tools.ldtk import io as ldtk_io

    # ⛔ THE IMPORT IS INSIDE THE FUNCTION, so hiding the module is what makes
    # the branch reachable — patching a name on `io` would not.
    monkeypatch.setitem(_sys.modules, "ambition_ldtk_tools.editor_format", None)
    with tempfile.TemporaryDirectory() as tmp:
        out = pathlib.Path(tmp) / "probe.ldtk"
        ldtk_io.write_project(out, {"nextUid": 2, "defs": {}, "levels": [{"uid": 1}]})
        assert out.exists(), "the fallback must still WRITE the file"
        assert "PLAIN JSON" in capsys.readouterr().err, (
            "the fallback path took itself silently — which is the defect, not "
            "the fallback"
        )
