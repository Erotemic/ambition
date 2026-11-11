# Radiant NPC planning — authored lives on a replaceable planner

**State:** OPEN. Slice 1 is done (2026-11-10): the core is chosen and built
(`crates/ambition_htn`); nothing drives a body with it yet. This page owns the plan for
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
lab (`game/ambition_map_assets/ambition_content/generators/eureka/eureka_coil_lab.ron`; both are
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
| An HTN library | `crates/ambition_htn` since 2026-11-10 (slice 1, below); no third-party one | — |

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

## Slice 1 result (2026-11-10): a standalone core

The two published candidates were read at their sources (`bevy_htn` 0.0.1,
`bevy_bae` 0.1.0); neither builds against this tree's Bevy 0.19, so each is
judged by its design. The third candidate was built and measured.

| | `bevy_htn` 0.0.1 | `bevy_bae` 0.1.0 | `ambition_htn` (built) |
|---|---|---|---|
| Bevy | 0.15, whole `bevy` | 0.18 (`bevy_ecs`, `bevy_app`, `bevy_reflect`) | none (`serde`, `ron`) |
| Domain as data, no compile | yes: a pest-parsed `.htn` text asset | no: Rust bundles; conditions and operators are systems | yes: RON, validated when read |
| Planner input | a `Reflect` `Component` state | properties on entities, read from the `World` | a `Facts` snapshot (`BTreeMap`) |
| Determinism | plan ids from `rand::rng()` | depends on entity and system order | a pure function; ordered maps, no RNG or clock |
| Explanation | the plan's task ids | debug logging | each step's method chain, and every method set aside with the first condition that failed |
| Execution | `bevy_behave` trees as operators | operator systems | none: a step is an `Action` (verb, target) the executor turns into a goal on a body |
| Planning time | not measured (does not build here) | not measured (does not build here) | 4.86-4.88 us per plan, release, the benchmark domain over 24 hours x 2 scientists |

**Choice: `ambition_htn`.** It is the only candidate that meets the
boundary above as written: facts in, plan out, no `World`, no Bevy type, the
same plan on every peer. `bevy_htn`'s text authoring is the idea worth
keeping, and the RON domain keeps it in the format the rest of the content
uses. The executor contract is unchanged.

The benchmark domain is `crates/ambition_htn/tests/data/eureka_scientists_day.ron`:
one domain, each scientist's hours as their own facts, so Nikobolt (8-12-18)
and Coulomba (10-13-20) keep different days. Witnesses
(`tests/scientists_day.rs`): each follows their own hours; a taken bench is a
different plan whose explanation names the method set aside and its failed
condition (control: a free bench); the same facts give the same plan; a domain
naming an undeclared task is refused by name; a domain that recurses forever
stops at its budget. Run the measurement with
`cargo test -p ambition_htn --release -- --ignored --nocapture`.

What slice 1 does not settle: where a game's domain file lives and how it is
reloaded (slice 6), and the executor's verbs (slice 2).

## Slice 2, first part (2026-11-10): a body sent to a place

The executor's `go_to` needs a goal that is a place, not an item. `Visit
{ place, outcome }` (`body::visit`, rollback state, `actor.visit`) is the
errand's sibling: the navigation advisor answers it with the same sight
(`ErrandSight::At`, the feet point on the surface under the place, or
`NoRoute` when no surface the body gets to is under it), a navigating brain
goes there with the body's own movement, and `settle_visits` ends it
`Arrived` when the advisor says the body stands at the brain's goal, or
`NoRoute`. A pending errand comes before a visit. In-room only.

Witnesses (`game/ambition_app/tests/a_dog_sent_to_a_place.rs`): the basement
dog sent to a surface only its jump reaches arrives with its feet at the
place, and without its jump refuses the same place where it stood. Its own
sync test (a visit given from inside the timeline resimulated to the same
world) was cut 2026-11-10 to save 141 s of suite time: a visit under a sync
test is witnessed by slice 3's bench hand-over.

## Slice 2 (2026-11-10): Nikobolt lives his day

The executor is `body::life` (engine; the actor crate). A game publishes a
`LifeBook` (domains, a life per character id, and how long each verb holds a
body); Ambition's is the pack file `data/npc_lives.ron` (schema `npc_lives`,
owned by `ambition_content_schemas`, published at startup and again by a
content reload, so an edited domain is the next plan's domain with no Rust
build). A body whose character has a life gets a `Life` (rollback state,
`actor.life`): its domain's name, its facts, the plan's remaining steps, what
it is doing, a plan revision, and why the last plan stopped short.

- **Plan on each new hour.** The `hour` fact is the whole hour of the
  `DayClock`; the planner runs on the tick the hour changes, from the body's
  facts. A plan that ends early waits for the next hour.
- **`go_to <place>` is a `Visit`.** The place is what the body's room names:
  the room's own name (the body is there), else the first authored thing of
  that name: a sign, a placement, a prop, a door. A name the room lacks stops
  the plan with `NoPlace`; a visit refused for want of a route, `NoRoute`.
- **Any other verb holds** for the hours the book says (`holds`), or is done
  the tick it starts.
- **A step done makes its effects facts** (`Domain::apply_step`): the next
  plan starts from what the body did.
- **The brain goes only where it is sent.** `keeps_to_its_tasks` is a `Roam`
  preset with no company and an endless rest, so the advisor's visit is the
  only place it goes.

The coil lab names its places with signs (generator
`game/ambition_map_assets/ambition_content/generators/eureka/make_specs.py`): `workbench` ("COIL
No. 27", at the bench) and `lunch` ("LUNCH. NAPKINS: 18", by the crate);
home is out of `lab_door`.

Witnesses (`game/ambition_app/tests/nikobolt_lives_his_day.rs`): from 11:30
Nikobolt walks to the bench and does an experiment there (his facts count
it and the bench is released); at noon a new plan sends him to the crate,
where he eats; Coulomba, with no life, does not move (the control). His
morning under a sync test (202 s) was cut 2026-11-10: slice 3's bench
hand-over resimulates the same walk, claim, experiment and release.

Not yet (both done since: review 2026-11-10, findings 2 and 3): a fact that
turns over with the day (`ate`), and `say` as a spoken line.

## Slice 3 (2026-11-10): two scientists, one bench

Coulomba has a life too (10-13-20), with the same domain. Three things were
added to the executor:

- **A claim is simulation state.** `claim <thing>` takes the thing for one
  body in its live room (`Life::claim`); `release` lets it go, and so does a
  replaced plan. A claim on a thing another body holds is lost: the body
  plans again at once, from what it now sees, and its life keeps why
  (`Life::because`, `LifeStop::Taken`).
- **Facts read from the world.** A life row names what it observes:
  `Free(thing)` (no other body in the room holds it) and `Present(character)`
  (a body of that character is in the room). They are written into the
  facts when the body plans, so `bench_free` and `partner_here` are no longer
  the body's beliefs.
- **A spent plan replans on what it sees.** A body whose plan is spent plans
  again when an observed fact differs from its facts: the one who read notes
  takes the bench once it is free.

Bodies are looked at in the order of their character ids, so of two claims
on one tick the first is a fact of the cast, not of query order.

Witnesses (`game/ambition_app/tests/two_scientists_share_one_bench.rs`), from
10:05: no tick has two holders or two users of the bench; Nikobolt arrives
second, finds it taken (10:09) and plans `read_notes` instead; Coulomba's
experiment ends at 10:23 and Nikobolt's, at the bench he took when it was
free, at 10:39. Under a sync test the bench passes from one holder to the
other with the session healthy. Slice 2's witness now runs at 9:30, before
Coulomba is in, with Dmitri (no life) as its control.

## Slice 4 (2026-11-10): interrupt and replan

Three causes stop a plan, and each time the body plans again from the new
facts, its life saying why (`Life::because`):

- **A conversation.** `interrupt_talked_to_lives` stops the plan of the body
  `ActiveConversation` names as its talker: it lets go of its claim and its
  walk (`LifeStop::Interrupted("conversation")`). It runs outside the
  gameplay gate, because a conversation puts the game in dialogue and the
  gated executor does not run then (measured: the gated version never saw the
  conversation). When the conversation ends, the body plans again.
- **No route.** A refused visit (`NoRoute`) plans again at once and leaves
  `unreachable:<place>` among the facts until the next hour.
- **A place gone.** A place the room no longer names (`NoPlace`) does the
  same with `absent:<place>`.

A domain reads those facts (the scientists' `use the bench` method requires
neither), so the replan chooses another way. The same failure twice in a row
waits for the next hour, so a domain that ignores the fact does not plan
every tick; an interrupt always replans.

Witnesses (`game/ambition_app/tests/a_scientists_plan_is_interrupted.rs`):
talked to on his way to the bench, Nikobolt stops, stands while they talk,
then plans again (because of the conversation) and does the experiment at
the bench; stood on the east gantry with no movement, his visit is refused
and he plans to read notes (because `NoRoute("workbench")`, with
`unreachable:workbench`); with the bench sign removed from the room he plans
to read notes (because `NoPlace("workbench")`, with `absent:workbench`).

## Slice 5, first part (2026-11-10): offscreen and re-entry

A body in a room that is not live is not simulated. `LifeLedger` (rollback
state, `actor.life_ledger`; cleared when a session activates) keeps each
character's facts and the hour it last planned while its body lives. A body
built again in a room that comes back takes its life from its row and
catches up by schedule arithmetic:

- each whole hour it missed is planned and the steps' effects applied, with
  no walking. An exclusive thing is arbitrated by schedule, not simulated
  (review 2026-11-10, finding 8): each thing has the hour to give, a plan
  books the holds of its steps between its claim and its release, and the
  bodies that catch up on one tick book in character-id order (the same on
  every peer). A body sees the thing free while the hour has time left, and a
  plan that would overrun the hour plans again with it taken
  (`plan_an_hour_away`; unit test `bodies_away_share_a_thing_by_the_hours_it_has`).
  The booking is never overrun: a body with no `Free` of the thing, or whose
  planner claims it although told it is taken, has no plan that hour and
  nothing it would have done is applied (review 2026-11-10, second pass;
  `a_plan_that_would_overrun_a_thing_is_never_booked`). It counts hours, not
  intervals: two quarter-hour uses fit in an hour without deciding which came
  first, so it bounds the sum of uses and is not a timetable.
  The booking outlives the tick (2026-11-10): the ledger keeps it
  (`LifeLedger::bookings`, by the room's definition id, since a rebuild
  changes the live instance), so a body that comes back on a later tick
  books against the hours the earlier ones took
  (`a_body_back_on_a_later_tick_books_against_the_earlier_ones`, whose control
  with no ledger double-books). Hours older than the longest catch-up (a
  week) are forgotten. The bookings are in the save (2026-11-10, save v8,
  `PersistedBooking`, hours in thousandths so the save keeps `Eq`): they
  ride the lives' mirror and restore, so a body catching up after a load
  books against the hours the saved lives took
  (`the_bookings_come_back_from_the_save_as_they_went`). Not yet: a thing
  held by a body that was live (not away) in an hour another body catches
  up is not booked for it. The open question is the measure: booking the
  whole hour for any hold over-books it (a five-minute sit blocks the
  hour), and booking the time held needs the claim's start on `Life`
  (rollback state) and a ledger write when the claim ends or the hour
  turns, not every tick (the ledger's mirror serializes the lives);
- the current hour is planned and its leading `go_to` steps are done at once;
- the body is put at the last place its schedule sent it
  (`put_returning_lives`), and the rest of the plan runs live.

A catch-up plans at most a week of hours.

Witness (`game/ambition_app/tests/a_scientist_away_is_where_the_schedule_says.rs`):
the player leaves the lab at 9:30 (the lab is not live: nobody in it is
simulated) and comes back at 19:30. Read against the schedule: Nikobolt
(8-12-18) has gone home (he stands at the lab door, x 64 against the door's
64), has eaten, and did an experiment in each working hour (7: 10, 11, 13-17);
Coulomba (10-13-20) is at the bench (x 1072) at work, has eaten, and did 8
(10-12, 14-18).

## Slice 5, second part (2026-11-10): the lives in the save

The save keeps the ledger (`lives`, save version 6: each character's facts as
the planner's RON text and the hour it last planned in). Like every durable
family, `persist_lives_to_save` mirrors it once a loaded save is applied,
`restore_lives_from_save` puts it back on the tick the loaded save is applied
(and gives a life that attached before the restore its saved row, so a fresh
life is not written over the saved one), and a New Game clears it. A death
keeps it, as it keeps flags and quests.

**The clock in the save (2026-11-10, save version 7):** the save keeps the
day clock (`day_clock`: the day and whole seconds), written once an hour by
`persist_day_clock_to_save` (not each tick, so the save is not marked
changed every frame) and put back by `restore_day_clock_from_save` before the
lives are, on the tick a loaded save is applied. A session started from the
save goes on at the hour its lives planned in. Before, it went on at the
cycle's start hour with the saved facts, so a life planned in at 9:30 met an
8:00 clock. The witness below also asserts the hour: the save's clock reads
9.5, the session from it is within half an hour of it, and the control
session is at its start hour; with the restore disabled it read 8.02.

Witness (same file): Nikobolt's experiment at 9:30 is in the save; a session
started from that save builds him with it (1 experiment); a session started
without it builds him with none (the control).

## Slice 6 (2026-11-10): an edited life in the running game

The lives are pack content (`data/npc_lives.ron`), so a content reload
publishes an edited book with no Rust build. Witness
(`game/ambition_app/tests/a_life_edit_reaches_the_running_game.rs`, the
shipped visible composition in the coil lab): as shipped, Nikobolt's plan at
eleven is "a working day"; an edit moving his lunch to eleven, compiled into
a candidate pack in memory and published by `request_reload`, makes his plan
at eleven "lunch". The reload prepares the session again, so the lab and
Nikobolt are built anew from the edited pack (measured: another entity).

A life row now separates `authored` facts (the hours, read from the book
each time the body plans and never kept as its own) from `facts` (what it
starts with and changes: `ate`, `experiments`). Slice 5 needs that split: a
body rebuilt with facts restored from a ledger must not carry an hour the
book no longer states.

Saving the file in a running build that reads its content off disk asks for
the same reload: `content_watch` watches every source the pack declares, the
lives file among them (not witnessed for this family on its own).

## Review fixes (2026-11-10)

A source review of the slices above found lifecycle gaps. Fixed:

- **The second day.** `ate` and `talked` were set once and never reset, so
  from day 2 the scientists never lunched or talked again. A life now
  authors `daily` facts (`npc_lives.ron`): each takes its authored value on
  the first plan of a new day, in the live executor and for each midnight an
  offscreen catch-up crosses. The planner stays generic; the content says
  what recurs. Witness: `nikobolt_works_at_his_bench_and_lunches_at_noon`
  now lunches on day 2 too (without the reset it fails: he is still fed).
- **Verbs that did nothing.** `say`, `use` and `rest` fell into a branch
  that held the body (or did nothing) and then counted the step done.
  Now: `say <topic>` speaks the line the life authors for the topic
  (`lines`, a speech bubble over the body, the same message the ambient
  barks use) and stops the plan with `NothingToSay` when there is none; a
  verb must be one the executor carries out (`go_to`, `claim`, `release`,
  `say`) or a timed task declared in `holds` (`use`, `rest`), and any other
  stops the plan with `Unsupported` instead of passing. The schema refuses a
  domain whose verb is neither and a life with no line for a topic it says.
  Still open: `use <thing>` is a timed task at the place, not an interaction
  with a world object; NPCs have no object-use path yet. Witnesses:
  `two_scientists_take_turns_at_one_bench` (both lines heard, `talked` set),
  `npc_lives::tests` (the two refusals).
- **Work every tick for nothing.** The ledger cloned every life's facts
  each tick to compare them, and the save mirror wrote every row to RON
  each tick before comparing. Now the ledger compares in place and clones a
  row only when it changed, and the mirror serializes only on a tick the
  ledger was written or the save was applied.
- **The planner behind a contract.** The executor called `Domain::plan`
  and the actor crate depended on `ambition_htn`. Now
  `crates/ambition_life_planning` is the contract (facts, values, actions,
  steps with their reasons, an explained refusal, and `LifePlanner`: plan,
  and apply a done step's effects); `ambition_htn::Domain` implements it,
  and the executor holds `Arc<dyn LifePlanner>` per domain and depends on
  the contract only. Navigation and the verbs' execution do not change with
  the planner. A second planner (utility scoring) implements the same trait.

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
