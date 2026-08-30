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
  not a log line. This is M5 of
  [`authored-gameplay-logic-and-orchestration.md`](authored-gameplay-logic-and-orchestration.md)
  and it is a product requirement, not polish.

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
