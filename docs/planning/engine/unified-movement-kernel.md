# Unified movement kernel — remaining work

The frame-aware movement kernel, typed resolution seams, rollback
registration, surface-momentum operations and portal transit geometry are
implemented. Git history has the architecture and migration record. Two items
remain, both waiting on a customer, not on effort.

## Remaining

### Block-to-chain crawl transfer (customer-gated)

A crawler attached to a block surface does not transfer directly onto an
overlapping chain surface (or the reverse) without detaching first.
`step_adhesive_crawler` (`movement/kernel.rs`) dispatches
`CrawlAttachment::Chain` into `crawl_chain` and returns, while `Block` falls
through to the riding path. Only the block road has a face-transition rule
(`wall_ahead`, then re-attach). The fix is one shared attachment-transfer rule,
without a second crawler controller.

No authored level places both a `SurfaceChain` and a crawler
(`npc_puppy_slug` and variants). `SurfaceChain` is placed only in the Sanic <!-- cite-ok: a character id in the catalog data, not a Rust item -->
levels. Do not build the rule speculatively: a rule nothing exercises cannot be
falsified.

How to re-check: count placements per **level**, not per file. Every world file
carries the `SurfaceChain` entity definition, so a file-level grep matches all of
them. `sandbox.ldtk` holds many levels. A crawler is placed through a generic
spawn entity with `npc_puppy_slug` in a field value, so read field values, not <!-- cite-ok: a character id in the catalog data, not a Rust item -->
only `__identifier`.

### Portal transit inside gravity zones (behavior test owed)

Transit resolves "down" per body (`ambition_portal2d/src/transit.rs` calls
`GravityCtx::dir_for` with the transiting body's box). It does not read `GravityField`,
which mirrors the primary body's frame. `ResolvedMotionFrame` is the per-body
answer for every other reader. `GravityField` is a presentation mirror that no
simulation reader takes.

`gravity_dir` reaches two decisions in `portal2d/src/placement.rs`
(`somersault_roll_for_convention`, `portal_facing_flips_for_convention`). Both
apply only under `MapConvention::Reflection`. The shipped default is
`PortalConvention::Rotation`; Reflection is selectable only from the dev portal
inspector. So the per-body repair is behavior-neutral in shipped play and is not
yet verified as a behavior fix.

Customer: `sandbox.ldtk:symmetry_room` places four `GravityZone`s and a portal
gun spawn, so a player can put a portal inside a zone. No room authors a portal
pair inside a zone.

Owed: a test where a non-primary body and the player are in different zones,
the non-primary body transits, and its orientation follows its own frame. Two
obstacles:

- The orient-to-gravity system also writes `ActorRoll` from the same per-body
  fact, so a body in a zone moves both. The test needs an observable only
  `wall_to_wall` can move, or must subtract the roll contribution.
- A pair of exactly opposed normals is degenerate: `portal_transit_roll`
  returns 0, so both branches agree. Use a pair with a non-zero transit roll,
  under Reflection.

A direct unit test of the wiring may be cheaper than an end-to-end assertion.

## Re-measuring these gates

The `.ldtk` worlds are versioned in the `game/ambition_map_assets` submodule, so
a count over them is a repository fact another machine can check. See
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).
