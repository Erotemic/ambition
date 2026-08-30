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
| 8 | `collect_ecs_pickups` / `collect_world_items` decide by query order | P1 | ☐ |
| 9 | Deterministic selection as a shared primitive (metric, then `SimId`) | architecture | ☐ |
| 10 | Fuse arming still reads `vel != ZERO` instead of `Release::Throw` | P1 | ☐ |
| 11 | Submerged: sweep knows the passability policy, penetration repair does not | P1 | ☐ |
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
