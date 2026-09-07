# Unified movement kernel — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The frame-aware movement kernel,
> typed resolution seams, rollback registration, surface-momentum operations,
> portal transit geometry, and the previously listed residual items 1–4 are
> implemented or refuted. The full architecture/migration record is archived at
> `../../archive/planning-superseded/2026-08-13/engine/unified-movement-kernel.md` (docs/archive/planning-superseded/2026-08-13/engine/unified-movement-kernel.md — removed from the checkout 2026-09-05; still in git history).

## Remaining

- ▢ **Block ↔ chain crawl transfer — REAL, and CUSTOMER-GATED.** A crawler
  attached to a block surface does not transfer directly onto an overlapping
  chain surface (or vice versa) without detaching first. `step_adhesive_crawler`
  dispatches `CrawlAttachment::Chain` into `crawl_chain` and returns, while
  `Block` falls through to the riding path — two roads, and only the block road
  has a face-transition rule (`wall_ahead` → re-attach). Defining the shared
  attachment-transfer rule is the fix, and it must not introduce a second crawler
  controller.

  ⛔ **but NO AUTHORED LEVEL PUTS THE TWO TOGETHER. Measured 2026-08-20,
  re-measured 2026-09-02 and unchanged** across every `.ldtk` in
  `game/ambition_map_assets`, counting placed entity instances rather than the
  definition each file carries: `SurfaceChain` appears in exactly two levels,
  `sanic_sandbox` (2) and `sanic_speedway` (2), and neither places a crawler —
  the three crawler characters (`npc_puppy_slug` and its two variants) live in
  `hall_of_characters`, `intro` and `sandbox`, none of which authors a chain in
  the same level.

  ✔ **RE-MEASURED 2026-09-02 and the claim HOLDS, by entity INSTANCE rather than
  by text.** `SurfaceChain` is *placed* in exactly two LEVELS — `sanic_sandbox`
  and `sanic_speedway` — and neither places a crawler. Crawler placements are
  `hall_of_characters` (6), `intro`'s three levels (12) and five levels of
  `sandbox.ldtk` (17); none of them is `sanic_sandbox`.

  ⚠ **AND A FILE-LEVEL GREP SAYS THE OPPOSITE, which is worth recording because
  it is the obvious way to re-check this row.** `grep -c SurfaceChain` reports
  hits in `intro`, `hall_of_characters`, `you_have_to_cut_the_rope` and `mary_o`
  as well — those are entity DEFINITIONS in `defs`, not placements, and
  `you_have_to_cut_the_rope`/`mary_o` define the entity and never place it. And
  `sandbox.ldtk` is one FILE holding many LEVELS: its crawlers are in
  `central_hub_basement`, `vertical_shaft`, `basement_enemies`, `gravity_lab`
  and `proving_grounds`, while its chain is in `sanic_sandbox`. ⇒ Same file,
  different levels — the row says "in the same level" and it means it.

  ⇒ leave it ▢ for the same reason as the portal/gravity-zone row below: it is
  waiting on a customer, not on effort. ⚠ **and do not build it speculatively** —
  a transfer rule nothing exercises is a rule nothing can falsify, and the
  crawler is the subsystem this repository has already had to correct three times
  from play.

