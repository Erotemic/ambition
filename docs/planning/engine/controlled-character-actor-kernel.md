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

## Milestone status against HEAD (2026-08-18)

Every property holds except the **semantic decomposition** of
`tick_actor_brains`, which reached a deliberate resting point rather than
completion.

✔✔ **The integrator fork is resolved (2026-08-18).** Jon ruled on 2026-08-17
that hitlag is a combat/body semantic, not something that should depend on
whether a body occupies the primary local-control road — a body freezes on its
own hitstop, on both roads. The axis-tuning refresh and hitlag freeze now live
in `ambition_characters::actor::step_body`, taking `&BodyCombat` rather than a
`dt` (a caller-computed `dt` was wrong for months). Ruling in
[`../maintainer-decisions.md`](../maintainer-decisions.md). ⛔ if hitlag ever
feels too sticky, tune its duration or shape — restoring a controlled-body/actor
asymmetry is forbidden.

⚠ **what is left of the fork is the ORCHESTRATION around the step**, not the
step: each road still builds its own input, decides its own reset, and
publishes its own hurtbox, in disjoint queries
(`With`/`Without<PlayerEntity>`) that cannot share one Bevy loop. ⛔ measure
whether each remaining pair is genuinely species-specific before merging it —
one pair already wasn't: the home road was re-compositing the whole collision
world per body from inputs the actor loop had already used, so both now take
the one composited world.

- ✔ **generic crowd/combat arbitration.** No longer anchored on a primary
  player, because the slot board it anchored is gone: `assign_slots` filled it
  every tick with no production reader. Spacing comes from the crowding
  signal, which reads positions and a ground/aerial kind and has no anchor at
  all. A crowd board, if wanted as a feature, is a product decision needing a
  real reader first.

