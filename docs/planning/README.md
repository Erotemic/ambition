# docs/planning — THE single source of truth for where we're going

**Consolidated 2026-07-05 (fable).** This directory is the master plan:
the vision, the roadmap, the live work queue, and the design doc for
every planned system. Historical reviews/execution records live in
`docs/archive/reviews/`; what exists TODAY is described (sometimes
stalely — refreshing them is a scheduled track) in `docs/concepts|
systems|mechanics`; `docs/brainstorms/` is Jon's — **agents never write
there**. If a doc outside this directory contradicts one inside it, this
directory wins.

## Read in this order

1. **[`vision.md`](vision.md)** — what we're building and the executor
   rules (grades, the no-deviation rule). Read every session.
2. **[`decision-principles.md`](decision-principles.md)** — Jon's own
   criteria for autonomous choices.
3. **[`tracks.md`](tracks.md)** — the live queue + execution log. Find
   your task here; append your results here.
4. **[`roadmap.md`](roadmap.md)** — phases, the M/U registers, Jon's
   open questions.
5. Then ONLY the engine/demo doc your task touches.

**Fable availability ENDED (2026-07-06 night; the last window executed
E4+E5 and ruled every open design question — ZERO live `QUESTION FOR
FABLE` markers remain).** The historical convention: design decisions
were tagged `QUESTION FOR FABLE [tag]` vs `OPUS-SAFE` (mechanical,
execute freely) — convention in
[`decomposition.md`](engine/decomposition.md) preamble. The consolidated
**LAST-CHANCE FABLE QUESTION REGISTER** (now ✅ ALL RULED; a new
ambiguity follows the post-fable decision-brief protocol) is at the top
of [`tracks.md`](tracks.md). If a
`QUESTION FOR FABLE` outlives fable, opus breaks it down carefully against
the nearest ruling — it never invents a contradicting doctrine.

## The map

- **`engine/`** — design docs, one per planned system:
  [`architecture.md`](engine/architecture.md) (the target crate stack) ·
  [`decomposition.md`](engine/decomposition.md) (**the highest-priority
  track**: the monolith teardown playbook) ·
  [`collision-and-ccd.md`](engine/collision-and-ccd.md) (the sweep law,
  OOB kill, non-axis-aligned geometry, moving portals) ·
  [`combat-model.md`](engine/combat-model.md) (the full smash stack) ·
  [`netcode.md`](engine/netcode.md) (determinism → local-N → rollback) ·
  [`fighter-brain.md`](engine/fighter-brain.md) (the no-cheat level-9
  CPU) · [`boss-design.md`](engine/boss-design.md) (the measured
  fight-quality pipeline) ·
  [`falling-sand.md`](engine/falling-sand.md) · plus the standing
  manifestos: [`spatial-model.md`](engine/spatial-model.md),
  [`frame-awareness.md`](engine/frame-awareness.md),
  [`slower-light.md`](engine/slower-light.md),
  [`unified-actors.md`](engine/unified-actors.md),
  [`headless-verification.md`](engine/headless-verification.md), and the
  sprite/boss pipeline docs.
- **`demos/`** — the acceptance games (doctrine in
  [`demos/README.md`](demos/README.md); Sanic, Super Mary-O, Super Smash
  Siblings, Hollow Lite written in stone).
- **`game/`** — Ambition-the-game: [`game/vision.md`](game/vision.md)
  (story spine + pillars), [`game/bosses.md`](game/bosses.md),
  [`game/characters.md`](game/characters.md),
  [`game/ambition.md`](game/ambition.md) (how the game sits on the
  engine + hosts the demos).

## The living-plan discipline (BINDING on every executor)

These docs are the tasking surface for "implement the plan in
docs/planning" — they must NEVER be stale or read as stale:

1. **Same-commit updates.** A commit that completes, partially completes,
   or invalidates a planned slice updates the relevant doc IN THAT COMMIT:
   flip the status in [`tracks.md`](tracks.md), mark the slice DONE (one
   line + commit hash), and correct any design text the code proved wrong.
2. **Prune, don't accrete.** DONE detail gets compressed to a one-line
   record after it's a session old; superseded design text moves to
   `docs/archive/planning-superseded/` with a pointer, never left inline
   with a warning banner. A plan is a plan, not a changelog — the
   execution log at the bottom of `tracks.md` is the only append-only
   section.
3. **No stale banners.** If you find text that contradicts the code,
   fix the doc (or archive it) in your current commit — "⚠ stale" flags
   are a 24-hour bridge at most, never a resting state.
4. **New work gets planned first.** Development that isn't covered by a
   doc here gets a slice added (with grade + exit checks) before or with
   its first commit — the plan stays the superset of the work.
5. **Drift is a finding.** If the code no longer matches a design sketch
   or the ledger, update the sketch/ledger in the same commit and note
   the drift in the execution log — the next agent must be able to trust
   every table here without re-verifying.

## The spine (unchanged, binding)

North star: *every upgrade a theorem, every boss a failed objective
function, every biome a math world model.* Engine-first: the game is the
first content crate. The oracle: *could another platformer be built by
ADDING a content crate without editing core?* Elegance is the objective
function; behavior is not sacred pre-release; verify against the real
headless sim; feel ships BLIND; delete, don't bridge.
