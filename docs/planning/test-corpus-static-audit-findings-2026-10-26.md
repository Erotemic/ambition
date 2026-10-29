# Ambition test-suite audit: static-analysis findings

Date: 2026-10-26

Execution handoff: [Test-suite consolidation action plan](test-suite-consolidation-action-plan-2026-10-26.md).

Source data: `ambition-test-audit-results.tar.gz`, with `ambition-app-test-audit/audit.json` and `ambition-workspace-test-audit/audit.json`. The review also examined representative Rust source from `Erotemic/ambition` on GitHub. This is a static audit; Cargo tests, mutation tests, and execution-time profiling were not run.

## Executive findings

1. **The workspace scan is contaminated by alternate checkouts.** It traversed four directories under `.worktrees/`. Consequently, its 8,487 test-bearing files, 51,021 extracted test functions, 619,004 duplicate windows, and 250 top near-duplicate pairs must not be used as base-repository measurements.
2. **The corrected base-checkout inventory** contains 1,881 test-bearing Rust files, 11,052 extracted test functions, 80 ignored tests, 61 probe-named tests, and 963,695 entire-file lines. The entire-file number includes production code in modules that also contain tests; it is *not* test LOC.
3. **The `ambition_app` scan is usable.** It contains 283 Rust files under `tests/`, 36 source files with test markers, and 1,673 extracted test functions (1,419 under `tests/`, 254 in source modules). Of the 1,673 functions, 56 are ignored.
4. **The chief verified duplication is setup, not whole tests.** The app has 314 overlapping 10-line matching-window groups. Merging overlapping line positions yields 3,826 distinct matching line positions in 219 per-file regions across 59 files. This is a footprint of code repetition, not an estimate of deletable LOC.
5. **Only six high-similarity test pairs** were reported within `ambition_app`. Source inspection shows that the pairs vary meaningful inputs (subpixel wall penetration, dash-plus-down, direction, missing cutscene versus missing boss). These cases should be parameterized if useful, but not deleted merely because they look alike.
6. **Ignored diagnostics are a tractable trim queue, not the principal cause of test size.** All 56 ignored app tests account for approximately 2,909 test-function-body lines. The 14 ignored functions in `a_bag_changed_mid_window_reaches_the_save.rs` account for 599 of those lines. Preserve diagnostic scripts still needed for an active rollback issue.

## Corrected inventory

| Metric | Contaminated `--scope .` | Base checkout (excluding `.worktrees/`) | `ambition_app` |
|---|---:|---:|---:|
| Test-bearing Rust files | 8,487 | 1,881 | 319 |
| Extracted test functions | 51,021 | 11,052 | 1,673 |
| Ignored functions | 354 | 80 | 56 |
| Probe-named functions | 289 | 61 | 45 |
| Entire-file LOC in scanned files | 4,434,787 | 963,695 | 158,474 |

The workspace scan included 6,606 file entries from `.worktrees/ab-f40`, `.worktrees/agent-worktree1`, `.worktrees/agent-worktree2`, and `.worktrees/sidework`. These are *other checkouts*, not more test coverage. The 250 highest near-duplicate pairs in that scan all have at least one worktree path and are therefore uninformative for base-checkout redundancy.

In `ambition_app` specifically:

- `tests/`: 283 files, 134,374 whole-file lines, 1,419 extracted tests, 55 ignored functions.
- Test-marked `src/`: 36 files, 24,100 whole-file lines, 254 extracted tests, one ignored function.
- `tests/` comments: 29,569 lines, 22.0% of `tests/` whole-file LOC.
- The top 20 `tests/` source files contain 42.3% of its whole-file lines.
- Extracted test body length: median 37 lines, mean 48.8, approximately 90th percentile 94.
- 97 extracted tests have no *direct* assertion according to the scanner; 56 of these are not ignored. This is a call-graph-review queue, **not** a deletion queue. Some invoke assertion-bearing helpers or deliberately prove that real code does not panic.

## Refactoring targets: proven repeated code

The numbers below are *distinct line positions covered by one or more matching 10-line windows*, including all copies. They must not be presented as likely savings.

