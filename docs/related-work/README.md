# Related work

**How other engines solve the problems Ambition is solving, with citations.**

## Why this section exists

Jon, 2026-08-07, on Ambition's shell vocabulary: *"I wonder if we should start a
related work section in the docs to document how everyone else does it, and give
us a better reference for how we compare."*

Ambition is written from first principles on purpose, and that is a strength
until it becomes an excuse. A design argued only against itself has no way to
find out that a concept it invented already has a name, or that a distinction it
collapsed is one three other engines kept. This section is the outside view.

## What belongs here

A page per QUESTION, not per engine. The useful unit is *"how is this problem
solved elsewhere"* — a page per engine would be a tour, and nobody reads a tour
while making a decision.

Each page owes:

* **The Ambition concept it is about**, named in our terms, with a pointer to
  where we implement it.
* **What each engine calls it**, and — more importantly — *whether they have the
  concept at all*. An engine that does NOT split something we split is the most
  informative row in the table.
* **Citations that were checked**, with the date. See the rule below.
* **What it changed**, if anything. A related-work page that changed no decision
  should say so; that is a finding too.

## ⛔ The citation rule

**Every claim carries a link, and the link was FETCHED, not remembered.**

This is not bureaucracy. The first draft of the vocabulary page below asserted an
Unreal URL option from memory; the Epic page it was attributed to does not
document it, and the real citation turned out to be a different page entirely
(the claim was true, the source was wrong). A confidently-wrong citation is worse
than no citation, because the next reader stops checking.

So: fetch the page, quote the line, record the URL, date the check. Mark
third-party sources as third-party — a community blog is often the only place a
sample project's internals are written down, and that is fine as long as the
reader can see what kind of source it is.

⚠ **APIs move.** A citation is a point-in-time observation, same as everything
else in this repo. Re-check before acting on a detail, and prefer claims about
CONCEPTS over claims about spellings.

## Pages

* [Shell vocabulary: provider, experience, route](shell-vocabulary-in-other-engines.md)
  — what Unreal, Unity and Godot call the things our shell calls providers,
  experiences and routes. Checked 2026-08-07.
* [Participant input, control authority, and possession](participant-input-control-and-possession.md)
  — per-user devices, contexts, possession, spatial interpretation, local-N, and
  why view ownership should remain a separate relation. Checked 2026-08-07.
* [Actions, abilities, and temporal ownership](actions-abilities-and-temporal-ownership.md)
  — Unreal GAS versus Ambition's landed action seam, and the minimum shared
  lifecycle still worth designing. Checked 2026-08-07.
* [Deterministic simulation, rollback, and replay](deterministic-simulation-rollback-and-replay.md)
  — Photon Quantum, Unity and Unreal prediction compared with Ambition's
  headless/rollback contract and scenario-tooling frontier. Checked 2026-08-07.
* [Diagnostics, causality, and frame inspection](diagnostics-causality-and-frame-inspection.md)
  — where to integrate general profilers and where semantic cross-tick
  explanation can distinguish the engine. Checked 2026-08-07.
* [Authoring, world composition, and deterministic preparation](authoring-world-composition-and-preparation.md)
  — prefabs/scenes/world layers versus `PreparedContent` and transactional
  construction. Checked 2026-08-07.

## Competitive design frontiers

The pages above are not a checklist for copying large engines. They should make
three outcomes explicit:

1. **Integrate** where Bevy or mature engine tooling already solves a general
   problem well (profiling, renderer/editor-facing visualization, asset plumbing).
2. **Compete** where a 2D platformer engine needs a complete conventional author
   experience (participant-local input, actions, content reuse, rollback proof).
3. **Differentiate** where Ambition's architecture enables a stronger contract:
   persistent participant/control authority, body-owned spatial interpretation,
   deterministic preparation, rollback outside networking, and semantic causal
   explanation.

The competitive roadmap is the binding plan. Related-work pages are evidence and
design pressure: recommendations here become architecture only when the relevant
plan/ADR adopts them.

## Highest-leverage open comparisons

| Frontier | Design still owed | Competitive/differentiating bar |
|---|---|---|
| Participant/control | action-specific context arbitration; PA5 participant-keyed routing; PA6 synchronized frame policy | Unity/Unreal-grade device/context ergonomics without conflating participant, control authority, body, spatial frame, or view |
| Action lifecycle | minimum shared temporal record beyond melee; cost/cooldown transaction timing; confirmed/speculative effects | GAS-quality lifecycle semantics without forcing every mechanic through one universal ability/effect framework |
| Rollback/replay | declarative scenarios; correction-aware effect identity; measured rollback workload gates | deterministic rollback as a normal headless/replay/debug contract, not merely a multiplayer mode |
| Diagnostics | cross-tick causal edges; corrected-history inspection; enforced per-host budgets | answer platformer-specific *why* / *why not* questions on the same facts CI and headless tests use |
| Authoring/construction | public reusable-definition shape; hot-reload transaction UX; schema evolution; dependency readiness | prefab-like authoring convenience lowered through immutable prepared content and transactional construction rather than making an editor object graph authoritative |
