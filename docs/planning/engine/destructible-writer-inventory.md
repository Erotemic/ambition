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

## Q96 is ruled: the COMPOUND CONTACT

⭐⭐ **RULED 2026-09-10.** A published collision surface participates in projectile
collision, and a contributor supplying both a surface and a damageable volume at
the same time of impact yields **ONE compound contact**: damage the target once AND
apply the projectile's physical surface response. Exemptions are a projectile's
POLICY against a collision CLASS, never a per-target carve-out. Contributor identity
must be **real identity** — not matching AABBs, not name strings. Ruling in
[`maintainer-decisions.md`](../maintainer-decisions.md); engineering state in the
[projectile contact protocol](projectile-contact-protocol.md).

⇒ **What it changes for THIS page: nothing in the table.** The inventory is about
writers, the ruling is about contact. The six mutation sites and three crates are
unaffected.

⇒ **What it changes for A5: contributor identity is now REQUIRED for projectiles**,
not only for the player road. ⚠ **That is a statement about what must EXIST, not
about which crate should own it.** The ownership question is unchanged and this page
still decides nothing.

⛔ **AND THE IDENTITY CLAUSE IS THIS SESSION'S OWN DEFECT FAMILY, NOW RULED.** "Not
inferred from matching AABBs or name strings" is the same failure the SystemSet
census hit on the same day: **135 declarations collapsing to 134 names**, where a
shared name pools memberships and hides an empty set behind a populated one. **A key
two things can share is not an identity.**

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

`breakables.rs:128` was `BreakableFeature::new(b)` inside `fn stand_breakable`, a <!-- cite-test: a `#[cfg(test)]` line cited ON PURPOSE — the row's claim IS that this line is a test, so a production-role citation here would mean the opposite of what it says. Triaged individually 2026-09-12, not swept. -->
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

⭐ **Q96 IS NOW RULED (see the top of this page), so that sentence has a successor
rather than a blocker** — and the successor is contributor identity, not a move.

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

## Re-derived 2026-09-11, and the transition-authority census A5 asks for

**BOTH HALVES OF THE HOLD ARE SATISFIED.** A2's contact contract closed at
`0157476ba`; this inventory is the other half and it reproduces at HEAD — all six
production mutation sites, at the same lines, with `0157476ba` adding none
(it added fixtures and a publisher comment). The `breakables.rs:128` row is still <!-- cite-test: a `#[cfg(test)]` line cited ON PURPOSE — the row's claim IS that this line is a test, so a production-role citation here would mean the opposite of what it says. Triaged individually 2026-09-12, not swept. -->
a `#[cfg(test)]` helper and still correctly excluded. ⚠ The frontier's A5 row
still reads *"HOLD until … writer inventory is complete"* and links neither page;
that is the third time a frontier summary has outlived the page it summarises
(see A7 and A6).

### Falling chests, chests and breakables do NOT share a transition authority

The packet says *"Falling chests and switches are separate mechanisms unless they
demonstrably share the same transition authority."* Measured, they are three
different mechanisms and no two of them share one:

| family | where the transition lives | what it writes |
|---|---|---|
| **Breakable** | `Breakable::apply_damage`, a `#[must_use]` DOMAIN method | `BreakableState` on the domain struct |
| **Chest** | `commands.entity(e).insert(Opened)` — an ECS MARKER COMPONENT | no domain state at all |
| **FallingChest** | nothing — it is a position tick | `CenteredAabb.center`, then removes its own marker |

⇒ A destructible owner would inherit **one** of these. There is no consolidation
available because there is nothing duplicated to consolidate: the three do not
disagree about a transition, they do not have the same kind of transition.

⛔⛔ **AND THE CHEST FINDING IS THE ONE WORTH THE SPACE: `Chest::state` IS
WRITE-ONLY.** `ChestState { Closed, Opening, Opened }` is constructed, mapped from
`ChestStateSpec` at authored spawn (`spawn_static.rs:106`), serialized — and read <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
by **nothing in production**. The only read in the repository is an assertion
inside `ambition_interaction`'s own test module. The runtime's open-gate is the
`Opened` marker, written at **five production sites across three crates**:
`features/ecs/chests.rs:98`, `features/ecs/encounter_rewards.rs:74` and `:101`,
`ambition_boss_encounter/src/rewards.rs:73` and `:106`.

⇒ **Nothing derives the marker from the authored state.** A chest authored
`Opened` would be constructed with `ChestState::Opened`, carry no `Opened`
marker, and be opened again by `open_ecs_chests` — granting its reward a second
time, which is A5's own acceptance line *"no duplicate effects/rewards"*.

