# Controlled-character actor kernel

**Scope:** the target contract for the controlled-character actor kernel.
**Authorities:** [target logical authorities](architecture.md#target-logical-authorities)
and the [A4 packet](actor-monolith-work-frontier.md). The kernel is defined by
this contract, not by an SCC size.

## Kernel contract

Human input, deterministic AI, possession and a remote participant supply intent
to the same body's movement/action/reaction execution. Body physics and live
mutation do not move to an actor-spawn crate or an authored-character catalog.
The kernel owns accepted intent projection, body execution and actor-local state
transitions. Spatial algorithms and motion primitives have their own coherent
lower owner; higher action policy may be optional.

It does not own session/room restoration, persistent save policy, named content,
all world objects, independent projectile/item lifecycles, asset/device loading,
UI, audio, story cutscenes or host selection.

## Identities and relations

Keep participant/peer, input seat/device, current driven body, home/avatar body,
physical occurrence, camera subject and presentation focus distinct. They may
coincide in the default single-player case; that is not permission to substitute
one for another in an API.

The accepted driver relation coordinates simultaneous claims and projects
control deterministically. Mount custody and ability eligibility can contribute
to that decision without owning a second control answer. An ability can propose
a possession transition; the accepted relation belongs with control projection.
This close relationship may justify one package with internal cycles.

Retraction on disappearance, unmount, release and session retirement is part of
the relation. A saved room-transition intent captures the body subject when it
is admitted, not whichever body is driven when it eventually commits.

## Current shape and open work

`crates/ambition_platformer2d_actor_monolith/src/control/authority.rs` projects
control from possession/claim state. Its neighboring input code invokes
possession helpers and also reaches feature animation advancement. The latter
edge needs a phase/geometry classification; replacing it with a message without
checking its consumers is not a design.

`crates/ambition_platformer2d_actor_monolith/src/features/ecs/actors/update.rs`
uses shared body integration from avatar code, but still contains policy and
population decisions. One shared integration call does not prove every actor
gets one tick or that home-avatar assumptions have disappeared.

`crates/ambition_platformer2d_actor_monolith/src/actor_clusters.rs` is the live
query/mutation authority restored by the spawn correction. Builders remain in
`ambition_platformer2d_actor_spawn`; live provocation, ladder projection and rider
rebuild remain kernel operations. Keep that distinction.

A held item's shot is an `ActorActionMessage::Ranged` on the one projectile
road (`ambition_held_items`). There is no second projectile simulation.

## One box for a turned body

A body that is not square has one collision box, turned to the DOWN of the
body: `BodyKinematics::collision_box(last_step)`, which reads the DOWN from the
body's record (`SweepSample`), because a crawler on a wall is not turned to its
gravity. The published footprint (`CenteredAabb`) is a different fact: the
envelope of a body that has one (a boss is drawn larger than the box that
stops it). `BodyKinematics::aabb` and `size * 0.5` are the LEVEL box. The level box is right for a shot (a free body
with no frame), for a reader of the centre only, for the gravity lookup that
resolves the frame, and in a game with no turned gravity. For any other reader
it is a second statement, equal to the rule in normal gravity and wrong where
gravity turns.

Inside the kernel, each policy arm states the box its step moved
(`BodyKinematics::half_oriented`; the crawler arm from
`AdhesiveCrawlerMotion::body_down`), and gives that one value to the sweep
record, the hazard gate, water, climbables, the ledge carry and the rebound
pad (queue CRAWLER-HAZARD-FOOTPRINT, 2026-10-05/06; witnesses in
`movement/tests/hazard_footprint.rs` and `step_box_world_reads.rs`).
`ActorMut::aabb` asks the same rule (`integration/body_box_tests.rs`).

Built 2026-10-06 (queue LEVEL-BOX-READERS): the transit record
(`reconcile_transit`, schema 315), the world and reach readers, the arrival of
each traversal, the portal carve and eviction, the brain's floor queries, the
trace, and the two procedures written in world axes (the pet and
`CommandedMove`'s facing). `scripts/check_level_box_readers.py` compares the
search with `scripts/baselines/level-box-readers.json`, where each remaining
read has a class and a reason; it is red on a new read and on a baseline line
that is gone. It finds only the spellings it searches: a box built another way
is not found.

⛔ Rejected, do not retry:

- Turning the half that the perception view gives a brain. It is on the body's
  own axes, and its three readers (`RecoveryLens`, the rollout, the option
  scorer) each apply a frame: a half turned at the source is turned twice or
  laid on the wrong axis.
- A reader of a zero-length transit record that falls back to `CenteredAabb`.
  A boss's footprint is its draw envelope, not its collision box.

Open, each with what makes it live:

- `floor_ahead` and `situation.rs` measure the floor on world x. Not built:
  no fighter stands in turned gravity on a shipped road.
  `the_gravity_key_turns_a_hosted_smash_match_and_no_cpu_fighter_stands_in_turned_gravity`
  goes red when one does; build it then, or before a cognition benchmark in a
  room that turns gravity (`AMBITION_ACTOR_BRAIN_PROFILE`).
- The perception view has no DOWN of a peer, so a reader uses the viewer's.
- The order of the transit collapse and the contact readers: a body that
  blinks into an ECS hazard is hit on its arrival tick, and the path it
  travelled before the blink is not read that tick
  (`a_wielded_transit_is_settled_before_the_path_is_read.rs` holds the order).
- `possession_trigger_system` transits a body through `transit_body`; what the
  hazard and loading-zone readers see of it is not measured.
- A body walked with no frame (a boss in an encounter script) is on world x.
  No boss is in a room that turns gravity.
- A petted body in another frame than its petter is not handled.
- The portal presentation draws the gun and the indicator on world axes; a
  crossing in a shipped room whose gravity is turned is not measured.
- The actor view (`ambition_sim_view/src/view_index.rs`) derives a surface
  walker's draw size by inverting the footprint when the normal is mostly
  sideways. That is exact for a cardinal normal only; the raw size is
  `BodyKinematics::size`. Read, not measured.
- Water in a room whose gravity is not down: where the surface of a pool is,
  and whether a body that is not axis-swept gets wet (the axis arm is the only
  writer of `BodyEnvironmentContact`), is
  [Q160](../awaiting-maintainer-decision.md#q160--water-in-a-room-whose-gravity-is-not-down-where-is-the-surface-of-a-pool-and-does-a-body-that-is-not-axis-swept-get-wet).


Possession acquisition/traversal policy may remain an optional ability. Its
accepted control relation should be co-located with input projection rather than
put into a shared bag because two modules need it. No generic control-service
registry is necessary.

`features` is not a retained kernel abstraction. Keep only the specific live
actor execution/reaction responsibilities that belong here; move room mechanisms,
checkpoints and object construction to their owners. Item relations coordinate
with the kernel through explicit custody facts, while accounting and physical
item lifetime remain item responsibilities.

Do not wait for an SCC target before you apply these decisions. Unresolved
edges use the [edge dispositions](actor-monolith-decomposition.md#edge-dispositions)
vocabulary.

## Invariants

One live body execution per simulation tick; one accepted driver relation; one
source of authored simulation geometry; one owner for each lifetime transition;
rollback covers canonical state and deterministic derived projection; presentation
reads the result rather than becoming gameplay authority.

Legitimate policy differences remain explicit. A body may lack an action, have
different movement parameters or be ineligible for possession. Removing those
differences just to make all code paths look identical changes the game.

## Required acceptance pressure

Run a concrete matrix on the same body definition: human, brain, possession
handoff and remote/replay intent. Include zero human bodies, two simultaneous
participants, controlled-body disappearance, mount/dismount, equipment use and
rollback over each transition. Assert action continuity/teardown, driver identity,
movement state and attribution, not merely the presence of marker components.

Camera following, home-avatar placement and body control must be independently
changeable without redirecting input or damage credit. Tick counts and contact
publication must not double when a body qualifies for two historical query
populations. A minimal body/world profile and one richer combat profile provide
external SDK pressure; they do not replace the focused state-machine tests.

## Exit

A new actor-kernel crate name is justified only after its exported API describes
this authority and its consumers do not import unrelated monolith internals.
A coherent internal cycle is acceptable. A renamed directory with the same
session/item/world mixture is not completion.
