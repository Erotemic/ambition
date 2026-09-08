# Controlled-character actor kernel

**Target contract, baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
Use the [responsibility map](architecture-responsibility-map.md) and
[A4 packet](actor-monolith-work-frontier.md). This is not a claim that every
current body policy has converged or that a future six-module SCC defines the
kernel.

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

## Current source evidence and unresolved work

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
rebuild remain kernel operations. Preserve that distinction during further work.

## Decisions for the remaining cycles

Possession acquisition/traversal policy may remain an optional ability. Its
accepted control relation should be co-located with input projection rather than
put into a shared bag because two modules need it. No generic control-service
registry is necessary.

`features` is not a retained kernel abstraction. Keep only the specific live
actor execution/reaction responsibilities that belong here; move room mechanisms,
checkpoints and object construction to their owners. Item relations coordinate
with the kernel through explicit custody facts, while accounting and physical
item lifetime remain item responsibilities.

Do not wait for an SCC target before establishing these decisions. The
[edge ledger](actor-monolith-hard-core-edge-ledger.md) can keep unresolved
operations on HOLD while A1/A3 proceed independently.

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
