# Ambition documentation map

Use this file to choose what to read. Do not load the whole documentation tree by default.

## Start here

- [`../README.md`](../README.md) — stable project overview.
- [`../AGENTS.md`](../AGENTS.md) — short agent operating instructions.
- **[`planning/README.md`](planning/README.md) — THE master plan** (vision,
  roadmap, live work queue in [`planning/tracks.md`](planning/tracks.md),
  and a design doc per planned system). It is the primary coordination surface
  for direction and current work.
- [`adr/README.md`](adr/README.md) — durable architectural decisions.
- [`../dev/README.md`](../dev/README.md) and [`../dev/SEARCH.md`](../dev/SEARCH.md) — engineering memory and lookup protocol.

## Durable memory (what exists today)

- [`concepts/index.md`](concepts/index.md) — concepts, aliases, invariants, edit protocols, validation anchors.
- [`systems/index.md`](systems/index.md) — current subsystem docs.
- [`recipes/index.md`](recipes/index.md) — current procedures.
- [`tools/index.md`](tools/index.md) — author-time tools.
- [`mechanics/index.md`](mechanics/index.md) — mechanics expressibility and current gameplay primitives.
- [`adr/`](adr/) — decisions that supersede older plans.

These describe the EXISTING system and may lag the code; refreshing them is
a scheduled track in the plan. Where they contradict `planning/`, the plan
wins for direction and the code wins for current fact.

## Direction and planning

- [`planning/`](planning/README.md) — the master plan (see Start here).
- [`brainstorms/`](brainstorms/) — Jon's design incubation. Alive, not
  archive — and agents never write here.
- [`vision/index.md`](vision/index.md) — auxiliary vision notes
  (movement-instrument sandbox, external references). Direction itself
  lives in `planning/vision.md`.
- [`archive/README.md`](archive/README.md) — superseded notes, old
  handoffs, retired systems, historical reviews
  ([`archive/reviews/`](archive/reviews/)), and the retired
  `docs/current/` snapshots.

## Generated agent indexes

Generated indexes are optional navigation aids. Refresh them when their source
material changes or during repository-maintenance work; they are not a universal
completion gate. They do not override source files, ADRs, or concept pages. The
current archive includes:

- [`../.agent/manifest.yaml`](../.agent/manifest.yaml)
- [`../.agent/retrieval_evals.yaml`](../.agent/retrieval_evals.yaml)

## Reading rule

Read the smallest packet that answers the task:

1. `planning/README.md` → `planning/tracks.md` for tasking,
2. one concept page,
3. one focused system/tool doc or recipe,
4. relevant dev-memory search results,
5. generated indexes when locating files, symbols, or tests.

Avoid broad context dumps. They make agents slower and less precise.
