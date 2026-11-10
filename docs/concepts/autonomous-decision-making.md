---
id: autonomous-decision-making
aliases:
  - autonomous agent decisions
  - design fork policy
  - Jon-style decisions
last_verified: 2026-11-10
---

# Autonomous decision-making

Ambition is an experimental, pre-release game and engine. The maintainer
delegates implementation decisions to agents and expects them to make progress
without routine approval. The user will playtest and steer the result. A
reasonable, reversible implementation is more valuable than a stalled task.

Two standing principles are:

> Correctness is emergent from elegance.

> We can always fix things after.

These support ambitious changes and iterative development. They do not require
treating a successful compile as proof of correct gameplay.

## Decide and continue

Make architecture and implementation decisions using the task, existing code,
and the criteria below. Choose among reasonable interpretations instead of
asking the maintainer to resolve details that do not materially affect the
requested result.

For an overnight or long-running assignment, do not stop the entire task over
a nonessential question. Make an explicit assumption, take a reversible path,
and continue. If one part is truly blocked, pursue independent parts of the
assignment. Include unresolved choices in the final handoff.

Ask when an essential product decision cannot be reasonably inferred, or when
an action is destructive, difficult to reverse, or externally consequential.
The user's explicit scope and working-directory instructions are authoritative.
Do not create a separate worktree or stop for permission merely because
concurrent development could be inconvenient.

## Choose the design

Prefer implementations with:

- A clear owner and source of truth for each fact and transition.
- Shared body, controller, and capability paths rather than game-name branches.
- Appropriate dependency direction: reusable engine code does not import
  named game content. Rust owns behavior, RON owns authored structured content,
  and LDtk owns world space.
- Idiomatic Bevy composition and a narrow extension seam proven by a real
  consumer. A different platformer should be addable through a provider
  without editing core for game-specific cases.
- Explicit ordering, authority, and lifetime rather than hidden schedule
  dependencies or duplicated state.
- Clear, grep-friendly names and code that a new engineer can extend.
- Efficient hot paths without unnecessary allocations or speculative caches.

A refactor may be the best way to deliver a feature. Remove redundant paths
and compatibility shims when the unified implementation can replace them.
Avoid abstract frameworks built only for hypothetical future consumers.
Preserve an engine concept because it is useful, not simply because it is
currently used; likewise, absence of current users is not grounds to delete
a useful concept. See [the review guide](../reviewer-guide.md#keep-redesign-or-delete-usage-is-not-worth).

Favor the best long-term shape over the easiest local patch. Code length is
secondary to clarity, and names should identify their domain rather than
duplicate generic names across modules.

## Implement and learn

Existing gameplay is a working reference, not an immutable specification.
Improve provisional behavior when it serves the requested intent. Preserve
behavior known to be intentional, especially controls, sequences, and
mechanics the user explicitly cares about. If behavior changes, identify
that change in the handoff so it can be playtested.

Make the change, compile the affected code when feasible, and verify its
consequential behavior by a focused test, real headless scenario, or
playtest. Replay equality helps when a change is intended to preserve
behavior; it is not a reason to reject an intentional improvement.

Do not spend the task writing a large collection of tests for choices that
could reasonably work either way. Add permanent tests for important
invariants, known regressions, or acceptance behavior, and weigh their
execution cost. Use a temporary probe when a one-time answer suffices.
See [testing and validation](testing-and-validation.md).

If a visual or feel change cannot be verified in the current environment,
implement the best-supported fix rather than returning only a request for
another playtest. Keep such fixes easy to review or revert, and state the
specific in-game check needed. The established `blind fix:` commit prefix
may be used for this case.

A coherent commit records progress; it does not automatically end an
ongoing assignment. Continue toward the requested outcome until it is done,
the allotted run ends, or further useful progress genuinely requires input.

## Handoff

Report what changed, what is playable, what was actually checked, and the
most important remaining issue. Be specific about a blocked decision without
turning routine design alternatives into requests for approval. Do not claim
a test or gameplay check ran when it did not.

Related: [engine mental model](engine-mental-model.md),
[simulation/presentation boundary](sim-presentation-seam.md), and
[architecture boundaries](../architecture/architecture-boundaries.md).
