# A5: who writes destructible state

**Delivered 2026-09-10; the writer table RE-DERIVED the same day at `352a08806`**
after an outside review found a production transition missing from a table headed
*"complete"*. A5's hold reads *"HOLD until A2's contact contract is
established AND writer inventory is complete."* A2 is closed. This is the second
half. It is the same shape as the [accepted-control writer
map](accepted-control-writer-map.md), which lifted half of A4's hold.

⛔ **MEASUREMENT ONLY. IT DECIDES NOTHING.** Q96 asks whether a projectile should
collide with an ECS breakable's published surface. That is a behaviour ruling and
it is not answered here. This page records who writes destructible state TODAY, so
that a later ownership change has a subject and Q96 has a population. **Nothing
here licenses an extraction.**

## The state, and where the machine lives

| carrier | crate |
|---|---|
| `Breakable` (health, state, trigger, collision) | `ambition_interaction` |
| `BreakableState { Intact, Cracking, Broken }` | `ambition_interaction` |
| `BreakableFeature { breakable }` — the ECS component | `ambition_combat` |

⭐⭐ **THE TRANSITION IS NOT IN THE MONOLITH.** `Breakable::apply_damage`
(`ambition_interaction/src/lib.rs:255`) owns the whole state machine: it damages
the health, sets `Broken` or `Cracking`, and returns `true` on the break. Its
`#[must_use]` says the caller owes the break its consequences. ⇒ Every ECS writer
below is an ORCHESTRATOR of that one method, not a second interpretation of it.

## Writers of destructible state — the complete production set

⭐ **RE-DERIVED 2026-09-10, KEYED ON THE MUTATION RATHER THAN THE QUERY.** The
first version of this table cited two systems by their `&mut BreakableFeature`
QUERY line. A query is where a system asks for write access; it is not where the
write happens, and a system can hold one query and mutate in several places. Both
citations turned out to hide a transition.

| site | writes | authority |
|---|---|---|
| `monolith features/ecs/spawn_static.rs:613` | constructs `BreakableFeature` | authored placement spawn |
| `ambition_combat/src/breakables.rs:41-42` | `state = Intact`, `health.reset()` | respawn transition |
| ⭐ `ambition_combat/src/breakables.rs:75` | `apply_damage(health.current.max(1))` | **stand-collapse transition** |
| `monolith features/ecs/damage/mod.rs:599` | `apply_damage(event.damage.max(1))` | damage transition, pogo-refresh path |
| `monolith features/ecs/damage/mod.rs:966` | `apply_damage(event.damage.max(1))` | damage transition, hit-volume path |
| `ambition_interaction/src/lib.rs:258,260` | `state = Broken` / `Cracking` | the domain state machine |

**Six mutation sites, three crates.** `begin_ecs_breakable_respawn` is
`ambition_combat`'s and is called from **three** places — `damage/mod.rs:602`,
`damage/mod.rs:989` and `breakables.rs:77` — so the respawn authority is one
place called from three, the third being the collapse this table used to miss.

### How the count moved, because the ladder is the point

⛔⛔ **"FIVE" WOULD HAVE SURVIVED THE REPAIR WITH A DIFFERENT MEMBERSHIP.** The
outside reviewer named this as a forward risk before it happened, and it is the
count-versus-membership failure that appeared four times elsewhere on 2026-09-10:

```
5   the original total, computed over a row set containing a test helper
4   delete the test row
5   add the stand-collapse transition        <- back to the original NUMBER
6   re-key the damage row from its query to its two real mutation sites
```

⇒ **Anyone who deleted and added and stopped would have printed the original
number over a membership that differs in two rows.** The total is re-derived from
the greps below, not adjusted from the old one.

### The test row is resolved: DELETED

`breakables.rs:128` was `BreakableFeature::new(b)` inside `fn stand_breakable`, a
helper under `#[cfg(test)]` at `:89`, in a table headed *"the complete production
set"*. Found by `check_planning_citations.py --roles` (`7a392427e`).

It is deleted rather than re-pointed, because **both things it might have meant
are already rows**: initial construction is `spawn_static.rs:613`, and the respawn
transition is `breakables.rs:41`. "Re-insert" was the wrong verb regardless —
respawn is a MUTATION through the per-frame tick's `&mut BreakableFeature`, and
nothing re-inserts the component.

⚠ **Deleting a row from a table claiming completeness is a claim about
completeness, which is why it waited for the stand-collapse row.** With that row
present the table can be re-derived instead of patched, and the deletion is part
of a derivation rather than a subtraction.

⭐ **The first pass classified this as a citation fix and that was wrong.** The
pointer was bad AND the note beside it was wrong. **Reading the citation and
reading the note beside it are different acts.**

## What `update_ecs_breakables` actually owns

