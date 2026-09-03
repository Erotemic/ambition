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

  ⛔ **but NO AUTHORED LEVEL PUTS THE TWO TOGETHER. Measured 2026-08-20,
  re-measured 2026-09-02 and unchanged** across every `.ldtk` in
  `game/ambition_map_assets`, counting placed entity instances rather than the
  definition each file carries: `SurfaceChain` appears in exactly two levels,
  `sanic_sandbox` (2) and `sanic_speedway` (2), and neither places a crawler —
  the three crawler characters (`npc_puppy_slug` and its two variants) live in
  `hall_of_characters`, `intro` and `sandbox`, none of which authors a chain in
  the same level.

  ⇒ leave it ▢ for the same reason as the portal/gravity-zone row below: it is
  waiting on a customer, not on effort. ⚠ **and do not build it speculatively** —
  a transfer rule nothing exercises is a rule nothing can falsify, and the
  crawler is the subsystem this repository has already had to correct three times
  from play.

- ▢ **Exercise portal transit inside authored gravity zones.** The code resolves
  projectile gravity per body and portal transit itself is pure portal geometry,
  but current portal rooms do not nest gravity zones. Add a behavioral exercise
  if/when a room authors that combination; there is no known porting bug to fix.
