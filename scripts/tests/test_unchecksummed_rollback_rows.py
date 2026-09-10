"""The uncovered-rollback-row census finds its population and refuses an empty one."""

from __future__ import annotations

import importlib.util
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/measure_unchecksummed_rollback_rows.py"


def _module():
    spec = importlib.util.spec_from_file_location("uncovered", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_it_finds_the_population_and_resolves_every_type():
    module = _module()
    subjects = module.rows()
    assert len(subjects) >= 40, (
        f"{len(subjects)} rows carry the 'not in the session checksum' detail; "
        "measured at 59. A drop means the registration arm was renamed or the "
        "baseline moved — not that the gap closed."
    )
    # ⛔ EVERY ROW MUST RESOLVE TO A DEFINITION. A `?` row is the census failing
    # to find a type and reporting it as a row with no floats, which reads as
    # SAFER than the truth.
    unresolved = [name for name, ty in subjects if module.definition(ty)[0] == "?"]
    assert not unresolved, (
        f"these rows name a type this census cannot locate: {unresolved}. An "
        "unlocated type is reported with no float-bearing fields, which is the "
        "reassuring direction."
    )


def test_it_refuses_an_empty_baseline(monkeypatch, tmp_path):
    module = _module()
    empty = tmp_path / "baseline.txt"
    empty.write_text("ggrs-rollback-schema-v0\n")
    monkeypatch.setattr(module, "BASELINE", empty)
    try:
        module.main()
    except AssertionError as error:
        assert "lost the baseline" in str(error)
    else:
        raise AssertionError(
            "the census reported 0 uncovered rows from an EMPTY baseline instead "
            "of refusing; 0 is the reassuring direction and it must not be "
            "reachable by the scan going blind"
        )


def test_the_reader_triage_sees_a_static_borrow_and_ignores_a_probe():
    """⛔⛔ THE TWO DEFECTS THE TRIAGE SHIPPED WITH, both in the reassuring direction.

    A Bevy `SystemParam` type alias spells every borrow `&'static T`, so a
    pattern without the lifetime reported `OwnedPortalGunPair` as having NO
    READER when its reader is a menu query. And `fn seat_credit_probe(credit:
    &SeatCredit)` is the CHECKSUM PROBE, not a reader — counting it made the one
    component nothing in production consults look well used, hiding the exact
    case this census exists to surface.
    """
    module = _module()
    per_tick, _ = module.reader_sites("OwnedPortalGunPair")
    assert any("menu/effects.rs" in site for site in per_tick), (
        "the menu's `Option<&'static ..OwnedPortalGunPair>` is the only production "
        "reader of that row and the triage must see it; missing it reports the row "
        "as unread, which is a claim about the scan"
    )
    seat_tick, seat_gated = module.reader_sites("SeatCredit")
    assert not seat_tick and not seat_gated, (
        f"`SeatCredit` has no production reader; the triage found {seat_tick + seat_gated}. "
        "A checksum probe taking `&T` is not a reader."
    )
