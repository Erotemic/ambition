# Planning status

This page is a short orientation snapshot. Live execution is in
[`queue.md`](queue.md). Rulings are in
[`maintainer-decisions.md`](maintainer-decisions.md). Open choices are in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). Durable
architecture is in
[`../architecture/engine-architecture.md`](../architecture/engine-architecture.md)
and the focused owner documents.

Do not copy counts or row history into this page. A copy does not get the
corrections that the owner row gets.

## Current architecture posture

Ambition is converging on **one mechanical owner per fact, plus explicit
projections**. Bevy is the normal implementation substrate. Custom engine
mechanisms exist only for semantics Bevy does not define: rollback authority,
deterministic identity, content generations, transactional
construction/publication, session ownership, and persistence/custody.

## Converged shapes

These are settled. Code against them; do not reopen them without a new ruling.

- **One constructor, candidate publication.** Fresh session, room transition,
  same-room replay, New Game and save restore use one construction model. A new
  session or room is built as a hidden candidate, verified, and published at one
  switch. A failed candidate leaves the live world unchanged. (Consolidation
  C01, C04 and C06 are closed.)
- **One canonical `SessionRoot`.** A candidate carries `CandidateSessionRoot`
  until it is published (`Q132`).
- **Several live rooms.** Each live room is a root entity; entities carry an
  `InRoomInstance` stamp; each view draws the room it frames. Ambition has no
  production join road for a second seat yet (`Q153`). Owner:
  [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).
- **Content is data, and it reloads.** Content packs compile through
  `ambition_content_pack::compile`. A running session reads the content
  generation it was prepared against (`SessionCast` for the cast). Duplicate
  Rust content tables are migration scaffolding (`Q104`); registry changes use
  explicit lifecycle semantics (`Q110`).
- **Rollback-safe mechanical editing.** Live editors use a
  proposal/admission/publication boundary. A new editable domain extends that
  protocol; it does not add a second rollback road.
- **Durable save state rewinds.** Every writer of hashed save state runs in the
  simulation schedule. Disk I/O stays outside the simulation.
- **Rigged sprites are on** in the shipped game. Owner:
  [`engine/runtime-rigged-sprite-animation.md`](engine/runtime-rigged-sprite-animation.md).

## Active campaigns

| Campaign | Where it is tracked |
| --- | --- |
| One owner per mechanical fact | [AUTHORITY-POLISH](queue.md#authority-polish--one-owner-per-mechanical-fact-and-no-mirror-in-the-rollback-kernel) (C11) |
| Session-owned App state | C03 in [`consolidation/consolidation-plan.md`](consolidation/consolidation-plan.md) — startable |
| Composition contracts | C07 in the consolidation plan — startable |
| Safe reload across every registry | [I2/I3](queue.md#i2i3--finish-independent-content-authoring-and-safe-reload) |
| Truthful minimal engine profiles | A9 in the queue |
| Persistent world | OW cuts in [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md) |

## Ruled but not yet implemented

- `Q122`: mechanical identity fingerprints mechanical facts, not prose. The
  rollback schema fingerprint still hashes the prose `detail`.
- `Q138`: an invalidated harness must refuse or fail. The harness step does not
  yet check rollback health.

Netplay is not a goal this year. No session in this repository observes a real
peer: `SyncTestSession` is the only one constructed. A green local rollback lane
says nothing about two peers agreeing.

## Current execution

Read the rows in `queue.md` before you pick up work. In summary:

- **P0:** sync-point resimulation, peer-stable identity, the rollback mutator
  population, settings/rollback admission, throw modifiers, the CPU duel guard,
  A4 control/body execution, and authority polish.
- **P1:** per-actor gates, boss replay retraction, menu over dialogue,
  candidate generation order, content reload (I2/I3), duplicate content
  authorities, A9 profiles, A7 item occurrences, fighter attack selection, Smash
  parity, character authoring, dead-session refusal, baked sheet identity, the
  durable-horizon checksum, and test lanes.

One open P1 row is blocked on a maintainer ruling: `BAKED-SHEET-IDENTITY`, on
`Q157`.
`scripts/check_blocking_set_names_every_gate.py` keeps the queue's
`**Blocked by:**` fields and the blocking-set table in
`awaiting-maintainer-decision.md` in agreement.

If a row closes, remove it from the queue unless another open row needs a short
receipt.

## Evidence discipline

Keep source facts, inferred behavior and runtime claims apart. Do not turn a
static observation into a runtime claim. Re-run the measurement that supports a
number before you use it as a new baseline. The architecture census marks each
claim `SOURCE_CONFIRMED`, `SOURCE_INFERRED`, `DOC_CLAIM`,
`NEEDS_COMPILED_VERIFICATION` or `NEEDS_RUNTIME_VERIFICATION`. Counts are
campaign observability, not quality budgets.

## Planning control plane

Use the planning tree as [`README.md`](README.md) says:

- `queue.md` — executable work only;
- `awaiting-maintainer-decision.md` — open maintainer choices only;
- `maintainer-decisions.md` — rulings;
- focused owner documents — current authority, shape, open work and acceptance;
- `tracks.md` — standing reservoir, not active execution;
- Git history and `dev/` — investigation chronology.

Do not create archive files inside `docs/`. When a document is superseded,
delete or rewrite it.
