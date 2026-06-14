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

Consider whether the change affects **game behavior**. Behavior changes are not
automatically bad — the game still contains buggy, inconsistent, or provisional
behavior, so making behavior more *coherent* may be the right outcome. Preserve
behavior only when the existing behavior is intentional or relied upon. **When in
doubt about whether behavior is load-bearing, the replay/scripted-gameplay tests
are the arbiter:** a change meant to be behavior-neutral must keep replay
bit-identical; a change meant to fix behavior should come with a focused test
that pins the new, intended behavior.

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

---

_Companion reading: `docs/concepts/rust-module-boundaries.md` (the crate layering
this serves), `docs/concepts/sim-presentation-seam.md` (the biggest active seam),
and the three-layer stance + commit/patch discipline in `AGENTS.md`._