⛔ **IT IS NOT DAMAGE PLUMBING, AND THE CRATE NAME SAYS OTHERWISE.** One system in
`ambition_combat` (`breakables.rs:10`) holds both transitions above and everything
around them:

| responsibility | site |
|---|---|
| respawn countdown, and the respawn transition | `:38-52` |
| **collapse policy**: `blocks_movement() && allows_stand()` | `:57-58` |
| stand accumulation and decay against a threshold | `:72`, `:84` |
| the collapse transition | `:73-75` |
| banner text, on both transitions | `:44`, `:79` |
| VFX burst on respawn; SFX/VFX/debris on collapse | `:45`, `:80` |

⇒ **The runtime schedules it in `FeatureInteractionSet::WorldObjects`** — added at
`monolith features/mod.rs:1381`, pinned by
`features/feature_interaction_order_tests.rs:62`, re-exported at
`features/ecs/mod.rs:116`.

⚠ **AND THE COLLAPSE RULE IS SPLIT ACROSS TWO CRATES.** The predicates are the
domain's: `BreakableTrigger::allows_stand` (`ambition_interaction/src/lib.rs:185`)
and `BreakableCollision::blocks_movement` (`:209`). The threshold and the geometry
are not: `BREAK_ON_STAND_SECONDS = 0.85` (`ambition_combat/src/lib.rs:118`) and
`player_is_standing_on` (`ambition_combat/src/util.rs:5`). ⇒ The domain type says
*whether* a breakable may collapse under weight; `ambition_combat` says *how long*
and *what counts as standing*.

⛔ **THIS IS NOT A RECOMMENDATION TO MOVE ANYTHING, AND EXPLICITLY NOT ON THE
STRENGTH OF A CRATE NAME.** It is here so A5 can decide ownership from state and
behaviour. `RespawnTimer` and `StandTimer` are this system's own orchestration
components, not destructible state, and are counted nowhere above.

## Readers — measured, and NOT writers

`damage_predicates.rs`, `projectile/systems.rs`, `projectile/intercept.rs`,
`world/physics.rs`, `world/overlay.rs`, `features/ecs/anim_helpers.rs`,
`construction/mod.rs`, `world/rooms/reconstitution.rs`, `features/ecs/summon.rs`,
`features/ecs/damage/boss_hit.rs`.

⚠ **`features/ecs/target_volumes.rs` LOOKS like a writer and is not.** It takes
`(&CenteredAabb, &BreakableFeature, &mut DamageableVolumes)`: it READS the
breakable and WRITES a different component. A first pass of this inventory counted
it as a destructible writer on the strength of a `&mut` on the same line. **A
`&mut` in a query is not a `&mut` on the thing you are counting**, and the file
with the most references (`spawn_static.rs`, 53) turned out to hold mostly
spec-to-domain converters and exactly one construction.

## What this says about A5's premise

The packet's destination is *"one logical destructible-object owner"*. ⇒ The
inventory does not find scattered authority to consolidate. It finds **a domain
state machine with one method, and five ECS sites that call or construct around
it, split across three crates.** The split is by crate, not by duplicated
interpretation.

⭐ **THE CORRECTED INVENTORY DOES NOT OVERTURN THAT, AND SAYING SO IS A FINDING.**
Two rows changed and one was added; every ECS site still calls `apply_damage` or
constructs the component. **Nothing interprets damage twice**, so the conclusion
above survives its own repair.

⚠ **What the correction DOES change is one clause.** *"Not duplicated
interpretation"* is true of damage and not of the collapse trigger: the domain
owns the predicates, `ambition_combat` owns the threshold and the standing test.
⇒ A destructible owner would inherit **one damage interpretation and a collapse
rule that is currently two-thirds in the domain and one-third beside it.**

⛔ That is a statement about writers and nothing else. It does not say the split is
wrong, and it cannot: *"moving all destructible state first would preserve an
incorrect split interpretation"* is the frontier's own warning, and whether the
interpretation is correct is what Q96 decides.

## Reproduce

```
grep -rn "&mut .*BreakableFeature" --include=*.rs crates/ game/
grep -rn "BreakableFeature::new" --include=*.rs crates/ game/
grep -rn "\.state = BreakableState::" --include=*.rs crates/
grep -rn "apply_damage" --include=*.rs crates/ game/ | grep -v "fn apply_damage"
```
Read every hit before classifying one. Two of the first three greps produce a
false positive that a count alone would keep.

⛔⛔ **THE FOURTH GREP IS NEW AND IT IS THE ONE THAT FINDS THE TRANSITIONS.** The
first three find the FILE and stop: `breakables.rs` appears through its query at
`:23`, `damage/mod.rs` through its query at `:397`, and neither hit is a write.
**A recipe that finds the right file is not a recipe that finds the right line** —
that is how a system with two transitions got recorded with one, twice.
