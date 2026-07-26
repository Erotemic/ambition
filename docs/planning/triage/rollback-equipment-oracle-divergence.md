# The combat-equipment rollback divergence

**Status:** OPEN, narrowed. The oracle is `#[ignore]`d with this document as its
reason. Opened 2026-07-26 while establishing a green baseline for the character
definition work; **not caused by that work.**

## The failure

```
rollback_exit_oracle::combat_equipment_switch_and_breakable_survive_forced_rollback_identically
frame 153: GGRS sync-test checksum mismatch at frames [149, 150, 151]
  (events at failure: melee=true armor=true brick=false switch=false)
```

First flagged in `560c923cd`: *"It went red the moment the player's sheet changed
and stayed red across every combination since, so the new body reaches a
resimulation path the old one never did."*

## Two real bugs it exposed, both fixed

Neither closed the divergence. Both are genuine unrewound state, and both were
invisible before the instrument was widened.

1. **`IdentityKit` was not rollback-registered.** It is the un-granted baseline
   the live `ActionSet` / `ActorMoveset` are a pure function of
   (`identity + worn equipment`). Both derived halves were registered; the base
   was not. A rewind therefore restored the live kit but left the baseline at
   whatever an abandoned future derived, so the next `reconcile_equipment_grants`
   — fired by any armor spend or pickup — recomputed from the wrong base. Exactly
   the `WornEquipment` oversight of deep-review 2026-07-19 §2.2, one layer down.

2. **`PlayerVisual` was not rollback-registered.** bevy_ggrs destroys and
   recreates rollback entities, so the tag was simply absent afterwards, and
   `ambition_host::portal` asks `With<PlayerVisual>, Without<PortalSceneBody>` to
   decide what to stage as a portal body. Same reasoning as its already-registered
   sibling `RoomVisual`.

## Why the instrument missed them, and what it does now

`rollback_coverage::unaccounted_components` reported a confident **empty** result
the whole time, for two independent reasons:

- **It never inspected the player.** Its population was `With<FeatureSimEntity>`,
  and `PlayerBundle` does not insert that tag. The single most heavily-mutated
  body in the game was outside the sweep. It now sweeps the union with
  `With<BodyKinematics>` — anything the sim integrates every tick.
- **It never inspected transients.** It sampled one instant, so an attack's hit
  volume, a projectile, or a debris chunk — alive for a handful of frames — could
  never appear. `walk_the_combat_route` now unions the census **every frame** and
  the oracle asserts it empty.

Both holes are closed, and with them closed the census is empty across the whole
route. That is the useful negative result below.

## What is ruled OUT

Do not re-derive these; each was measured on 2026-07-26 at `560c923cd` + the two
registrations above.

- **Missing component registration — ruled out.** Per-frame census over the whole
  route, including transients and the player, is empty.
- **Missing resource registration — ruled out.** The sibling resource sweep is
  green.
- **The props — ruled out.** `which_population_does_the_rollback_divergence_need`
  (the `#[ignore]`d localizer beside the oracle) removes one entity class at a
  time. `no_brick`, `no_switch`, and `no_pickups` all still diverge. `no_brick`
  diverges later (frame ~218 instead of ~153) purely because the route takes
  longer to walk. The divergence needs only **player + enemies + melee + armor**.
- **`sync_sprite_posed_bodies` running outside the rollback schedule — ruled
  out.** It is added to the sim schedule (`features/mod.rs:411`), so it does run
  during resimulation. This was the leading hypothesis, because the failure
  arrived with the player's new sheet and the system moves the body feet-anchored
  on every pose change; it is wrong, but the *pose→geometry* path remains the
  most suspicious surviving area precisely because of that timing.

## Where to look next

It is a **value** divergence in registered state, not a coverage gap. In both
timings the player is essentially **stationary and swinging on a 6-frame cadence**
(`px ≈ target_x`), so the route is repeatedly changing pose and re-deriving
geometry. Candidates, in order:

1. A registered component whose value depends on something the rollback does not
   restore — an asset, an `Update`-schedule resource, or iteration order. Note
   `feedback_query_order_determinism`: entity recreation changes archetype order,
   so any "first match wins" tie-break or order-dependent float accumulation
   differs after a rewind.
2. The feet-anchored resize in `sync_sprite_posed_bodies`. The write is guarded by
   `if kin.size != geometry.collision`, so the *position* it produces depends on
   the HISTORY of size changes. That is history-dependent state derived from a
   pose, which is a fragile thing to have inside a rewind window even when every
   input is registered.
3. `SimId` allocation for attack-spawned transients across a rewind.

**The missing tool** is per-component checksum localization. A GGRS sync test
reports one aggregate, so "frames [149, 150, 151] differ" is all it can say. The
registry already installs per-component checksum projections
(`RollbackApp::checksum_component`), so dumping those per frame and diffing across
the save/load boundary would name the component directly instead of by bisection.
That is the next thing to build, and it would have answered this in minutes.

## Related

- `docs/planning/engine/character-definition-design.md` §4.11 — the same disease
  one layer over: simulation geometry must never be derived from presentation.
  The posed-body path is the *legitimate* version of that (it reads the content
  pose pin, not the renderer), which is why it is suspicious rather than wrong.
- ADR 0023 (determinism contract), ADR 0027 (GGRS).
