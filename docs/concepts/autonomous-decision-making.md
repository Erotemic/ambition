---
id: autonomous-decision-making
aliases:
  - autonomous agent decisions
  - design fork policy
  - Jon-style decisions
last_verified: 2026-06-17
---

# Autonomous decision-making

When you are operating autonomously and hit a fork — an architecture choice, a
refactor shape, "which of two designs" — **make the choice Jon would most likely
make** using the criteria below, and **act**. Do not stall, and do not bounce the
decision back to the user when the decision is yours to make.

This is the procedure Jon uses himself. Internalize it so you can act with the
same judgment he would.

## First: is this decision actually yours?

Most architecture and implementation forks are **yours** — decide them with the
criteria below and record the reasoning in the commit message. Reserve a question
for the genuinely user-owned decisions:

- **Product / scope / priority** — what to build, what it should do, what's worth
  the time. (Architecture *in service of* an agreed goal is yours.)
- **Irreversible or outward-facing** acts — deleting something you didn't create
  and can't regenerate, publishing, anything hard to undo.
- A **true ambiguity in intent** that the code, the docs, and these criteria
  cannot resolve, where guessing wrong is expensive.

If you are mid-way through a timed or autonomous run, the bar for stopping to ask
is higher still: **infer what Jon would want from elegance, efficiency, and the
end-state, and keep going.** Work around blockers rather than halting. A
plausible, reversible decision made now beats a perfect decision made after a
stall. "I wasn't sure" is not a reason to stop when the criteria point somewhere.

## Get the shape right first; fix what's wrong after

When the task is to improve the ARCHITECTURE — break up the monolith, move a layer
up or down, invert a dependency, data-drive a named enum — your job is to get the
code into the **right shape**. Do that first. Find and fix what's wrong **after**.

- **The only hard gate is that it compiles and the workspace builds.** NOT
  replay-bit-identical. NOT "no behavior change." A bold move that leaves a
  temporary mess is expected — keep going and resolve it. Do **not** retreat to a
  smaller, "safe" move because the big one is messy.
- **A better architecture usually plays BETTER, not just reads better.** Cleaner
  state ownership, a removed special-case, a unified code path — these change
  gameplay, and the change is usually an *improvement*. "Different from before" is
  not "worse." Assume the cleaner shape is the better-feeling shape until proven
  otherwise; we tune the feel afterward.
- **Actively pursue gameplay IMPROVEMENTS — the current feel is the baseline to
  BEAT, not to preserve.** The game is not good yet; lots of it could feel much
  better (tighter control, clearer feedback, fewer special-cases, snappier
  responses, better-tuned numbers). The job is not only to make the code cleaner —
  it is to make the *game* better. When a change is a chance to improve the feel,
  take it; do not protect the current behavior, improve on it. A refactor that also
  makes the game play better is the ideal outcome, not a risk to be managed.
- **Replay-bit-identical is a verification TOOL, not a gate.** Use it to confirm a
  change you *intended* to be behavior-neutral actually was. It is never a reason
  to avoid, shrink, or defer a structural change. When a refactor changes behavior,
  that is expected — let replay diverge.
- **When given a duration, use all of it.** Do not stop early. Do not hand back at
  the first hard problem. Do NOT declare the work "exhausted," "entangled,"
  "comprehensive," "diminishing returns," "supervised-tier," or "needs a careful /
  supervised pass" to dodge the harder, higher-impact moves — *that evasion* is
  the failure, not a behavior change. When the easy wins are gone, it's time for the
  structural ones, not time to stop. In a REFACTOR run the evasion wears a disguise:
  extracting a test module, splitting out the easy half of a god-file and leaving
  the tangled core, "safe-subset lifts," renames, and reverting a file the moment it
  throws a compile error. Those shuffle line counts without breaking the monolith. A
  file is broken only when its HARD core is decomposed; the file that fights you IS
  the task. A mid-refactor compile error is the middle of the work — fix it forward,
  never `git checkout` to escape it; revert only a wrong APPROACH, then re-attack the
  same target a different way.
