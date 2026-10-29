# Test-suite consolidation: action plan

**Status:** Active. **Date:** 2026-10-26. **Scope:** Ambition Rust tests, first in `ambition_app`.

This document gives a new agent a start point for reducing test maintenance cost.
It is a plan, not a claim that the tests named here are safe to delete.
Do not expand the test corpus to complete this plan.

## Why this work matters

Ambition is a Bevy game host and engine with several shipped game modes,
shared input and rendering, multi-room world state, live content reload, and
rollback simulation. Some long tests check failures at those integration
boundaries. Those tests cannot be replaced by a unit test of one component.

Ambition has a large, agent-written regression suite. Agents tend to add a new test
for each change and seldom remove older tests. This can leave repeated setup,
obsolete diagnostic code, tests of incidental details, and multiple proofs of
the same contract. Each extra test carries review, compile, and update cost.

The goal is **less code to maintain for the same or better defect detection**.
Do not optimize for a target test/source LOC ratio or a target test count.
A test is useful when it can fail for a meaningful defect that another retained
test would not catch. A test is costly when it repeats setup or asserts an
incidental detail that does not matter to the product.

The engine has real hazards at cross-system boundaries. Removing those witnesses
because they are long would make the suite smaller but less trustworthy.
This campaign must therefore distinguish repeated *implementation* from
repeated *behavioral evidence*.

## Read before editing

- [Static audit findings](test-corpus-static-audit-findings-2026-10-26.md): measured inventory, duplicates, limitations, and protected examples.
- [Initial audit and tool notes](test-corpus-static-audit-phase2.md): why the analyzer exists and what it can detect.
- [Test placement](../concepts/test-placement.md): tests belong at the lowest layer that owns their invariant; cross-crate behavior belongs in app acceptance tests.
- [Cheapest sufficient check](../recipes/cheapest-sufficient-check.md): the focused build and test commands; do not start with the full test lane.
- [Heavy app test lane](../recipes/running-the-heavy-app-it-lane.md): why a green headless or demo-app test does not always prove a shipped-host contract.
- [Reviewer guide](../reviewer-guide.md): assess runtime paths and independent authority, not a green test count or a comment.

The source files named below are under `game/ambition_app/tests/` unless a full
path is given. `game/ambition_app/Cargo.toml` sets `autotests = false` and uses
one `app_it` test binary. `tests/app_it.rs` lists its modules;
`tests/app_it_sync.rs` checks that this list stays complete. Never remove or
rename a module without checking both files.

## Evidence baseline: do not mistake it for a savings estimate

The audit data was collected on 2026-10-26. The source review used repository
commit `8e27113c7ec79b8c628d3d0a483fccc61286e425`; check the live HEAD before
making a change. The first workspace-wide scan included four alternate checkouts
under `.worktrees/`. **Do not use its 8,487-file or 51,021-test counts.**

| Baseline measure | Corrected workspace | `ambition_app` |
|---|---:|---:|
| Rust files that contain test code | 1,881 | 319 |
| Test functions found by the analyzer | 11,052 | 1,673 |
| Ignored test functions | 80 | 56 |

For the app, 283 of these files are under `tests/`; 36 are source files with
test code. The 314 repeated 10-line block groups contain overlapping windows.
Their union covers 3,826 line positions in 59 files. **These are not removable
LOC.** Only six high-similarity app test pairs were identified, and their input
differences matter. The 56 ignored app tests together have about 2,909 body
lines, so deleting all diagnostics would not resolve most of the bloat.

The numbers above are historical measurements for comparison, not a live count.
File LOC in the audit report includes non-test code when a source file also
contains tests. Do not compare that measure with a dedicated test LOC counter
without using the same classification rule.

## Step 0: establish an accurate, repeatable inventory

The analyzer is `scripts/audit_test_corpus.py`. The companion tests are
`scripts/tests/test_audit_test_corpus_worktrees.py`. This campaign fixes the
original traversal: it now prunes `.worktrees/`, `worktrees/`, nested Git
checkouts, symlinked directories, `target/`, and Git metadata before visiting
the files. Filesystem mode includes untracked work. Use `--tracked-only` to
count just files in the current Git index (which excludes untracked tests and
submodules). Do not mix the two modes in a trend graph.

Run from the repository root:

```bash
cd ~/code/ambition
git status --short
git rev-parse HEAD

python3 -m unittest discover -s scripts/tests -p 'test_audit_test_corpus_worktrees.py' -v
python3 scripts/audit_test_corpus.py \
  --scope game/ambition_app --out /tmp/ambition-app-test-audit
python3 scripts/audit_test_corpus.py \
  --scope . --out /tmp/ambition-workspace-test-audit
```

Inspect the scan boundary and results before proposing deletions:

```bash
python3 - <<'PY'
import json
from pathlib import Path
for folder in ('ambition-app-test-audit', 'ambition-workspace-test-audit'):
    report = json.loads((Path('/tmp') / folder / 'audit.json').read_text())
    paths = [f['path'] for f in report['files']]
    assert not any('.worktrees' in p.split('/') or 'worktrees' in p.split('/') for p in paths)
    print(folder, report['summary'])
    print('Excluded:', report['excluded_tree_paths'][:12])
PY
```

If counts differ, check HEAD, new tests, feature-gated modules, scan mode,
and tool version. Do not interpret a changed count as code reduction without
comparing the actual tests.

## Step 1: remove repeated Smash match setup, keep all cases

**First edit:** `game/ambition_app/tests/smash_ride.rs` (3,259 lines, 14 tests at
baseline). The analyzer found 882 line positions in repeated code. Fourteen
tests construct the shipped Bevy app, and 13 share match-ready polling logic.

Build small, private helpers in that file first. Consider a constructor for
an independent, shipped admiral match and a bounded wait for a live round.
Only move a helper into `tests/common/` when another file needs it.

**Preserve:** one fresh `App` per test, real shipped composition, roster setup,
the human-controlled seat, and the two parts of the live-round premise:
there are seated fighters **and** none remain under `ControlHolds`. Preserve
checks on `DrivingParticipant`, `MatchSeat`, `MountReservedFor`, and `RidingOn`.
Do not replace readiness conditions with a fixed count of frames.

**Stop condition:** all 14 test cases and their distinct assertions remain.
The source file gets shorter and the tests still run in the focused lane.
Record actual before/after LOC; do not claim that all 882 matching positions
were removed.

```bash
scripts/setup/target_bindmount.sh --status
cargo test -p ambition_app --test app_it -- smash_ride::
```

Follow the bind-mount setup recipe if the target mount is not ready. Do not
run an exhaustive workspace test as the default response to a fixture-only edit.

## Step 2: simplify Versus setup without hiding phase boundaries

**Next edit:** `game/ambition_app/tests/versus_stage.rs` (3,168 lines and 32
tests at baseline). About 495 line positions are covered by repeated blocks.
The route-launch and bounded wait sequences appear at least 12 times.

Extract setup by the *state observed*, not by a single generic "ready" flag.
The following are different facts: the roster exists, the room activated, both
seats exist, and the round reached `MatchPhase::Fighting`. A test for a
transition at an earlier stage must still observe that earlier stage.

**Preserve:** controller-to-seat routing, countdown, KO freeze, score and
round reset, roster cleanup, and the shipped-host composition.

**Stop condition:** all 32 test cases keep their distinct transition and
ownership assertions, with less duplicate construction and polling code.

```bash
cargo test -p ambition_app --test app_it -- versus_stage::
```

## Step 3: classify and retire obsolete diagnostics

This is a separate change from Steps 1 and 2. The highest-confidence review
queue is `enemy_body_scale.rs` (three ignored print-oriented probes) and
`sky_census.rs` (one ignored measurement). Then review
`hall_redecode_census.rs` and the wide fighter-matrix measurement in
`smash_cpus_damage_each_other.rs`.

For **each** ignored test, record which of these roles it has:

| Role | Action |
|---|---|
| Active regression, or unique anti-vacuity control | Keep as a test |
| Diagnostic needed for an active defect | Keep available as an explicit opt-in probe |
| Repeatable performance measurement with an owner | Keep in a named measurement lane or move to a tool |
| Superseded experiment with no remaining question | Delete after checking references and callers |
| Test that needs process isolation | Keep the isolation contract and the required runner |

Check current issue and planning state before touching the 14 ignored probes
in `a_bag_changed_mid_window_reaches_the_save.rs` (599 test-body lines). Some
investigated the rollback/save issue called Q135 in the audit. The old report
does not prove that the issue remains open **today**. Likewise, retain the
failure-bisection helpers in `rollback_exit_oracle.rs` and the full collision
oracle sweep unless an equivalent tool has taken their place.

