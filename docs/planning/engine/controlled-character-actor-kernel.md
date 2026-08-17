# Controlled-character actor kernel — immediate Engine 1.0 priority

**State:** OPEN / PRIORITY — begin here before large new D115/D116 implementation campaigns.

## Goal

Finish the post-D73 runtime refactor so Ambition's protagonist, possessed bodies,
AI actors, local participants and future remote-controlled bodies all use one
ordinary actor/control pipeline.

The protagonist should be special because of current control assignment,
capabilities, authored content and game policy — not because generic simulation
has a hidden `PrimaryPlayer` coordinate system.

## First milestone

Start from current `tick_actor_brains` and related targeting/crowd/control code.
The milestone is reached when:

- generic crowd/combat arbitration is target-relative rather than anchored on a
  primary player;
- actor world observation, decision and mutation are separated by semantic
  phase enough that Bevy's parameter limit is not hidden through tuple packing;
- controlled and AI bodies use the same body/control contracts;
- remaining `PlayerEntity`, `PrimaryPlayer`, `ControlledSubject`, `PlayerSlot`,
  `ParticipantId` and `Brain::Player` uses have distinct documented meanings;
- important schedule ordering is explicit and deterministic;
- no replacement god `ActorContext`/service bag is introduced;
- existing Ambition behavior is preserved.

This directly unlocks cleaner multiplayer/multiview, open-world population,
navigation, possession, item custody and actor-monolith decomposition.

## Milestone status against HEAD (2026-08-14 local)

Every property now holds except the **semantic decomposition** of
`tick_actor_brains`, which is partial. Deleting the dead slot board was the right
architectural result, but reducing a system's parameter count is not the same as
completing its decomposition, and several of the properties that do hold were
reached by DELETING what the milestone described rather than by building it.
Check HEAD before starting the next slice.

⚠ **the one thing this milestone still waits on is narrow**: merging
`integrate_home_body` with `integrate_actor_body` needs the hit-emphasis/proper-time
decision, because those two differ in hit-stop/time semantics. Control authority
converged without it — do not let the feel decision be quoted as a blocker on
anything but time integration.

- ✔ **generic crowd/combat arbitration.** No longer anchored on a primary
  player, because the slot board it anchored is gone: `assign_slots` filled it
  every tick and **no production reader consumed the assignment**, so making it
  target-relative would have re-anchored a mechanism with no consumer. Spacing
  comes from the crowding signal, which reads positions and a ground/aerial kind
  and has no anchor at all. If a crowd board is wanted as a feature, it is a
  product decision and needs a real reader first.