- **A commit IS your checkpoint — not a stopping point.** Land each coherent step
  as a commit (that's how you save progress and stay safe to roll back), then
  *immediately keep moving* to the next move. "Checkpoint" never means "hand back to
  the user"; it means "commit and continue." The only reasons to actually stop are:
  the clock you were given ran out, you are truly blocked on something only the user
  can unblock, or the user interrupts.
- **We can always fix things after. We make no progress if every move is held
  hostage to bit-identical output.**

## The procedure

1. Confirm the decision is yours (above).
2. Generate the real candidate solutions — including the one that requires a
   **refactor** to become possible.
3. Score them against the high-weight criteria below.
4. If a refactor lets a candidate score better, **do the refactor** — ease of
   implementation barely counts (see low-weight).
5. Pick the candidate that best serves the end-state, with a **narrow validation
   path** that proves the important part.
6. Act. State the reasoning in the commit message (per AGENTS.md commit rules).

## High-weight criteria

Prefer the solution that is more **elegant**. In this project, "elegant" means the
solution composes cleanly, has an **obvious single source of truth**, follows
existing seams, and does not require callers to remember hidden ordering rules or
workaround behavior.

Prefer the solution that best respects the project's **layer boundaries**:

- Rust is for behavior.
- RON is for content.
- LDtk is for space.
- **Machinery must not import named game content.** (Enforced by
  `architecture_boundaries`; see the three-layer split in AGENTS.md.)

The end-state these boundaries serve — the **oracle** for any structural choice:
_could a different platformer be built by ADDING a content crate without editing
core?_ Choices that move toward "yes" are choices Jon wants. They serve his four
standing goals: **(1)** incremental compile time, **(2)** agent-navigability,
**(3)** idiomatic Bevy plugins, **(4)** audit-grade reuse.

Prefer the solution that is more **runtime efficient**, especially in hot paths or
repeated simulation work. Keep hot paths allocation-free and avoid repeated
runtime work — while **not** over-optimizing cold authoring paths.

Prefer the solution that is more **maintainable**: easy to understand, easy to
modify, hard to accidentally misuse.

Prefer the solution that is **concise**. Shorter, simpler solutions are better
when they preserve clarity and correctness. **Any reduction in line count, in any
crate, toward a more elegant system is a win.**

Prefer the solution that **minimizes confusion for a new developer**. Ownership,
data flow, and intent should be apparent from the code structure.

Prefer the solution that **avoids parallel paths, compatibility shims, and
duplicate mechanisms**. This project is pre-release and nothing depends on it, so
**direct, single-commit replacement is usually better** than preserving an old
path when the replacement makes the architecture simpler. (AIs tend to over-value
backwards compatibility here — resist that.)

Prefer the solution that creates a **stable extension seam** instead of adding
another special-case branch to a core system. A narrow, specific type beats a
wide generic one; add knobs only when a concrete use case lands. Do not open wide
tech-debt surfaces.

Prefer the solution that does not depend on incidental ordering — system-schedule order, query-iteration order, frame timing. Make the invariant explicit (consume the input, sort by a stable key) rather than relying on an order that happens to hold today.

Prefer the solution that fixes the whole class at one seam over patching each symptom. If two things do the same job by different code, that divergence is itself a smell — unify them onto one path; "why is this case special?" is a question to chase, not accept.


## Important considerations

If the code can be **refactored** so the solution better satisfies the high-weight
criteria, do the refactor. The refactor is not a distraction from the task; it is
often the task.

Look for ways to **unify** the change with an existing system. Unification is
desirable when it does not over-scope the system: if a specific case can become an
instance of a general case without a major runtime or clarity cost, that is
usually worth doing. (But do not generalize speculatively — unify what exists, not
what might.)

**Game behavior is allowed to change — and during architecture work it usually
will, often for the better.** The game is pre-release and full of buggy,
provisional, inconsistent behavior; a cleaner architecture (clearer ownership, a
removed special-case, a unified path) routinely produces *different* behavior, and
different is usually an improvement. Do NOT preserve current behavior for its own
sake — preserve it only where it is known-intentional and relied upon, and never
let "this might change behavior" shrink or block a structural move.

Replay-bit-identical and the scripted-gameplay tests are **verification tools, not
gates.** When you *intend* a change to be behavior-neutral, replay confirms it was.
When you intend to change behavior, let replay diverge — that is the expected,
correct outcome; pin the *new* intended behavior with a focused test if it's worth
locking, and move on. Never reach for "keep replay identical" as a reason to avoid
the move that actually improves the shape. (See "Get the shape right first" above —
this is the same rule, restated where it bites.)

Prefer a **narrow validation path**. A good architecture change usually has a
focused test, check, or tool command that proves the important part. If one
doesn't exist, adding it is part of the change.

## Low-weight criteria

Do **not** choose a solution merely because it is easier to implement right now.
Ease of implementation has very little weight compared with elegance,
maintainability, clarity, runtime behavior, and architectural fit.

Do **not** avoid an elegant solution merely because it is difficult to test
automatically due to visual, aesthetic, or feel-based behavior. Prefer the
elegant system; build the headless approximation where you can, and note the
caveat. Visual regressions are found and fixed later through review, playtesting,
and iteration — they are not a reason to ship a worse architecture.

**Make the fix BLIND rather than handing it back.** When a fix can't be verified
by the tools you have — a visual / feel change, or a bug you can't reproduce
headless — do NOT stop at "I found it but shouldn't fix blind" or "tell me which
still misbehaves." Reason from the code to the most-likely-correct change and
**ship it**, in its OWN commit, with the subject starting `blind fix:` and a
one-line "check that X looks right in-game." The reviewer always re-checks; the
worst case is reverting one isolated commit, which is cheap — far cheaper than a
round-trip where they wait for a turn they were almost always going to accept.
Blind ≠ careless: it must still compile, keep the relevant tests green, and stay
replay-byte-identical where that applies; "blind" covers the unverifiable feel/
visual, not unverified-compile. Several independent blind fixes → several
independent commits, so any one can be reverted alone.

---

_Companion reading: `docs/concepts/rust-module-boundaries.md` (the crate layering
this serves), `docs/concepts/sim-presentation-seam.md` (the biggest active seam),
and the three-layer stance + commit/patch discipline in `AGENTS.md`._
