---
id: testing-and-validation
aliases:
  - validation commands
  - headless smoke
  - regression tests
  - property tests
implemented_by:
  - crates/ambition_engine_core/src/movement
  - crates/ambition_app/tests
  - crates/ambition_content/src
  - crates/ambition_sandbox/src
  - .github/workflows
related_docs:
  - docs/systems/testing-strategy.md
  - docs/systems/headless-simulation.md
  - docs/current/risks.md
related_memory:
  - dev/journals/cargo-test-command-lessons-2026-05-11.md
  - dev/benchmark-candidates/cargo-test-single-filter-question-2026-05-11.md
last_verified: 2026-05-17
---

# Testing and validation

## Definition

Testing and validation are part of the knowledge system. Every non-trivial patch should identify the narrowest useful test, the broader package check, and any manual/device validation that remains.

## Core invariants

- Run targeted tests before broad tests when debugging.
- Validate command grammar before handing off commands.
- Headless smoke protects sim/presentation boundaries without needing full interactive play.
- Spatial changes should add regression tests or trace/debug evidence when practical.
- Platform-specific changes need platform-specific evidence; desktop success does not prove Android/web success.

## Edit protocol

1. Use concept pages to identify expected tests.
2. Search `dev/` for known command pitfalls.
3. Prefer one precise regression test for a bug fix.
4. Report what ran and what did not run.

## Common commands

```bash
cargo fmt --check
cargo test -p ambition_sandbox
cargo test -p ambition_sandbox --lib
cargo run -p ambition_sandbox --bin headless
```

Cargo accepts one test-name filter position per test binary. Use separate invocations or package/module filters instead of inventing multi-filter grammar.
