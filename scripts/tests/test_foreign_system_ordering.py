"""Prerequisite C: cross-domain ordering must not name foreign system identities.

⭐⭐ THE RULE, from the architecture program: *"Every load-bearing cross-capability
ordering relationship must be expressible using public phase/set vocabulary rather
than foreign system identities."* A crate that fixes the relative order of two
OTHER crates' private systems has taken authority over domains it does not own,
and moving those domains into separately composable crates does not make them
separately composable — the edge is in the code, not in the packaging.

⛔ THIS IS A RATCHET, NOT A GATE, AND THE NUMBERS ARE NOT ZERO. Eighty-seven of
these exist today. A test asserting zero would have to be deleted the moment
anybody ran it, which is how a measurement stops being kept; a ceiling that may
only fall is what survives the distance between "measured" and "fixed".

⚠ THE MEASURE ITSELF TOOK FOUR CORRECTIONS AND EACH ONE MATTERED, recorded in
`scripts/measure_foreign_system_ordering.py` so the next reader inherits them
rather than the conclusions:

* a bare module path (`actors::sync_visuals`) is INTRA-crate, not foreign — the
  first run said 28 violations and most were a crate ordering itself;
* being the runtime is not an exemption, and the version that made it one
  excused the architecture note's own named example;
* `.before(...)` is not the only spelling — that example is a CHAINED TUPLE and
  a regex over `.before(` scored it as absent;
* but a chain is only the defect when it spans TWO DIFFERENT foreign crates. A
  composition layer sequencing systems it installs is what a runtime is for; the
  version without that discriminator reported 174 and most of it was a runtime
  doing its job.
"""

from __future__ import annotations

import importlib.util
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
MEASURE = REPO / "scripts" / "measure_foreign_system_ordering.py"

#  measured 2026-09-06. Both may fall; neither may rise.
#
# ⭐ 15 -> 10 AND 87 -> 82 IN THREE STEPS, AND ONLY TWO OF THEM WERE FIXES:
#   · the dismount worked example (below) — 15 -> 13;
#   · `apply_summon_effects` joining `EffectExecutionSet` — 13 -> 12;
#   · a peer's dormancy fix — the set was already published and unused;
#   · the two `scripted_input.rs` edges, redundant with a set edge already
#     beside them — 9 -> 7;
#   · ⛔ and a MEASUREMENT correction, 12 -> 10: `#![cfg(test)]` is a FILE-level
#     gate, invisible both to a filename heuristic and to the inline-`mod`
#     stripper, so `features/ecs/fighter_harness.rs` was counted as production
#     and contributed two false violations. Exactly one file in the tree carries
#     that attribute, which is why it hid — a rule with a single instance is one
#     nobody trips over until it matters.
#
# ⭐ 15 -> 13 AND 87 -> 85 BY THE WORKED EXAMPLE: `ambition_mount` now publishes
# `DismountRequestsApplied`, the runtime installs `apply_dismount_requests` into
# it, and `ambition_demo_smash`'s two shark-ride orderings name the SET. The
# ruleset was naming the function because there was nothing else to name — the
# system belonged to no published set — which is the shape of most of what is
# left here.
CAPABILITY_ORDERING_CEILING = 7
TOTAL_ORDERING_CEILING = 79


def _module():
    spec = importlib.util.spec_from_file_location("measure_foreign_ordering", MEASURE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _rows():
    module = _module()
    foreign = [r for r in module.findings(include_local=False) if r[3] == "foreign"]
    ordering = [r for r in foreign if r[4] == "ordering"]
    capability = [r for r in ordering if not module.is_composition_layer(r[0])]
    return ordering, capability


def test_a_capability_does_not_order_another_crates_systems() -> None:
    """⛔ THE SHARP HALF. A ruleset or capability crate reaching into another
    crate's schedule is the case with no defence at all: it is not composing
    anything, it is asserting that somebody else's system runs at a particular
    moment relative to its own."""
    _, capability = _rows()
    assert len(capability) <= CAPABILITY_ORDERING_CEILING, (
        f"{len(capability)} capability-written foreign orderings "
        f"(ceiling {CAPABILITY_ORDERING_CEILING}). Each one is a crate that "
        "cannot be composed away from the crate it names:\n  "
        + "\n  ".join(f"{c} -> {t}  ({w})" for c, t, w, _, _ in sorted(set(capability)))
    )


def test_foreign_ordering_overall_only_shrinks() -> None:
    """The whole population, composition layers included — because the runtime
    owning PHASES is not the same as the runtime owning the pairwise order of two
    capabilities' private systems, and only the second is counted here."""
    ordering, _ = _rows()
    assert len(ordering) <= TOTAL_ORDERING_CEILING, (
        f"{len(ordering)} foreign orderings (ceiling {TOTAL_ORDERING_CEILING}); "
        "the prerequisite is that this number reaches zero, so it may not rise"
    )


def test_the_capability_split_is_not_degenerate() -> None:
    """⛔⛔ THE POISON THAT CAUGHT THIS ONE PASSED, WHICH IS WHY IT EXISTS.

    Making `is_composition_layer` return `True` for everything — the exact
    regression an earlier version of the measure shipped, excusing the runtime —
    empties the capability bucket, so a ceiling of 15 is satisfied by zero and
    both other tests stay green. ⇒ A ceiling can only see the number growing; it
    cannot see the CLASSIFIER collapsing, and a collapsed classifier reports
    perfect compliance.

    ⭐ So this pins the split itself against a case with no ambiguity:
    `ambition_demo_smash` is a RULESET. If it is ever counted as a composition
    layer, the split has stopped meaning anything."""
    module = _module()
    assert not module.is_composition_layer("ambition_demo_smash"), (
        "a ruleset crate is being classified as a composition layer, so every "
        "capability-written ordering is being excused as 'the runtime owns global "
        "order' — which is the exemption the architecture note's own example "
        "disproves"
    )
    assert module.is_composition_layer("ambition_platformer2d_runtime"), (
        "the runtime is no longer recognised as a composition layer, so the "
        "split has collapsed the other way and every row reads as a capability"
    )
    _, capability = _rows()
    assert capability, (
        "zero capability-written foreign orderings would be excellent news and is "
        "not what this tree contains; a zero here means the measure stopped "
        "finding them rather than that somebody fixed them"
    )


def test_the_measure_still_sees_the_example_the_program_named() -> None:
    """⭐⭐ THE POSITIVE CONTROL, AND IT IS A SPECIFIC ONE ON PURPOSE.

    The architecture note names one instance by hand:
    `ambition_mount::enforce_mount_rider_link` chained with
    `actor_monolith::rebuild_dismounted_rider_brains`. Two versions of this
    measure scored it as absent — once by excusing the runtime, once by matching
    only `.before(`. ⇒ Pinning the named example means a future simplification of
    the matcher cannot quietly stop finding the thing it was built for."""
    ordering, _ = _rows()
    named = {t for _, t, _, _, _ in ordering}
    assert any("enforce_mount_rider_link" in t for t in named), (
        "the measure no longer sees `ambition_mount::enforce_mount_rider_link`, "
        "which the architecture program names as the reference instance"
    )
    assert any("rebuild_dismounted_rider_brains" in t for t in named), (
        "the measure no longer sees the other half of the named chain"
    )
