# Radiant NPC planning — authored lives on a replaceable planner

**State:** OPEN. Nothing of the planner is built. This page owns the plan for
[RADIANT-NPC-HTN](../queue.md#radiant-npc-htn--authored-npc-lives-and-replaceable-planning).
**Related:** [agentic character runtime](agentic-character-runtime.md) (the
layering: facts → observations → goals → policy → typed action),
[world facts, observations and memory](world-facts-observations-and-memory.md),
[navigation and reachability](platformer-navigation-and-reachability.md),
[instance lifetime and persistence](instance-lifetime-provenance-and-persistence.md),
[hosted world experiment](hosted-world-networking-experiment.md) (which
machine simulates an NPC).

## The question

Civilian NPCs should keep schedules, share places and objects, react and
replan, and go on living while no player is in their room. The content for
that (who goes where, when, and why) must be authored as data a game can
change without recompiling the engine host. Which planner does that, behind
which boundary, so that the planner can be replaced (by a utility scorer, a
scripted fallback, or later an LLM-backed policy) without touching the actor
and world crates?

**First customer:** Nikobolt and Coulomba Tesseract, who share the Eureka coil
lab (`tools/ambition_ldtk_tools/specs/eureka/eureka_coil_lab.ron`; both are
placed there today with `brain_override: "stand_still"` and their own
dialogue, `eureka_nikobolt` and `eureka_coulomba`).

## What the tree already has (surveyed 2026-11-10)

| Need | What exists | Where |
|---|---|---|
| A clock for schedules | `DayClock` and the game-authored `DayCycle` (morning, day, evening, night); rollback state that stops while paused | `ambition_time::day`, Ambition's cycle in `ambition_content::day_cycle` |
| A goal given to a body from outside | `Errand { fetch: SimId, outcome }` with `Pending` / `Done` / `Refused(NoRoute \| Gone \| HandFull)`, rollback state | `ambition_platformer2d_actor_monolith::body::errand` |
| Going somewhere with the body's real movement | the in-room surface graph and flight map, the `NavAdvice` the advisor writes, the `Roam` brain that follows it | [navigation](platformer-navigation-and-reachability.md#the-first-slice-a-surface-graph-a-brain-can-follow-2026-10-09) |
| Going to another room | an errand body crosses a zone into the next live room (`cross_on_errands`); into a room that is not live it leaves through its `Placed` whereabouts row and is rebuilt there when the room is live again | `body::errand`, [the cross-room slices](platformer-navigation-and-reachability.md#the-cross-room-slices-a-character-fetches-an-item-from-another-room-design-2026-10-09) |
| Facts a plan can test | `world_facts`, `WorldFactConditionsPlugin`, `WorldMemory`, `PerceptionMemory` | [world facts](world-facts-observations-and-memory.md) |
| A typed action to carry out | `ActionRequest` in `ActorActionMessage` | [agentic runtime](agentic-character-runtime.md#current-state) |
| A policy seam | **closed**: `CharacterBrainTemplate` is a nine-variant enum; a new policy is about ten edits in two crates and cannot come from a game crate | [agentic runtime](agentic-character-runtime.md#current-state) |
| An HTN library | none in `Cargo.lock` (`bevy_htn`, `bevy_bae` not present) | — |

So a planner here sits **above** errands and navigation: it picks the next
task, and a task becomes an errand-like goal on the body that the existing
brain and advisor carry out. It does not move bodies itself.

## The boundary to build

```text
authored domain (data: tasks, methods, preconditions, effects, schedules)
        ↓ loaded and validated outside the tick
planner core (engine-independent; no Bevy types)
   input:  a typed world-state snapshot for one NPC (facts it may know)
   output: a plan = [primitive task], with the method chain that chose it
        ↓
executor (Bevy, in the simulation)
   each primitive task → a typed goal on the body (go to place, use object,
   wait until, say line, fetch item) → existing brain / errand / navigation
   reports Done / Refused(reason) / Interrupted → replan
```

Rules the boundary keeps:

- **The planner reads facts, never the `World`.** Its input is a value built
  by the executor from world facts, memory and the clock; the same input
  gives the same plan, so a rewind or a replay replans to the same tasks.
- **The plan is rollback state, the planner is not.** What an NPC is doing
  (current task, its goal on the body, the plan's remaining tasks and a plan
  revision) is simulation state; the planner runs only when the executor asks
  for a plan, and its result is consumed on that tick.
- **A smart object is a fact with a claim.** An exclusive affordance (the
  workbench) is reserved by one NPC at a time through a typed claim in the
  simulation, not by the planner's belief; a lost claim is a `Refused` that
  causes a replan.
- **Offscreen life is the whereabouts row, advanced by schedule.** An NPC in
  a room that is not live is not simulated frame by frame. Its row records
  where its schedule puts it at the current clock, so a room built later puts
  it there. The "meaningful offscreen life" acceptance is met by schedule
  arithmetic plus the facts its tasks would have changed, not by a hidden
  simulation.
- **Explainable.** The executor keeps the method chain and the failed
  preconditions of the last plan, for a headless inspector and for dialogue
  ("why are you here?").
- **Replaceable.** The executor depends on a `Planner` trait (snapshot in,
  plan out). HTN is the first implementation; a utility scorer or a scripted
  schedule is a second, chosen per character by data.

## Slices

1. **Compare, then choose the core.** One benchmark domain (the two
   scientists' day: arrive, work at the bench, eat, talk, go home), authored
   in each candidate: `bevy_htn` authoring, `bevy_bae` execution, and a small
   standalone HTN core with no Bevy types. Measure: authoring as data (no
   compile), determinism (same input, same plan), planning time per NPC,
   explanation available, and how much of each depends on Bevy internals.
   Record the result here and choose. The executor contract above stands
   whichever wins.
2. **One NPC, one room, a schedule.** Nikobolt follows an authored day in the
   coil lab through the executor and the existing navigation brain. Witness
   (app_it): at each phase boundary he goes to the authored place and does
   the task; a control NPC with no domain stands still; the same run under a
   sync test replans to the same tasks.
3. **Two NPCs, one exclusive workbench.** Both want the bench; one claims it,
   the other replans to another task and takes the bench when it is free.
   Witness: never two users at once, both get a turn within the day, and the
   claim survives a rewind.
4. **Interrupt and replan.** A player interaction, a blocked route or a
   removed object interrupts the current task; the NPC replans from the new
   facts. Witness for each cause, with the reason the inspector shows.
5. **Offscreen and re-entry.** Leave the lab for half a day and come back:
   each scientist stands where the schedule says, with the facts their tasks
   would have set. Save, reload, and the same holds. Witness against the
   schedule, not against a recorded run.
6. **Change a policy without a host compile.** Edit the domain file (a new
   task, a moved time) and reload; the NPC follows the new plan in the
   running build. This is the acceptance's no-compile clause.
7. **Hosted authority.** Once the [authority experiment](hosted-world-networking-experiment.md)
   chooses, NPC plans run where the world is authoritative. Under GGRS the
   plan state is rollback state already (slice 2's sync test); under a hosted
   world the guest receives the NPC's body and current task, not the planner.

## Acceptance

From the queue row: both scientists follow different authored schedules,
share an exclusive lab affordance, react and replan, persist on departure and
re-entry, explain their plans, and stay correct under save/reload and hosted
authority. Their activity policies change without an engine-host compile. No
LLM inference and no all-peer rollback of the planner are required.

## Do not

- Do not add a tenth `CharacterBrainTemplate` variant per NPC behaviour; the
  planner drives goals through the existing brain, and opening the policy
  seam is the [agentic runtime's](agentic-character-runtime.md) change.
- Do not simulate rooms that are not live to make offscreen life visible.
- Do not let the planner write components; it returns a plan.
- Do not make the LLM path a dependency of the planner core.
