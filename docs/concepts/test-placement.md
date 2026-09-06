---
id: test-placement
aliases: []
status: current
authority: durable-concept
last_verified: 2026-09-03
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

## ⛔ `ambition_app` is ONE test binary, and that changes what you may assert

The table above sends cross-crate behaviour to the `ambition_app` integration
surface *"filtered by test name"*. The filtering is not a convenience:
`game/ambition_app/Cargo.toml` declares a SINGLE `[[test]]` target, `app_it`, so
all **150** files under `game/ambition_app/tests/` are modules of one binary and
libtest runs them as threads in ONE PROCESS.

⇒ A test there may not assume it owns process-wide state. A sibling booting its
own `App` populates `Assets<Image>`, resources and asset servers underneath your
assertions, and the failure that produces is a GREEN one — your assertion was
satisfied by somebody else's world.

**If a test genuinely needs the process to itself**, say so at the test and give
it a runner:

* mark it `#[ignore]` with a reason naming the isolation requirement, not
  "slow" — e.g. `parallax_theme_retires_on_walk`, `hall_redecode_census`;
* drive it from a script with an exact filter (`scripts/measure_parallax_retire.sh`);
* and know that `./run_tests.sh --heavy` re-enables ignored tests, which is why
  that plan runs `--test-threads=1`.

⚠ **`#[ignore]` carries three different meanings in this repo** — *slow*,
*prints or panics instead of asserting*, and *invalid unless alone* — and no
flag distinguishes them. Print-only and panicking tests are therefore named
`probe_*` and skipped by the heavy plan; the naming is enforced by
`scripts/tests/test_probe_tests_are_named_probe.py`. If you add an ignored test,
decide which of the three it is before choosing its name.

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