| File | Repeated line positions | Merged local regions | Treatment |
|---|---:|---:|---|
| `game/ambition_app/tests/smash_ride.rs` | 882 | 27 | P0: extract a local shipped-Smash fixture; preserve live-round premise and per-seat/summoner assertions. |
| `game/ambition_app/tests/versus_stage.rs` | 495 | 30 | P0: centralize route entry/ready and round-phase wait without forcing tests that need different readiness conditions to use one incorrect condition. |
| `game/ambition_app/tests/carried_item_crosses_rooms.rs` | 252 | 18 | P1: share repeated custody/identity observation helpers; keep separate ownership transitions and room lifetimes. |
| `game/ambition_app/tests/repro_walls.rs` | 205 | 9 | P1: parameterize geometries/starting poses while retaining the subpixel case. |
| `game/ambition_app/tests/hall_transition_cover.rs` | 184 | 13 | P2: share cover state capture/readiness setup, preserving independently checked stage-boundary transitions. |
| `game/ambition_app/src/menu/kaleidoscope_app/lunex_kaleidoscope_app_tests.rs` | 132 | 10 | P2: group repeated UI interactions if readability improves. |
| `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs` | 116 | 10 | P1: share harness/timing setup; treat ignored diagnostic cleanup separately. |
| `game/ambition_app/tests/director_teleport_blink.rs` | 105 | 3 | P2: centralize the three identical match-live preconditions. |
| `game/ambition_app/tests/smash_in_the_host.rs` | 86 | 7 | P2: simplify repeated route activation; the 76 hosted-input cases need a behavioral matrix before consolidation. |

The app has 314 overlapping repeated-window groups, but the merged footprint is only 3,826 positions across its 158,474 scanned lines. Literal duplication alone does not explain the entire test-to-source ratio; repetitive unique scenarios, large real-host fixtures, lengthy test histories, and the number of distinct cases also contribute.

### Suggested first change: `smash_ride.rs`

- Introduce a **test-only** `admiral_match()` / `wait_until_cast_live()` helper, ideally in a local `tests/common/smash.rs` module only if several files will use it.
- Preserve the actual shipped `build_visible_app` composition and independently constructed `App` per test (avoid shared mutable app state).
- Preserve the bounded wait's two-part premise: *fighters exist* and *no fighter remains under `ControlHolds`*. A fixed frame count is not equivalent.
- Preserve ownership-sensitive checks on `DrivingParticipant`, `MountReservedFor`, `RidingOn`, and `MatchSeat`; pulling them into a helper must not change their meaning.
- Preserve all 14 test functions initially. Measure the reduction before considering any behavioral deletion.

### Suggested second change: `versus_stage.rs`

- Consolidate common entry/readiness code, but preserve test-specific boundary observations: roster installed, room active, seats present, or fighting phase reached are not interchangeable.
- Do not silently replace earlier observations with a later all-in-one readiness condition if the test explicitly validates an intermediate phase.
- Keep countdown, KO freeze, roster ownership, and controller routing assertions.

## Near-duplicate tests: retain the behaviors

| Pair/family | Why it matches | Correct treatment |
|---|---|---|
| `repro_walls.rs`: square wall cling versus subpixel penetration, and mob-lab variant | Similar geometry setup but different initial contact/scene conditions. Similarity score was **1.0** for the first two despite a meaningful 0.01-pixel perturbation. | Parameterize or share a geometry fixture; retain all three scenarios. |
| `dash_stability.rs`: horizontal dash versus dash-plus-down | Slide adds vertical input and can exercise a different body-mode transition. | Share trajectory assertion; retain both inputs. |
| `movement_axis.rs`: move left versus right | Opposite signs are separate control routing bugs. | Table-driven symmetric test is fine; retain both signs. |
| `an_edit_reaches_the_shipped_game.rs`: missing cutscene versus missing boss referenced by quest | They exercise different content reference validators. | Share edit/rejection harness, retain both validators. |

**Do not delete any of these six pairs from similarity scores alone.** The analyzer scores structural resemblance; it does not encode equivalent failure detection.

## Exact body fingerprints across the corrected base checkout

- `ambition_app`: **zero** repeated test-body fingerprints.
- Base workspace: **12 pairs** of matching fingerprints, involving 24 tests in total.
- Several are cross-provider parity witnesses in Mary-O and Sanic demo app tests (`room_replay.rs`, `ov1_draws_the_world.rs`, `exit_3.rs`). Shared assertions are desirable, but a successful run of one provider does not prove the other provider's app wires the system correctly.
- The other exact matches include small math/rotation conventions, time-scale fallback, portal edge cases, and content contract fragments. Inspect the invoked functions and inputs before considering any of these removable.

This argues against blindly treating token-identical test bodies as redundant coverage. It also shows the biggest opportunity is not wholesale copy-paste duplicate tests.

## Ignored diagnostics: classify rather than bulk delete