- ◐ → ⏸ **parameter pressure is fixed; semantic decomposition reached a
  deliberate resting point** (see the ⇒ at the end of this bullet — read it
  before acting on the first paragraph, which describes the state BEFORE the
  three cuts). The 72h ledger row was reconciled to this verdict on 2026-08-14;
  it had been calling D117 the top executable priority while this said resting,
  and agents were skipping it rather than resolving the contradiction.
  `tick_actor_brains` went from sixteen parameters packed in a tuple to ten named
  ones. Deleting the dead slot board and adopting `CollisionWorld` were both good
  changes, and `PerceivedWorld` names a real observation concept. But the system
  is still large and still performs disposition mutation, snapshot/view
  construction, memory mutation, brain decision and `ActorControl` publication in
  one function. The next slice should split those responsibilities by
  authority/phase rather than declaring victory because Bevy accepts the
  signature.

  **First cut taken 2026-08-14:** liveness/index collection and crowding
  derivation moved out to `actors::crowd_observation` — the OBSERVATION half as a
  value, with the derivations testable without an App (three tests, one of which
  pins that query order cannot reach the result). The system is 619 lines, down
  from 663; the honest read is that the boundary is now a type rather than a
  place in a long function, and the bulk that remains is the per-body decision
  loop. That loop is the next cut.

  ⚠ **and measure the right thing when sizing it.** Of those 619 lines, 192 are
  the parameter list with its docs and 429 are the body — and the body splits
  214 code / 209 comment. So the executable logic is roughly 214 lines across
  snapshot construction, memory mutation, disposition mutation, brain decision
  and control publication. That is still multi-responsibility and still worth
  splitting, but a slice chosen by line count would mostly be moving prose. **Cut
  by responsibility.**

  **Second cut:** `PerceptionBody` construction — sixty lines of struct literal
  sitting between a snapshot build and a brain call — moved beside the type it
  builds, in `perception::perception_body_for`. "What does a body know about
  itself" is answered by the perception module now, not by reading a decision
  loop. Body code 214 → 188.

  **Third cut:** belief. The memory update and the sighted-target override sat
  adjacent but separate in the loop; they are one question — *where does this
  body believe its target is, after seeing and remembering* — and are now one
  call, `perception::believed_target`. Keeping them apart is what lets a caller
  update memory and forget to consult it, or consult it without updating. Body
  code 188 → 181.

  ⇒ **214 → 181 across three cuts**, and what remains in the loop is snapshot
  assembly, disposition mutation and control publication.

  ⛔ **the reaction-timer decay is NOT the next cut, and checking cost less than
  moving it.** `combat.decay_reaction_timers(dt)` looks like unrelated work
  riding in the brain tick, and a test fixture in `character_runtime` even
  hand-writes the system production appears to lack. But the rule is already
  consolidated into one function called from three places — controlled bodies in
  `control::input_systems`, actors here, bosses in the boss tick — one per
  population, and the controlled site decays on `frame_dt` where the other two
  use sim `dt`.

  ⛔⛔ **THIS PARAGRAPH USED TO CALL THAT DIFFERENCE DELIBERATE AND WARN AGAINST
  "deciding D114 by refactor". IT IS SUPERSEDED — D114 WAS RULED ON 2026-08-17
  AND THE PER-BODY FREEZE IS NOW THE INTENDED BEHAVIOUR.** `818218949` gave the
  actor road `let sim_dt = if combat.is_in_hitlag() { 0.0 } else { dt };`, so a
  hit between two actors freezes both, and Jon kept it: *"hitlag is a combat/body
  semantic, not something that should depend on whether a body happens to occupy
  the primary local-control road."* ⇒ **the per-road distinction was the defect,
  not a feel fork.** ⛔ if hitlag ever feels too sticky, tune its DURATION or
  SHAPE — **restoring a controlled-body/actor asymmetry is forbidden.** Ruling in
  [`../maintainer-decisions.md`](../maintainer-decisions.md).

  ⚠ **what survives of the warning, and it is the part worth keeping**: the old
  prohibition was measured on a build where every authored launch direction was
  vertically inverted and a tumbling launch resolved as a landing, i.e. where
  nobody was ever knocked anywhere (queue D155). ⇒ **a feel verdict inherits the
  build it was formed on**, and a prohibition written from one is only as durable
  as that build. ⚠ note also that `ladder_probe` CANNOT measure this either way,
  because it disables the opponent, so no hits land and no hitlag occurs.

  ⚠ **the `frame_dt` vs sim `dt` split at the three decay sites is a SEPARATE
  question** and this ruling does not settle it; what it settles is the hitlag
  freeze. Do not cite the D114 closure as permission to merge the three systems.

  ⚠ **the automatic pacify is a candidate with a stated cost.** `if
  disposition.is_hostile() && target.entity.is_none() && !in_a_fight` reverting a
  body to `Peaceful` is a consequence of target LOSS, so it belongs with
  `select_actor_targets`, which owns the target and runs immediately before —
  adding `&mut ActorDisposition` and `Has<ActiveCombatant>` there is
  straightforward. But it would take effect one tick earlier than today: the
  observation pass currently reads the pre-pacify disposition, so a body that
  just lost its foe leaves the crowd on the NEXT tick. Probably an improvement,
  certainly a behaviour change, and nothing pins it. Do it with a test that names
  the tick, or not at all.

  ⭐ **and one duplicate found while sizing the integrator fork, now deleted.**
  `engine_input_from_actor_control` — the controlled road's control-frame →
  `InputState` translation — spelled the mapping out again, sixty lines
  field-for-field identical to `ActorControlFrame::to_input_state`, which the
  ACTOR road already calls. Two spellings of one vocabulary, agreeing today and
  kept agreeing by nothing: a field added to the frame and wired into
  `to_input_state` reached every actor body and silently missed the controlled
  one. It calls the shared translation now and keeps only what is genuinely its
  own — stamping `control_dt` (a brain runs at sim time, a human's
  responsive-aim window does not) and the body's post-hit gates. 71 lines → 23.

  ⇒ with those three cuts the loop reads as a sequence — observe, decay, pacify,
  assemble the snapshot, build the view, apply belief, decide, publish — and
  `build_enemy_brain_snapshot` was already extracted. **This system is at a
  reasonable resting point; the remaining structural work in this milestone is
  the two-producer/two-integrator fork above, which needs a design decision
  rather than another extraction.**
