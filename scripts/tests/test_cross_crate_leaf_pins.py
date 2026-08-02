"""The cross-crate leaf-pin counter, shown finding and shown declining.

⛔ **every case here is planted, because this campaign has twice read a silent
limit as an absence.** A guard whose only evidence is "it printed 33 against the
real tree" is a guard that has never been shown to find anything: 33 is also
what it prints if the walk is broken and the number is stale. So each test
builds a tiny fake workspace and checks the row that comes back — and, just as
importantly, the shapes that must NOT come back, since a counter that
over-reports gets its ceiling raised until it means nothing.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from check_cross_crate_leaf_pins import (  # noqa: E402
    MAX_CROSS_CRATE_LEAF_PINS,
    collect,
)


def _workspace(tmp_path: Path, rel: str, body: str) -> Path:
    src = tmp_path / rel
    src.parent.mkdir(parents=True, exist_ok=True)
    src.write_text(body, encoding="utf-8")
    return tmp_path


def test_a_pin_naming_another_crates_function_is_counted(tmp_path):
    root = _workspace(
        tmp_path,
        "crates/ambition_render/src/lib.rs",
        "app.add_systems(Update, draw.after(ambition_combat::hazards::tick_hazards));",
    )
    found = collect(root)
    assert len(found) == 1, f"the planted pin was not found: {found}"
    file, line, crate, target = found[0]
    assert crate == "ambition_combat"
    assert target == "ambition_combat::hazards::tick_hazards"
    assert line == 1


def test_a_pin_rustfmt_WRAPPED_is_still_counted(tmp_path):
    """⛔ the regression that made the guard undercount its own ceiling.

    rustfmt wrapped one long pin onto its own line with a TRAILING COMMA, in a
    file this campaign had just edited. The pattern required `)` immediately
    after the path, so the row vanished and the tree looked one pin cleaner than
    it was. The newline was never the problem — `\\s` spans newlines — the comma
    was, which is exactly the kind of detail that is invisible until pinned.

    Fixing it surfaced TWO uncounted rows, which is also why the ceiling has to
    be re-derived from the tree rather than adjusted by hand.
    """
    root = _workspace(
        tmp_path,
        "crates/ambition_render/src/lib.rs",
        "\n".join(
            [
                "draw",
                "    .after(",
                "        ambition_combat::hazards::tick_hazards,",
                "    );",
            ]
        ),
    )
    found = collect(root)
    assert len(found) == 1, f"the wrapped pin was not found: {found}"
    assert found[0][3] == "ambition_combat::hazards::tick_hazards"


def test_a_pin_naming_another_crates_SET_is_the_shape_we_want(tmp_path):
    root = _workspace(
        tmp_path,
        "crates/ambition_render/src/lib.rs",
        "app.add_systems(Update, draw.after(ambition_combat::hazards::HazardTickSet));",
    )
    assert collect(root) == [], (
        "a CamelCase tail is a set the owning crate chose to expose — the "
        "destination of every conversion, and counting it would make the "
        "ratchet refuse the fix"
    )


def test_a_crate_ordering_ITSELF_is_not_a_finding(tmp_path):
    root = tmp_path
    (root / "crates/ambition_render/src").mkdir(parents=True)
    (root / "crates/ambition_render/src/lib.rs").write_text(
        "\n".join(
            [
                "a.after(crate::actors::sync_visuals);",
                "b.after(self::camera::camera_follow);",
                "c.after(actors::animate_props);",
                "d.after(ambition_render::rendering::animate_bosses);",
            ]
        ),
        encoding="utf-8",
    )
    assert collect(root) == [], (
        "all four address a system inside ambition_render itself, including the "
        "last one, which spells its own crate name out"
    )


def test_prose_about_an_edge_is_not_an_edge(tmp_path):
    root = _workspace(
        tmp_path,
        "crates/ambition_render/src/lib.rs",
        "\n".join(
            [
                "/// Runs .after(ambition_combat::hazards::tick_hazards) so the",
                "/// overlay sees this frame's damage.",
                "// a.after(ambition_combat::hazards::tick_hazards);",
            ]
        ),
    )
    assert collect(root) == [], (
        "the campaign asks people to DOCUMENT why an edge exists; counting the "
        "documentation would tax the behaviour it wants"
    )


def test_test_code_is_out_of_scope(tmp_path):
    root = tmp_path
    for rel in ("crates/ambition_render/src/thing/tests.rs", "crates/ambition_render/tests/it.rs"):
        p = root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(
            "a.after(ambition_combat::hazards::tick_hazards);", encoding="utf-8"
        )
    assert collect(root) == [], "a test may reach wherever it needs to reach"


def test_the_ceiling_matches_the_tree():
    """The ratchet itself, against the REAL repository.

    Failing in EITHER direction is deliberate: above the ceiling is a new pin,
    below it is a conversion whose commit forgot to tighten the bar.
    """
    assert len(collect()) == MAX_CROSS_CRATE_LEAF_PINS
