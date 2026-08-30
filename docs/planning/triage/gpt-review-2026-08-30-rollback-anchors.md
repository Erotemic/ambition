# GPT re-review 2026-08-30 — dynamic spawn anchors, schedule order, deterministic selection

Snapshot reviewed: `e3499e3d2f57`, 42 commits after the `23e472c39fd4` tree of
[the Rust-correctness review](gpt-review-2026-08-29-rust-correctness.md). Cargo was
unavailable in the reviewer's environment, so every finding below is source review
until this file says it was measured here.

**The re-review's own headline**, and it is the right one: *registering a component
for rollback is being confused with making the dynamically spawned entity
participate in rollback.* `rollback_component_clone::<T>` installs a CODEC.
`require_rollback::<T>` installs the ANCHOR. A type can have the first and lack the
second, and every registry-shaped check reads that as covered.

The repo already owns the assertion that catches it —
`assert_no_inert_registrations` in `game/ambition_app/tests/rollback_coverage.rs`,
which says in as many words that a registration on an unanchored archetype is
INERT. What it lacks is a POPULATION: it sweeps a booted room and a live match, and
its own doc comment already warns that state which only exists after an EVENT is
structurally invisible to it. That is the gap, and it is the gap the review found
twice independently.

## Rows

| # | Row | Rank | State |
|---|-----|------|-------|
| 1 | Quality seed runs in `PreStartup`, settings load in `Startup` | P1 | ▣ landed |
| 2 | `PortalShot` has a codec and no anchor | P0/P1 | ▣ landed |
| 3 | `FallingHazard` has a codec and no anchor (after an attempted fix) | P1 | ▣ landed |
| 4 | Sentry / Vortex: rollback lifetime, effective allegiance, deterministic targets | P1 | ◐ lifetime landed; allegiance + ordering open |
| 5 | One spawn seam for authoritative dynamic sim state | architecture | ◐ named seams landed; a universal `spawn_sim_entity` is argued against below |
| 6 | Scenario-driven inert/coverage sweep that fires abilities | architecture | ▣ landed |
| 7 | `portal_fire_system` keeps only the LAST intent per tick | P1 | ▣ landed |
| 8 | `collect_ecs_pickups` / `collect_world_items` decide by query order | P1 | ▣ landed |
| 9 | Deterministic selection as a shared primitive (metric, then `SimId`) | architecture | ▣ landed; three adopters, more to convert |
| 10 | Fuse arming still reads `vel != ZERO` instead of `Release::Throw` | P1 | ▣ landed |
| 11 | Submerged: sweep knows the passability policy, penetration repair does not | P1 | ▣ landed |
| 12 | Fighter-brain L3 rollout — **measure before changing** | — | ☐ deliberately not a code change |

Rows carried forward from the 2026-08-29 review and NOT re-litigated here: Mary-O
`Local` room-transition state, quest/map-room edge detectors, held-projectile
attacker provenance, folding `HeldProjectile` into `ProjectileSpawnRequest`,
`ControlledSubject` vs `DrivenBodies` for custom item abilities, and the D199
swept-AABB / one-way policy. They stay open in
[their own file](gpt-review-2026-08-29-rust-correctness.md).

---

## ▣ 1 — The hardware quality seed migrated nobody

**Confirmed by measurement, not by reading.** Four arms were written against the
shipped schedule first and three of them were RED: an existing settings file on a
`Cpu` adapter resolved `High` where the policy says `Potato`, and the
`hardware_seeded` guard came back `false`.

The order that shipped:

```text
PreStartup   default UserSettings -> seed reads the adapter -> hardware_seeded = true
Startup      load_settings_at_startup  *settings = load_settings(path)
             ...which REPLACES THE WHOLE RESOURCE: tier back to the file's,
             hardware_seeded back to false
Update       resolve the budget from the un-seeded tier
```

and the system is startup-only, so it never runs again. The machine the feature was
written for — an existing install on an integrated GPU carrying a persisted `High`
from the OS-based default — migrated on **no boot, ever**.

⭐ **A fresh install migrated correctly**, because the loader returns early when
there is no file. That is why the pure unit tests on `seed_from_hardware` and a
first-run play-through both looked right: the only broken path was the one nobody
tests, an upgrade.

