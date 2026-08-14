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

## Milestone status against HEAD (2026-08-14)

Six of the seven properties hold. ⛔ **two were reached by DELETING what the
milestone described, not by building it** — check before starting a slice.

- ✔ **generic crowd/combat arbitration.** No longer anchored on a primary
  player, because the slot board it anchored is gone: `assign_slots` filled it
  every tick and **no production reader consumed the assignment**, so making it
  target-relative would have re-anchored a mechanism with no consumer. Spacing
  comes from the crowding signal, which reads positions and a ground/aerial kind
  and has no anchor at all. If a crowd board is wanted as a feature, it is a
  product decision and needs a real reader first.
- ✔ **parameter limit no longer hidden.** `tick_actor_brains` went from sixteen
  parameters packed in a tuple to ten named ones. The room came from deleting the
  slot board and from adopting `CollisionWorld` — a contract that already existed
  and that the six largest systems had never taken, each hand-composing the same
  three-ingredient collision world.
- ▢ **controlled and AI bodies on the same contracts.** Movement is genuinely one
  path: `integrate_home_body` and the actor integration both reach
  `ae::step_motion`. **Decision is not.** `tick_player_brains` and
  `tick_actor_brains` are two producers of `ActorControl`, and until 2026-08-14
  the first was unfiltered, so a possessed body got both — from materially
  different snapshots (`max_run_speed: 0.0` versus the body's real top speed,
  which `tick_player_brain` multiplies the stick by). The populations are
  disjoint now; **collapsing the two producers is the remaining work**, and it is
  blocked on the home body having no actor cluster for the actor query to match.
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