- ◐ **parameter pressure is fixed; semantic decomposition reached a
  deliberate resting point.** `tick_actor_brains` went from sixteen parameters
  packed in a tuple to ten named ones, dropped the dead slot board, and adopted
  `CollisionWorld`. The system still performs disposition mutation,
  snapshot/view construction, memory mutation, brain decision and
  `ActorControl` publication in one function; the next slice should split those
  by authority/phase.

  Three cuts taken 2026-08-14: liveness/index collection and crowding
  derivation moved to `actors::crowd_observation` (three tests, one pinning
  that query order cannot reach the result); `PerceptionBody` construction
  moved to `perception::perception_body_for`; memory update and sighted-target
  override merged into `perception::believed_target` (keeping them apart is
  what lets a caller update memory and forget to consult it, or vice versa).
  Body code went 214 → 188 → 181 lines; what remains in the loop is snapshot
  assembly, disposition mutation and control publication.

  ⛔ **the reaction-timer decay is NOT the next cut.** `combat.decay_reaction_timers(dt)`
  looks like unrelated work riding in the brain tick, but the rule is already
  consolidated into one function called from three places — controlled bodies
  (`control::input_systems`), actors (here), bosses (the boss tick) — and the
  controlled site decays on `frame_dt` where the other two use sim `dt`. ⚠
  **that `frame_dt`/sim-`dt` split is a separate open question**; the D114
  hitlag ruling above does not settle it.

  ⚠ **the automatic pacify is a candidate with a stated cost.** Reverting a
  disposition to `Peaceful` on target loss belongs with `select_actor_targets`,
  which owns the target and runs immediately before — but it would take effect
  one tick earlier than today (the observation pass currently reads the
  pre-pacify disposition). Probably an improvement, certainly a behaviour
  change, and nothing pins it. Do it with a test that names the tick, or not at
  all.

  ✔ **one duplicate found while sizing the integrator fork, deleted.**
  `engine_input_from_actor_control` spelled the controlled road's
  control-frame → `InputState` translation sixty lines field-for-field
  identical to `ActorControlFrame::to_input_state`. It now calls the shared
  translation and keeps only what is genuinely its own (`control_dt`, the
  body's post-hit gates). 71 lines → 23.

  ⇒ **this system is at a reasonable resting point**; the remaining structural
  work in this milestone is the orchestration fork described above.

- ✔ **controlled and AI bodies on the same contracts — decision converged
  2026-08-14.** Movement was already one path (`integrate_home_body` and the
  actor integration both reach `ae::step_motion`). Decision now is too: one
  producer, `tick_controlled_brains`, translates participant control into
  `ActorControl` for any controlled body, and `tick_actor_brains` skips a body
  carrying `Brain::Player`.

  What made the cut possible: the possessed body previously paid for a full
  actor observation (crowd observation, enemy brain snapshot, perception
  policy, world view, believed-target derivation, and a mutation of
  `PerceptionMemory`) just to move a stick, reading only six of those fields.
  The one actor-specific fact, a movement scale, now comes from
  `MotionModel::commanded_top_speed()` — a projection every movable body
  already carries, no new component, absent policy ⇒ `0.0` (matching prior
  behaviour). Moving the producer into `PlayerInput` phase also fixed two live
  seams: `blank_scripted_control_frames` sits in `PlayerInput::ControlGate`,
  the only position where blanking is observable, so a scripted sequence
  driving a possessed body previously blanked nothing; and the causal
  movement-intent observer now correctly sees a possessed body's intent.
  Guarded by `a_scripted_sequence_silences_a_possessed_body` and
  `a_possessed_actor_is_driven_by_the_controlled_brain_producer` (the latter is
  the one that can only fail silently — a missing component makes the query an
  empty iterator, not a compile error).

  The schedule the cut had to satisfy:

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

  `PlayerInput → WorldPrep` was already the contract ("CONTROL-SEAM ORDERING"
  in `schedule.rs`), so the cut needed no new ordering edge — it moved a
  producer into the phase whose whole reason for running first is that
  participant input is final there.

  ⚠ **one filter is deliberately not inherited:** `tick_actor_brains` carries
  `Without<Dormant>`, and the controlled producer does not — dormancy sleeps a
  BRAIN as an AI optimisation, and a participant is not an AI to be optimised
  away.

  ⇒ **the remaining blocker is narrow:** merging `integrate_home_body` and
  `integrate_actor_body` was the open item this waited on; the D114 ruling
  above resolves it. Control authority itself no longer waits on anything.

- ✔ **the six names have distinct documented meanings** —
  [`../../concepts/one-body-one-path.md`](../../concepts/one-body-one-path.md)
  maps all six side by side, which is where the confusions happen.
- ✔ **important ordering explicit.** Perception → brain tick is chained inside
  `WorldPrep`. Overlay rebuild → migrated `CollisionWorld` readers is the same
  chain, so a trace shows the world the simulation actually collided against.
  The clock → platform advance is frame-stable (`WorldTime` is snapshotted at
  frame top). The one relationship that was genuinely implicit — which
  `ActorControl` producer wins for a possessed body — is removed rather than
  documented: disjoint populations need no order at all, which is the
  preferred resolution whenever available.
- ✔ **no replacement god context.** No `ActorContext` or service bag was
  added; the new types are `PerceivedWorld` (three perception channels a view
  needs together) and an adopted `CollisionWorld`.
- ✔ **behaviour preserved**, plus one defect fixed: `advance_moving_platforms`
  asked the home avatar's hitstop for permission through `single()` + `return`,
  so every match ran with its moving platforms frozen.

### Answering one of the open questions below

*"Which current `PlayerEntity` semantics represent a legitimate home-avatar
concept versus obsolete generic-simulation assumptions?"* — measured: 22 live
query filters across engine crates. Presentation/read-model uses (`sim_view`,
`render`) are legitimate view edges; item/save/shrine/room-reset uses are
home-avatar policy, correctly inert in a match; the gravity-flip switch is game
policy with its design question named in place. The obsolete-assumption
category had one member and it is fixed. The failure shape to grep for is not
the marker but `single()` + `else { return }` around it — four of those have
now been removed.

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
