"""What the output-digest cache of `scripts/regen/sprites.sh` keys on
(`scripts/lib/sprite_install_names.py`)."""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "sprite_install_names", REPO / "scripts" / "lib" / "sprite_install_names.py"
)
names = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(names)


def test_the_cache_keys_only_what_stays_after_the_publish_boundary() -> None:
    """The defect: a claimed diagnostic was keyed, the sweep moved it out, and
    the target was published again on every run (`gnu_ton_boss`,
    `mockingbird_boss`)."""
    claimed = {
        "gnu_ton_boss": [
            "gnu_ton_boss/gnu_ton_boss_spritesheet.png",
            "gnu_ton_boss/gnu_ton_boss_canonical.png",
            "gnu_ton_boss/gnu_ton_boss_preview_labeled.png",
        ],
        "mockingbird_boss": [
            "mockingbird_boss/mockingbird_boss_spritesheet.ron",
            "mockingbird_boss/mockingbird_boss_parts_debug.png",
            "mockingbird_boss/canonicals/pose.png",
        ],
        "goblin": ["goblin_spritesheet.png", "goblin_portraits.ron"],
    }
    assert names.runtime_install_names(claimed) == {
        "gnu_ton_boss": ["gnu_ton_boss/gnu_ton_boss_spritesheet.png"],
        "mockingbird_boss": ["mockingbird_boss/mockingbird_boss_spritesheet.ron"],
        "goblin": ["goblin_spritesheet.png", "goblin_portraits.ron"],
    }
