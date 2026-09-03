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

  ⇒ leave it ▢ for the same reason as the portal/gravity-zone row below: it is
  waiting on a customer, not on effort. ⚠ **and do not build it speculatively** —
  a transfer rule nothing exercises is a rule nothing can falsify, and the
  crawler is the subsystem this repository has already had to correct three times
  from play.

- ▢ **Exercise portal transit inside authored gravity zones.** The code resolves
  projectile gravity per body and portal transit itself is pure portal geometry,
  but current portal rooms do not nest gravity zones. Add a behavioral exercise
  if/when a room authors that combination; there is no known porting bug to fix.

## Re-measured 2026-09-03 — both gates still closed, and both counts reproduce

Swept every `.ldtk` in the `game/ambition_map_assets` submodule (6 files, at the
committed pointer `71f17383`, clean) at LEVEL granularity — the granularity the
2026-08-20 row was careful to use.

* **Chain ↔ crawl transfer: still no customer.** `SurfaceChain` is PLACED in
  exactly two levels, `sanic_sandbox` and `sanic_speedway` — the same two the
  row named a fortnight ago. Crawlers are placed in nine levels across
  `hall_of_characters`, `intro` and `sandbox`. **Levels containing both: none.**
* **Portal transit inside a gravity zone: still no customer.** `portal_lab`
  authors 14 `Portal`s and zero `GravityZone`s; `gravity_lab`, `symmetry_room`,
  `wall_run` and `ceiling_cross` author zones and no portals. **No level authors
  both.**

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