**Fix:** `PreStartup` → `PostStartup`. Still before the first `Update`, so the
resolved budget is seeded on frame one; now after the loader, so the file it has to
migrate is actually present.

**Guarded by** `seed_schedule_tests` in `crates/ambition_render/src/quality.rs`,
five arms on a real booted `App` rather than a direct call to the policy function:

- an existing un-seeded file receives its seed (RED before the fix)
- a tier the player CHOSE survives the seed (RED before — and only reachable at all
  once the load stops winning by accident)
- a fresh install with no file is still seeded (GREEN before; the arm exists so the
  move could not lose it)
- the first frame RESOLVES the seeded tier (RED before)
- the seed REACHES DISK, read back out of the file (RED before — without the write
  the flag is decorative and the player is re-examined every boot)

⚠ The review asked for exactly this shape and it was right to: the existing tests
call `seed_from_hardware()` directly, and no test of a pure function can see an
ordering defect between two plugins.


---

## ▣ 2, 3, 6 and half of 4 and 5 — the anchors, and the sweep that can see them

### What was measured, before anything was changed

A census of the shipped sim registry: **262 component-state registrations, 17
anchors.** `require_rollback::<T>` compiles to
`register_required_components::<T, bevy_ggrs::Rollback>`, so an anchor is a
STATIC fact about a type; a codec is a fact about its bytes. The two are
independent, and 245 state registrations rely on a SIBLING component carrying the
anchor for the archetype they live on.

That is not itself wrong — `PortalBody` riding a `PlacedPortal` entity is correct
and cheap. It is only wrong when the archetype has no anchored sibling at all,
and nothing static can tell those apart. Only a live population can.

### Two different defects, not one

The review treated these as one finding. They are not, and the difference decides
what the fix is:

| Type | Codec | Anchor | In the schema |
|------|-------|--------|---------------|
| `PortalShot` | yes | **no** | yes — and INERT |
| `FallingHazard` | yes (+ entity mapping) | **no** | yes — and INERT |
| `Sentry` | **no** | **no** | **absent entirely** |
| `VortexWell` | **no** | **no** | **absent entirely** |
| `TemporaryZone` / `GravityZone` | **no** | **no** | **absent entirely** |

The first two are what the review described: a registration the engine does not
honour. The last three are quieter and worse — no codec, no anchor, no waiver, no
line in the schema. A turret's `fire_cooldown` decides which tick a bolt is
emitted on and a well's `remaining_s` decides how long every body in radius keeps
being pulled, and neither was saved by anything.

⚠ The review's "Sentry is still spawned as ordinary session-scoped state rather
than a stable rollback simulation entity" understates it in one direction and
overstates in another: there was no partial registration to complete, and the
sentry's `pos` is not separately at risk (it never moves after deploy). The
timers were the whole exposure, and they were total.

### The fix

`v136 → v137`. Two anchors on already-registered types, and four new
registrations with their anchors:

```text
entity:portal_shot          entity:falling_hazard
entity:sentry               ability.sentry
entity:vortex_well          ability.vortex_well
entity:temporary_zone       gravity.temporary_zone
                            gravity.zone
```

⭐ **The anchor for a gravity well is `TemporaryZone`, not `GravityZone`**, and
the distinction is the whole reason the pair is registered separately. An
authored gravity column is room geometry a room load rebuilds; enlisting every
one of them in the rollback sweep would pay snapshot cost for state that never
changes. Only the zone with a LIFETIME is dynamic, so only it carries the anchor
— and `GravityZone` still needs a codec, because a restored temporary entity that
came back without its aabb would pull nothing.

### The instrument — `every_event_created_entity_is_registered_derived_or_waived_and_anchored`

The review asked for "a scenario-driven coverage test [that] should actually fire
abilities and encounter effects, then inspect the resulting dynamic simulation
entities," and it was right that nothing existing could. The repo already OWNED
the assertion — `assert_no_inert_registrations` says in as many words that a
registration on an unanchored archetype is inert — and `unaccounted_components`
already warned in its own doc comment that event-created state is structurally
invisible to it. What was missing was a POPULATION.

