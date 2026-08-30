# `docs/planning` — forward-work control plane

This directory coordinates work that is still moving. It is not the canonical
home for every durable fact about the engine, and it is not a changelog.

The planning surface has four jobs:

1. keep a capable agent doing the next highest-value work instead of stopping;
2. preserve open product and architecture intent until it is implemented or
   explicitly rejected;
3. point active work at the focused design that owns its technical decisions;
4. move settled architecture and completed execution history to more durable
   homes when they stop being planning.

## Where the open work is

[`queue.md`](queue.md) is the **live execution queue and continuation engine**.
It has no dated filename because it outlives any one run; the mechanism is
intentionally self-replenishing.

A queue with no actionable rows is not a signal to stop. It is a signal to
inspect the standing backlog, focused plans, direct maintainer observations,
and current code; write down the next highest-value work; and continue.

The queue owns **execution order**. A focused plan owns the **technical design**
of the work it names. The queue should link to that design rather than becoming
a second full specification when a focused authority already exists.

[`tracks.md`](tracks.md) is the **standing backlog and work reservoir**. It keeps
valuable work available across runs, but an item becomes immediate execution
work when the live queue selects it.

Two other files contain work an agent must not silently resolve by inference:

- [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) — explicit
  questions that need Jon's decision;
- [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
  — Jon's direct observations, which outrank inferred status.

### Queue contract

- `▢` means an actionable open queue row and nothing else.
- `✔` or another explicit closed verdict means the row is no longer work.
- **A closed row is a receipt, not a case file.** Once a row closes, compress it
  in the same commit to at most a few lines in this shape:

  > `✔ **D123 — what was wrong, in one sentence.** Fixed by `<commit>`: what the
  > fix was. Guarded by `<test or check>`. ⛔ <only a standing prohibition that
  > would otherwise be rediscovered>.`

  The evidence that justified the fix stays in the commit message and in git
  history for this file; it does not stay here. A reader who wants the
  investigation runs `git log -p`.
- **The same rule applies inside an open row.** A row that has run for days
  accumulates superseded layers, and a stale `⇒ NEXT` sentence is how a later
  session re-does landed work. Keep the current model at the top; delete the
  layers it supersedes rather than stacking a correction on them.
- The exception, and it is narrow: keep a sentence that would otherwise be
  **rediscovered at cost** — a prohibition, a measurement that was wrong in an
  instructive way, or a design refused for cause. One clause, not a section.
- Re-measure an old row against HEAD before implementing it; queue entries are
  claims about a changing repository.
- When the currently enumerated rows are exhausted, replenish the queue from
  the highest-value unresolved work and keep going.
- Preserve the queue's ability to continue work. Do not optimize it for a small
  file, a short run, or a natural stopping point.
- Keep deep architectural reasoning in the focused plan when one exists. The
  queue should carry enough context to choose and resume the work reliably.

The repository has a small mechanical guard around the live-ledger pointer and
row-state consistency because a broken pointer or contradictory row directly
breaks this continuation mechanism. That is an exceptional use of document
checking, not a general invitation to turn planning prose into source-scanned
policy.

## Read in this order

1. [`vision.md`](vision.md) — product and engine north star.
2. [`maintainer-decisions.md`](maintainer-decisions.md) — explicit maintainer
   decisions.
3. [`decision-principles.md`](decision-principles.md) — decision doctrine when
   Jon has not ruled on the question.
4. [`status.md`](status.md) — orientation to the current repository state; it is
   not an execution queue.
5. [`queue.md`](queue.md) — current execution
   order and the place an autonomous run continues from.
6. The focused engine, demo, game, or campaign document linked by the selected
   queue row.
7. [`tracks.md`](tracks.md) when replenishing the queue or surveying standing
   work.
8. [`roadmap.md`](roadmap.md) and
   [`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md)
   for longer-horizon strategy and capability programs.
9. [`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md)
   when judging whether those programs add up to a competitive 2D engine product.

## Document roles

### Live execution

- `queue.md` — one live execution order, intentionally self-replenishing.
- `tracks.md` — standing backlog and reservoir feeding the live queue.
- active campaign documents — focused implementation authority for a bounded
  architectural or product push.

### Product intent

- `game/` — **Ambition-the-game direction; Ambition is the flagship and primary product driver.**
- `demos/` — serious secondary engine customers / acceptance-game specifications. A customer may later graduate into a first-class game without displacing Ambition.
- focused engine plans — open engine capabilities and design work.
- `awaiting-maintainer-decision.md` — unresolved maintainer questions.
- `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md` — direct maintainer reports.

### Durable truth

Planning may originate a durable rule, but settled material should increasingly
live where a cold reader expects current truth:

- `docs/concepts/` for cross-cutting doctrine;
- `docs/systems/` for current subsystem behavior and contracts;
- `docs/architecture/` and `docs/adr/` for architectural boundaries and
  decisions;
- `docs/archive/` for completed campaigns, reviews, handoffs, migration
  narratives, and other evidence that remains useful historically.

A planning file can remain detailed while it owns open work. Completion is the
point where its surviving design is distilled into durable documentation and
its execution history leaves the live planning surface.

## Semantic closure before removal

Age, a dated filename, a `shelved` label, or the absence of incoming links does
not make a plan obsolete. Some intentionally deferred features exist only in a
single focused document.

Before removing or archiving a planning document, account for every substantive:

- feature request;
- architectural requirement;
- unresolved defect;
- product decision;
- unanswered question;
- implementation task; and
- important observation.

Each item needs one clear disposition:

- **completed** — HEAD implements the intended behavior or architecture;
- **superseded** — a surviving live document carries the still-relevant intent;
- **obsolete** — the premise no longer exists and no desired intent remains;
- **history** — no unresolved requirement remains, but the execution evidence is
  worth retaining outside the live planning surface.

When closure is uncertain, keep the material live until it is reconciled. The
cleanup goal is trustworthy authority, not a target file count.

## Evidence and testing doctrine

Planning should drive engine and product outcomes rather than process ceremony.
Use the strongest representation available for the invariant:

- runtime behavior -> behavioral or integration test against the real system;
- architecture/dependency boundary -> types, visibility, API shape, crate edges;
- authored-content validity -> preparation/compiler/schema validation with
  useful diagnostics;
- migration census -> a one-off measurement when useful, normally retired with
  the migration.

A completion claim should cite concrete evidence a later reader can inspect,
but durable prose does not need to be reshaped around a scanner. Source-text
checks, poison/falsification fixtures, and permanent ratchets are exceptional:
use them when they protect a concrete failure mode that cannot reasonably be
made structural or behavioral.

A straightforward assertion does not need to be deliberately broken merely to
prove that assertions can fail. Add a non-vacuity control when vacuity is a
realistic failure mode.

⛔⛔ **AND THERE IS A THRESHOLD, because "when vacuity is realistic" was being
read as "always".** Jon, 2026-08-18, verbatim: *"minimize poison tests unless you
have less than 60% certainty. if it's probably right, don't waste the cycles a
vacuous false negative is not costly."* ⇒ **if you are confident the guard bites,
write it and move on.** Poisoning a test you already believe in buys a number for
a commit message and costs a build cycle; a test that turns out vacuous is a cheap
mistake, caught the next time the code moves under it. Reserve the poison for the
cases where you genuinely cannot predict whether the assertion can fail.

## Living-plan writing

Write the current model first. Preserve durable rationale, acceptance criteria,
and genuinely open questions. When an old assumption is disproved, replace the
stale guidance instead of making every future reader replay the entire sequence
of mistakes before reaching the answer.

Execution diaries, reviewer archaeology, temporary measurements, and the story
of how a migration was discovered are useful evidence while the work is active.
Once the campaign closes, archive them or rely on git history while keeping the
surviving design concise.

One fact should have one current planning authority. `status.md`, `tracks.md`, a
focused plan, and the live queue should link to one another rather than each
maintaining independent copies of the same completion narrative.

Dated reviews are evidence, not another status hierarchy. A live review finding
must be promoted into `queue.md`, `tracks.md`, a focused plan, a maintainer
decision, or a direct-observation owner; do not maintain a parallel review-status
ledger.

## Ambiguity is part of planning

A focused plan does not need to pretend that every design answer is known.
Instead, every substantial new plan should distinguish:

- **settled direction** — decisions an implementation slice may rely on;
- **open design questions — deliberately unresolved** — choices where the right
  answer needs more evidence, prototype pressure, or maintainer judgment;
- **things we should not pre-generalize** — abstractions that require additional
  real customers before hardening.

An agent may investigate an open question when execution reaches it. It should
record the evidence and proposed answer rather than silently treating an
under-specified paragraph as doctrine.

For reusable engine domains, also state the plausible **Bevy/plugin/crate seam**:
what the domain would own, how its plugin registers itself, and whether there is
credible ecosystem value. See
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md).

## Binding spine

North star: *every upgrade a theorem, every boss a failed objective function,
every biome a mathematical world model.* **Ambition is the flagship game and
primary product driver.** Engine capabilities should make Ambition better while
remaining reusable enough that another substantial game can consume them through
supported seams rather than editing Ambition-specific engine internals.

Forward Engine 1.0 architecture is organized under
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
New capability programs feed the live queue; they do not create parallel
execution queues.

Prefer reusable composition, explicit ownership, deterministic/headless
simulation, strong public APIs, modular capability selection, useful authoring
diagnostics, low change amplification, and reasonable compile/iteration cost.
For the Godot-class 2D target, judge parity by engine capability,
expressiveness, efficiency and supported composition rather than by whether a
feature has an equivalent visual-editor panel. LLM-first semantic operation is a
primary authoring surface. Delete duplicate authority rather than preserving it
indefinitely behind compatibility paths.