⚠ **IT IS LATENT, NOT LIVE, AND THE DIFFERENCE IS THE POPULATION.** Nothing can
author an open chest: LDtk's `ChestSpawn` entity declares exactly two fields —
`name` and `reward` — in all four shipped worlds; `ChestSpec::new` defaults
`state: ChestStateSpec::Closed`; and **no converter anywhere populates
`ChestStateSpec`**. So `Opening` and `Opened` are unreachable from content, two of <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
the three spec variants are dead, and both non-`Closed` arms of
`chest_state_from_spec` are dead with them. ⇒ The honest statement is *a <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
write-only field whose two unreachable variants would be a duplicate-reward bug
the day something authors one* — not *a bug*.

### Geometry agreement: the WHERE agrees by construction, the WHETHER does not

A5's acceptance names *"melee/projectile geometry agreement"*. Both publishers
read the SAME `CenteredAabb`, so a breakable's hurt volume and its contributed
surface are the same rectangle for structural reasons rather than by coincidence
— and that is exactly the kind of agreement that stays true until somebody
offsets one, silently and asymmetrically. ⇒ Guarded now, and poison-verified by a
3px offset:
`world/overlay.rs::breakable_geometry_agreement`.

⚠ **THE ELIGIBILITY PREDICATES DELIBERATELY DIVERGE, AND THE GUARD DOES NOT TOUCH
THAT.** The two systems answer different questions:

| breakable | damageable volume (`trigger.allows_hit() \|\| pogo_refresh`) | contributed surface (`collision != None && !pogo_refresh`) |
|---|---|---|
| `OnHit`, `Solid` | ✔ | ✔ |
| `OnStand`, `Solid` | ✖ | ✔ — a surface with no hurt volume, on purpose |
| `OnHit`, `None` | ✔ | ✖ |
| `pogo_refresh` | ✔ | ✖ — it contributes through `PogoTargetContributor` instead |

Row two is the discriminating case `0157476ba` found: with no surface a bolt flies
through a solid crate it cannot damage. Row four is why the overlay skips
pogo-refresh breakables, documented at the site.

### The authored population: the solid variant IS selected, six times

`0157476ba` closed compound contact after finding that *"every projectile-vs-breakable
fixture in the suite uses a non-solid crate, because `Breakable::new` defaults
`collision` to `None`"*, and expected the same blind spot here — *"a destructible's
solid variant is the configuration nobody's fixture selects."* ⇒ **Measured over the
four shipped LDtk worlds, that is true of the FIXTURES and false of the CONTENT.**

| authored entity | collision | trigger | count |
|---|---|---|---:|
| `BreakablePlatform` | `Solid` | `OnHit` | **4** |
| `BreakablePlatform` | `Solid` | `OnStand` | **2** |
| `BreakablePlatform` | `OneWayUp` | `OnStand` | 3 |
| `BreakablePogoOrb` | *(no such fields)* | | 5 |

**14 authored breakables; six are solid.** ⭐ And the two `Solid` + `OnStand`
platforms are exactly the discriminating case that commit's own poison identified —
a solid object a shot CANNOT damage, which a bolt flew straight through before the
fix. The repair was not hypothetical: it changed behaviour for two shipped
placements, and no fixture in the repository selected that configuration.

⚠ `BreakablePogoOrb` declares neither `collision` nor `trigger`, so all five take
the type's defaults. A census keyed on the authored FIELD would report five
breakables with no collision setting; they have one, and it is `None` by the
constructor rather than by an author.

### What this adds to A5's premise

The inventory already found no scattered authority to consolidate. This adds the
neighbouring families and finds the same: **three mechanisms, three different
kinds of transition, nothing duplicated between them.** ⇒ *"Falling chests and
switches are separate mechanisms"* is not a caveat A5 has to work around — it is
the measured answer, and the packet should say so plainly rather than leave it
conditional. ⛔ **No type moved and none is proposed.**

⇒ What A5 could still land, smallest first: derive the `Opened` marker from the
authored state at spawn (or delete the two unreachable `ChestStateSpec` variants <!-- cite-ok: the DELETED chest-state vocabulary, named on purpose. These rows are the CENSUS that justified removing it (2026-09-12); a resolvable citation here would mean the deletion did not happen. See Q105. -->
and the write-only field with them) — a decision about whether an author should
ever be able to place an already-opened chest, which is content design rather than
ownership.
