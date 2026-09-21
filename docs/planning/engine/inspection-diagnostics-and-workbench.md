# Inspection, diagnostics and workbench — Engine 1.0 program

**State:** OPEN — machine-readable inspection is the priority; a monolithic graphical editor is not.

## Goal

Make the engine explain itself well enough that an LLM agent or human developer
can understand current runtime/content state without repository archaeology or a
collection of unrelated debug hotkeys.

The first-class product is **structured introspection**. A future GUI workbench
may consume the same APIs.

This is part of the engine's competitive surface. A graphical inspector is one
way to make state discoverable; Ambition's primary requirement is stronger: the
same facts must be queryable, diffable and explainable to tools and LLM agents
without screen-scraping a GUI. The structured surface should also be useful to a
human debugger and may power optional visual views later.

See [`godot-class-2d-capability.md`](godot-class-2d-capability.md).

## Questions the engine should answer

- What authored/prepared definition produced this entity/body/item?
- Who controls this body and which view observes it?
- Why does this body have this capability?
- Where is this persistent actor/item and why does it exist?
- What room/region owns this instance?
- What rollback/save state participates?
- Why was this authored reference rejected?
- What facts did this character observe and why did it choose this action?
- What happened during these simulation ticks?
- What conditions and commands exist, what are their schemas, and which domain
  owns each one?
- **Why did this rule not fire?**

## ⭐ Authority is distributed; discovery is composed

This distinction is load-bearing and has been misread before, so state it here:

- ⛔ **bad:** a low-level generic runtime owns an *authoritative* census of every
  gameplay domain and must be edited whenever a new domain participates. That is
  what [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)
  rejects, and it remains rejected.
- ⭐ **good:** each domain owns its own semantics and **contributes descriptors to
  a composed, read-only discovery index.** Nothing is authoritative there; it is
  derived.

### ⚠ And the word "census" now means two different things — say which you mean

⛔ **The `⛔ bad` bullet above is about a runtime OWNING an authoritative census
of every gameplay domain. It is NOT about the `[census]` diagnostic lines, and a
reader who greps this page for "census" finds only the prohibition.** Measured at
`768c67c6e` and re-measured 2026-09-20, the engine's SOURCE can emit
**26 distinct `[census]` surfaces** — `grep -rnE '\[census\] [a-z_]+'` over
tracked `.rs`, with comment lines excluded, because a naive grep also finds
`perception` in a test comment and reports 27:

```text
assets  camera  churn  conditions  config  draws  ecs  frame  ggrs_driver
membership  owners  owners_in  phases  phases_cpu  phases_trust  phases_warning
populations  portal  render_pass  render_pass_summary  render_targets  rooms
schedules  sim_phases  views  visual_quality
```

⛔⛤ **AND UNTIL 2026-09-20 EVERY ONE OF THEM DESCRIBED THE MACHINE, NOT THE
WORLD.** Entities, archetypes, schedules, draw calls, render passes, phase
costs — and no answer to *"where am I"*. A room transition that stalled,
committed into the wrong room, or opened a transaction nobody closed was
diagnosable only with a debugger or by reading four files across three crates.
`[census] rooms` is the missing one:

```text
[census] rooms t=1.500 sessions=1 [scope=0 rooms=72 active=blink_run[7]
  start=blink_run[7] live=#0 biome=lab] crossing=none
```

⚠ That row is MEASURED, not illustrative — it is what
`the_room_census_names_the_room_the_session_is_actually_in` emits from the
composed `ambition_app` host. ⛔ **IT SAID `scope=?` HERE UNTIL 2026-09-20**,
copied from a version of the census that asked for the scope as a sibling
component of `SessionRoot` when the scope lives inside it; the page had
faithfully reproduced a defect.

⭐ **AND `live=` IS THE ONE FIELD THAT IS NOT A FACT ABOUT THE DEFINITION.**
`active=blink_run[7]` reads the same on the way out of a room and on the way
back in; `LiveRoomInstance` is minted by the one road that seats a session in
a published room, so it does not. A root carrying no instance prints `live=?`,
because a partial composition and a session in its activation room are
different worlds and `#0` is the second one.

⭐ **IT PRINTS THE INDEX BESIDE THE AUTHORED ID ON PURPOSE.** Today *"which
room is live"* is a private `usize` index into a list of DEFINITIONS — which
is exactly the conflation OW1 on
[`open-world-runtime-and-residency.md`](open-world-runtime-and-residency.md)
has to unpick. Printing both is the cheapest way to watch the day they stop
corresponding.

