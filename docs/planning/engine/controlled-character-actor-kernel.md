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

## Milestone status against HEAD (2026-08-13 local)

One property is **partial** and one remains **open**; the rest hold. Deleting the
dead slot board was the right architectural result, but reducing a system's
parameter count is not the same as completing its semantic decomposition, and two
of the properties that do hold were reached by DELETING what the milestone
described rather than by building it. Check HEAD before starting the next slice.

- ✔ **generic crowd/combat arbitration.** No longer anchored on a primary
  player, because the slot board it anchored is gone: `assign_slots` filled it
  every tick and **no production reader consumed the assignment**, so making it
  target-relative would have re-anchored a mechanism with no consumer. Spacing
  comes from the crowding signal, which reads positions and a ground/aerial kind
  and has no anchor at all. If a crowd board is wanted as a feature, it is a
  product decision and needs a real reader first.
- ◐ **parameter pressure is fixed; semantic decomposition is not.**
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
  assembly, disposition mutation and control publication. The next cut is the
  mutation half: pacifying a hostile whose foe died, and publishing
  `ActorControl`, are both writes sitting inside what should be a decision.
- ▢ **controlled and AI bodies on the same contracts.** Movement is genuinely one
  path: `integrate_home_body` and the actor integration both reach
  `ae::step_motion`. **Decision is not.** `tick_player_brains` and
  `tick_actor_brains` are two producers of `ActorControl`, and until the current
  run the first was unfiltered, so a possessed body got both — from materially
  different snapshots (`max_run_speed: 0.0` versus the body's real top speed,
  which `tick_player_brain` multiplies the stick by). The populations are
  disjoint now. **Collapsing the two producers is the remaining work.** Do not
  solve it by giving the home body an actor cluster merely so one query matches;
  that asymmetry is evidence that the common decision seam must be expressed in
  terms of common body/brain/control facts, with actor-only facts supplied only
  where a brain actually needs them.

  ⭐ **and the fork is one layer below the producers.** Traced `velocity_target`
  to its only consumer: the actor integrator's flight limb. `integrate_home_body`
  has no flight limb and never reads the field, which is why the home path passes
  `max_run_speed: 0.0` — deliberate and inert, and the brain's own comment says
  so. The actor integrator projects `velocity_target` onto its flight axis, so
  its brain must be handed the body's real top speed. The two producers differ
  because the two INTEGRATORS do; a common decision seam has to account for that
  rather than pick one integrator's convention.

  ⚠ **no live defect today**: `BodyMode` has no flight variant, so the home
  avatar cannot reach the case where the zeroed field would matter. A structural
  fork with a latent edge, and the edge arrives the moment a controlled body can
  fly — which the open-world and possession programs both point at.
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
