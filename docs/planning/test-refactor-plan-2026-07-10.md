# Full Ambition test-organization refactor plan

## Current baseline

After the completed policy campaign:

* approximately **258,497 Rust LOC**;
* approximately **86,137 identifiable test LOC**;
* tests are about **33.3% of Rust LOC**;
* approximately **58,054 test LOC** is already sequestered in dedicated test files or packages;
* approximately **28,083 test LOC** remains inline in production-named files;
* `ambition_workspace_policy` now owns repository-policy and source-scanning ratchets;
* eleven implementation files have had large inline test modules extracted;
* `game/ambition_app/tests` still contains roughly forty integration-test source files;
* several system-contract, scenario, diagnostic, and reliability concerns remain mixed together.

The completed campaign should be treated as **Phase 1**, not the whole effort.

---

## Phase 1 — close the policy-runner campaign cleanly

This is a small cleanup phase, not another redesign.

1. Correct the final report:

   * five module-size waivers remain, not six;
   * eleven files were extracted across nine crates;
   * record the exact LOC counting method;
   * say the campaign introduced no known regression rather than claiming a fully green workspace gate.

2. Normalize custom-policy metadata:

   * stable policy ID;
   * repository, engine, or game scope;
   * canonical owning workspace packages;
   * watched roots;
   * source document;
   * severity;
   * reviewed exceptions.

3. Extend policy metadata self-tests to custom scanners, not only declarative TOML rules.

4. Split the policy runner’s poison/self-tests into separately named `#[test]` functions while retaining one compiled integration-test binary.

5. Keep the existing independently runnable scopes:

```bash
cargo test -p ambition_workspace_policy repository_policies
cargo test -p ambition_workspace_policy engine_policies
cargo test -p ambition_workspace_policy game_policies
cargo test -p ambition_workspace_policy
```

### Exit criterion

The workspace-policy migration ledger is accurate, all custom and declarative policies have uniform ownership metadata, and no further repository-policy test remains under `crates/**` or `game/**` accidentally.

---

## Phase 2 — finish extracting large inline private tests

This is the largest navigation cleanup.

The rule is:

> Keep tests crate-local and privately scoped, but move a large `#[cfg(test)] mod tests { ... }` out of the primary implementation file.

Preferred form:

```text
src/foo.rs
src/foo/tests.rs
```

with:

```rust
#[cfg(test)]
mod tests;
```

The extracted module may continue to use:

```rust
use super::*;
```

No visibility changes are allowed.

### Wave 2A — remaining modules of at least 500 lines

Current scope:

* 4 files;
* approximately 2,556 test LOC.

Likely candidates include:

* `ambition_characters/src/brain/smash/mod.rs`
* `ambition_actors/src/features/ecs/perception.rs`
* `ambition_actors/src/enemy_projectile/systems.rs`
* `ambition_characters/src/perception.rs`

Each commit should touch one implementation file and its new test module only.

### Wave 2B — 400–499-line test modules

Current scope:

* 4 files;
* approximately 1,790 test LOC.

Examples:

* boss-pattern control flow;
* Gnu Ton content behavior;
* player-state tests;
* boss-pattern validation.

### Wave 2C — 300–399-line test modules

Current scope:

* 10 files;
* approximately 3,479 test LOC.

Group only when several files belong to one tightly related module tree and rustfmt cannot cascade into unrelated siblings. Otherwise commit per file.

### Wave 2D — 200–299-line test modules

Current scope:

* 18 files;
* approximately 4,342 test LOC.

At the end of this wave, all remaining inline modules of at least 200 lines should be gone.

Total removed by Waves 2A–2D:

* 36 production files;
* approximately **12,167 inline test LOC**.

### Wave 2E — 150–199-line test modules

Current scope:

* 22 files;
* approximately 3,765 test LOC.

Do these when the test block materially obscures the implementation. A file whose test block is 150 lines but only 15% of a large, coherent implementation may reasonably remain inline. A file that is 40–60% tests should be extracted.

### Wave 2F — optional 100–149-line cleanup

Current scope:

* 42 files;
* approximately 5,120 test LOC.

