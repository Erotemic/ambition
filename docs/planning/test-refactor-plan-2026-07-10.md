# Test organization — current policy and remaining debt

**Status:** the policy-package migration landed; the large-inline-test cleanup is
**not complete**.

The detailed 2026-07-10 migration ledger is archived at
[`docs/archive/reviews/planning-history-2026-07-11/test-refactor-execution-ledger-2026-07-10.md`](../archive/reviews/planning-history-2026-07-11/test-refactor-execution-ledger-2026-07-10.md).

## Current ownership model

Tests are organized by what they prove, not by a blanket rule that every test
must be external.

- Unit tests for private local invariants may remain beside implementation.
- Large behavioral suites belong in a sibling `tests.rs`, `*_tests.rs`, or crate
  `tests/` target when they obscure production code or force broad recompilation.
- Cross-crate architecture policy belongs in `tests/ambition_workspace_policy`.
- Historical references to
  `game/ambition_app/tests/architecture_boundaries.rs` describe the migrated
  source file only; that deleted file is not a current policy owner.

The migration matrix and policy modules under `tests/ambition_workspace_policy`
are the current authority for the old architecture-boundary checks.

## What the campaign accomplished

- Architecture checks moved into a dedicated policy package.
- Policy families were split into declarative and custom modules.
- Migration completeness is checked against a frozen source-test inventory.
- Determinism, `ControlFrame`, module-size, dependency, and other structural
  rules can run without linking the entire application test binary.
- Many large inline test modules were extracted into dedicated files.

These accomplishments do not imply that every large inline module was removed.

## Remaining large inline modules

The machine inventory currently finds these production files with inline
`#[cfg(test)]` modules of at least 200 lines:

- `crates/ambition_characters/src/equipment.rs`
- `game/ambition_demo_smb1/src/flag.rs`

The machine-maintained inventory is compared with the `planning-evidence` markers
in [`status.md`](status.md). A future extraction must update the marker list in
the same commit.

Exact line counts are intentionally not copied here; they change when either the
implementation or tests change and do not affect the policy decision.

## Decision still required

Choose one of two honest policies:

### A. Keep the 200-line threshold

Extract the two remaining modules and keep the inventory check. New production
files crossing the threshold must fail the documentation/policy check until they
are extracted or explicitly waived.

A waiver must name why local private access is worth the production-file cost. A
waiver is not “this was inconvenient to move.”

### B. Retire the threshold

If the project does not want a repository-wide size rule for inline tests, remove
the threshold and the debt claim. Continue organizing tests by locality,
compilation cost, and readability without saying that no large inline modules
remain.

Do not retain the threshold in prose while treating violations as invisible.

## Extraction procedure

For each remaining file:

1. Determine whether tests require private implementation details.
2. Prefer a sibling `tests.rs` or scoped test module before widening production
   visibility.
3. Preserve test names and behavior.
4. Run the crate's focused tests and the relevant workspace policy tests.
5. Update the exact debt set in [`status.md`](status.md) in the same commit.

## Exit criteria

The test-organization track is complete only when:

- architecture policy is owned by `tests/ambition_workspace_policy`;
- the migration matrix and its completeness check are green;
- every current production inline test module at or above the chosen threshold
  is either extracted or explicitly waived;
- a poison fixture proves the inventory catches a newly introduced oversized
  inline module;
- the live planning tree contains no universal cleanup claim that the machine
  inventory contradicts.
