---
id: test-placement
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
related_docs:
  - docs/concepts/testing-and-validation.md
---

# Test placement

A test lives at the narrowest scope that owns the invariant. Do not widen a
production API merely to move a test.

## Placement rules

| Invariant | Placement |
|---|---|
| Pure helper, parser, geometry rule, or small private state machine | inline `#[cfg(test)]` module or adjacent `tests.rs` |
| Large tests of private module internals | adjacent `src/<module>/tests.rs` |
| Public crate contract or assembled owner plugin | owning crate's `tests/` or public-module tests |
| Provider/content contract | provider crate tests |
| Cross-crate app/host behavior | `ambition_app` integration surface, filtered by test name |
| Reset/step/observation behavior | `ambition_sim_harness` or provider harness tests |
| Workspace dependency/layout/source policy | `tests/ambition_workspace_policy` only |
| Browser/device/render feel | explicit manual or heavy acceptance check, backed by headless invariants where possible |

`tests/ambition_workspace_policy` links no production crate. Keep workspace
scanners there so architecture checks do not compile the full app graph.

## Rules against brittle tests

- Test behavior, ownership, and properties rather than exact source spelling.
- Do not pin pre-polish tuning unless the value itself is the contract.
- Use poison/non-vacuity fixtures for reusable scanners or historically
  recurring harmful states, not for every declarative rule.
- Remove migration-only matrices and source-text checks when the migration is
  complete.
- Keep test helpers private when the production API does not need them.

## Running the test

Find the current test before naming a Cargo target:

```bash
python scripts/agent_query.py tests "<invariant>"
./run_tests.sh -p <owner> -k <substring>
```

For the complete headless merge gate:

```bash
./run_tests.sh
```