⚠ Three things it does rather than the obvious alternative, each because the
alternative hides a state somebody is looking for: it prints a row for a world
with NO session (a `Single`-based system silently does not run, which reads as
the census being off); it prints EVERY session root rather than the first
(two of them is where OW1 is heading); and it prints `crossing=none` rather
than omitting the field (a stalled crossing and a quiet world must not produce
the same text).

Those are DERIVED, read-only emission — the `⭐ good` half of the distinction
above, not the forbidden half. They are the largest existing implementation of
this program's "structured introspection is the first-class product", and this
page did not name a single one. Beside them sit four tooling entry points:
`scripts/agent_query.py`, `scripts/ecs_inventory.py`,
`scripts/non_ecs_inventory.py`, `scripts/core_import_census.py`.

⭐ **One of them deserves special mention because it reports its own
trustworthiness**, which is exactly what a self-explaining engine should do:
`[census] phases_trust` prints `trustworthy=no_render_backend … Phase splits from
this run are usable` — the instrument stating the conditions under which its own
numbers may be read, rather than leaving the reader to infer them.

⇒ A future GUI workbench consuming "the same APIs" should be understood to mean
these, plus whatever replaces the line-oriented format.

⛔⛔ **do not sacrifice discoverability in the name of avoiding central
authority.** LLM-native engine development requires exceptional discoverability —
an agent that must read the implementation to learn the vocabulary is the failure
this program exists to prevent. This applies to authored rule vocabulary,
schemas, capabilities, animation bindings, semantic commands, diagnostics and LLM
tool discovery alike.

## Program areas

- structured entity/domain inspection;
- preparation/provenance queries;
- world/item/actor accounting audits;
- pause/step/headless capture where useful;
- trace/replay/rollback inspection;
- collision/navigation/world-residency visualization data;
- profiler/compile/runtime measurements surfaced through stable reports;
- concise agent review products;
- public/project-level capability inspection: what is installed, what depends on
  what, what target/profile is active, and which provider owns a vocabulary;
- performance-budget reports that distinguish simulation CPU, render/GPU, asset
  materialization/residency, build/test cost and target-profile configuration;
