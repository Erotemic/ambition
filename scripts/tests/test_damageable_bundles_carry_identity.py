"""D-DAMAGEABLE-BODY-IDENTITY: a ratchet on bodies built OUTSIDE construction.

⭐⭐ **THE QUESTION IS NOT "WHO FORGOT AN IDENTITY".**
`ambition_platformer2d_shared_tangle::construction`'s `commit_entity` spawns the
root and inserts `planned.sim_id`, the origin and the transaction stamp BEFORE
the recipe runs — its own comment says *"so a recipe cannot forget them"*. A body
built through that road CANNOT lack identity. The population that can is the
bodies assembled somewhere else, and a bundle-shaped scan can see exactly those.

⛔⛔ **A LOWER BOUND, RATCHETED — AND SAYING SO IS THE POINT.** A body that
receives `CenteredAabb` from one insert and `ActorFaction` from another is
invisible to this scan. It is NOT a census of unidentified bodies and this file
never claims to be one; the completeness arm is the runtime census
(`damageable_bodies_carry_identity` in `game/ambition_app/tests`), which walks
`StrikeVictim`'s own query but reaches only the roads it exercises.

⇒ What a ratchet on a lower bound buys is real and narrow: **a NEW damageable
bundle assembled without identity cannot be added silently.** It says nothing
about the ones the method cannot see.

⚠ AND THE FLOOR MATTERS AS MUCH AS THE CEILING. A scan that matched nothing —
a renamed bundle type, a changed spawn spelling — would report the same zero as a
perfectly identified tree. The population is floored so "found nothing" cannot be
mistaken for "nothing is wrong".
"""

from __future__ import annotations

import importlib.util
import pathlib

MEASURE = (
    pathlib.Path(__file__).resolve().parent.parent
    / "measure_damageable_bundles_without_identity.py"
)

# ⛔⛔ ZERO, AND IT CAME DOWN BY RE-DERIVING RATHER THAN BY AN EDIT. Both former
# leads were read by hand, and BOTH were false:
#   * `actor_monolith/src/character_runtime/match_activation.rs` — a match
#     fighter. Cleared by a live-match census
#     (`ambition_demo_smash_app/tests/every_fighter_in_a_match_carries_identity.rs`,
#     2026-09-11): 2 of 2 seated fighters identified.
#   * `game/ambition_content/src/bosses/cut_rope/victory.rs` — a damageable
#     post-boss NPC spawned bare. This file called it REAL; it is not.
#     `ensure_sim_id` names it `SimId::placement(CUT_ROPE_VICTORY_NPC_ID)` from
#     the `FeatureId` its own `FeatureRenderedBundle` carries. ⛔ Adding an
#     explicit `SimId` there was tried and REVERTED: it is a second authority for
#     a value the derive already computes, and the two would drift.
# ⇒ The scan now classifies a `DERIVABLE` site separately, so a "lead" means what
#   it says: nothing in the tree can name this body. A ratchet, never
#   re-baselined upward to make a new bundle green.
UNIDENTIFIED_BUNDLE_CEILING = 0

# ⭐ THE FLOOR ON THE WHOLE POPULATION, not on the offenders. If this scan stops
# matching damageable bundles at all it reports zero offenders, which reads
# identically to success.
DAMAGEABLE_BUNDLE_FLOOR = 4


def _module():
    spec = importlib.util.spec_from_file_location("measure_damageable_bundles", MEASURE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _rows():
    """(every damageable bundle site matched, the LEADS among them).

    ⛔⛔ **THIS USED TO RE-IMPLEMENT THE WALK, AND THAT IS A SECOND AUTHORITY ON
    WHAT A LEAD IS.** The copy classified a site by `IDENTITY` and recipe-host
    alone; when the measure script learned that `ensure_sim_id` DERIVES an
    identity from an authored `FeatureId`, the copy did not, so this ratchet went
    on guarding a classification its own subject had stopped using. It calls
    `classify()` now — the module's one keeper.

    ⛔ THE TWO LISTS ARE DIFFERENT POPULATIONS AND AN EARLIER VERSION CONFLATED
    THEM. It dropped construction-recipe sites from BOTH, so the floor — which is
    about whether the SCAN still matches anything — was computed over the
    offenders only and reported 2 where the scan matches 4. A floor measured on
    the filtered set cannot see the filter itself going wrong.
    """
    rows = _module().classify()
    return rows["matched"], rows["leads"]


def test_no_new_damageable_bundle_is_assembled_without_an_identity() -> None:
    _matched, leads = _rows()
    assert len(leads) <= UNIDENTIFIED_BUNDLE_CEILING, (
        f"{len(leads)} damageable bundles assembled with no `SimId` in the "
        f"same call, outside a construction-recipe host (ceiling "
        f"{UNIDENTIFIED_BUNDLE_CEILING}). A body in `StrikeVictim`'s population "
        "without a stable identity cannot win a geometric tie, so every tie it "
        "takes part in is resolved by Bevy query order — which a rollback "
        "resimulation does not promise to reproduce:\n  "
        + "\n  ".join(f"{w}:{n}" for _, w, n in sorted(leads))
    )


def test_the_scan_still_matches_damageable_bundles_at_all() -> None:
    """⛔⛔ THE FLOOR, AND IT IS NOT DECORATION.

    Change how a body is spawned, or empty the scan's `DAMAGEABLE` list, and it
    matches nothing and reports zero offenders — the same green a perfectly
    identified tree gives. The ceiling can only ever see the number GROW; nothing
    in it notices the instrument going blind.

    ⚠ **MEASURED, AND THE FLOOR IS COARSER THAN IT LOOKS.** Poisoned by renaming
    ONE token (`CenteredAabb`) out of `DAMAGEABLE`, this test still PASSES — the
    four matched sites match on the BUNDLE types beside it, so losing one token
    costs no rows. It fails only when the whole list is blinded. ⇒ It catches the
    instrument dying, not the instrument narrowing. Narrowing is what the row's
    other arm — the runtime census over `StrikeVictim`'s own query — is for.
    """
    matched, _leads = _rows()
    total = len(matched)
    assert total >= DAMAGEABLE_BUNDLE_FLOOR, (
        f"the scan matched {total} damageable bundles and this tree had at least "
        f"{DAMAGEABLE_BUNDLE_FLOOR} when the floor was set. Fewer means the "
        "matcher stopped seeing the population, not that the population shrank — "
        "check `DAMAGEABLE` / `FACTION` in "
        "`scripts/measure_damageable_bundles_without_identity.py` against the "
        "bundle types that exist now"
    )