The new test builds one, through the production seam in every case: `deploy_sentry`,
`open_vortex_well`, `open_temporary_gravity_well`, `drop_hazard`, and a real
`PortalFireIntent` driven through `portal_fire_system`. Then both existing sweeps
run over the result.

**Poison-verified in two passes**, because the two defects fail through different
assertions:

- Remove the four new registrations → `assert_components_accounted` names
  `Sentry`, `VortexWell`, `GravityZone`, `TemporaryZone`.
- Restore those, remove only the two anchors → `assert_no_inert_registrations`
  names `FallingHazard + CenteredAabb` and `RoomScopedEntity + PortalShot + Name`
  — the second one by the entity's `Name`, "Portal shot".

⭐ **The seams are half the fix, and were the harder half.** Three of these five
had no callable spawn function at all: they were spawned inline inside a system
that first needs a held gauntlet, spent mana, an aim vector, or a burnt fuse. An
archetype with no seam is an archetype no sweep can reach, so its state stays
registered on trust forever. `deploy_sentry` already existed and already said why
("ONE PLACE, so a test cannot assemble a turret production never builds") —
`open_vortex_well`, `open_temporary_gravity_well` and `drop_hazard` now say the
same thing.

### ⚠ Where I disagree with the review: a `spawn_sim_entity` seam is not the guard

The review's headline recommendation is one API that guarantees rollback
participation. A seam like that can only guarantee it by inserting a marker
component that is itself `require_rollback`'d — and **nothing forces a spawn site
to use the seam.** The next `commands.spawn_session_scoped((AuthoritativeThing
{..},))` compiles exactly as easily as it does today, and the seam's guarantee
would be silently absent, which is the same failure in a new costume.

The thing that actually closes the class is the CHECK, and the check needs
reachable archetypes. So: named seams where they buy testability (landed, four of
them), and a sweep that drives them (landed). If a universal `spawn_sim_entity`
is wanted later it should come with an absence contract that forbids the raw
spawn for rollback-registered bundles — the seam without the contract is
convenience, not a guard.

### Also fixed on the way past

`cargo tree --locked` in `fixtures/minimal_game` and `examples/capability_demo`
had been failing since `915068407` added `wgpu` to `ambition_render` without
refreshing the sub-workspace lockfiles — which crashed
`check_absence_contracts.py` before it reached the rollback-wire-format ratchet
this change had to satisfy. Both locks refreshed; **36 of 36 contracts hold.**


---

## ▣ 7 — A channel that says "any emitter" kept one emitter

`PortalFireIntent`'s own doc says a host may lower an intent from a "gun, replay,
script, AI, or any future emitter". `portal_fire_system` read
`fires.read().last()`, so every other intent in the tick went on the floor: two
players firing on the same frame made ONE shot, a script firing beside an actor
made one, a four-seat couch match made one.

⭐ **The singleton reading was a leftover.** It was correct when the only emitter
was the primary player's held gun, and it outlived the generalisation that
removed that assumption — which is what makes it worth naming rather than just
fixing. `MessageReader::last()` is an implicit global winner wearing a reader
method.

The `return` on a zero aim was the same bug in miniature: one degenerate aim
cancelled the tick for everybody. It is a `continue` now.

⚠ **Order is the write order, and that is deliberate.** The intent buffer is
cleared on `LoadWorld::Mapping`, and a resimulated tick re-writes the same intents
from the same inputs in the same order, so two shots on one channel resolve
identically on every peer. If a same-channel same-tick winner is ever wanted, it
belongs in this system as a stated policy over emitter identity.

**Four arms, all four poison** against a restored `.last()`:

- two emitters in one tick each get their shot
- each shot keeps the channel of the intent that made it (dropping all but the
  last also silently re-coloured the survivor)
- a zero aim cancels only its own shot — ⭐ with the degenerate intent written
  LAST on purpose, because with it first a `.last()` implementation still finds
  the good one and the arm passes for the wrong reason
- every shot emits its own `PortalShotFired`

portal 59/59, content 296/296.


---

## ▣ 8 and 9 — query order was deciding who gets the ring

