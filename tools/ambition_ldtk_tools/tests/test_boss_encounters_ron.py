"""Schema pins for `crates/ambition_sandbox/assets/data/boss_encounters/*.ron`.

These files mirror `ae::BossEncounterSpec` and are loaded at runtime by
`load_boss_specs_from_disk` (per ADR 0017). The Rust side already pins
field-for-field equivalence against the hardcoded constructors; these
Python tests catch authoring drift earlier (a missing field shows up
on the python side before the Rust test even compiles).

Pinned fields chosen to overlap the runtime's required surface; not
exhaustive — the Rust pin is."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.ron_parse import load  # noqa: E402

BOSS_DIR = (
    REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "data" / "boss_encounters"
)

REQUIRED_FIELDS = {
    "id",
    "name",
    "max_hp",
    "phase1_to_transition_hp",
    "transition_to_phase2_hp",
    "phase2_to_enrage_hp",
    "intro_seconds",
    "transition_seconds",
    "stagger_seconds",
    "death_seconds",
    "stagger_threshold",
    "stagger_window_seconds",
    "music_intro",
    "music_phase1",
    "music_phase2",
    "music_enrage",
}


def _ron_files() -> list[Path]:
    return sorted(BOSS_DIR.glob("*.ron"))


def test_boss_encounters_dir_is_populated():
    files = _ron_files()
    assert files, f"no .ron files under {BOSS_DIR} — the loader will return empty"


def test_every_boss_encounter_ron_parses():
    """Each file under boss_encounters/ must parse — a corrupted file
    would cause `load_boss_specs_from_disk` to silently drop the boss
    (warns + skips) and fall back to the hardcoded constructor."""
    for ron_path in _ron_files():
        data = load(ron_path.read_text())
        assert isinstance(data, dict), (
            f"{ron_path.name}: top-level should be a struct/dict"
        )


def test_every_boss_encounter_ron_has_required_fields():
    """The full set of fields needed to deserialize as
    `BossEncounterSpec`. Missing any one trips the Rust loader's
    `ron::from_str::<BossEncounterSpec>` step."""
    for ron_path in _ron_files():
        data = load(ron_path.read_text())
        missing = REQUIRED_FIELDS - set(data.keys())
        assert not missing, f"{ron_path.name}: missing {sorted(missing)}"


def test_every_boss_encounter_ron_id_matches_filename():
    """`<id>.ron` discipline — the loader doesn't require this but
    keeping filename and id aligned makes the registry easier to
    reason about and matches the existing 3 specs."""
    for ron_path in _ron_files():
        data = load(ron_path.read_text())
        assert data["id"] == ron_path.stem, (
            f"{ron_path.name}: id={data['id']!r} doesn't match filename"
        )


def test_phase_threshold_fractions_in_valid_range():
    """HP transition thresholds are normalized fractions; values outside
    [0, 1] mean a phase will never trigger (>1) or trigger immediately
    (<0)."""
    for ron_path in _ron_files():
        data = load(ron_path.read_text())
        for field in (
            "phase1_to_transition_hp",
            "transition_to_phase2_hp",
            "phase2_to_enrage_hp",
        ):
            v = data[field]
            assert 0.0 <= v <= 1.0, f"{ron_path.name}: {field}={v} out of [0, 1]"


def test_timing_fields_are_positive():
    """Negative durations would skip the phase entirely; zero means
    instant. The loader doesn't enforce — this guards content-side."""
    for ron_path in _ron_files():
        data = load(ron_path.read_text())
        for field in (
            "intro_seconds",
            "transition_seconds",
            "stagger_seconds",
            "death_seconds",
        ):
            assert data[field] > 0.0, (
                f"{ron_path.name}: {field}={data[field]} should be positive"
            )


def test_max_hp_positive():
    for ron_path in _ron_files():
        data = load(ron_path.read_text())
        assert data["max_hp"] > 0, (
            f"{ron_path.name}: max_hp={data['max_hp']} must be positive"
        )