- ✔ **controlled and AI bodies on the same contracts — DECISION CONVERGED
  2026-08-14.** Movement was already one path (`integrate_home_body` and the actor
  integration both reach `ae::step_motion`). Decision now is too: **one producer,
  `tick_controlled_brains`, translates participant control into `ActorControl` for
  any controlled body**, and `tick_actor_brains` skips a body carrying
  `Brain::Player`. The two-producer fork is gone, not arbitrated.

  ⭐ **what made the cut possible was measuring what the possessed body was
  paying for.** `tick_player_brain_from_control` reads SIX snapshot fields:
  `player_input`, `control_down`, `movement_frame_mode`, `aim_frame_mode`,
  `actor_facing`, `max_run_speed`. What the actor tick built before reaching it:
  a crowd observation, an enemy brain snapshot, a perception policy, a world view
  over the collision world, a believed-target derivation, and a MUTATION of the
  body's `PerceptionMemory`. A human piloting a body was constructing AI
  perception — and decaying that body's own sight memory — to move a stick.

  ⭐ **the one actor-specific fact was a movement scale, and it did not need
  actor configuration to state it.** `velocity_target` is an absolute world-space
  command, so the translation needs the body's top speed; that number now comes
  from `MotionModel::commanded_top_speed()`, a projection on the one
  movement-policy component every movable body already carries (the sibling of
  `jump_squat_remaining`). No new component, no mirror of `ActorConfig`, and no
  actor cluster granted to the home body to make one query match. Absent policy ⇒
  `0.0`, which is what the home avatar stated explicitly before.

  ⭐ **and the phase move fixed two live seams.** Both were consequences of the
  possessed body's frame being written in `WorldPrep`, a phase after the
  controlled one: `blank_scripted_control_frames` sits in `PlayerInput::ControlGate`
  — *"the only position where blanking is observable"* — so a scripted sequence
  driving a possessed body **blanked nothing**, and the causal movement-intent
  observer (`.after(ControlledBrainTick)`) recorded a possessed body intending to
  stand still. One producer in one phase settles both. Guarded by
  `a_scripted_sequence_silences_a_possessed_body` and
  `a_possessed_actor_is_driven_by_the_controlled_brain_producer`, which is the
  one that can only fail silently: a component the query requires and a
  production body lacks is an empty iterator, not a compile error.

  ⇒ **the remaining blocker is narrow and is not this.** Movement/time
  integration convergence — merging `integrate_home_body` and
  `integrate_actor_body` — awaits the hit-emphasis/proper-time decision (open
  item #5), because those two differ in hit-stop/time semantics. Control
  authority does not, and no longer waits on it.

  **The schedule the cut had to satisfy, enumerated before moving anything:**

  | fact | where it becomes true | relation to the controlled phase |
  | --- | --- | --- |
  | participant input | `populate_slot_controls`, `PlayerInput::Device` | before, chained |
  | `ResolvedMotionFrame` | `FrameResolveSet`, `.before(CoreSimulation)` | before, for BOTH populations — one system, three archetype queries |
  | **controlled decision** | `ControlledBrainTick` in `PlayerInput::Brain` | — |
  | scripted blanking | `blank_scripted_control_frames`, `PlayerInput::ControlGate` | after |
  | causal intent record | `.after(ControlledBrainTick)` | after |
  | AI decision | `tick_actor_brains`, `WorldPrep` | after |
  | mount relay | `steer_mount_from_rider`, after the actor tick | after |
  | `ActorControl` consumers | action emitters (after `WorldPrep`), integration (`PlayerSimulation`) | after |

  ⭐ **`PlayerInput → WorldPrep` was already the contract** ("CONTROL-SEAM
  ORDERING" in `schedule.rs`) and the frame resolver already published for actor
  bodies before both. The cut needed no new ordering edge — it moved a producer
  INTO the phase whose whole reason for running first is that participant input
  is final there.

  ⚠ **one filter is deliberately not inherited:** `tick_actor_brains` carries
  `Without<Dormant>`, and the controlled producer does not. Dormancy sleeps a
  BRAIN as an AI optimisation; a participant is not an AI to be optimised away.
- ✔ **the six names have distinct documented meanings** —
  [`../../concepts/one-body-one-path.md`](../../concepts/one-body-one-path.md)
  maps all six side by side, which is where the confusions happen.
- ✔ **important ordering explicit.** Audited the four relationships these slices
  introduced or exposed. Perception → brain tick is chained inside `WorldPrep`
  with the reason written down. Overlay rebuild → the migrated `CollisionWorld`
  readers is the same chain; the readers outside `WorldPrep` (the pogo resolver
  in `Combat`, the OOB recorder in `Trace`) are trivially later, which is what
  makes a trace show the world the simulation actually collided against. The
  clock → platform advance is frame-stable, because `WorldTime` is snapshotted at
  frame top. And the one relationship that was genuinely implicit — which of the
  two `ActorControl` producers wins for a possessed body — is **removed rather
  than documented**: disjoint populations need no order at all. ⭐ that is the
  preferred resolution whenever it is available; an ordering constraint you do
  not need is stronger than one you have written down.
- ✔ **no replacement god context.** No `ActorContext` or service bag was added;
  the new types are `PerceivedWorld` (three perception channels a view needs
  together) and an adopted `CollisionWorld`.
- ✔ **behaviour preserved**, plus one defect fixed: `advance_moving_platforms`
  asked the home avatar's hitstop for permission through `single()` + `return`,
  so **every match ran with its moving platforms frozen**.

### Answering one of the open questions below

*"Which current `PlayerEntity` semantics represent a legitimate home-avatar
concept versus obsolete generic-simulation assumptions?"* — measured: 22 live
query filters across engine crates. Presentation and read-model uses
(`sim_view`, `render`) are legitimate view edges; item/save/shrine/room-reset
uses are home-avatar policy and correct to do nothing in a match; the gravity
flip switch is game policy with its design question named in place and nothing
spawns one. **The obsolete-assumption category had one member and it is fixed.**
The failure shape to grep for is not the marker but `single()` + `else
{ return }` around it — four of those have now been removed.

## Decomposition direction

Once the kernel boundary is honest, peel surrounding domains from
`ambition_platformer2d_actor_monolith` by ownership. Registration should move
with the domain plugin; a carve that leaves the old owner importing/registering
it is not a successful boundary.

Use:

- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md)
- [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)
- [`bevy-plugin-and-crate-strategy.md`](bevy-plugin-and-crate-strategy.md)

The eventual actor kernel should be a coherent Bevy simulation plugin/crate,
not an Ambition composition root renamed after files moved around it.

## Acceptance pressure

- Ambition possession and body switching;
- zero-human-controlled-body headless simulation;
- two independently targeted groups without global-player arbitration;
- local/remote participants controlling ordinary bodies;
- future persistent NPCs using the same body/control/navigation seams.

## Open design questions — deliberately unresolved

- What is the smallest stable observation/decision input without creating a
  giant context struct?
- `PerceivedWorld::peers_seen_by` currently clones a peer `Vec` for each body;
  what borrowed/indexed observation shape avoids making per-body O(N) allocation
  part of the permanent open-world AI contract?
- Which targeting/crowd facts should be cached resources versus derived per
  phase?
- Where should long-lived controller state live relative to body state?
- Which current `PlayerEntity` semantics represent a legitimate home-avatar
  concept versus obsolete generic-simulation assumptions?
- Which extraction should follow first once the kernel is clean?
- Does the final actor kernel deserve a new crate name or should an existing
  domain crate become its owner?

Do not answer these by preserving today's directory layout. Re-measure HEAD and
choose the smallest boundary that improves authority and dependency direction.
