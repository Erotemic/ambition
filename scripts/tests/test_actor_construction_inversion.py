"""F1: the actor construction domain must not name the feature layer.

⭐⭐ WHAT THIS GUARD IS FOR, AND WHY IT IS TWO ASSERTIONS RATHER THAN ONE.
The work frontier's F1 packet asks for one thing — *"production `construction`
must stop naming `features`"* — and that is a single grep. But a reverse
dependency can be SATISFIED and RELOCATED by the same edit: move the symbols one
module down and `construction` names the new module instead, while the cycle
closes through it. The packet forbids exactly that (*"no new upward dependency is
introduced to hide the old one"*), and a direct-reference check cannot see it.

⇒ So the first test pins the requirement and the second pins the ESCAPE HATCH:
the spawn primitives' upward reach into `features` is a number that may only go
down. Together they say "construction stopped naming features, and the edge did
not simply move".

⛔ THE SECOND NUMBER IS NOT ZERO AND THE CARVE IS NOT FINISHED. At the time of
writing `crate::actor_spawn` still names `crate::features` sixteen times, so
`construction -> actor_spawn -> features -> construction` still closes. That is
recorded here as a CEILING rather than hidden, because the honest state of a
half-finished inversion is a number somebody can drive down, and a guard that
asserted zero would have to be deleted to land the first half.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
SRC = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "src"

# ⛔ A COMMENT NAMING A MODULE IS NOT AN EDGE. Three of the references this file
# would otherwise count are prose explaining the cut itself.
LINE_COMMENT = re.compile(r"//.*$", re.MULTILINE)

#  the residual reach measured 2026-09-06, after two PURE LEAVES
# (`brain_builders`, `actor_clusters` — no `crate::` and no `super::` reference
# between them) and `conversion` came down with the primitives. 16 -> 13.
#
# ⚠ AND THE FIRST VERSION OF THIS NUMBER RESTED ON A BAD MEASUREMENT: I called
# three modules "pure leaves" having counted only `crate::` paths, and
# `conversion.rs` reaches `autonomous_reconcile` through `super::super::`. A
# relative path is a reference. Two of the three were genuinely pure; the third
# traded two references for one and is still worth having moved.
#
# What is left stands on five: `ecs::spawn::character_spawn_plan` (3),
# `ecs::held_items` (3), `npcs` (3, one of them a re-exported constant),
# `ecs::spawn_static` (1), `ecs::autonomous_reconcile` (1), plus
# `ecs::boss_component_snapshot` and the `EnemyActorBundle` pair.
PRIMITIVE_UPWARD_REACH_CEILING = 5


def _production_files(root: pathlib.Path) -> list[pathlib.Path]:
    """Production sources only: a test reaching across modules is a fixture."""
    return [
        p
        for p in sorted(root.rglob("*.rs"))
        if "tests" not in p.name and "/tests/" not in str(p)
    ]


def _references(path: pathlib.Path, target: str) -> list[str]:
    """Every mention of `crate::<target>`, in EVERY spelling it has.

    ⛔⛔ THE FIRST VERSION MATCHED `crate::features::[A-Za-z_]\\w*` AND ITS POISON
    PASSED. A brace import — `use crate::features::{SpawnActorKind, ...}` — has
    `{` after the `::`, so the exact line this guard exists to catch was the one
    spelling it could not see, and putting the original reverse dependency back
    left it green. ⇒ Match the module path and let the tail be anything: a
    one-spelling search is how a guard comes to assert nothing at all.
    """
    text = LINE_COMMENT.sub("", path.read_text(encoding="utf-8", errors="ignore"))
    return re.findall(rf"crate::{target}\b(?:::[A-Za-z_{{][A-Za-z_0-9]*)?", text)


def test_the_construction_domain_does_not_name_the_feature_layer() -> None:
    """⛔ THE F1 REQUIREMENT ITSELF. `construction/` held the actor domain's
    `ConstructionDomain` impl and its nine `construct_*` recipes, and every one
    of the fifteen names it reached upward for was defined in ONE file —
    `features/ecs/spawn_actors.rs`, the spawn primitives. A recipe consuming a
    primitive is the right direction; the module holding the primitives being
    ABOVE the recipes is not."""
    offenders: list[str] = []
    for path in _production_files(SRC / "construction"):
        for ref in _references(path, "features"):
            offenders.append(f"{path.relative_to(REPO)}  {ref}")
    assert not offenders, (
        "the actor construction domain names the feature layer again, which is "
        "the reverse dependency F1 removed — a recipe may consume a spawn "
        "primitive (`crate::actor_spawn`), never a feature system:\n  "
        + "\n  ".join(offenders)
    )


def test_the_spawn_primitives_upward_reach_only_shrinks() -> None:
    """⛔⛔ THE HALF A DIRECT-REFERENCE CHECK CANNOT SEE.

    `construction` stopped naming `features` because the symbols moved to
    `crate::actor_spawn`. If that module's own reach into `features` were free to
    grow, the edge would simply have relocated and the test above would keep
    passing while the cycle got longer. ⇒ This is the ratchet on the relocation:
    the number may fall to zero (which finishes the carve) and may not rise."""
    primitives = SRC / "actor_spawn"
    assert primitives.is_dir(), f"{primitives} moved; re-point this guard"
    # ⛔ THE WHOLE MODULE, NOT ONE FILE. It became a directory the moment leaf
    # helpers came down with it, and a guard reading `actor_spawn.rs` would have
    # measured a file that no longer exists — or, worse, one of several.
    reach = [
        ref
        for path in _production_files(primitives)
        for ref in _references(path, "features")
    ]
    assert len(reach) <= PRIMITIVE_UPWARD_REACH_CEILING, (
        f"the spawn primitives now reach {len(reach)} times into `features` "
        f"(ceiling {PRIMITIVE_UPWARD_REACH_CEILING}). F1's cycle is not closed "
        "yet, so this number is what stands between the two — raising it moves "
        "the reverse dependency deeper instead of removing it:\n  "
        + "\n  ".join(sorted(set(reach)))
    )


def test_the_checker_would_see_a_reference_if_there_were_one() -> None:
    """The positive control. Without it the first test passes against a reader
    that never finds anything — and the reference it must find is spelled
    exactly as the real ones were."""
    probe = SRC / "construction" / "mod.rs"
    text = probe.read_text(encoding="utf-8")
    assert "crate::actor_spawn::" in text, (
        "construction no longer names the primitives at all, so the first test "
        "is passing because there is nothing to find rather than because the "
        "direction is right"
    )
    assert len(_references(probe, "actor_spawn")) >= 10, (
        "the fifteen recipe/type references that used to point at `features` "
        "should now point at `actor_spawn`; far fewer means they went somewhere "
        "this guard is not watching"
    )
