---
id: testing-and-validation
aliases: []
status: current
authority: durable-concept
last_verified: 2026-09-03
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

The runner executes the default workspace suite. It deliberately does not use
workspace-wide `--all-features`, which would mix incompatible platform/device
feature sets.

⛔ **CORRECTED 2026-09-03: THE DEFAULT RUN DOES NOT COVER FEATURE-HIDDEN TESTS,
and this page used to say it did.** The runner's own coverage footer is explicit
and counts them — *"this was the default BACKBONE plan, which does NOT cover:
tests behind `#[cfg(feature = "...")]` — MEASURED 2026-09-03 at 783 tests across
29 crates, the largest single omission this footer names"*. The union job that
executes them lives in the exhaustive plan. `python3 scripts/feature_gated_tests.py`
prints the current per-crate figure, and `--verify <crate>` asks cargo for the
exact pair.
⇒ A green `./run_tests.sh` is SILENT about 783 tests. That is a fine default —
it is a backbone, and the footer says so — but a reader who trusted this page
would have believed the opposite.

Useful modes:

```bash
./run_tests.sh --list                 # inspect the generated job plan
./run_tests.sh --fast                 # DEPRECATED no-op; the backbone IS the default now
./run_tests.sh -p ambition_platformer2d_world      # restrict to an owning crate
./run_tests.sh -k room_transition     # libtest name substring across jobs
./run_tests.sh --heavy                # ignored (probes skipped, serial) + acceptance cycles
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

⚠ **That list is three of SIXTEEN `scripts/check_*.py`.** The ones a change is
most likely to owe are not on it:

| check | when you owe it |
|---|---|
| `check_absence_contracts.py` | any carve — 37 contracts, including the capability-footprint ratchet |
| `check_planning_citations.py` | any commit that MOVES a file a planning doc cites |
| `check_no_warnings.py` | before a tip: warnings are CI-red under `-D warnings` and the per-crate runs do not see them |
| `feature_gated_tests.py` | to see what a green default run was silent about |
| `check_disk_headroom.py` | already wired into the runner; see its own measured counter-example |

⇒ Prefer `./run_tests.sh` over hand-picking from this list — the point of naming
them here is that a reader who reaches for a focused check should know the list
is longer than three, not that they should run them one at a time.

Formatting is useful but not a correctness oracle. A patch should not be blocked
solely because formatting tooling is unavailable when behavior/invariants are
otherwise validated.
