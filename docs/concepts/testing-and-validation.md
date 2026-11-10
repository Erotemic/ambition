---
id: testing-and-validation
aliases: []
status: current
authority: durable-concept
last_verified: 2026-10-02
implemented_by:
  - scripts/run_tests.py
  - run_tests.sh
  - crates/ambition_sim_harness
  - tests/ambition_workspace_policy
---

# Testing and validation

Use the cheapest check that can reveal an important defect in the change.
Use assembled-game or playable acceptance when integration or feel is the
question; small implementation details often need no permanent test.

## Canonical front door

```bash
./run_tests.sh
```

The runner executes the default workspace suite. It deliberately does not use
workspace-wide `--all-features`, which would mix incompatible platform/device
feature sets.

⛔ **The default run does not cover feature-hidden tests.** Tests behind
`#[cfg(feature = "...")]` run only in the feature-union job of the exhaustive
plan. The runner's coverage footer states the count.
`python3 scripts/measure/feature_gated_tests.py` prints the per-crate figure, and
`--verify <crate>` asks cargo for the exact pair.

Useful modes:

```bash
./run_tests.sh --list                 # inspect the generated job plan
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

This is a menu of verification tools, not a sequence every change must run.
Choose the level that can falsify the meaningful claim. Gameplay and presentation
changes may need a playable witness; a unit test cannot substitute for it.

## Validation states and cadence

Maintainer ruling (2026-10-03, Q59). A gate has three outcomes, and evidence
keeps them apart:

```text
PASS
FAIL
NOT RUN   — either "not required at this boundary" or "required before this
            handoff/merge/release"
```

Never record a lane that did not run as PASS or as FAIL. `NOT RUN` does not mean
"stop and run it now". Each lane has a cadence:

- a cheap focused check for the current change: run it often;
- the large workspace or integration suite: at a milestone, before handoff or
  merge, or on a schedule;
- expensive broad validation: nightly or scheduled; repair what it finds;
- a gate that the current edit cannot affect: it may stay `NOT RUN`.

A gate blocks only where its policy requires it at the current boundary. Do not
run a long suite after each of N edits to keep every receipt green: that costs
implementation time and buys no correctness.

Which lane belongs to which boundary. The path rules have one owner each; this
list only says when to ask them:

| Boundary | Run | Owner of the rule |
| --- | --- | --- |
| During implementation | The cheapest relevant check, when the changed behavior warrants it | [cheapest sufficient check](../recipes/cheapest-sufficient-check.md#the-matrix) |
| Push | Inspect `python3 scripts/required_checks.py` for relevant outstanding coverage; use targeted checks first | `scripts/required_checks.py` reports but does not block the push (Q166) |
| Handoff or milestone | Focused acceptance plus the default plan when justified by integration scope or requested by the maintainer | `scripts/run_tests.py`; its footer names what the plan does not cover |
| Release, or a change to features, an SDK surface or the web path | `./run_tests.sh --run-everything-you-probably-dont-need-this` | the same footer |

At a push, `required_checks.py` requires the union feature job for a change to
the feature wiring of a manifest, and names it as `NOT RUN, not required at this
boundary` for a change whose dependent crates hide tests behind features.

A change to prose or pictures only (`.md`, `.png`, `.svg`, `.txt`) requires no
check.

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

### Not every check belongs in the suite

Maintainer direction (2026-11-10): tests are not bad, but not everything needs
to become a test that runs in the full suite, especially when the behaviour is
marginal. A suite that takes hours hinders more than it helps.

- **Check once, keep only what earns its place.** Verifying a change by running
  something (a probe, a one-off scenario, a control, a poison) does not oblige
  you to keep it. Keep a test in the default suite when it guards behaviour
  that matters and could plausibly break again; leave a one-off check in the
  commit message or the owner doc.
- **Weigh the cost.** An `ambition_app` test can require expensive app setup.
  Prefer one meaningful scenario over near-duplicates; keep a unit test only
  when it establishes an important invariant more cheaply.
- **Measurements and controls are not suite members.** A print-only measurement
  is an ignored `probe_*`. A control that proved a witness could fail once
  needs no permanent slot unless the witness's premise is fragile.
- **Marginal behaviour gets no test.** Tuned values, presentation details and
  one-step glue are settled by reading and running the game, not pinned.
- **Run the cheapest sufficient lane.** Run the tests that can see the change
  ([cheapest sufficient check](../recipes/cheapest-sufficient-check.md#the-matrix));
  the full suite belongs at a milestone, not at every edit.

A passing check is useful only if it exercised the relevant code and could
have failed for the defect at issue. The `AGENTS.md` section "Avoid bullshit
guardrails" also governs reviews: verification does not require turning each
probe, source-text scan, or process rule into a permanent test.
[`checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md) is the
doctrine: a check that never executed, a check that executed and could not fail,
a guard whose source-text anchor a formatter or a language rule can silently
change, and a guard that is perfectly sound about a subject nothing reads. The
four print the same green and want four different fixes.

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
python scripts/checks/check_doc_links.py
python scripts/generate_agent_index.py
python -m ambition_ldtk_tools validate <world.ldtk>
```

Other `scripts/check_*.py` that a change often owes:

| check | when you owe it |
|---|---|
| `check_absence_contracts.py` | any carve; the architectural absence contracts, including the capability-footprint ratchet |
| `check_planning_citations.py` | any commit that MOVES a file a planning doc cites |
| `check_no_warnings.py` | before a tip: warnings are CI-red under `-D warnings` and the per-crate runs do not see them |
| `feature_gated_tests.py` | to see what a green default run was silent about |
| `check_disk_headroom.py` | already wired into the runner |

Use the checks that bear on the change. Inspect `./run_tests.sh --list`
when a full lane is justified; otherwise keep verification targeted.

Formatting is useful but not a correctness oracle. A patch should not be blocked
solely because formatting tooling is unavailable when behavior/invariants are
otherwise validated.