| File | Ignored functions | Approx. body LOC | Proposed disposition |
|---|---:|---:|---|
| `a_bag_changed_mid_window_reaches_the_save.rs` | 14 | 599 | Review per probe. Retain active rollback/Q135 diagnostics until replaced or issue closed; archive obsolete exploratory traces. |
| `how_much_of_the_peer_checksum_actually_varies.rs` | 4 | 236 | Treat as rollback investigative instrumentation; decide against an open-issue list. |
| `sky_census.rs` | 1 | 186 | Candidate to move to an opt-in measurement script or delete when no customer remains. |
| `hall_redecode_census.rs` | 1 | 173 | Retain as explicit performance/asset-read investigation if still used; otherwise archive. |
| `collision_invariant_oracle.rs` | 3 | 171 | Protect the diagnostic full sweep or replace with equivalent opt-in fuzz harness. |
| `smash_cpus_damage_each_other.rs` | 2 | 147 | Keep behavior assertion; move one wide fighter-matrix measurement out of normal regression tests if appropriate. |
| `enemy_body_scale.rs` | 3 | 136 | High-priority review for retirement; all three are ignored print-oriented measurements. |
| `rollback_exit_oracle.rs` | 2 | 135 | Preserve the failure-bisection capability; it is useful when the active oracle goes red. |
| `rollback_lifecycle_reset.rs` | 2 | 105 | Keep targeted rollback diagnostics if they remain the supported troubleshooting tools. |

The 56 ignored test bodies total 2,909 lines; this is at most a modest reduction compared with 134,374 lines under `tests/`. Removing their bodies may not eliminate helpers that remain shared with active tests, so even 2,909 is not a guaranteed net saving.

## Preserve these regression mechanisms

- **Rollback completeness / anti-vacuity:** `rollback_coverage.rs` uses census and deliberately unregistered poison state. Preserve checks that the checker detects its own planted failures.
- **Dynamic rollback correctness:** `rollback_exit_oracle.rs`, `rollback_populated_timeline.rs`, and active checks in `a_bag_changed_mid_window_reaches_the_save.rs` exercise changed state across a real rewind. Registering a type is not the same as restoring it correctly.
- **Canonical identity and persistence:** `canonical_reconstitution.rs`, `death_restores_the_checkpoint.rs`, `carried_item_crosses_rooms.rs`. They witness lifecycle transitions and ownership boundaries.
- **Composed host contracts:** `shell_host_lifecycle.rs`, `shell_host_rendered.rs`, `smash_in_the_host.rs`, `an_edit_reaches_the_shipped_game.rs`. Tests that only call a component directly cannot replace an application wiring test.
- **Bevy schedules and test visibility:** `update_schedule_census.rs` and `app_it_sync.rs`. The latter prevents `autotests = false` from silently omitting new integration test files.
- **Two-peer/room and deterministic behavior:** `two_peers.rs`, `two_players_two_live_rooms.rs`, collision geometry and tick-step oracles.

Protect the *failure mechanisms*, not the present shape or exact size of each test module. Fixture and comment refactors are allowed if they do not weaken assertions or widen production APIs.

## Suggested order of work

1. **Fix scanner boundaries:** use `git ls-files` or exclude `.worktrees/`, `target/`, and generated/vendor copies. Run near-duplicate selection *after* filtering, not before a global top-N cap. Update terminology: "test-bearing file LOC" is not "test LOC".
2. **Refactor repeated app startup:** `smash_ride.rs` and `versus_stage.rs`, retaining all cases and assertions. Run focused `app_it` checks according to `docs/recipes/cheapest-sufficient-check.md`.
3. **Audit optional diagnostic retention:** first `enemy_body_scale.rs`, `sky_census.rs`, then the rollback-specific probes after consulting active issue state. Do not treat ignored as obsolete by definition.
4. **Table-drive meaningful variants:** wall penetration, dash/slide, left/right axis. Keep every case and per-case error context.
5. **Audit cross-app duplication:** compare shared fixture code in Mary-O and Sanic, but keep each shipped-app acceptance witness.
6. **Run mutation experiments on selected seams:** make a bounded wait return early, omit a registry, skip a `ShellCommand` consumer, or break rollback restore. Confirm remaining tests fail for the intended reason before deleting any assertions.
7. **Measure runtime costs separately:** collect per-test duration and app boot counts. Source duplication and slow tests are related but not the same objective.

## Acceptance conditions for each cleanup

- Same distinct preconditions, observed outcomes, and scenario inputs before/after, or an explicit reviewed semantic substitution.
- No silent loss from `app_it` module registration. Run `app_it_sync` and the directly affected module filter.
- No broken poison/non-vacuity controls.
- Relevant feature configurations still compile and run, including feature-gated integration tests where touched.
- A focused intentional break of the relevant production behavior still fails the retained test (where practical).
- Report LOC savings *after* compiling, not as estimated savings from duplicate-window counts.

## Limitations

- No full Rust AST, type resolution, call-graph reachability, mutation score, or runtime cost data is included in the uploaded audit.
- Ten-line whitespace-normalized matches miss shorter, renamed, and semantically similar fixtures and overcount overlapping windows.
- The test fingerprint is a *shape* measure. It can consider two variants identical even when a literal value changes the scenario.
- No exact test has been proved removable by this analysis alone.
- File line counts and extracted test-body counts use different denominators; do not compare either one to the user's earlier LOC measurement without harmonizing its classification rule.