- ▢ **Exercise portal transit inside authored gravity zones — ⭐ THE CUSTOMER
  THIS WAS WAITING FOR HAS ARRIVED (re-measured 2026-09-02).** The code resolves
  projectile gravity per body and portal transit itself is pure portal geometry;
  the row was parked because "current portal rooms do not nest gravity zones".

  ⇒ **`sandbox.ldtk:symmetry_room` now places FOUR `GravityZone` entities and a
  `PortalGunSpawn`** (at `[448,880]`, `[32,448]`, `[448,32]`, `[880,448]`, and <!-- cite-ok: an LDtk entity identifier; the Rust type is PortalGunSpawnSpec -->
  the gun at `[648,1208]`). ⚠ Stated precisely, because the distinction decides
  what the exercise is: **no room authors a portal PAIR nested in a gravity
  zone.** What `symmetry_room` does is hand the player the portal gun in a room
  with four zones — so the player can put a portal inside one, which is a wider
  exercise than an authored pair and is reachable in play today.

  ⛔ Every other room is one or the other: `portal_lab` has 14 portal entities
  and no zones; `gravity_lab`, `wall_run` and `ceiling_cross` have zones and no
  portals. So `symmetry_room` is the only place the two can meet, and it meets
  them through the gun rather than through authoring.

  ⇒ The row is no longer customer-gated. There is still no known porting bug —
  the exercise is to find out whether one exists.

  ⭐⭐ **SCOPED 2026-09-06 — REASONED FROM SOURCE, NOT MEASURED. There is a specific
  thing to assert, and a specific population it would bite.** `portal2d/transit.rs:238`
  takes `Option<Res<GravityField>>` and derives `gravity_dir` from it (:260), which
  `portal2d/placement.rs:110` uses for the wall↔wall upright accommodation. And
  `GravityField` is not a per-body fact:

  ```rust
  // shared_tangle/src/gravity.rs:432
  pub fn resolve_active_gravity(
      base: Option<Res<BaseGravity>>,
      bodies: Query<&ResolvedMotionFrame, With<PrimaryBody>>,   // ⭐ PRIMARY ONLY
      mut gravity: ResMut<GravityField>,
  ) { gravity.dir = bodies.single().map_or(base_dir, |frame| frame.down())… }
  ```

  ⇒ **`GravityField` is a MIRROR OF THE PRIMARY BODY'S resolved frame**, and this is
  not my inference — a sibling consumer already identified the trap and avoided it.
  `sim_view/src/pose_view.rs:206`, verbatim:

  > This body's own resolved basis, so the locomotion metric is measured along ITS run
  > axis. deliberately not the global `GravityField` read below: that one drives the
  > facing flip and **is a mirror of the PRIMARY body's frame**.

  ⇒ So one reader of this fact took the per-body frame ON PURPOSE and wrote down why;
  `transit.rs` takes the global one and mentions `ResolvedMotionFrame` zero times. Two
  consumers, one hazard, and only one of them heeded it. That asymmetry is the finding
  — much more than a suspicion about a resource.
  `transit.rs` mentions `ResolvedMotionFrame` ZERO times. ⇒ **The predicted defect is
  that a NON-PRIMARY body transiting a portal is oriented by the PRIMARY body's
  gravity**, which is wrong whenever the two are in different zones — and
  `symmetry_room` has FOUR zones, so it is the room that can produce that state.

  ⚠ **AND I NEARLY FILED A WRONG MECHANISM HERE.** My first reading said transit uses
  the ambient field and ignores zones entirely. That is FALSE: the doc I took it from
  is on `BaseGravity`, not `GravityField`, and it says `resolve_active_gravity` copies
  "an overlapping zone's direction" INTO the live field. Zones ARE honoured — for one
  body. The defect is the single global mirror, not zone-blindness.

  ✔ **AND THE POPULATION IS WIDE, checked rather than assumed.** The transit system
  filters `With<PortalBody>`, not `With<PrimaryBody>` — and
  `ambition_content/src/portal/transit_body_adapter.rs`'s `ensure_portal_bodies` grants
  `PortalBody` to EVERY body with `BodyKinematics` (`Without<ProjectileGameplay>`), not
  only the player. ⇒ So "non-primary body transiting a portal" is not a contrived
  state: it is every enemy, NPC and actor in any room that has portals. The prediction
  is reachable in production rather than theoretical, which is what decides whether the
  exercise is worth writing.

  ⛔⛔⛔ **CORRECTION 2026-09-06, AND IT DE-PRIORITISES EVERYTHING BELOW: THE
  `gravity_dir` PATH DOES NOT RUN IN SHIPPED PLAY.** `gravity_dir` reaches exactly two
  decisions — `somersault_roll_for_convention` and `portal_facing_flips_for_convention`
  (`placement.rs:112,131`) — and BOTH open with
  `convention == MapConvention::Reflection`. `PortalTuning::default()` ships
  `PortalConvention::Rotation` (`tuning.rs:91`), and Reflection is selectable ONLY from
  the dev portal inspector (`dev/portal_inspector.rs:772`).

  ⇒ **So the wiring asymmetry is real and the player cannot reach it.** Everything below
  — the contract, the symptom, the population — is CORRECT ABOUT THE CODE and describes
  a branch the shipped convention never enters. ⚠ I escalated four claims about this
  before checking whether the path executes, which is the denominator question and
  should have come first.

  ✔ **The repair landed anyway** (transit now resolves per body via `gravity_dir_for`),
  because it removes a genuine wrong authority and is behaviour-neutral: portal2d 71/71
  and `app_it` 578/578 green before and after. ⚠ It is UNVERIFIED as a behaviour fix,
  and deliberately so — see the deleted test below.

  ⭐⭐⭐⭐ **AND THE PROJECT ALREADY WROTE THE CONTRACT THIS BREAKS.**
  `shared_tangle/src/gravity.rs:150`, the `GravityZone` doc, verbatim:

  > …center is inside the zone's `aabb` feels gravity along `dir` (and reorients via
  > the shared `ActorRoll`); outside every zone it falls under [`BaseGravity`]. **So an
  > NPC standing in a gravity column feels the column even when the player is
  > elsewhere.**

  ⇒ That last sentence is the invariant, stated as the reason the zones resource exists
  at all. Portal transit derives its "down" from the player-mirrored global field, so a
  transiting NPC does NOT feel its own column when the player is elsewhere — which is
  the promise, negated. ⭐ This is no longer an inconsistency I inferred between two
  call sites: it is a written contract with one consumer that honours it and one that
  does not.

  ⭐⭐⭐ **AND THE CORRECT API ALREADY EXISTS AND A SIBLING USES IT — this is the
  strongest form of the finding.** `shared_tangle::gravity::gravity_dir_for(aabb, zones,
  base_dir)` resolves gravity for a body AT ITS OWN POSITION. `character_sprites`'
  `sync_sprite_posed_bodies` takes BOTH `Res<GravityField>` and `Res<GravityZones>` and
  calls it with the body's own box (`posed_body.rs:154`):

  ```rust
  let gravity_dir = match (gravity.as_deref(), zones.as_deref()) {
      (Some(field), Some(zones)) =>
          gravity_dir_for(ae::Aabb::new(kin.pos, kin.size * 0.5), zones, field.dir),
      (Some(field), None) => field.dir,
      _ => ae::DEFAULT_GRAVITY_DIR,
  };
  ```

  `portal2d/transit.rs` calls `gravity_dir_or_default(field)` and mentions
  `GravityZones` **ZERO** times. ⇒ Two consumers of "which way is down for THIS body",
  one per-body resolver, and only one caller uses it. ⭐ **The repair is therefore named
  rather than designed:** transit takes `zones` and calls `gravity_dir_for` with the
  transiting body's AABB, inside the per-body loop rather than once above it.

  ⚠ **TWO WRONG READINGS WERE DISCARDED GETTING HERE, both recorded so nobody repeats
  them:** (1) "transit ignores zones entirely" — false, `resolve_active_gravity` copies
  a zone's direction into the field; the defect is the single global mirror, not
  zone-blindness. (2) "`posed_body`'s comment claiming *the body's LOCAL gravity* is
  false" — also false; it takes `GravityZones` on the next line and genuinely resolves
  per-body. It is the CORRECT example, not a second defect.

  ⭐ **AND THE SYMPTOM IS SPECIFIC, which is what makes it findable in play.**
  `gravity_dir` feeds exactly one decision — `placement.rs:92`:

  ```rust
  fn wall_to_wall(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> bool {
      let g = gravity_dir.normalize_or_zero();
      let in_wall  = n_in.normalize_or_zero().dot(g).abs() < 0.5;
      let out_wall = n_out.normalize_or_zero().dot(g).abs() < 0.5;
      in_wall && out_wall
  }
  ```

  It classifies each aperture as WALL or FLOOR/CEILING by its angle to "down", and that
  classification gates the somersault/upright accommodation. ⇒ With the wrong down, a
  wall reads as a floor. **The predicted symptom is a non-primary actor somersaulting
  through a portal when it should stay upright (or the reverse), only while the PLAYER
  stands in a differently-oriented gravity zone.** A bug whose trigger is where a
  DIFFERENT body is standing is exactly the kind that survives play-testing.

  ⛔⛔ **I WROTE THE OBVIOUS INTEGRATION TEST AND IT CANNOT ISOLATE THE DEFECT. Deleted
  rather than landed, because it PASSED for a reason unrelated to transit.** The shape
  was: transit one body through `portal_lab`'s first authored pair twice, once with a
  `GravityZone` (horizontal down) covering only that body, and require the outcomes to
  differ. It passed. Measured, `(vel, roll)`:

      without zone   (Vec2(0.0, -400.0), 6.179)
      with zone      (Vec2(0.0, -400.0), 0.000)

  ⇒ **Velocity is IDENTICAL, so the whole difference is `ActorRoll` — and `ActorRoll`
  is exactly what the ORIENT-TO-GRAVITY system writes**, per `GravityZone`'s own doc
  ("reorients via the shared `ActorRoll`"). That system is already per-body and
  zone-aware, so it explains the entire delta with portal transit contributing nothing.
  A green assertion here would have been read as "transit honours per-body gravity"
  when it shows only that *something* does.

  ⛔ **AND THE ISOLATED VERSION DID NOT CONVERGE EITHER.** A minimal `App` with ONLY
  `portal_transit` registered (no orient system, so any delta must be transit's) still
  read `(Vec2(400,0), 0.0)` with and without the zone — first because the default
  tuning is Rotation, and then, with Reflection forced, because a pair of exactly
  OPPOSED normals is degenerate: `wall_to_wall` flips as predicted but
  `portal_transit_roll` returns 0 for that geometry, so both branches agree. ⇒ A real
  test needs a pair whose `portal_transit_roll` is NON-ZERO, so the two branches differ.
  That is the missing ingredient, and it is geometry, not wiring.

  ⚠ **THE OBSTACLE, for whoever writes the real one:** transit's `gravity_dir` and the
  orient-to-gravity system consume the same fact and write the same observable
  (`ActorRoll`). Any test that puts a body in a zone moves both. ⇒ It needs an
  observable only `wall_to_wall` can move — a portal pair whose WALL-vs-FLOOR
  classification flips under the body's own down while the ambient stays put, with the
  roll contribution held constant or subtracted. Confirming the defect may be cheaper
  as a direct unit test of the wiring (does transit ever read `GravityZones`? it does
  not) than as an end-to-end assertion.

  ▢ The exercise still wants: a non-primary body and the player in DIFFERENT zones,
  transit the non-primary one, and assert its post-transit orientation follows ITS OWN
  frame — with the confound above defeated rather than ignored. ⛔ Nothing above is measured — it is a source reading, and
  `a_coherent_source_reading_is_not_a_measurement` applies. The test is still the
  deliverable.

## Re-measured 2026-09-03 — both gates still closed, and both counts reproduce

Swept every `.ldtk` in the `game/ambition_map_assets` submodule (6 files, at the
committed pointer `71f17383`, clean) at LEVEL granularity — the granularity the
2026-08-20 row was careful to use.

* **Chain ↔ crawl transfer: still no customer.** `SurfaceChain` is PLACED in
  exactly two levels, `sanic_sandbox` and `sanic_speedway` — the same two the
  row named a fortnight ago. Crawlers are placed in nine levels across
  `hall_of_characters`, `intro` and `sandbox`. **Levels containing both: none.**
* **Portal transit inside a gravity zone: no AUTHORED pair, but a customer.**
  `portal_lab` authors 14 `Portal`s and zero `GravityZone`s; `gravity_lab`,
  `symmetry_room`, `wall_run` and `ceiling_cross` author zones and no portal
  PAIRS. ⚠ Corrected the same day by the row above: `symmetry_room` places a
  `PortalGunSpawn` beside its four zones, so the player can put a portal inside <!-- cite-ok: an LDtk entity identifier; the Rust type is PortalGunSpawnSpec -->
  a zone in play — the customer is the gun, not an authored pair.

⇒ Both rows stay ▢ for the reason they already give: waiting on a customer, not
on effort. Nothing here argues for building either rule speculatively.

⚠ **A file-level grep TODAY would say the first gate has OPENED, and it has
not.** `grep -rl SurfaceChain` matches **6 of the 6** world files, including all
three worlds where the crawlers live. Five of those matches are the entity
DEFINITION carried in every world sharing the definition set, not a placement.
Placement is a property of a LEVEL and the definition is a property of the FILE;
only the finer instrument separates them.
⚠ And the first level-granularity pass reported **zero crawlers anywhere**, which
contradicted the row and was my instrument, not the data: it read entity
`__identifier` only, while a crawler is placed through a generic spawn entity
carrying `npc_puppy_slug` in a FIELD value.

⭐ **Worth noting WHY these numbers reproduce when others measured the same day
did not:** `.ldtk` worlds are tracked — in a submodule, so `git ls-files` from
the parent shows none of them, but they are versioned and pinned. A count over
tracked, pinned inputs is a repository fact another machine can check. See
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md)
for the counts that were not.

