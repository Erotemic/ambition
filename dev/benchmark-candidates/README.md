# Benchmark-candidate strategy

Ambition is both a game project and a useful source of small, realistic
software-engineering benchmark problems. Every time an agent-generated patch
fails in a way that required project context to diagnose, capture a benchmark
candidate before the details fade.

The goal is not to record trivia. A good candidate tests whether another model
can preserve an engineering invariant while doing the original task, ideally
before seeing the compiler error or test failure that exposed the mistake.

## Workflow

1. **Fix the project first.** The benchmark candidate should be grounded in a
   real repair, not a speculative lesson.
2. **Save the failure evidence.** Keep the exact compiler/test output in the
   conversation or journal so the repair can be audited later.
3. **Reconstruct the pre-error context.** Ask: what information was available
   to the agent when it made the mistake? The benchmark should usually start
   from that context, not from the final compiler error.
4. **Distill the invariant.** Convert the mistake into a compact rule such as
   "attributes and doc comments move with the item they annotate" or "a private
   child module's `pub` item is not visible through the facade unless
   re-exported."
5. **Write a hard question.** Include enough surrounding code shape that a model
   must reason about Rust modules, attributes, Bevy resources, assets, or game
   architecture; avoid giving away the answer in the prompt.
6. **Write the expected answer and validation.** Include the minimal code-shape
   change and the command that should catch the bug.
7. **Tag future dimensions.** Useful tags include `rust-module-refactor`,
   `rust-visibility`, `bevy-resource`, `serde`, `touch-ui`, `asset-generation`,
   `android`, `game-input`, and `procedural-audio`.

## Candidate quality bar

A candidate is worth keeping when it has all of these properties:

- **Realistic:** it came from an actual Ambition maintenance task.
- **Contextual:** a generic StackOverflow answer is not enough; the model must
  preserve a project-specific invariant or API surface.
- **Minimal enough:** unnecessary files are removed, but the prompt still has
  enough context that the original mistake is tempting.
- **Checkable:** there is a compile/test/static command or a precise expected
  patch shape.
- **Non-leaky:** the question can be asked without saying "the compiler says X"
  unless the benchmark is explicitly an error-repair task.

## Prompt levels

For each significant issue, prefer writing at least one candidate at the
highest level that still feels fair:

- **Level A: pre-error operation.** "Split this Rust module into private child
  modules while preserving behavior and public API." This is the hardest and
  best test of planning.
- **Level B: error repair.** Provide the compiler error and ask for the fix.
  Useful for debugging ability, but less predictive of avoiding the bug.
- **Level C: distilled micro-question.** A tiny Rust example that captures the
  language rule. Useful for focused unit benchmarks, but less game-specific.

The `rust-questions.md` file should favor Level A/B candidates with an
"Expected answer" section. If a separate minimal benchmark harness is created
later, Level C variants can be extracted from those entries.

## Example distillation pattern

Raw issue:

> After splitting `trace.rs`, `cargo fmt` fails with `expected item after doc
> comment`, `#[derive]` is attached to a function, and a sibling module cannot
> call a helper that still exists.

Benchmark distillation:

> You are splitting a Rust facade module into private children. Which comments,
> attributes, derives, helper visibility changes, and facade re-exports must move
> with the extracted items, and what static checks should you run before handoff?

This keeps the hard part: the agent must preserve adjacency and module-boundary
semantics while doing the split, without being spoon-fed the final line numbers.

## Additional benchmark dimensions discovered during module splits

Recent Ambition refactors exposed two more useful candidate categories:

- **Physical file-location invariants.** Macros such as `include_str!` and
  `include_bytes!` resolve relative to the source file containing the macro.
  Moving tests into a child module can break checked-in game asset fixtures even
  when the module path and crate API are correct.
- **Test-only trait requirements.** New regression tests may need derives such
  as `PartialEq` or `Eq` on small marker enums. A good candidate should ask
  whether the expected invariant is clearer as a production derive or as a more
  limited assertion like `.is_none()`.

When writing questions for these classes, prefer the pre-error operation prompt:
"move this code into child modules while preserving compile-time fixtures and
new regression tests." Then include an error-repair variant only as a secondary
Level B prompt.
