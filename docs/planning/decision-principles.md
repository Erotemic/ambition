# Decision principles — how to choose like Jon when operating autonomously

*(Jon's own criteria, verbatim — relocated from `docs/vision/driving_decision_principles.md` into the planning stack, 2026-07-05. Read this before any architectural choice; [`vision.md`](vision.md) §8 is the digest.)*

If you need to make an architecture decision while operating autonomously, use these criteria to make the choice Jon would most likely make.

## High-weight criteria

Prefer the solution that is more elegant. In this project, “elegant” means the solution composes cleanly, has an obvious source of truth, follows existing seams, and does not require callers to remember hidden ordering rules or workaround behavior.

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

## Low-weight criteria

Do not choose a solution merely because it is easier to implement right now. Ease of implementation has very little weight compared with elegance, maintainability, clarity, runtime behavior, and architectural fit.

Do not avoid an elegant solution merely because it is difficult to test automatically due to visual, aesthetic, or feel-based behavior. Prefer the elegant system. Visual regressions can be found and fixed later through review, playtesting, and iteration.
