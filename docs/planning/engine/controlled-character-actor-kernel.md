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

Body, form, skin, possession and body mode are distinct (Q159, ruled
2026-10-19). The physical body is the world instance: position, collision
geometry, health, custody, identity. The character form (`CharacterForm`) is
the authored physical and mechanical configuration the body instantiates; a
change of form updates the same body's geometry, moveset, movement and
appearance. The skin (`WornSkin`) is presentation only. Possession moves a
participant's control to another existing body, which keeps its identity,
geometry and state, and the original body stays in the world. The body mode
(`BodyMode`) is a posture of the same body and form. Transformation and
possession share the body simulation and the control relation above and stay
two operations. The completed CHARACTER-FORM geometry work stands;
[FORM-REBIND](#form-rebind--admit-and-apply-the-complete-intrinsic-form)
below closes the remaining admission and intrinsic-configuration boundary.

Retraction on disappearance, unmount, release and session retirement is part of
the relation. A saved room-transition intent captures the body subject when it
is admitted, not whichever body is driven when it eventually commits.

## FORM-REBIND — admit and apply the complete intrinsic form

**State:** planned, from static source `00bf92c22142`, 2026-11-22.
This is a residual of Q159 and Q103. It keeps the existing body, native
movement/ability owners and prepared-character grant. It does not reopen
the completed geometry, skin or possession separation.

### Admit before publishing the new form

The narrative `WearRequested` path in
[`body/wear.rs`](../../../crates/ambition_platformer2d_actor_monolith/src/body/wear.rs)
checks the App's `CharacterCatalog`, writes `CharacterForm`, and queues
a potentially invulnerable transformation beat from current sprite data.
The gameplay applier in
[`character_execution.rs`](../../../crates/ambition_platformer2d_actor_monolith/src/avatar/character_execution.rs)
checks the session's prepared cast later and can restore the previous ID.
That refusal is too late to prove that the body did not change.

[`CharacterProjection`](../../../crates/ambition_platformer2d_actor_monolith/src/character_projection/mod.rs)
runs before the Persona phase. A target absent from the session cast
becomes `incoming == None`, which retracts the old grant and removes the
projection record. The queued
[transform beat](../../../crates/ambition_platformer2d_actor_monolith/src/body/transform_beat.rs)
can still grant transforming invulnerability after the ID is restored.
See the [input phase order](../../../crates/ambition_platformer2d_runtime/src/schedule.rs)
and [narrative request registration](../../../crates/ambition_platformer2d_runtime/src/world_schedule.rs).

The existing [SessionCast contract](../../../crates/ambition_characters/src/session_cast.rs)
allows the App to publish N+1 while a live session still selects N.
A target present only in N+1 therefore supplies a bounded refusal case.
Even if both generations contain the ID, the beat must not use N+1's
sheet while mechanics use N. This audit establishes the split in source;
it does not measure how often an interactive reload exposes it.

Resolve the existing form request against `SessionCast` and the selected
immutable mechanical/sheet inputs before changing the applied identity,
withdrawing a grant, or creating a beat. Pass that admitted selection
through the existing application phases. `CharacterForm` must describe
the applied form; a failed lookup is not an instruction to remove it.
If selection changes between admission and application, retain the
admitted inputs while valid or refuse/re-admit before any native write.
Do not re-resolve against the App's current catalog halfway through.

A refusal consumes the request with the whole old body and its active
beats intact. Existing native clearance, custody and relationship checks
still own mutable eligibility. This does not require a new transaction
framework or a new body constructor. Remove the App-catalog membership
and beat lookup and the late restore-by-ID role after the admitted path
owns the operation. Keep internal refusal checks as protection, not as a
repair for already published consequences.

### Rebind intrinsic configuration through the native state owners

The shipped Hall conversation changes the same body between
`npc_fell_sheep` and `npc_sheep_cloud`. The
[catalog rows](../../../game/ambition_content/assets/data/character_catalog.ron)
explicitly change grounded, Walk-only locomotion to intrinsic free flight
with provider-resolved abilities. The
[conversation](../../../game/ambition_content/assets/dialogue/sandbox/hall.yarn)
describes lift-off and return. This is separate from the death script's
fresh cloud spawn.

Fresh construction in
[`ambition_body_seed`](../../../crates/ambition_body_seed/src/lib.rs)
uses the prepared flight fact to set the ability baseline, initial flight
mode, intrinsic gravity scale and `ActorTuning.is_aerial`. Form replacement
updates geometry and kit, but only refreshes `AbilityBase` in its
home-body, non-match arm. It does not rebind those other intrinsic flight
facts for the ordinary sheep. Runtime
[steering](../../../crates/ambition_platformer2d_actor_monolith/src/body/movement.rs),
[integration](../../../crates/ambition_platformer2d_core/src/movement/integration.rs)
and [gravity](../../../crates/ambition_platformer2d_runtime/src/gravity/resolve.rs)
read the retained values, so correct preparation alone cannot fix the
changed body.

Construction and replacement must consume the same resolved intrinsic
form decisions. Reuse the current prepared definition, physical baseline
and native application boundary. Extend their bounded value/transition
helpers where needed; do not duplicate a seed constructor in the re-wear
system or copy a newly built scratch body over live components.

Keep these inputs distinct:

| Input | Owner and replacement rule |
| --- | --- |
| Intrinsic form configuration | Prepared form: base abilities, movement model/tuning and intrinsic flight/gravity policy; rebind when the admitted form changes |
| Role and occurrence terms | The selected home experience, match seat, boss role or summon: retain, convert or refuse its owned grants/overrides under that role's existing policy |
| Current effective permissions | Ability contributions, ceilings and room/body access at the native pre-integration cut; never cached as the form's identity |
| Live movement mode | Native movement state: preserve an unchanged toggle; apply a deliberate enter/leave policy when the selected intrinsic mode changes |
| Environment and effect forces | Their current native owners; a form's intrinsic gravity response does not replace their acceleration or lifetime |

The ordering matters. Settle flight against the selected intrinsic policy
and current effective permissions before movement reads it, including
when the effective ability bitset is unchanged. Merely setting
`fly_enabled = true` then relying on
[`project_body_abilities`](../../../crates/ambition_platformer2d_core/src/ability_projection.rs)
to clear it is unsafe: that projector returns early when effective bits
do not change, and some steering readers test the live mode alone.
Likewise, `fly && !fly_toggle` after masks is not proof of intrinsic
free flight; a mask can deny the toggle independently. Intrinsic gravity
response, permitted control, and live activation remain separate facts.
The native movement owner settles their combination once, when the
selected flight policy or its effective permissions change. Extend that
owner's enter/leave operation; spawn/reset initialization and room-arrival
reset are not live form-change APIs. A grounded intrinsic baseline does
not deny a separately granted flight capability, and denied flight
permission does not itself replace intrinsic gravity response. See the
[intrinsic locomotion contract](../../../crates/ambition_characters/src/actor/intrinsics.rs).

Role terms are real inputs, not just home/match exceptions. Existing
[boss and summon construction](../../../crates/ambition_platformer2d_actor_spawn/src/actor_spawn/mod.rs)
supplies role-specific flight, health or contact terms. Preserve their
declared ownership when sharing intrinsic form resolution; not every
difference from a bare character seed is stale form state.

Preserve velocity, carried momentum, pending launches, accumulated damage,
controller identity and custody unless the owning transition explicitly
converts or retires that state. `BodyFlightState` carries more than a
flight Boolean; do not reset the whole component. Keep the current
state-preserving motion-model switch. A possessed NPC uses the same form
rebind as an unpossessed NPC; possession itself changes no intrinsic form.
Explicit role/home/match terms remain in force, and a skin changes no
mechanical configuration.

Retraction must read what the old application owned. Keep enough of that
fact in the current applied/grant record; do not recover it by looking up
the old ID in a new registry. The existing death-trait refresh does that
lookup for `previous_authored`, so a same-ID field removal can erase its
evidence. Check this touched withdrawal with the native rebind record;
do not open a separate general grant journal.

### Delivery, deletions and acceptance

1. Move form and beat admission ahead of consequences on the existing
   request road. Exercise N active with N+1 published, including selected
   metadata. Observe refusal before any repair tick can hide a mutation.
2. Share resolved intrinsic decisions between construction and replacement,
   then apply them through the ability/movement owners for ordinary bodies
   as well as home bodies. Compare a fresh cloud's intrinsic configuration
   with a sheep transformed to cloud and back, both unpossessed and
   possessed. Compare configuration, not fresh and live mutable state.
3. Check same-form reapplication with carried momentum or a pending launch,
   and a form change while a ceiling already denies flight, then lift
   that ceiling. Denying only the toggle must not create permanent flight.
   Preserve explicit role/home/match terms. Extend existing focused body
   witnesses; no full controller/roster matrix is needed for this slice.

Delete the home-only refresh as the sole form-ability path. After the
shared rebind covers its real composition, remove Mary-O's duplicate
per-tick cosmic AirJump lend:
[authored ability](../../../game/ambition_demo_mary_o/assets/data/character_catalog.ron),
[current duplicate](../../../game/ambition_demo_mary_o/src/cosmic.rs).
Other named cosmic mechanics remain provider policy.

Resolve stable inputs at admitted form/generation changes. Add no
per-body name search, default interpreter, all-combinations cache or new
active schedule. This plan claims no runtime or build-time gain.
[Character A3](character-authoring-package.md#a3--preserve-one-preparation-authority)
and [linking L2](capability-linking-and-prepared-facets.md#l2--link-one-candidate-across-two-mechanical-owners)
own its preparation dependencies. The existing native operation owns
application; it can be corrected before wider linking work.

## Current shape and open work

`crates/ambition_platformer2d_actor_monolith/src/control/authority.rs` projects
control from possession/claim state. Its neighboring input code invokes
possession helpers and also reaches feature animation advancement. The latter
edge needs a phase/geometry classification; replacing it with a message without
checking its consumers is not a design.

`crates/ambition_platformer2d_actor_monolith/src/body/actors/update.rs`
owns the shared body integration entry. Its internal home-body helper and
`body::BodyFrameOutput` no longer depend on avatar policy. The entry still
contains policy and population decisions. One shared integration call does not prove every actor
gets one tick or that home-avatar assumptions have disappeared.

`crates/ambition_body_seed/src/clusters.rs` is the live
query/mutation authority restored by the spawn correction. Builders remain in
`ambition_platformer2d_actor_spawn`; live provocation, ladder projection and rider
rebuild remain kernel operations. Keep that distinction.

A held item's shot is an `ActorActionMessage::Ranged` on the one projectile
road (`ambition_held_items`). There is no second projectile simulation.

## One box for a turned body

The accepted physical material owns body scale, turn and parity. Gravity is a
separate input. `BodyKinematics::canonical_box` gives the source box before
placement; `BodyKinematics::material_map` places that source in the room.
`BodyKinematics::aabb` and `BodyKinematics::half_extent` give its physical
axis-aligned bounds. They are not an unturned level box.

The published footprint (`CenteredAabb`) can use a separate canonical body
envelope. Physical footprint publication uses the same accepted material.
`SweepSample` retains departure material and an admitted source turn. Do not
reconstruct physical orientation from gravity. Legacy movement-policy contacts
still use coarse boxes; their native-contact replacement remains open in the
[portal plan](portal-parts-similarity-and-chirality.md).

Code: physical material `02cabae98`; physical footprint publication
`fcb9f7280`. The body-box and admitted-righting tests guard the shared pose.

Built 2026-10-06 (queue LEVEL-BOX-READERS): the transit record
(`reconcile_transit`, schema 315), the world and reach readers, the arrival of
each traversal, the portal carve and eviction, the brain's floor queries, the
trace, and the two procedures written in world axes (the pet and
`CommandedMove`'s facing). `scripts/checks/check_level_box_readers.py` compares the
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
- Water in a room whose gravity is not down (Q160, ruled 2026-10-19): fluid
  simulation is deferred. A pool's surface stays its world-horizontal top
  edge, a stated limitation: in a turned room it reads wrong to the eye and
  to `submersion`. The axis arm is the only writer of
  `BodyEnvironmentContact`, so a crawler or momentum body ignores water it
  touches; a narrow contact fix for that is allowed, a fluid project is not.


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
