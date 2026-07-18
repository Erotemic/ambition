---
id: testing-and-validation
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
implemented_by:
  - scripts/run_tests.py
  - run_tests.sh
  - crates/ambition_sim_harness
  - tests/ambition_workspace_policy
---

# Testing and validation

Validation should prove the invariant at the narrowest owning layer, then prove
that the assembled provider/host still uses that path.

## Canonical front door

```bash
./run_tests.sh
```

The runner executes the default workspace suite plus headless-safe feature jobs
for crates whose tests are otherwise hidden behind features. It deliberately
does not use workspace-wide `--all-features`, which would mix incompatible
platform/device feature sets.

Useful modes:

```bash
./run_tests.sh --list                 # inspect the generated job plan
./run_tests.sh --fast                 # workspace default-feature backbone only
./run_tests.sh -p ambition_world      # restrict to an owning crate
./run_tests.sh -k room_transition     # libtest name substring across jobs
./run_tests.sh --heavy                # ignored/acceptance cycles too
./run_tests.sh -- --nocapture         # forward args to libtest
```

Unknown packages and empty selections are errors.

## Validation ladder

1. **Pure/local invariant** — unit/property test in the owning module or crate.
2. **Domain ECS behavior** — owner plugin/system test with realistic resources.
3. **Cross-domain assembly** — provider/runtime/harness or `ambition_app` test.
4. **Headless scenario** — step the real simulation, replay, or room flow.
5. **Visible/device acceptance** — only for visual feel, focus, layout, audio
   device behavior, packaging, or performance.

Do not skip levels 1–4 merely because the visible binary is hard to automate.
Improve the headless seam instead.

## What to test

Prefer invariants and properties over tuned values:

- actor/controller parity and one-path execution;
- covariance under gravity/reference-frame changes;
- no tunneling / no partial transactional commit;
- deterministic registration, ordering, replay, and reconstruction;
- provider/session isolation;
- prompt/gameplay resolution agreement;
- headless/visible authoritative-state agreement.

Replay hashes and snapshot bytes are canaries. Re-baseline them when an intended
pre-release semantic change preserves the real invariants.

## Current integration layout

App-level integration tests are aggregated under the `ambition_app` integration
surface (including the `app_it` binary). Do not invent old standalone `--test`
target names from historical docs. Use `-k` through the runner unless you have
confirmed an exact current Cargo target:

```bash
python scripts/agent_query.py tests "<behavior>"
./run_tests.sh -p ambition_app -k <substring>
```

## Non-Rust checks

Use focused checks when relevant:

```bash
python scripts/check_doc_links.py
python scripts/generate_agent_index.py
python -m ambition_ldtk_tools validate <world.ldtk>
```

Formatting is useful but not a correctness oracle. A patch should not be blocked
solely because formatting tooling is unavailable when behavior/invariants are
otherwise validated.
