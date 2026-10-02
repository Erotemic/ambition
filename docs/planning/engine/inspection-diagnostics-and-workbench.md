# Inspection, diagnostics and workbench — Engine 1.0 program

**State:** OPEN. Machine-readable inspection is the priority. A monolithic
graphical editor is not.

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

## Authority is distributed; discovery is composed

- **Rejected:** a low-level generic runtime owns an authoritative census of
  every gameplay domain and must be edited whenever a new domain participates
  ([`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md)).
- **Required:** each domain owns its semantics and contributes descriptors to a
  composed, read-only discovery index. Nothing there is authoritative; it is
  derived.

Do not sacrifice discoverability to avoid central authority. An agent that must
read the implementation to learn the vocabulary is the failure this program
exists to prevent. This applies to authored rule vocabulary, schemas,
capabilities, animation bindings, semantic commands, diagnostics and tool
discovery.

## Current surfaces

### `[census]` lines

"Census" here means derived, read-only diagnostic emission, not the rejected
authoritative census above. Enable with `AMBITION_PROFILE_CENSUS=1`; the lines
go to stderr. List the surfaces with `git grep -nE '\[census\] [a-z_]+' -- '*.rs'`
(exclude comment lines). They cover the machine (entities, schedules, draws,
render passes, phase costs) and the world:

- `[census] rooms` prints every session root, its room set, each live room
  (definition, instance `#n`, holders) and the live crossing
  (`crossing=none` when quiet). It prints a row for a world with no session and
  for every session root, so an absent state is visible.
- `[census] verdicts` is a projection of `AuthoredVerdictLog`: counts, the set
  of blocked askers, and the last refusal. `log=absent` means the ring is not
  installed, not that nothing was asked.
- `[census] phases_trust` states the conditions under which this run's phase
  split may be read.

Tooling entry points: `scripts/agent_query.py`, `scripts/ecs_inventory.py`,
`scripts/non_ecs_inventory.py`, `scripts/core_import_census.py`. A future GUI
workbench consumes these surfaces.

### Structured why-not (M5)

An unsatisfied condition reports the term that blocked it, the object it names
and that object's current state. This is a product requirement.
[`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)
owns the authored-logic contract; the requirement is stated here.
`scripts/check_absence_contracts.py` enforces it.

- Vocabulary: `ConditionOutcome::NotSatisfied(WhyNot { term, subject, observed })`.
  Every production evaluator states one. `from_bool_unexplained` is the fixture
  arm only.
- A standing lock wall publishes its verdict on `GatedLockWallVerdicts`
  (`why_standing(wall)`).
- `AuthoredVerdictLog` is a bounded ring recorded at the one door of
  `ConditionCatalog::evaluate` and `CommandCatalog::run`, so it also sees the
  catalog's own refusals (a misspelled id). It holds
  `AuthoredVerdict::{Asked, Ran}`. Lookups: `recent()`,
  `latest_for(id, args)`, `why_not_for(id, args)`, `latest_run(id, args)`,
  `refusal_of(id, args)`. An invocation is `(id, args)`, not `id`.
  `latest_for_id` and `latest_run_of_id` are browsing helpers only.
- Every invocation carries an `AuthoredAsk { kind, subject }`
  (`lock_wall:<id>`, `dialogue:<node>`, `switch:<id>`). The kind is an open
  `&'static str`, not a closed enum.
- **Opt-in by presence.** A composition that wants the log inserts the
  resource. The composed host does not.
- **Not simulation state.** Nothing in the simulation reads it, and it is not
  registered for rollback. Condition order in the ring is not deterministic
  (conditions evaluate from `&World`, possibly in parallel). Command order is
  (one dispatcher with `&mut World`). Assert on what is in the log, not on the
  order of entries from different systems.
- **Rollback identity.** Each entry carries
  `VerdictStamp { simulation: Option<(session, frame)>, confirmed }`.
  `begin_pass(session, frame)` drops what an earlier pass over that frame
  recorded, so a resimulated frame replaces its whole batch; `record` always
  appends. `confirm_through(session, frame)` marks settled frames.
  `AuthoredVerdictTimelinePlugin`
  (`crates/ambition_platformer2d_runtime/src/authored_verdict_timeline.rs`)
  runs both before `GameplaySimulationRoot` under
  `resource_exists::<ConfirmedFrameBoundary>`. An unstamped entry is never
  cleared.

A diagnostic kept out of rollback state is not independent of rollback. "Out
of rollback" decides what is rewound; the timeline stamp decides what the
record means.

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
- structured why-not explanation (M5, above).

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
