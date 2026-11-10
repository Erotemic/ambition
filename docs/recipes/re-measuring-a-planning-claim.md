# Re-measuring a planning claim

Planning and review documents describe the repository at the time they were
written. Before acting on a technical claim, read the relevant implementation
in the current checkout. Keep historical observations where they explain a
decision, but do not treat them as current interfaces or blockers.

This is a focused reading procedure, not a required test or audit campaign.

## Start with the actual claim

1. Identify the exact statement that matters for the task: a missing
   capability, an API dependency, a behavior, a cost, or a known defect.
2. Identify the current owner in source and the composition that uses it.
   `python scripts/agent_query.py "<task words>"` can help locate both.
3. Inspect the implementation and a real consumer. A type declaration shows
   the interface; it may not show what a runtime path does.
4. Keep what is still true, correct what has changed, and leave historical
   results explicitly dated. Do not redo an entire inventory to answer one
   concrete question.

## Match the scope of evidence to the claim

| Evidence | What it can establish | What it cannot establish alone |
| --- | --- | --- |
| Search in `src/` | References in those files | All crate uses, including `tests/`, `benches/`, `examples/` and `build.rs` |
| Grep for a symbol | Matching source spelling | Runtime ownership or execution |
| `cargo check` with default features | Selected code compiles | Optional feature and target behavior |
| Dependency tree | Resolved edges for that feature/target selection | Runtime plugin installation or gameplay semantics |
| Symbol-citation checker | Named paths and symbols resolve | Whether prose describing them is true |
| Passing headless scenario | Behavior exercised by that composition | Visual, input-device, or platform behavior not assembled there |

For an absence claim, check the actual population. A known positive example
can reveal a broken search. Avoid creating permanent source-text ratchets to
validate prose. The Rust type system and actual consumers are better evidence
for ownership boundaries when they can express them.

## Read the owning path

Useful questions:

- Who writes the state, and who reads it during gameplay?
- Does the behavior depend on a feature or on a plugin being installed?
- Is a default value used only in tests or also during normal construction?
- Does a consumer use a public API or reach into an implementation module?
- Does this state survive reset, save restore, room transition, or rollback?
- Is the planning page describing a target architecture rather than what
  runs today?

Look at the function body, construction path, registration, and calling
schedule where those details affect the claim. Do not infer current behavior
from a proposal, a type name, or a test name alone.

## Treat numbers as measurements, not standing facts

A numerical claim should identify the population, method, context, and
revision. This applies to test counts, dependency counts, frame times,
package sizes, and memory use.

Examples of scope differences that matter:

- A direct Cargo dependency line is not the resolved transitive closure.
- Removing a dependency edge may not reduce closure if another path remains.
- A warm build measures different work from a clean build.
- A default-feature build does not cover a feature-union configuration.
- A count of source references may include comments and documentation while
  excluding generated code and registrations.
- A performance comparison must use comparable scenes, devices, workloads,
  and measurement phases.

If the old number is not needed to make a decision, remove it from evergreen
instructions instead of re-measuring it for decoration.

## Check the environment before attributing a failure to code

- A superproject Git revision and the submodule revisions are different facts.
- Ignored and generated art may be present in one checkout and absent in
  another. Check its declared source and publication workflow.
- A feature flag or narrower test invocation may remove the code under study.
- Render frames and authoritative simulation ticks are different measures.
- Rollback can replay a tick; compare the matching generation and attempt.
- Device availability can prevent a browser, Android, audio, or graphics
  verification from executing.

Do not create compatibility code, a new guard, or a large new test fixture to
compensate for a stale sentence. Correct the sentence and act on the actual
implementation.

## Record a useful result

For a material correction, write the current owner or behavior and the source
that establishes it. Include a commit SHA and date when a measurement depends
on a particular revision. Do not change `last_verified` without examining the
claims it would appear to cover.

The detailed past failures that motivated these methods belong in `dev/`
engineering memory, Git history, and focused reviews rather than in every
operational recipe.

Related: [`checks-that-did-not-run.md`](checks-that-did-not-run.md),
[`../concepts/engineering-memory.md`](../concepts/engineering-memory.md).