Do not blanket-move these.

Extract when at least one is true:

* tests exceed roughly one third of the file;
* tests introduce large fixture/helper sections;
* tests make implementation navigation difficult;
* the file already exceeds the module-size target;
* tests are behavioral scenarios rather than explanatory examples.

Keep compact explanatory tests inline.

### Special cases

Some files do not contain one clean terminal `mod tests` block. They may include:

* test-only trait implementations;
* test-only helper types interspersed with implementation;
* multiple `#[cfg(test)]` sections;
* macros needed by both implementation and tests.

Handle these in a separate sub-wave. First move test-only helpers into:

```text
src/foo/test_support.rs
src/foo/tests.rs
```

Both remain behind `#[cfg(test)]`.

Do not contort production structure merely to achieve an LOC statistic.

### Validation per extraction

For every commit:

```bash
cargo fmt --check
cargo test -p <owning-crate> --lib
git diff --check
```

Also verify:

* test count unchanged;
* test names unchanged;
* no visibility changed;
* no implementation logic changed;
* no unrelated rustfmt cascade;
* only intended files touched.

### Exit criterion

All inline test modules of at least 200 lines are extracted. The 150–199-line population has been reviewed individually. Remaining inline tests are intentionally small, explanatory, or structurally inseparable.

### Execution ledger (2026-07-11, Opus 4.8)

Phase 2 **executed and complete**. Actuals differed from the plan's estimates
(the estimates predated a fresh measurement); the measured routing was:

* **Wave 2A (≥500 tail LOC): 6 files** — `smash/mod.rs` (29), `features/bosses.rs`
  (23), `features/enemies/mod.rs` (17), `features/ecs/perception.rs` (12),
  `enemy_projectile/systems.rs` (7), `characters/perception.rs` (20).
* **Wave 2B (400–499): 5 files** — `boss_pattern/control_flow.rs`,
  `boss_pattern/validator.rs`, `bosses/gnu_ton.rs`, `player_state.rs`, `world.rs`.
* **Waves 2C+2D (200–399): 33 files** — swept together; **no inline block ≥200
  lines remains anywhere in the workspace.**
* **Wave 2E (150–199): 14 of 25 files** — extracted where the test block was
  ≥40 % of the file; the 11 low-ratio files stay inline by the plan's own rule.
* **Wave 2F (100–149): 29 of 50 files** — same ≥40 % threshold; 21 low-ratio
  files stay inline.

Totals: **87 implementation files touched, 99 new adjacent test-module files,
687 `#[test]` functions relocated (~21 k test LOC moved out of production
files).** Every extraction is a pure move — same test names + logic, private
`use super::*`, direct-sibling module depth (so `super::…` paths are unchanged),
one implementation file per commit.

Conventions established for future waves:

* single terminal `#[cfg(test)] mod tests` → `foo/tests.rs`;
* a single named block (e.g. `mod dash_tests`) → `foo/dash_tests.rs` (name kept);
* several contiguous tail blocks → one **sibling file each** (never a nested
  `tests` wrapper — the extra layer breaks `super::super::…` paths);
* trailing test-only helpers (e.g. a `CountOrNone` trait) fold into the test
  file, shedding their now-redundant `#[cfg(test)]` guards;
* **relative `include_str!` paths must be re-anchored** (`"foo.rs"` →
  `"../foo.rs"`) — caught in `falling_sand.rs` and `clip_material.rs`;
* crate roots (`lib.rs`/`main.rs`) and `mod.rs` resolve submodules in the same
  directory; never `rustfmt` a module root directly (it cascades into siblings);
* `mod` declarations are emitted alphabetically to satisfy `reorder_modules`.

Side effect: two module-size waivers (`smash/mod.rs`, `falling_sand.rs`) fell
below the 1500-line gate once their tests moved out and were **deleted** from
`module_size.toml` — decomposition debt paid down.

All 13+ touched crates and `ambition_workspace_policy` are green.

---

## Phase 3 — normalize crate-local test structure

After large extraction, make local layout predictable.

Use these conventions:

