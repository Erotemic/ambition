---
status: current
last_verified: 2026-07-18
---

# Recipes

Recipes are copy-pasteable procedures for the current repository. They should
name a supported front door, state what is mutated, and finish with validation.
When a command changes, update or delete the recipe in the same patch.

## Start here

- [`fresh-agent-navigation.md`](fresh-agent-navigation.md) — localize a task with
  `.agent` without loading the repository into context.
- [`coordinator-and-worker-sessions.md`](coordinator-and-worker-sessions.md) —
  spawning and integrating subagents; ⛔ **mirror a worktree's assets or ~40 tests
  fail for reasons that are not your change.**
- [`headless-room-verification.md`](headless-room-verification.md) — prove a
  gameplay/world change through the real headless composition.
- [`ldtk-authoring.md`](ldtk-authoring.md) — safe world edits and tool-assisted
  spatial authoring.

## Content authoring

- [`adding-an-asset.md`](adding-an-asset.md) — which asset root, what path
  string loads it, and why git-ignored does not mean missing.
- [`adding-a-character.md`](adding-a-character.md)
- [`adding-a-capability.md`](adding-a-capability.md) — a custom mechanic that
  contributes behaviour, an authored schema, a semantic action, rollback state
  and causal facts without editing a central enum. `examples/capability_demo` is
  the worked example.
- [`validating-a-content-pack.md`](validating-a-content-pack.md) — the ~5 ms
  edit/validate loop, every refusal code and what it means, and how a capability
  registers its own authored schema.
- [`dialogue-authoring.md`](dialogue-authoring.md)
- [`extending-brains-and-action-sets.md`](extending-brains-and-action-sets.md)
- [`add-showcase-room.md`](add-showcase-room.md)
- [`goblin-encounter.md`](goblin-encounter.md)
- [`generated-music-workflow.md`](generated-music-workflow.md)

## Platform and diagnostics

- [`android-build.md`](android-build.md)
- [`web-build.md`](web-build.md)
- [`web-audio-manual-test.md`](web-audio-manual-test.md)
- [`profiling.md`](profiling.md)
- [`rollback-proof-mode.md`](rollback-proof-mode.md)
- [`explaining-a-tick.md`](explaining-a-tick.md) — "why did this actor change on
  this tick": turning causal recording on, what can be asked today, and how a
  capability publishes its own facts.
- [`cheapest-sufficient-check.md`](cheapest-sufficient-check.md) — the narrow
  command that settles a change, per row of what you touched, and what each row
  does NOT cover.
- [`re-measuring-a-planning-claim.md`](re-measuring-a-planning-claim.md) — seven
  ways a re-measurement lies, each learned by making the mistake and retracting
  it. ⛔ Six of the seven are an INSTRUMENT error, not a mistake about the code.
- [`checks-that-did-not-run.md`](checks-that-did-not-run.md) — its dual: the four
  questions that catch a check which is CORRECT and never executed. ⛔ nine
  members found in one gate script in one day, five of them by accident, and
  two are structurally unfixable — plus the four ways a search that finds
  nothing lies to you, and the positive control that catches all of them.
- **Testing headlessly** lives in
  [`../planning/engine/headless-verification.md`](../planning/engine/headless-verification.md)
  — `Platformer2dSimHarness::step`, the headless binaries, and the doctrine about what to
  assert. Linked rather than restated: a second page describing the same
  substrate is a second authority, and the two drift.

## Recipe quality rule

Before following an old exact path, localize the current owner:

```bash
python scripts/agent_query.py "<task words>"
python scripts/agent_query.py tests "<expected behavior>"
```

Use `./run_tests.sh` as the test front door. Use source/CLI `--help` as the
command authority when prose and code disagree.