Bevy query order is archetype order, which depends on the order entities entered
their archetypes — a thing that differs between a live tick and the resimulated
one that replaces it, and between two peers that reached the same state by
different routes. A system that resolves "who gets this" with `.iter().find(..)`
has written down no rule at all.

`collect_ecs_pickups` did, and its own comment said so in as many words: *"find
the first overlapping collector"*. That reads like a rule and is not one. With
two players standing on one pickup it decided who healed, who banked the
currency, and who took the flag — unrepeatably.

`collect_world_items` had it **twice**. Because the system spends a body on its
first match — one item per body per frame, so a second `WornEquipment::new`
cannot overwrite the first — the order ITEMS are visited decides which of two
overlapping items a body receives, on top of which body receives one.

### The primitive

`ambition_platformer2d_shared_tangle::sim_selection`, next to the `SimId` it
depends on:

- `winner_by(candidates, metric, identity)` — smallest metric, ties by stable id
- `in_deterministic_order(..)` — the same rule as a sort, for the outer loop
- `every_candidate_is_identified(..)` — see below

⚠ **An unidentified candidate cannot win a tie, and the module refuses to pretend
otherwise.** A candidate with no `SimId` has nothing to break a tie WITH, so it
loses to any identified candidate at an equal metric and ties with its own kind
by encounter order — which is the non-determinism this module exists to remove.
`every_candidate_is_identified` exists so a caller can assert the population
rather than discover it in a desync: sorting an unidentified population produces
a stable-LOOKING result that is still encounter-ordered underneath, and a guard
that cannot tell those apart is a guard that cannot fail.

### Adopters

| Site | Metric | Was |
|------|--------|-----|
| `collect_ecs_pickups` | nearest centre | `.iter().find(..)` |
| `collect_world_items` — which body | nearest centre | `.iter().find(..)` |
| `collect_world_items` — which item | identity alone | unsorted `for` over the query |
| `update_sentries` | nearest enemy | `min_by` on distance, no tie-break |

⭐ **Nearest-centre is a rule a player can see**, which is why it is the metric
rather than "whoever the engine yields first, but sorted". The item order has no
meaningful metric — there is no distance between two items competing for one body
— so it is the tie-break alone, and the code says that.

⚠ `update_sentries` used `min_by`, which keeps the FIRST minimum. Two badniks
abreast of a turret is an ordinary arrangement, not a corner case, and which one
eats the bolt changes who dies and when.

### Guarded

Seven unit tests on the primitive, the load-bearing one being *the winner does
not depend on the order the candidates arrived in* — asserted forward, reversed
and shuffled.

Two wiring tests per adopter shape, **both poison-verified** against the restored
`.find(..)` / unsorted loop:

- `who_gets_it::the_same_body_collects_whichever_order_the_two_were_spawned_in`
  — two equidistant players on one ring, spawned both ways round.
- `who_gets_it::the_nearer_body_wins_even_with_the_higher_identity` — ⭐ the arm
  that stops the tie-break quietly becoming the whole rule.
- `which_item::the_same_item_is_collected_whichever_order_the_two_were_spawned_in`
  — two items on one body, spawned both ways round. Red under the poison with
  `left: "alpha", right: "omega"`.

### Still to convert

The review lists possession candidates, projectile victim ties and magnet
ownership as further adopters. They are not converted here — each needs its own
population arm, and a conversion with no test that can fail is worth nothing.
`sim_selection` is the place they go.

monolith 1172/1172, tangle 242/242, app_it 509/509.


---

## ▣ 10 — A velocity does not know who moved it

`Release { Throw, Drop }` was already the right abstraction, and the review was
right that it did not own the decision it was introduced to describe. The enum
was computed, used to pick a launch vector, and **thrown away**; the fuses then
guessed it back from the consequence.

Both directions failed:

- A bomb the room authored begins at rest and then FALLS. `ground_item_physics`
  gives it a velocity, `vel != ZERO` reads that as a throw, and it arms itself
  on the way down. Ordinary gravity was evidence of a player's intent.
- Catching a live bomb zeroed the velocity and left the lit `BombFuse`. The
  ticker did not care whose hand it was in, so it counted down and detonated in
  custody.

