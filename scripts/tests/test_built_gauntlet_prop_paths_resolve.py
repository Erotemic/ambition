"""A path that is BUILT from an id is invisible to every check that reads literals.

`GAUNTLET_PROP_IDS` in `game/ambition_content/src/items/held_visuals.rs` lists
fourteen wielded-gauntlet abilities, and each draws
`sprites/props/gauntlet_<id>.png` — a path assembled at registration rather than
spelled anywhere. `declared_art_resolves` reads declared literals, so it cannot
see these; adding an id to that list and forgetting the art produces an ability
that registers cleanly and draws nothing.

⚠ **this is the same class as the gnu_ton boss parts**, which are also assembled
by suffix — and which a "sheets nobody names" census flagged as dead on
2026-08-05 precisely because no string search can find them. The difference is
that this construction rule is ONE line, so it can be re-run here instead of
merely warned about.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HELD_VISUALS = REPO / "game/ambition_content/src/items/held_visuals.rs"
PROPS = REPO / "crates/ambition_platformer2d_actor_monolith/assets/sprites/props"

_LIST = re.compile(r"const GAUNTLET_PROP_IDS: &\[&str\] = &\[(.*?)\];", re.S)
_ID = re.compile(r'"([a-z_0-9]+)"')


def _gauntlet_ids() -> list[str]:
    body = _LIST.search(HELD_VISUALS.read_text(encoding="utf8"))
    assert body, (
        "GAUNTLET_PROP_IDS is no longer a `const … &[&str]` in held_visuals.rs — "
        "this scan is reading a shape that moved, and would report nothing"
    )
    return _ID.findall(body.group(1))


def test_every_gauntlet_ability_has_the_prop_its_path_is_built_from():
    ids = _gauntlet_ids()
    assert len(ids) >= 10, (
        f"only {len(ids)} gauntlet ids parsed — the scan is broken and would "
        "report no missing art either"
    )
    missing = [i for i in ids if not (PROPS / f"gauntlet_{i}.png").is_file()]
    assert not missing, (
        "gauntlet ability id(s) whose built prop path resolves to nothing, so "
        "the ability registers and draws nothing and no declared-art check can "
        f"see it: {missing}. Expected `sprites/props/gauntlet_<id>.png`."
    )


def test_the_scan_would_notice_a_missing_prop():
    """The poison: the same construction, for an id nobody drew."""
    assert not (PROPS / "gauntlet_an_ability_nobody_drew.png").is_file()
    # …and the directory really is the one holding them, so the check above is
    # not passing because it is looking somewhere empty.
    assert (PROPS / f"gauntlet_{_gauntlet_ids()[0]}.png").is_file()
