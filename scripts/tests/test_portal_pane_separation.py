"""The pane-separation measurement has to be able to say NO.

It reports that the closest two panes a body could be far-covered by are 163.2px
apart, against a body about 32px wide -- which is the evidence that the far-side
compositing repair's THREE-clip-plane budget is enough for the shipped worlds.
A measurement that could only ever produce a comfortable number would be no
evidence at all, so these pin the two ways it could:

* the back-to-back DOORWAY exclusion is the whole margin. The closest pair in the
  content is 32px, and it is excluded because "far of both" there means standing
  inside the wall slab. If that exclusion silently widened, the reported margin
  would grow for a reason that has nothing to do with the content; and
* an empty corpus must FAIL rather than report a reassuring "no pairs".
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "portal_pane_separation.py"


def _module():
    spec = importlib.util.spec_from_file_location("portal_pane_separation", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _pane(x, y, normal, link, name="p"):
    return (x, y, normal, link, name)


def test_only_one_link_with_opposing_normals_is_a_doorway() -> None:
    """⛔ The exclusion is what buys the margin, so it must be NARROW.

    Two panes of the same link facing the SAME way, or opposing panes belonging
    to DIFFERENT links, are ordinary pairs -- a body can be far of both without
    standing inside a wall.
    """
    m = _module()
    assert m.back_to_back(_pane(0, 0, "left", "a"), _pane(32, 0, "right", "a"))
    assert not m.back_to_back(_pane(0, 0, "left", "a"), _pane(32, 0, "right", "b"))
    assert not m.back_to_back(_pane(0, 0, "left", "a"), _pane(32, 0, "left", "a"))
    assert not m.back_to_back(_pane(0, 0, "left", None), _pane(32, 0, "right", None))


def test_an_empty_corpus_fails_rather_than_reporting_no_pairs(tmp_path, capsys) -> None:
    """⛔⛔ A CHECK THAT CANNOT FAIL. With no portals the pair list is empty and
    every margin claim is trivially satisfied, which reads exactly like a wide
    margin. It must refuse instead."""
    m = _module()
    m.WORLDS = tmp_path
    (tmp_path / "empty.ldtk").write_text('{"levels": []}', encoding="utf-8")
    assert m.main() == 1


def test_it_reports_the_shipped_margin(capsys) -> None:
    """The live run, so a content change that moves two panes together shows up
    here rather than in a portal that quietly draws wrong."""
    m = _module()
    code = m.main()
    if code == 3:
        import pytest

        pytest.skip("map submodule absent; see the navigability check's note")
    assert code == 0
    out = capsys.readouterr().out
    assert "CLOSEST NON-DOORWAY PAIR" in out or "single-pane road is total" in out