```text
src/foo.rs                    # implementation
src/foo/tests.rs              # tests for one module
src/foo/tests/fixtures.rs     # local fixtures when needed
src/foo/tests/properties.rs   # large property/metamorphic group
```

For directory modules:

```text
src/foo/mod.rs
src/foo/tests.rs
```

or:

```text
src/foo/tests/
├── mod.rs
├── behavior.rs
├── edge_cases.rs
└── fixtures.rs
```

### Rules

* `tests.rs` remains private and crate-local.
* Test helpers used by one module stay under that module.
* Helpers used by several modules in one crate may move to `src/test_support/`, gated by `#[cfg(test)]`.
* Do not create a public `test_utils` API.
* Do not place ordinary behavioral tests in the workspace-policy package.
* Do not move tests to crate integration directories solely to remove them from source files.

### Targeted cleanup

Normalize inconsistent patterns such as:

* test blocks still embedded after implementation;
* files named `test.rs`, `tests.rs`, and `*_tests.rs` without a consistent reason;
* repeated test fixture builders inside one crate;
* test-only imports scattered through production code;
* large test data constants embedded in implementation files.

### Exit criterion

A reader can predict where private tests for any implementation module live, and production files no longer contain large test harnesses.

---

## Phase 4 — audit integration-test ownership

Not every file under `game/ambition_app/tests` belongs to the assembled app.

Review each integration test and classify it as one of:

1. crate-public behavior;
2. cross-crate engine contract;
3. assembled game contract;
4. scenario regression;
5. diagnostic or fuzz oracle;
6. smoke or configuration parity;
7. misplaced unit test.

### Ownership rule

Move a test downward when the lower-level crate can express the same contract without depending on `ambition_app`.

Examples to investigate:

* gravity symmetry;
* unified body movement;
* movement-axis behavior;
* phase-split contracts;
* replay primitives;
* projectile and portal translation behavior.

Do not move a test merely because its subject sounds low-level. If it depends on assembled plugins, authored rooms, app scheduling, or content, it belongs at the app/game level.

### Expected outcome

Some tests will remain in `ambition_app`; some should migrate to:

* `ambition_engine_core/tests`;
* `ambition_runtime/tests`;
* `ambition_portal/tests`;
* `ambition_actors/tests`;
* another clearly owning crate.

Every move must reduce dependencies or improve ownership. Do not create new cross-crate test packages merely to move files around.

### Exit criterion

Every integration test has an obvious owner and no test compiles the entire app when a lower-level public contract is sufficient.

---

## Phase 5 — reduce integration-test binary count

Cargo compiles each top-level file in a package’s `tests/` directory as a separate binary.

`game/ambition_app/tests` currently has roughly forty test source files. This should be measured and rationalized, but not blindly collapsed.

### Pilot

Create one grouped runner for a coherent family, such as portal contracts:

```text
game/ambition_app/tests/
├── portal_contracts.rs
└── suites/
    └── portal/
        ├── mod.rs
        ├── bridge_reachability.rs
        ├── floor_bounce.rs
        ├── lab_usable.rs
        ├── reset_preserves_authored.rs
        ├── translation_camera.rs
        ├── projectile_transit.rs
        └── held_projectile_transit.rs
```

`portal_contracts.rs` should include the modules and produce one test binary.

Compare:

* compile time;
* link time;
* warm test time;
* filtering ergonomics;
* failure isolation;
* parallelism;
* incremental rebuild after editing one scenario.

### Likely suite groups

Subject to measurement:

* `portal_contracts`
* `boss_and_combat_contracts`
* `movement_and_reachability`
* `replay_and_determinism`
* `app_and_plugin_smoke`
* `possession_and_control`
* `room_and_spatial_contracts`

### Keep separate binaries for

* `collision_invariant_oracle`
* `desync_canary`
* long fuzz/random-walk tests
* tests with special process-global setup
* expensive or ignored suites that need independent filtering
* tests that intentionally run under different environment settings

### Demo applications

Each demo app currently has two small integration-test files. Consider merging each pair into one binary:

```text
game/ambition_demo_sanic_app/tests/demo.rs
game/ambition_demo_smb1_app/tests/demo.rs
```

