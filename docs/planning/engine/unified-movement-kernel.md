# Unified movement kernel — remaining work

The frame-aware movement kernel, typed resolution seams, rollback
registration, surface-momentum operations and portal transit geometry
are implemented. Git history has the architecture and migration record.
Two older customer-gated items remain below. A new scripted-pose ownership
issue is recorded after them; it has concrete October 11 customers.

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

### Pose ownership under script, encounter and plant control (new customer)

**What landed (2026-10-12):** `PoseOwnedExternally` is a set
of holds (`PoseHold::{Ride, Conductor, Transit, Path}`), not a boolean.
Each producer claims and releases its own bit through `claim_pose_hold` /
`release_pose_hold`; the marker goes when the last hold goes, and readers
still test presence. The saddle, a boss conductor, the pipe ride and the
Snake Plant each hold their own bit. The record is in the rollback clone
(`mount.pose_owned_externally`). The Snake Plant releases `Path` when it is
dead, out of its pot or no longer a plant, is shy of each player of its own
live room (`LiveRoomOf::room_of`), and has its own out fraction
(`SnakePlantOut`, rollback state): it read its fraction back from its
position, which the movement step pushed out of the pipe solid, so it never
left its mouth. Witnesses: `mary_o_it a_hold_is_its_holders` (three tests,
five poisons). A hold across a rollback restore: a hold that no gameplay
made, put on her outside any input, is gone after the restore of a GGRS
sync-test session (`mary_o_it rollback_restore`; with the registration
removed it stays). Not witnessed: arbitration between two holders that want different paths (no customer
has two path holders on one body). The text below is the case as found.

`PoseOwnedExternally` tells the movement kernel not to integrate a body.
Three separate producers now use this marker: `ambition_boss_encounter`'s
conducted limbs, Mary-O's pipe transit and Mary-O's Snake Plants. The
marker is a **boolean**, not a record of its owner. One producer can
remove it while another still expects to hold the same body. Do not
address this by putting a second kinematic update in gameplay systems.

- `game/ambition_demo_mary_o/src/plant.rs:121-155` sets the marker while
  its brain is recognized and its pot is found. If either becomes false
  on a later tick, it `continue`s and does not release its marker. The
  same path uses `players.single()` globally (`:121`) for its shy-range
  test: a player in a different live room at overlapping coordinates can
  change a plant's cycle. These are distinct correctness failures.
- `game/ambition_demo_mary_o/src/pipe.rs:253-265` removes the common
  marker on exit, with no check that another authority owns the pose.
  Its position writes already use `movement::transit_body`, and that
  common geometry/remap seam should stay.

**Target:** a source-qualified *pose lease* or equivalent admitted
movement authority that projects to `PoseOwnedExternally` only while at
least one valid source holds a body. Define arbitration or explicit
refusal if two incompatible sources try to set one body's path in the
same frame; do not let system order decide who wins. The engine
movement kernel remains the one integrator/remap owner. A scripted path
publishes a transit/pose request and releases its claim on cancellation,
changed character/brain, room retirement, death, and rollback restore.
Observation of a player uses `LiveRooms.of(player) == LiveRooms.of(plant)`
and a deterministic target policy when more than one participant shares
the room. Port the Snake Plant's behavior to I4 only when `actor_think`
can express the cycle and its pose request without new native shortcuts.

**Witness:** stop a plant brain or remove/change its pipe while it remains
alive, and confirm the marker is released; a player in another live room
cannot make it shy; two pose sources do not erase one another; an active
pipe/boss limb restores its lease across rollback; the movement kernel
still sees exactly one final authority and emits no artificial landing.

## Re-measuring these gates

The `.ldtk` worlds are versioned in the `game/ambition_map_assets` submodule, so
a count over them is a repository fact another machine can check. See
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).
