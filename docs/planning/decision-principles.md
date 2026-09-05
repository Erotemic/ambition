# Decision principles — how to choose like Jon when operating autonomously

*(Jon's own criteria, verbatim — relocated from `docs/vision/driving_decision_principles.md` <!-- cite-ok: the path is named because it is GONE --> into the planning stack, 2026-07-05. Read this before any architectural choice.)*

> ⛔ **THE POINTER THAT USED TO SIT HERE IS DEAD, AND IT POINTED AT A REAL
> THING.** This line read *"[`vision.md`](vision.md) §8 is the digest"* until
> 2026-09-03. It was true when written: at `docs/planning/vision.md` in July the
> sections were numbered and **§8 was literally "Principles digest"**. That page
> has since been rewritten with unnumbered sections, the digest section was
> removed, and it was not relocated — a search of all of `docs/` finds the phrase
> nowhere, and vision.md's §8 is now "Execution", three paragraphs about the
> queue. ⇒ **So this page is the only home these criteria have**, and a reader
> sent looking for a shorter version should stop looking. ⚠ A dead
> cross-reference survives a link checker perfectly: `vision.md` still exists, so
> nothing was ever going to report this.

If you need to make an architecture decision while operating autonomously, use these criteria to make the choice Jon would most likely make.

## High-weight criteria

Prefer the solution that is more elegant. In this project, “elegant” means the solution composes cleanly, has an obvious source of truth, follows existing seams, and does not require callers to remember hidden ordering rules or workaround behavior.

Two tests make that judgeable rather than a matter of taste, and a change should pass at least one.

*One authority per fact.* If a fact is read or written in two places, the elegant change is not to keep them in step — it is to remove one of them. A test asserting that two copies agree is a guard compensating for a fact having two homes, and it is evidence the change has not been made yet.

*Make it impossible, not checked.* Prefer the structure that cannot express the defect over the test that catches it. A guard is the right answer only where a type cannot state the rule — an authored string, a content file, a fact that lives outside the compiler.

A refactor that only moves code is not elegance. Name the authority or the dependency edge the change removes. If it removes neither, it is churn.

*One absence must not answer two questions.* A lookup that returns nothing is answering exactly one question unless somebody made it answer two, and the second one then passes silently. `ParamSchemaRegistry` has no entry for a technique with no params and no entry for a technique that does not exist, so an authored typo validates clean at startup and is inert in play; `boss.cleared` read a missing save key as `Untouched`, so a wrong id was a shut door rather than an error. The fix is never to make absence fail — it is to split the facts, so that unknown fails, known-and-empty passes, and known-and-checked is checked. ⚠ The permissive default itself is usually right: an unconstrained move permits, an unclaimed slot is writable, and `ExperienceStaging::is_writable_by` states the distinction outright — *"nobody claimed this" and "somebody else claimed this" are different answers and only the second is a refusal*. Hunt the `None` that means two things, not the one that means yes.

A class claimed from one example is a hypothesis. Sweep for the other instances before writing the generalisation down, because they are as likely to refute the wording as to confirm it — the sentence above was first written as "an absence that reads as a pass", and every other instance in the tree turned out to be correct.

Three things follow from the second test, and all three were learned by getting them wrong.

Making something impossible changes what its guard is for. A guard whose property has become structural is worth keeping only if it still names a reachable failure; re-aim it and say which, or delete it. Leaving it asserting what the compiler now guarantees, with a comment claiming otherwise, is worse than either.

A claimed compile-time check must be shown to fail. Poison the case that only the new check can catch, not the nearest case to hand — a poison that fires through some other mechanism is indistinguishable from one that fires through yours. For a compile-time assertion specifically, make it unconditionally false and confirm the build breaks. An associated `const` holding an assertion can compile clean while never being evaluated, which is a guard that cannot fail wearing the strongest possible disguise.

A schedule ordering edge is a fact that lives outside the compiler, so a guard is the right answer — but only a guard that has been run red. Bevy expresses ordering with sets, and the one thing a set cannot express is order *within itself*: a system that reads a message written by another member of its own set is unordered against the writer, and no phase chain will say so. The cost is not lateness but frame misattribution — the reader picks the message up on the following tick and journals its effect under a frame that did not produce it, which is what the rollback quarantine then judges. A behavioural test does not catch this. The simulation runs single-threaded, so an unordered pair still gets an order, and that order is stable: the gameplay assertion passes whether or not the edge exists. Deterministic, reproducible, and arbitrary.

Check the edge itself. Bevy computes the schedule's conflict list unconditionally and consults `ambiguity_detection` only to decide whether to warn — so the list is readable even in the rollback host, which silences it. Assert zero conflicts on the *resource*, not between two named systems: system and resource names are compiled out without Bevy's `debug` feature, and the invariant is about the resource anyway. Two ways this guard fails green, both met in practice: composing a hand-assembled app rather than the shipped plugin group, which certifies an empty room because the writer's plugin was never added; and identifying the resource through anything that can silently return a different id. Falsify it by deleting the edge and watching the count rise, with the code that ships rather than an earlier draft.

Do not answer this with a workspace-wide ambiguity ratchet. A banked count of existing conflicts is a denominator nobody re-measures, and it rots in the direction that looks like progress: when a conflict is genuinely fixed and the number stays put, the ratchet goes on certifying a population that no longer exists. A guard scoped to one resource banks nothing.

Prefer the solution that best respects the project’s layer boundaries:

* Rust is for behavior.
* RON is for content.
* The world IR  is for space and it is authored by a backend like LDtk, tiled, or godot.
* Machinery must not import named game content.

Prefer the solution that is more runtime efficient, especially in hot paths or repeated simulation work.

Prefer the solution that is more maintainable. The code should be easy to understand, easy to modify, and hard to accidentally misuse.

Prefer the solution that is concise. Shorter, simpler solutions are better when they preserve clarity and correctness.

Prefer the solution that minimizes confusion for a new developer. Ownership, data flow, and intent should be apparent from the code structure.

Prefer the solution that avoids parallel paths, compatibility shims, and duplicate mechanisms. This project is still pre-release, so direct replacement is usually better than preserving an old path when the replacement makes the architecture simpler.

Prefer the solution that creates a stable extension seam instead of adding another special-case branch to a core system.

Prefer the solution that keeps hot paths allocation-free and avoids repeated runtime work, while not over-optimizing cold authoring paths.

## Important considerations

If the code can be refactored so the solution better satisfies the high-weight criteria, do the refactor.

Look for ways to unify the change with an existing system. Unification is desirable when it does not over-scope the system. If a specific case can become an instance of a general case without a major runtime or clarity cost, that is usually worth doing.

Consider whether the change affects game behavior. Behavior changes are not automatically bad. The game still contains buggy, inconsistent, or provisional behavior, so making behavior more coherent may be the right outcome. Preserve behavior only when the existing behavior is intentional or relied upon.

Prefer a narrow validation path. A good architecture change should usually have a focused test, check, or tool command that proves the important part of the change.

Do not let TUNING block architecture (Jon, 2026-07-06). Numeric feel/quality values — DI angles, boss-quality thresholds, slope feel, fighter-brain weights, visual-quality defaults, and the like — are KNOBS, not Jon-blocking decisions. When the right variable already exists as a knob, treat choosing its value as data/playtest work and pick a reasonable default (or leave the existing one); ship it BLIND and let Jon adjust. Only escalate when the KNOB ITSELF is missing (an architecture gap), not when only its value is unset. A tuning task is never a reason to stall a structural carve.

## Low-weight criteria

Do not choose a solution merely because it is easier to implement right now. Ease of implementation has very little weight compared with elegance, maintainability, clarity, runtime behavior, and architectural fit.

Do not avoid an elegant solution merely because it is difficult to test automatically due to visual, aesthetic, or feel-based behavior. Prefer the elegant system. Visual regressions can be found and fixed later through review, playtesting, and iteration.
