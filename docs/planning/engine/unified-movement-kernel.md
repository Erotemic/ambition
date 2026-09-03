# Unified movement kernel — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The frame-aware movement kernel,
> typed resolution seams, rollback registration, surface-momentum operations,
> portal transit geometry, and the previously listed residual items 1–4 are
> implemented or refuted. The full architecture/migration record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/unified-movement-kernel.md`](../../archive/planning-superseded/2026-08-13/engine/unified-movement-kernel.md).

## Remaining

- ▢ **Block ↔ chain crawl transfer — REAL, and CUSTOMER-GATED.** A crawler
  attached to a block surface does not transfer directly onto an overlapping
  chain surface (or vice versa) without detaching first. `step_adhesive_crawler`
  dispatches `CrawlAttachment::Chain` into `crawl_chain` and returns, while
  `Block` falls through to the riding path — two roads, and only the block road
  has a face-transition rule (`wall_ahead` → re-attach). Defining the shared
  attachment-transfer rule is the fix, and it must not introduce a second crawler
  controller.

  ⛔ **but NO AUTHORED LEVEL PUTS THE TWO TOGETHER. Measured 2026-08-20** across
  every `.ldtk` in `game/ambition_map_assets`: `SurfaceChain` appears in exactly
  two levels, `sanic_sandbox` and `sanic_speedway`, and neither places a crawler
  — the three crawler characters (`npc_puppy_slug` and its two variants) live in
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
  `PortalGunSpawn`** (at `[448,880]`, `[32,448]`, `[448,32]`, `[880,448]`, and
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
