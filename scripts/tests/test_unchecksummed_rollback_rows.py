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
    alike, and those rows carry six different `detail` sentences. Selecting on one
    of them returned 59 of 175.

    ⚠ REPOINTED AT SCHEMA v194, AND THE REASON IS WORTH KEEPING. This arm used to
    pin the 99 rows that read *"state checksum supplied by another authoritative
    projection"*, because matching that sentence was how you could tell a
    kind-selected population from a prose-selected one. v194 removed that claim
    and those rows now say *"not in the session checksum"* — the same words the
    old selector matched. The discriminator died with the defect it was pinned to.

    ⇒ So this recomputes the population from the baseline by a DIFFERENT
    expression and diffs the MEMBERS, not the count. Any selector that consults
    `detail` at all disagrees with a selector that consults only `kind`.
    """
    module = _module()
    subjects = {name for name, _, _ in module.rows()}

    independent = set()
    for line in module.BASELINE.read_text(encoding="utf-8").splitlines()[1:]:
        fields = line.split("\t")
        if len(fields) >= 2 and fields[1].endswith("-clone") and "-" not in fields[1][:-6]:
            independent.add(fields[0])

    missing = sorted(independent - subjects)
    extra = sorted(subjects - independent)
    assert not missing, (
        f"{len(missing)} unhashed row(s) the census does not report, e.g. "
        f"{missing[:5]}. A selector that reads `detail` drops rows a selector "
        "that reads `kind` keeps."
    )
    assert not extra, f"the census reports rows outside the two kinds: {extra[:5]}"
    assert len(subjects) >= 120, (
        f"both selectors agree on {len(subjects)} rows; measured at 175. They can "
        "agree on an empty set."
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


def test_the_triage_sees_a_presence_filter_and_a_resource_read():
    """⛔⛤ TWO MORE WAYS THE TRIAGE REPORTED A ZERO, both found by widening it.

    `reader_sites` looks for a BORROW (`&T`). Two whole classes of read are not
    borrows, and both arrived with the 59 -> 175 widening:

    * A MARKER IS NEVER BORROWED, ONLY FILTERED ON. `FeatureSimEntity` came back
      with NO PRODUCTION READER while 81 sites spell `With`/`Without`/`Has` of it.
    * A RESOURCE IS READ THROUGH `Res<T>`. `resource-clone` rows were outside this
      census entirely until the selector moved to the kind, so nothing here had
      ever been a resource; `SaveRestored`, `FriendlyFire` and
      `PortalFrameHistory` all reported zero readers while each is a live
      `Res`/`ResMut` parameter.

    Both zeros were in the reassuring direction. 20 rows read as unread; 1 is.
    """
    module = _module()
    assert len(module.presence_filter_sites("FeatureSimEntity")) >= 20, (
        "the triage cannot see a `With`/`Without` filter; a marker component has "
        "no other kind of reader, so it reports every marker as unread"
    )
    assert not module.presence_filter_sites("SaveRestored"), (
        "a resource is not presence-filtered; if this matches, the pattern is "
        "loose enough to match anything"
    )
    assert len(module.resource_read_sites("SaveRestored")) >= 5, (
        "the triage cannot see a `Res`/`ResMut` system parameter"
    )
    assert not module.resource_read_sites("FeatureSimEntity"), (
        "a component is not a `Res`; if this matches, the pattern is loose"
    )


def test_local_player_is_registered_for_rollback_and_read_by_nothing():
    """⛔ THE ONE ROW THE WIDENED TRIAGE STILL CANNOT FIND A READER FOR.

    `player.local_marker` is `component-clone`: snapshotted every frame, not in
    the session checksum, no probe. Its 16 production mentions are a definition, a
    re-export, two insertions, a rollback registration and doc comments — not one
    read. Its only `With<LocalPlayer>` is in `smash_in_the_host.rs`, a test.

    ⚠ THIS ARM IS PINNED TO A LIVE FINDING AND WILL DIE WHEN IT IS FIXED. If it
    fails because a production reader appeared, that is the good outcome: delete
    the arm and the row's entry. If it fails because the triage stopped seeing
    reads, that is the bad one — check `FeatureSimEntity` and `SaveRestored`
    above first, since those are the known-answer controls.
    """
    module = _module()
    per_tick, gated = module.reader_sites("LocalPlayer")
    assert not per_tick and not gated, f"borrowed somewhere now: {per_tick + gated}"
    assert not module.presence_filter_sites("LocalPlayer"), (
        "filtered on in production now — this row has a reader"
    )
    assert not module.resource_read_sites("LocalPlayer"), "LocalPlayer is not a resource"