Only retain separate binaries if their execution environments differ.

### Content package

`game/ambition_content/tests` has six binaries. Consolidate only if measurements show a benefit. Content validation, Yarn compilation, boss calibration, and catalog checks may deserve separate filtering.

### Exit criterion

The number of integration binaries is reduced where compilation improves, without turning all game tests into one monolithic binary or harming failure isolation.

---

## Phase 6 — consolidate shared test support

Repeated fixture and simulation setup should have one owner.

### App-level support

Keep app-specific support under:

```text
game/ambition_app/tests/common/
```

or rename it to:

```text
game/ambition_app/tests/support/
```

Organize by responsibility:

```text
support/
├── mod.rs
├── sandbox.rs
├── rooms.rs
├── inputs.rs
├── assertions.rs
├── replay.rs
├── snapshots.rs
└── diagnostics.rs
```

### Promote helpers only when truly cross-package

Create a non-published test-support package only when at least two independent packages need the same substantial harness:

```text
tests/ambition_test_support/
```

It may depend on production crates as necessary, unlike the policy package.

Do not create this package preemptively. First identify actual duplicated helpers.

### Rules

* no business logic in test support;
* no test support used by production code;
* no hidden global mutable state;
* fixtures must make requested room/content explicit;
* builders must assert that requested setup actually occurred;
* helper errors must preserve scenario context.

### Exit criterion

Common setup is centralized without creating a second shadow runtime or making tests harder to understand.

---

## Phase 7 — separate contract tests, scenarios, diagnostics, and exhaustive suites

The test tree should reveal intent.

### Contract tests

Fast, deterministic, merge-gating behavior:

* snapshot/restore contracts;
* mode-scope behavior;
* input replay equivalence;
* portal translation invariants;
* boss lifecycle contracts;
* public crate API contracts.

### Scenario regressions

Named past bugs and representative gameplay sequences:

* repro walls;
* possession end-to-end;
* floor bounce;
* player robot fight;
* authored room regressions.

### Diagnostic oracles

Useful measurement tools that are not always hard gates:

* collision invariant sweep;
* coverage ledgers;
* authored-content reports;
* calibration reports.

They must clearly say whether they fail or report.

### Exhaustive/fuzz suites

Long-running or ignored-by-default:

* large random walks;
* all-room sweeps;
* large seed matrices;
* exhaustive replay searches.

Move these into clearly named binaries or modules. Do not hide expensive diagnostics among fast contract tests.

### Naming convention

Use suffixes consistently:

```text
*_contract
*_regression
*_smoke
*_oracle
*_fuzz
*_exhaustive
```

### Exit criterion

A developer or agent can tell from path and name whether a test is a hard gate, scenario regression, diagnostic, or expensive exhaustive suite.

---

## Phase 8 — strengthen test non-vacuity and ownership without adding bloat

Non-vacuity assertions stay with their harnesses.

Standardize a few compact helpers:

```rust
require_room(...)
require_cases(...)
require_workspace_members(...)
require_ticks(...)
require_fixture_count(...)
```

These should produce contextual errors but remain small.

Every aggregate suite must assert:

* at least one case ran;
* every required named fixture ran;
* requested setup matched actual setup;
* no requested case silently fell back;
* ignored/excluded cases are explicitly enumerated.

Do not create a separate top-level package for these checks.

### Exit criterion

No aggregate test can pass because its discovery set was empty or because required fixtures silently disappeared.

---

## Phase 9 — test reliability campaign

The workspace still does not have a truly green all-tests gate.

Address separately:

1. Fix `the_demos_own_rules_run_because_its_room_claims_its_mode`.
2. Reproduce the full-parallel failures individually and under controlled concurrency.
3. Identify shared causes:

   * global environment variables;
   * process-global Bevy state;
   * shared files;
   * fixed ports;
   * GPU/audio/display resources;
   * excessive thread or memory pressure;
   * nondeterministic timing.
4. Replace accidental shared state with:

   * isolated temporary directories;
   * unique ports/paths;
   * scoped environment guards;
   * deterministic clocks;
   * explicit synchronization only where unavoidable.