- **structured "why not" explanation** — an unsatisfied condition should report
  the term that blocked it, the object it names and that object's current state,
  not a log line. **THIS BULLET IS `M5`** — the label eight places in the code
  and this page use for it. ⚠ It said *"this is M5 of
  [`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)"*
  until 2026-09-21, and that page labels its work `O1`–`O4` and has no `M5`:
  the link resolved, the label did not, and a reader following it to find the
  requirement found four other ones. That page owns the authored-logic
  CONTRACT this requirement is about; the requirement itself is here, which is
  where it has always been written out. It is a product requirement, not
  polish. ✔ THE VOCABULARY LANDED
  2026-09-02: `ConditionOutcome::NotSatisfied(WhyNot { term, subject, observed })`
  — every production evaluator states one (`world.flag_set`, `inventory.holds`,
  `custody.is_held`), `from_bool_unexplained` is the grep-able fixture arm, a
  standing lock wall publishes its verdict on `GatedLockWallVerdicts` (derived,
  keyed by wall id; `why_standing(wall)`), and the dialogue verb logs the
  structure at debug. ✔ **AND THE QUERYABLE SURFACE LANDED 2026-09-20:**
  `AuthoredVerdictLog`, a bounded ring recorded by `ConditionCatalog::evaluate`
  and `CommandCatalog::run` — `recent()`, `latest_for(id, args)`,
  `why_not_for(id, args)`, `latest_run(id, args)`, `refusal_of(id, args)`.
  Every `no` in the engine now survives the tick it was built on, not only the
  walls'. ⚠ It was named `ConditionVerdictLog` with id-only lookups on the day
  it landed; both changed within a day and this sentence is what a reader of
  the old names is looking for.

  ⛔⛤ **RECORDED AT THE CATALOG, NOT AT THE CALLERS**, for the reason the
  catalog already gives about arity checking: *"an evaluator that had to
  validate its own arguments would be fifty domains each writing the same four
  lines."* Twelve call sites remembering to log is twelve chances to forget,
  and the ones that forgot would be invisible. The one door is also the only
  place that sees the catalog's OWN refusals — a misspelled id reaches no
  evaluator, and a misspelled id is what an agent debugging authored content
  has just typed.

  ⚠ **ABSENT BY DEFAULT, AND ABSENCE IS THE OFF SWITCH.** No env var and no
  feature flag: a composition that wants the log inserts the resource, and one
  that does not pays a resource lookup per evaluation. The composed host does
  not install it, and the witness asserts that before installing its own —
  otherwise *"the log has my answer"* would be true of a world that never
  recorded anything.

  ⛔ **IT IS NOT SIMULATION STATE AND IT IS NOT REGISTERED FOR ROLLBACK.**
  Nothing in the simulation may read it, or a rule would branch on whether a
  diagnostic is installed; and its ORDER is not deterministic, because
  conditions evaluate from `&World` and two systems may ask in parallel. Assert
  on WHAT is in the log, never on the order of two entries from different
  systems.

  ✔ **AND THE VERBS JOINED THE QUESTIONS IN ONE RING, 2026-09-20.**
  `AuthoredVerdictLog` holds `AuthoredVerdict::{Asked, Ran}` and
  `CommandCatalog::run` records at its own one door, with `latest_run` and
  `refusal_of` beside `latest_for` and `why_not_for`. One stream rather than
  two, because *"the door did not open when I pressed it"* has two shapes
  needing different repairs — a condition answered no and the verb never ran,
  or the condition passed and the verb refused for a reason of its own — and
  the join between them IS the diagnosis.

  ⚠ The command half's ORDER is trustworthy where the condition half's is
  not: commands run through one dispatcher holding `&mut World`, so they
  serialise. A test may rely on the order of two entries it issued itself from
  one thread, and on nothing else.

  ⛔⛤ **AN INVOCATION IS `(id, args)`, NOT `id` — CORRECTED 2026-09-20, ONE DAY
  AFTER THE RING LANDED.** `world.flag_set` is one id with as many subjects as
  the game has flags and `inventory.holds` one with as many as it has items,
  so a tick that asks about two doors is ordinary:

  ```text
  world.flag_set("door_A") -> no
  world.flag_set("door_B") -> no
  ```

  An id-keyed lookup then hands somebody investigating door A the reason door
  B is shut — in the same units, in the same words, with nothing marking it.
  Lookups take the arguments; `latest_for_id` and `latest_run_of_id` survive as
  browsing helpers and are named so nobody reaches for them by accident.
  ⇒ **A DERIVED SURFACE CAN BE WRONG IN THE SHAPE OF ITS KEY, AND THAT IS
  INVISIBLE TO A TEST THAT USES THE SAME KEY.**

  ⛔⛤ **AND A DIAGNOSTIC OUTSIDE ROLLBACK STILL NEEDS ROLLBACK IDENTITY.**
  Conditions are evaluated inside the simulation schedule, so a host that
  re-simulates a mispredicted frame answers the same question twice. Without a
  stamp the ring holds a speculative `no` beside its corrected `yes` and *"this
  rule oscillated"* reads exactly like *"the first prediction was rolled back
  and never became history"*. `VerdictStamp { simulation: Option<(session,
  frame)>, confirmed }` is filled from `ConfirmedFrameBoundary`. ⚠ No planning
  page owns `GameplayTraceBuffer`, which the first version of this rule was
  borrowed from, which is why this cites the source
  (`crates/ambition_gameplay_trace/src/buffer.rs`): the obvious-looking
  `runtime-frame-history.md` is a GENERATED perf table and resolves as a link
  while answering a different question.

  ⛔⛤ **AND BORROWING THAT KEY WAS THE MISTAKE — REVIEWED AND REPAIRED
  2026-09-20.** `record` matched `(id, args, frame)` and overwrote, which is
  right for a buffer holding ONE observation per `(session, frame)` and wrong
  for a stream holding arbitrarily many. The engine runs
  `world.set_flag("x")` twice from one command buffer and two authored sources
  can ask one condition with one argument list in one frame; the old rule read
  the second as a rollback correction of the first, so the log claimed one
  thing happened where two did. ⇒ **THE UNIT OF REPLACEMENT IS THE FRAME'S
  WHOLE BATCH.** `begin_pass(session, frame)` drops what a previous pass over
  that frame recorded and lets the corrected pass refill; `record` always
  appends. An execution-order ordinal would have invented a correspondence the
  passes do not have, and conditions evaluate from `&World` — two systems may
  ask in parallel — so it would not have been stable either. An UNSTAMPED
  verdict is never cleared: an absent boundary means no rollback host, so it
  happened once.

  ⛔⛤ **AND THE CONFIRMATION SIDE WAS WIRED TO NOTHING — SAME REVIEW.**
  `confirm_through(session, frame)` re-stamps what the host later settles,
  keyed on the session too because a generation bump names a timeline that no
  longer exists — and its only caller was its own unit test, so in a real host
  `confirmed` never became true and every settled historical event went on
  reading as a guess. `AuthoredVerdictTimelinePlugin`
  (`crates/ambition_platformer2d_runtime/src/authored_verdict_timeline.rs`)
  runs both jobs before `GameplaySimulationRoot` on every simulated frame,
  under `resource_exists::<ConfirmedFrameBoundary>`, and no-ops when the
  opt-in ring is absent. ⚠ The arms drive the COMPOSED schedule rather than
  calling the two methods, because *"wired to nothing"* is exactly what a
  direct call cannot catch.

  ⚠ **THE GENERAL RULE THIS LEAVES:** a diagnostic kept out of rollback state
  is not thereby independent of rollback. Out of rollback is about what gets
  REWOUND; the timeline identity is about what the record MEANS.

  ⭐⛤ **AUTHORED SOURCE CONTEXT LANDED 2026-09-21, AND IT IS WHAT MAKES THE
  STREAM READABLE.** Every invocation carries an `AuthoredAsk { kind, subject }`
  — `lock_wall:alice_private_return_lock`, `dialogue:<node>`,
  `switch:<activation id>` — and the source LEADS the rendered entry, because a
  reader scanning the ring is looking for one authored interaction's entries
  among everybody else's. Required at the seam rather than optional: a
  `None` would be a silent default and a caller that forgot would be invisible.
  ⚠ The kind is an open `&'static str`, like a `ConditionId`'s domain — a
  closed enum would be the central registry this contract exists to avoid.
  ⚠ **IT DOES NOT MAKE AN INVOCATION UNIQUE.** One wall asking one question
  twice in one frame is two entries with one source, which is honest; what it
  adds is that a DIFFERENT source's identical question is no longer
  indistinguishable from a repeat.

  ⚠ Still open beyond this: a way to read the log out of a RUNNING process
  rather than out of a test.

## Candidate crate / Bevy ecosystem value

A generic inspection registry/protocol may become a reusable Bevy plugin if it
can introspect domain-provided views without depending on Ambition content.
Avoid one reflection-heavy god inspector that requires every internal type to be
public.

## Open design questions — deliberately unresolved

- Reflection registry, explicit typed inspectors, or a hybrid?
- In-process query API versus trace/report artifacts?
- How much historical state should be retained by default?
- What is safe/cheap enough for shipping builds?
- How do we expose deterministic state without exposing private implementation
  topology as public SDK?
- Which inspection pieces are generic enough to become independently consumable Bevy crates?

## Engine 1.0 acceptance

A competitive inspection surface should let a capable agent diagnose a failed
representative gameplay/content/build task without reading private implementation
modules first. At minimum it should be possible to obtain structured answers for:

1. installed capabilities/providers and their declared dependencies;
2. authored/prepared provenance and unresolved references;
3. live semantic entity/session/participant/view state;
4. action/rule/AI cause and why-not evidence;
5. rollback/reconstitution participation where relevant;
6. target/profile build or preparation failure;
7. representative runtime/build performance attribution.

A GUI workbench may visualize these queries. It is not required for the queries to
exist.

## Expose accepted contracts, not a mutable service locator

The [agent authoring protocol](authoring-and-tools.md) needs read-only discovery
of installed capabilities, technique/schema support, source provenance, prepared
revision, active profile and admission diagnostics. Derive this surface from the
same declarations and validation used by preparation. It must not become a second
catalog that can claim support for a handler that was never installed.

For a failed operation report the authority, subject/scope, source field, rejected
precondition and lifecycle stage. A plan should identify the base revision and
its required capabilities; a workbench must not conceal stale-plan conflicts.
Do not expose arbitrary Bevy World mutation or an unbounded query/callback bus as
the machine-authoring API. Existing rich visual inspectors can consume these same
facts without owning simulation or construction policy.

The [moveset observatory](../moveset-inspector.md) remains the focused owner for
combat scenario/take comparison and M3 art/geometry agreement. Its diagnostic
frontends must consume real runtime contacts and prepared content, not a second
combat model. A2/F2/F3 regression scenarios are useful customers of that surface,
not a reason to require a graphical inspector for every headless contact test.

## Iteration inspection is a protocol consumer

Use the generation/reload status result rather than re-derive readiness from file
mtime, a cache entry or one domain registry. Expose the dependency invalidation
reason, candidate/base/profile identity, phase, reconstruction policy and actual
observed generation. Distinguish submitted domain requests from applied/rejected
outcomes by occurrence. Expose owner-scoped residency reasons and state lifetime.

These are projections of the owners in [generation/reload](content-generation-and-reload.md)
and [domain contracts](extension-domain-contracts.md), not a second mutable catalog.
A GUI is optional; machine-readable queries and complete errors are not.