### The fix

`ReleasedAs(Release)` — the release decision made durable, stamped by the one
release transaction and retracted the moment a body takes custody. The heuristic
is **deleted**, not patched.

Three release paths, three different answers, all of them now stated:

| Path | Stamp |
|------|-------|
| `throw_held_item_system`, `Release::Throw` | `ReleasedAs(Throw)` — arms |
| `throw_held_item_system`, `Release::Drop` | `ReleasedAs(Drop)` — does not arm |
| `return_released_items` (menu stow / brandish swap) | nothing — not a release at all |

⚠ **A Z-drop does not arm.** That was the old answer too, but only because a drop
happens to launch at zero velocity, which is not a reason. It is now the stated
rule: handing the item to the floor is not an attack.

⭐ **Disarming is the same system as arming, deliberately.** `arm_thrown_bombs`
is chained ahead of `tick_bomb_fuses`, so a bomb caught this tick has its fuse
removed before anything can burn it down — catching a live bomb is a defined
outcome, not a race between two systems. Re-throwing re-arms with a *full* fuse,
which is what the old `Without<BombFuse>` arming already implied.

### It is rollback state, and the last commit is why that got noticed

`ReleasedAs` decides whether an object in the world is going to explode, and it
lives on a `GroundItem` — which is already an anchor. `v137 → v138`,
`item.released_as`. It replaced a heuristic that was rollback state **by
accident** (`GroundItem` carries `vel`), so this is the same coverage moved onto
the fact that actually decides.

### Guarded

Six bomb arms and two grenade arms. The two that shipped before were rewritten
rather than kept: `a_thrown_bomb_arms_but_a_resting_one_does_not` spawned a bomb
with a nonzero velocity and CALLED it thrown — the velocity was the heuristic
under test, so the test agreed with the bug by construction.

Poison-verified against the restored `vel != ZERO`: three of six bomb arms go
red — the falling bomb, the caught bomb, and the re-throw.

monolith 1177/1177, app_it 509/509, 36 of 36 contracts.


---

## ▣ 11 — Two stages, two ideas of what `Submerged` means

The continuous sweep took `BodyModeState` and knew a submerged body is not in the
world. `resolve_axis_repair` — the overlap/penetration stage that runs at the END
of the same function call — took neither it nor anything equivalent. A body that
legitimately travelled INTO a block was found overlapping it and pushed straight
back out, by the second half of the call that had just let it in.

`BodyCollisionPolicy` is that question asked once. The sweep's filter and the
repair's claim loop both call `policy.passes_through(block)`.

⚠ **CLIMBING HAD THE SAME GAP**, and it is why this is a shared value rather than
the `if Submerged` in the repair the review warned against. A climbing body
passes through exactly the blocks its climbable region overlaps, hazards
excluded; the repair could not tell which those were either. One policy, asked
the same question by both stages, is what makes "passable" mean one thing.

### ⛔ The measurement that corrected my own first test

The first version of this arm placed the body 24px into a 48px platform and
**passed under the bug**. Measuring the unfixed repair by depth:

| overlap | unfixed repair | fixed |
|---------|----------------|-------|
| 2px  | y 854 → **828** (pushed out) | 854 (stays) |
| 6px  | 858 (unchanged) | 858 |
| 12px | 864 (unchanged) | 864 |
| 24px | 876 (unchanged) | 876 |
| 40px | 892 (unchanged) | 892 |

The no-pushout-teleport rule (`is_contact_range_snap`) already refuses any claim
deeper than the body's own half-extent, so a deeply embedded body was being
deferred anyway. **The defect lives only in contact range** — which is exactly
where a body travelling just under a surface is, on every tick of the move. An
arm placed 24px deep proves nothing at all, and mine did not until it was moved
to 2px.

⭐ The control arm matters as much: the same body at the same place, NOT
submerged, is still repaired out (854 → 828). Without it, "submerged bodies are
left alone" reads identically to "penetration repair is broken".

Both arms poison-verified against the restored policy-blind repair: the submerged
one goes red, the control stays green.

core 533/533, app_it 509/509.