5. Mark tests serial only when a genuine process-global resource cannot be isolated.
6. Do not dismiss consistently reproducible parallel failures as harmless flakes.

### Exit criterion

```bash
cargo test --workspace
```

is genuinely green under the supported execution environment, not merely green when tests are rerun individually.

---

## Phase 10 — CI and local execution lanes

Define explicit lanes.

### Fast local lane

```bash
cargo test -p <crate> --lib
```

### Owning integration lane

```bash
cargo test -p <crate> --test <suite>
```

### Workspace policy

```bash
cargo test -p ambition_workspace_policy
```

### Fast cross-crate contracts

A documented set of engine/game contract suites.

### Scenario lane

Representative gameplay and content regressions.

### Exhaustive lane

Long random walks, ignored sweeps, fuzzing, and large seed matrices.

### Merge gate

```bash
cargo test --workspace
```

plus any intentionally ignored exhaustive command required by release policy.

Use CI sharding by semantic lane, not arbitrary file chunks.

### Exit criterion

Agents can run the narrowest relevant lane during iteration, while CI still guarantees full coverage.

---

## Phase 11 — compile-time and binary-count optimization

Measure after each structural wave.

Track:

* total Rust test LOC;
* inline test LOC;
* number of integration test binaries;
* cold `--no-run` compile time;
* warm compile time;
* link time;
* incremental rebuild after:

  * production edit;
  * test-only edit;
  * policy-data edit;
* peak memory during workspace test compilation;
* execution time by lane.

Expected effects:

* inline extraction: compile-neutral, navigation-positive;
* integration-suite grouping: potentially fewer links, but must be measured;
* policy package: already strongly compile-positive;
* moving tests to a lower owning crate: usually compile-positive;
* creating unnecessary cross-crate contract packages: potentially negative.

### Exit criterion

The final organization is not only cleaner; common edit-test loops are neutral or faster.

---

## Phase 12 — final documentation and enforcement

Update `AGENTS.md` with the stable final rules:

* small explanatory private tests may remain inline;
* private tests above the chosen threshold normally move to adjacent modules;
* public behavior belongs in the owning crate’s integration tests;
* repository policy belongs in `ambition_workspace_policy`;
* assembled behavior remains with the assembled app/game harness;
* non-vacuity stays with the harness;
* poison tests stay with their invariant;
* expensive diagnostics are clearly classified;
* no public API widening solely for tests;
* compile measurements are required when changing test-binary topology.

Add a short test-layout document listing:

* policy package;
* local-unit convention;
* integration-suite convention;
* support modules;
* hard gates;
* diagnostic suites;
* exhaustive suites;
* exact commands.

Remove migration-only ledgers after they cease to be useful, or archive them as historical records.

---

# Final target state

```text
tests/
├── ambition_workspace_policy/   # repository/engine/game policy
└── ambition_test_support/       # only if real cross-package reuse warrants it

crates/<crate>/
├── src/
│   ├── foo.rs
│   └── foo/
│       └── tests.rs             # private local behavior
└── tests/
    ├── contracts.rs             # public crate behavior
    └── suites/                  # internal modules, few binaries

game/ambition_app/
└── tests/
    ├── movement_contracts.rs
    ├── portal_contracts.rs
    ├── boss_contracts.rs
    ├── replay_contracts.rs
    ├── scenario_regressions.rs
    ├── collision_invariant_oracle.rs
    ├── desync_canary.rs
    └── support/
```

# Completion criteria

The full test-organization refactor is complete when:

* workspace-policy tests are fully sequestered;
* custom policy ownership metadata is uniform;
* all inline test modules of at least 200 lines are extracted;
* every 150–199-line inline module has been intentionally reviewed;
* integration tests live with their narrowest real owner;
* integration binaries have been consolidated where measurement supports it;
* shared test support is centralized without becoming a shadow runtime;
* contracts, regressions, diagnostics, and exhaustive suites are visibly distinct;
* no aggregate test can pass vacuously;
* the full workspace gate is reliably green;
* compile time is neutral or improved;
* `AGENTS.md` clearly directs future agents to the correct test home.