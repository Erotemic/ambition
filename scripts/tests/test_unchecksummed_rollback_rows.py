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
    assert len(subjects) >= 120, (
        f"{len(subjects)} rows carry an unhashed value-bearing kind; measured at "
        "175 on 2026-09-16. A number near 59 means the selector went back to "
        "matching the 'not in the session checksum' sentence, which is one of six "
        "sentences that kind can carry."
    )
    # ⛔ EVERY ROW MUST RESOLVE TO A DEFINITION. A `?` row is the census failing
    # to find a type and reporting it as a row with no floats, which reads as
    # SAFER than the truth.
    unresolved = [name for name, ty, _ in subjects if module.definition(ty)[0] == "?"]
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


def test_the_population_is_the_KIND_and_not_the_SENTENCE():
    """⛔⛤ THE CENSUS SELECTED ON PROSE FOR SIX DAYS AND SAW A THIRD OF ITS SUBJECT.

    `feeds_peer_checksum()` is false for `component-clone` and `resource-clone`
    alike, and those rows carry SIX different `detail` sentences. Selecting on
    one of them — *"not in the session checksum"* — returned 59 of 175. The 116
    it missed mostly say *"state checksum supplied by another authoritative
    projection"*, which is the reassuring direction: a claim, emitted by a
    registrar method bound only by `T: Clone`, that something else covers them.

    This arm dies if the selector goes back to matching a sentence.
    """
    module = _module()
    subjects = module.rows()
    claiming = [r for r in subjects if "supplied by another" in r[2]]
    probed = [r for r in subjects if module.PROBED_DETAIL in r[2]]
    assert len(claiming) >= 80, (
        f"only {len(claiming)} rows in this population claim another projection "
        f"covers them; measured at 99. If this is near zero the selector is "
        "matching prose again and the unverifiable half is invisible."
    )
    assert len(probed) >= 40, (
        f"only {len(probed)} rows carry the honest probe sentence; measured at 59"
    )
    # ⚠ AND THE TWO HALVES MUST NOT BE THE SAME ROWS, or the split above is a
    # description of one bucket counted twice.
    assert not (
        {r[0] for r in claiming} & {r[0] for r in probed}
    ), "a row cannot both claim coverage and say it is uncovered"
    assert len(subjects) > len(probed) + 10, (
        "the population is no wider than the old sentence-keyed one"
    )


def test_resource_rows_are_in_the_population():
    """⚠ `resource-clone` is unhashed for the same reason `component-clone` is.

    The old selector hard-coded `parts[1] == "component-clone"`, so eight
    resource rows with identical mechanical standing were outside the census
    twice over — once by kind and once by sentence.
    """
    module = _module()
    names = {name for name, _, _ in module.rows()}
    assert any(n.startswith("resource.") for n in names), (
        "no `resource.*` row in the population; the kind filter dropped them"
    )
