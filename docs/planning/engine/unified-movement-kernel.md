# Unified movement kernel — remaining work

The frame-aware movement kernel, typed resolution seams, rollback
registration, surface-momentum operations and portal transit geometry
are implemented. Git history has the architecture and migration record.
Two older customer-gated items remain below. A new scripted-pose ownership
issue is recorded after them; it has concrete October 11 customers.

## Remaining

### A crawler on a wall reports an unturned box (found 2026-11-11)

`02cabae983` moved a body's turn into its material (`BodyMaterial`) and
removed the separate surface-upright state; `write_sweep_sample` now records
`half_extent()` (the material envelope) where it recorded the box turned to
the step's DOWN. A crawler that climbs a wall still only changes its DOWN:
its material stays the identity. So its step collides a box turned to the
wall, while `BodyKinematics::aabb`, the step record (`SweepSample::half`)
and the hazard gate that shares that half all report the unturned box.

Measured on `ow1_bob` (53.76 x 14.15) walking into a party exit wall in
`party_follower_hits_an_exit_wall_and_can_cross_after_the_policy_changes`
(red on `fc7ce4d3eb`; not bisected): he stops with his centre
at x 685.59, flush with the wall (692.67) for a turned half of 7.07, and
climbs it; the record says half (26.88, 7.07) with down (1, 0), 20 units
into the wall. The doc comment on `write_sweep_sample` still states the turned
box. The owner is the body-material work under PORTAL-SIMILARITY (PSIM):
either the crawler turns its material when it attaches, or the record turns
its half to the DOWN again.

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

### Grounded attack momentum (GROUNDED-ATTACK-MOMENTUM, Q169)

**Measured 2026-11-10, the robot (axis law) in the shipped Ambition game.**
Probe: `game/ambition_app/tests/grounded_attack_momentum.rs`,
`probe_what_a_grounded_attack_does_to_a_running_body` (ignored). The robot
runs right in `central_hub_complex` for 45 frames (270 px/s, on the ground).
It then presses attack on one frame (`blade_rise`, which plays for 29
frames), with the stick as each row says, and is followed for 90 frames.

| Case | Speed at the press | Travel during the move | Travel in 90 frames | Speed after the press, every 6 frames |
|---|---|---|---|---|
| No attack, hands off (the brake) | 270 | – | 0.9 px | 57, 0, 0, … |
| Attack, hands off | 270 | 0.9 px | 0.9 px | 57, 0, 0, … |
| Attack, stick forward | 270 | 130.5 px | 405 px | 270 throughout |
| Attack, stick back | 270 | −98.2 px | −373 px | 183, −162, −270, … |
| No attack, stick forward (the run goes on) | 270 | – | 405 px | 270 throughout |

- **The move takes the stick.** With the stick held, the body walks through
  the swing at full run speed, forward or back. That is the authored intent
  (`player_robot.ron`: "a swing does not root it", the knight of Hollow
  Knight, Jon 2026-10-12).
- **It keeps no speed of its own.** Hands off, an attack brakes exactly as
  standing still does: 270 to 0 in under a pixel. The move adds no
  displacement.
- **Ledge carry follows from the two above.** Hands off, the body stops at
  once, so a grounded attack cannot carry it off a ledge. With the stick held,
  it walks off a ledge as a run would. The probe's "left the ground" flag also
  fires in the no-attack run, so the hall's floor sets it, not the move; a
  controlled-ledge witness is still owed.

**Recommendation for the robot:** no change. The measured behaviour is the
authored one, and the axis law's instant brake already rules out a slide.

**Measured 2026-11-11, the Smash fighters (the responsive axis law at
Ultimate's traction).** Instrument: `fighter_motion` (`ambition_app_tools`),
its run-attack rows: a full run (315 px/s) on the Smash stage, then Attack
with the stick let go on the press, and again with the stick held; read off
the body. Run with `demo` (the demo shell; this machine's app host waits for
entry music it does not have).

| Fighter | Case | Move | Length | Travel during the move | Slide to rest / speed at its end |
|---|---|---|---|---|---|
| any | no attack, let go (the brake) | – | – | – | 42.5 px (1050 px/s², Mario's 0.102 u/f² is 0.0972 here) |
| `smash_george_booul` | Attack, run let go | `jab` | 15 f | 42.1 px | 42.5 px to rest |
| `smash_george_booul` | Attack, run held | `george_booul_dash_attack` | 30 f | 115.6 px | 43 px/s at its end |
| `smash_duelist_a` | Attack, run let go | `jab` | 15 f | 42.1 px | 42.5 px to rest |
| `smash_duelist_a` | Attack, run held | `jab` | 15 f | 46.7 px | 58 px/s at its end |

- **A move does not take the stick.** Unlike the robot, a Smash jab held
  forward travels about what it does let go (46.7 against 42.1 px): the run
  brakes under it.
- **It keeps the run's speed and brakes at traction.** Let go, an attack
  out of a run slides exactly the stop slide, 42.5 px: the move adds and
  removes nothing.
- **A dash attack carries the run.** Stick held, the run is still a run on
  the press tick (`BodyMotionFacts::running`), so the press is the running
  attack; its authored forward impulse (`authoring::dash_attack`, 260 px/s at
  the swing) keeps it moving: 115.6 px in 30 frames. Let go on the press
  tick, the body is no longer running, so the press is a jab.
- The stand-in duelists bind no `attack_dash` (an incomplete kit by Q89),
  so a running press is their jab.

**Recommendation for Smash:** no change: this is Ultimate's model (traction
under a grounded move, the dash attack as the move that carries a run). A
dash cancel and a ledge are not measured.

**Not yet measured:**
- Sanic's surface-momentum law (the 2026-10-13 coast of 351 px from
  530 px/s comes from a brain test, not from a move played in the game);
  Sanic has no grounded attack, so the question there is the roll;
- a dash cancel, and a controlled ledge for every law.

**Found while measuring Sanic (2026-11-11, not fixed): a ball frozen at
speed.** In the demo's act, after 150 frames of running right from the
start (1314 px/s at x 2265), a spin dash (crouch, three revs, release)
launches the ball at 1517 px/s. At x 2919.3 it stops for good: `Riding`
chain 0 at s 9699.5 with `v_t` 1470.67 that friction never touches, because
every tick's movement step is refused (`BodyFrameOutput::events.motion_refusal`
is `Refusal { path: [], link: None, reason: WorkLimit }`), and a refused step
commits nothing. No control hold, no room pace, no component change. The
`WorkLimit` sites are the frame-threading and material-cast admission
(`frame/thread.rs`, `frame/crossing.rs`, `frame/material_cast/*`), the
portal owner's code, so it is left for that owner. Probe to rebuild it: a
`sanic_it` test on `build_demo_app()` driving `drive_control_frame` with the
inputs above. Measured on the same run: a crouch held at speed neither
brakes nor balls Sanic (identical to letting go: he coasts from 1314 px/s,
losing about 400 px/s per second). The act is not flat, so these are not
clean momentum numbers.

Compare them with Hollow Knight's Dash Slash before changing either.

## Re-measuring these gates

The `.ldtk` worlds are versioned in the `game/ambition_map_assets` submodule, so
a count over them is a repository fact another machine can check. See
[`../../recipes/re-measuring-a-planning-claim.md`](../../recipes/re-measuring-a-planning-claim.md).
