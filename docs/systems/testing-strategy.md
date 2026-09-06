---
status: current
last_verified: 2026-07-18
---

# Testing strategy

The test strategy is headless-first, owner-local, property-oriented, and
cost-aware. The canonical suite front door is:

```bash
./run_tests.sh
```

See [`../concepts/testing-and-validation.md`](../concepts/testing-and-validation.md)
for the full doctrine and [`../concepts/test-placement.md`](../concepts/test-placement.md)
for placement.

## Suite layers

1. pure kernel/model tests in the owning module;
2. domain plugin/ECS tests in the owning crate;
3. provider and cross-content validation;
4. runtime/sim-harness/app integration tests;
5. real headless scenario/replay/restore tests;
6. heavy/manual visible, browser, Android, and performance checks.

`./run_tests.sh` runs the workspace backbone and per-crate headless-safe feature
jobs. `--heavy` adds ignored/acceptance cycles. `-p` and `-k` are the normal
ways to narrow a failure.

## Architecture policy tests

Workspace dependency, path, source-boundary, determinism, and module-size rules
live in `tests/ambition_workspace_policy`. That crate inspects the repository as
data and links no production crate.

Prefer compiler-visible/type/visibility boundaries and behavioral tests. Keep a
scanner only for a concrete recurring harmful state that cannot be expressed
more naturally.

## Acceptance properties

High-value cross-cutting tests prove:

- one body/action path for different controllers;
- no partial room/session commit;
- provider/session isolation;
- exact reset/restore reconstruction at the supported boundary;
- prompt/gameplay resolver agreement;
- headless/visible authoritative-state agreement;
- stable ordering and replay under deterministic inputs;
- covariance/reference-frame behavior where claimed.

Do not preserve bad pre-release output merely to keep a snapshot/hash green.