**Stop condition:** for each deletion, name the old question, why it is closed,
and where an agent should now go for diagnosis. Do not delete probes solely
because `#[ignore]` appears above them.

## Step 4: consolidate repeated inputs, not the behaviors

- `carried_item_crosses_rooms.rs`: share custody/identity observations; keep
  each room and ownership transition as a separate witnessed case.
- `repro_walls.rs`: share geometry setup; keep the 0.01-pixel penetration case
  and each other start state.
- `dash_stability.rs`: share the feet-stability measurement; keep both normal
  dash and dash-plus-down.
- `movement_axis.rs`: share the movement check; keep both left and right.
- `hall_transition_cover.rs`: share state capture; preserve the loading,
  readiness, visibility, and teardown boundaries.
- `an_edit_reaches_the_shipped_game.rs`: share content edit/reject setup;
  preserve tests for each different content reference validator.

If parameterization makes error messages less clear, keep the separate named
tests and share only their setup. A similarity score of 1.0 is **not** proof
that two inputs expose the same failure mode.

## Step 5: do a behavioral coverage review before deleting active tests

Build a small per-area matrix with these columns: test name, observed failure,
input or initial state, owning layer, production path used, other tests that
would fail for the same defect, and proposed disposition.

Focus first on the 76 hosted-Smash tests and the larger reload and two-room
modules. Use an explicit local break on a **scratch branch or disposable worktree**
when two tests appear equivalent. Verify that the retained test catches the
same defect. Restore the break before publishing any change. Mutation checks
must be focused and temporary; do not add a broad new mutation-test framework.

Do not merge Mary-O and Sanic acceptance tests just because their code is
identical. Each standalone app must prove that it wires the common behavior.
Shared fixtures are fine; one app's passing test is not evidence for the other.

## Protect these behaviors throughout the campaign

- Rollback registration coverage and poison controls in `rollback_coverage.rs`.
- Changed state across a real rewind in `rollback_exit_oracle.rs`,
  `rollback_populated_timeline.rs`, and the active save-mirror tests.
- Room re-entry, replay, and saved state in `canonical_reconstitution.rs`.
- Checkpoint death and item custody in `death_restores_the_checkpoint.rs` and
  `carried_item_crosses_rooms.rs`.
- Cross-game session cleanup in `shell_host_lifecycle.rs` and
  `shell_host_rendered.rs`.
- Actual input-to-action and composition paths in `smash_in_the_host.rs` and
  `an_edit_reaches_the_shipped_game.rs`.
- Scheduler order and cycle detection in `update_schedule_census.rs`.
- Two peers and two live rooms in `two_peers.rs` and
  `two_players_two_live_rooms.rs`.
- The test discovery guard in `app_it_sync.rs`.

Protect the **failure mechanisms**, not the current file size or helper design.
An extracted helper must not remove a test's premise or make its assertion
vacuous. A passing test that never enters its intended world is not evidence.

## Acceptance evidence after each edit

1. Check `git status` and record the current HEAD. Do not modify files outside
   the current task. Do not overwrite planning documents owned by other work.
2. Capture the names and ignored status of the tests affected by the change.
   For pure fixture extraction, the list should not change. For a deliberate
   removal, record each deleted test and the coverage that remains.
3. Check the changed file's real source LOC and repeated-block footprint.
   Count only actual removed or shared code; do not add overlapping matches.
4. Run the **focused** `app_it` module checks. Check `app_it_sync` when modules
   or file layout changes. Follow the feature-specific rows in the cheapest
   sufficient check recipe. Do not let lint-only failures block evidence.
5. For an actual behavioral deletion, use a targeted test break when practical.
   Check that the retained test fails for the defect. Remove the break and
   rerun the retained test.
6. Report the result with three labels: **consolidated implementation LOC**,
   **retired diagnostic LOC**, and **removed behavioral tests**. Do not combine
   them into one claimed coverage gain.

Do not make the next agent add a test simply because it changed code. Before
adding one, find the nearest existing test and say which new failure mechanism
the new test covers. If an old test has become redundant, retire it in the
same change with evidence. If nothing is wrong, make no additional tests.

## Next handoff

Start with **Step 0** to verify the corrected scanner against the live checkout,
then make **Step 1** the first implementation change. Do not modify production
Rust to make tests easier to write. Leave the existing audit findings and older
planning documents intact, except for files this campaign has explicitly
created or obtained permission to edit.
