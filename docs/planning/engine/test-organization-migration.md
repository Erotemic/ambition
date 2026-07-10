# Test-organization migration ledger

**Status:** IN PROGRESS (started 2026-07-10, commit `5950499`).
**Goal:** three clear test homes — (1) local behavioral tests inline or in adjacent
`src/foo/tests.rs`; (2) public crate/assembled-system behavior in each crate's
`tests/`; (3) workspace *policy* tests (source scans, dependency boundaries,
module-size, architecture ratchets, forbidden-name checks) sequestered in one
top-level package `tests/ambition_workspace_policy`.

This is a migration ledger, not a design doc. It is the single source of truth for
"where does every old policy test go" until the campaign closes, then it is archived
as historical evidence.

## Baseline (commit `5950499`, HEAD `8099506`)

Physical LOC (measured by `scripts`-style walk over `crates/*` + `game/*`):

| category | LOC | files |
| --- | ---: | ---: |
| production Rust (excl. inline tests) | 163,146 | 701 |
| inline `#[cfg(test)]` modules | 45,352 | — |
| adjacent test modules (`src/**/tests.rs`, `src/**/tests/`) | 32,027 | 70 |
| crate integration tests (`*/tests/*.rs`, non-policy) | 13,340 | 54 |
| **repository-policy tests (7 files)** | **4,769** | **7** |

Repository-policy test binaries **today: 6 dedicated integration binaries**
(`architecture_boundaries`, `control_frame_lint`, `determinism_lints`,
`module_size`, `observation_boundary`, `host_names_no_content`,
`legacy_runtime_guardrail` — the last two share crates so it is 6 across
ambition_app ×2, ambition_runtime ×3, ambition_render ×1, ambition_host ×1) plus
one inline lib test (`ambition_world` dependency ratchet). **Target: 1 binary**
(`tests/policy.rs`).

Policy-file sizes: architecture_boundaries.rs 2923 (67 `#[test]`),
control_frame_lint.rs 650 (8), determinism_lints.rs 577 (7), module_size.rs 216
(4), observation_boundary.rs 180 (2), legacy_runtime_guardrail.rs 165 (2),
host_names_no_content.rs 58 (2).

Compile timings — **before** (warm tree, edit-test loop), full detail in Task 10:

- full workspace `cargo test --workspace --no-run`: **3:20** (cold-ish, warm registry).
- edit `architecture_boundaries.rs` → recompile that binary (links all of
  `ambition_app`): **13.6 s**.
- edit `determinism_lints.rs` → recompile (links `ambition_runtime`): **8.0 s**.

Baseline policy tests all green: 67 + 2 + 8 + 7 + 4 + 2 + 2 + 1 = 93 policy `#[test]`s.

## Destinations (Task 1 inventory)

| old test file | destination | form |
| --- | --- | --- |
| `crates/ambition_host/tests/host_names_no_content.rs` | policy pkg | declarative: dependency-denylist + forbidden-source-reference |
| `crates/ambition_render/tests/observation_boundary.rs` | policy pkg | declarative: forbidden-source-reference (sim-state ident list) + dependency-denylist |
| `crates/ambition_world/src/lib.rs` dep ratchet | policy pkg | declarative: dependency-allowlist |
| `crates/ambition_runtime/tests/module_size.rs` | policy pkg + `policies/module_size.toml` | module-size rule |
| `game/ambition_app/tests/legacy_runtime_guardrail.rs` | policy pkg | declarative: forbidden-source-reference (production-only, ALLOW marker) |
| `crates/ambition_runtime/tests/determinism_lints.rs` | policy pkg `src/custom/determinism.rs` | custom scanner + `policies/determinism.toml` config |
| `crates/ambition_runtime/tests/control_frame_lint.rs` | policy pkg `src/custom/control_frame.rs` | custom scanner + `policies/control_frame.toml` config |
| `game/ambition_app/tests/architecture_boundaries.rs` (67 tests) | policy pkg (declarative + custom modules) | see migration matrix |

**Inspected, deliberately NOT moved** (assembled-behavior / authored-content, not
repository structure): `desync_canary.rs`, `collision_invariant_oracle.rs`,
`dialogue_lint.rs`, `boss_fight_validator.rs`, `intro_sprite_catalog.rs`,
`input_stream_replay.rs`, boss lifecycle/scenario tests, headless game scenarios.

## Scope assignment

- **repository** — workspace membership, top-level layout, umbrella/demo homes,
  the spawn allowlist, cross-crate composition that is not tied to one engine or
  game crate.
- **engine** — `crates/*` layering + foundation purity + determinism + control
  frame + module-size (engine crates dominate it).
- **game** — `game/*` content ownership, app composition, named-content
  registration.

## Migration matrix

Every old `#[test]` must appear here exactly once with a destination
(`declarative:<policy-id>`, `custom:<module>`, `retained:<where>`, or
`removed:<why>`). The completeness of this matrix is itself asserted by
`migration_matrix_is_complete` in the policy package. Filled in as each batch
lands (Tasks 4–9).

<!-- MIGRATION-MATRIX-START -->

### Task 4 — smallest pure policy tests (LANDED)

| old test fn (file) | destination |
| --- | --- |
| `manifest_does_not_depend_on_ambition_content` (host_names_no_content.rs) | declarative:`engine.host-names-no-content` |
| `sources_name_no_content_crate` (host_names_no_content.rs) | declarative:`engine.host-source-names-no-content` |
| `render_never_names_live_sim_state` (observation_boundary.rs) | declarative:`engine.render-never-names-live-sim-state` |
| `render_has_no_actor_crate_dependency_after_f15` (observation_boundary.rs) | declarative:`engine.render-no-actor-crate-dependency` + `engine.render-source-names-no-actors` (manifest + source halves) |
| `ambition_world_dependency_allowlist_ratchets_world_ir_purity` (world/src/lib.rs) | declarative:`engine.world-ir-dependency-allowlist` (exact ratchet) |

Old files deleted after parity: `crates/ambition_host/tests/host_names_no_content.rs`,
`crates/ambition_render/tests/observation_boundary.rs`, and the inline
`dependency_tests` module in `crates/ambition_world/src/lib.rs`.

### Task 5 — module-size policy (LANDED)

All four `module_size.rs` tests → `custom:module_size` (scanner
`src/custom/module_size.rs`, config `policies/module_size.toml`):

| old test fn (module_size.rs) | destination |
| --- | --- |
| `the_size_scan_is_not_vacuous` | custom:`module_size` (vacuity assert, >300 files) |
| `no_production_module_exceeds_the_size_limit_unwaived` | custom:`module_size` (oversized-unwaived diagnostics) |
| `every_waiver_names_a_currently_oversized_file` | custom:`module_size` (stale-waiver diagnostics) |
| `every_waiver_has_a_real_reason` | custom:`module_size` (reason-quality assert, >20 chars) |

All 9 waivers moved verbatim to `policies/module_size.toml`. Production/test path
classification centralized in `workspace::is_test_path` (explicit basename rules:
adjacent `tests.rs`, sibling `_tests.rs`, `/tests/` dirs — not a loose substring,
so `attests.rs` is not misread). Deleted after parity:
`crates/ambition_runtime/tests/module_size.rs`. D-B doc links updated
(decomposition.md + tracks.md) to the new location; REOPENED status unchanged.

### Task 6 — legacy guard + architecture batch 1 + migration-matrix machinery (LANDED)

Legacy guard (`legacy_runtime_guardrail.rs`, both tests) →
declarative:`game.no-legacy-runtime-in-app-src` (production-only, skip-tests,
ALLOW_LEGACY_RUNTIME marker) + a `legacy_scanner_catches_each_forbidden_identifier`
self-test over a dedicated poison fixture. Old file deleted after parity.

**Architecture matrix is now machine-checked.** `migration_matrix.toml` maps all
67 `architecture_boundaries.rs` tests to a disposition; the frozen canonical list
is `fixtures/architecture_boundaries_source_tests.txt`;
`custom::migration_matrix::check` asserts the bijection, that every
declarative/custom destination resolves to a real policy ID, and — the honesty
lock — that a migrated entry's fn is GONE from the legacy file while a
`legacy-pending` entry's fn is still there. `legacy_file_is_fully_tracked` stops a
new legacy test slipping the ledger. When the file is deleted (Task 9), zero
entries may remain `legacy-pending`.

Batch 1 (5 crate-purity tests migrated, 67→62 remaining in legacy):

| old test fn | destination policies |
| --- | --- |
| `..._render_and_actor_crates_are_decoupled` | `engine.render-decoupled-member`, `engine.actor-manifest-no-render`, `engine.actor-source-no-render` (+ Task 4's `engine.render-no-actor-crate-dependency`, `engine.render-source-names-no-actors`) |
| `..._menu_crate_stays_content_free` | `engine.menu-crate-manifest-no-actors`, `engine.menu-crate-source-no-actors` |
| `..._persistence_crate_owns_stored_shapes_only` | `engine.persistence-crate-member/-manifest-purity/-source-purity` |
| `..._encounter_crate_is_state_only` | `engine.encounter-crate-member/-manifest-purity/-source-purity` |
| `..._host_does_not_depend_on_actors` | `engine.host-crate-member`, `engine.host-manifest-no-actors`, `engine.host-source-no-actors` |

Remaining 62 architecture tests: `legacy-pending` (Tasks 7–9).

<!-- MIGRATION-MATRIX-END -->

## Commands (target model)

```bash
cargo test -p ambition_workspace_policy repository_policies
cargo test -p ambition_workspace_policy engine_policies
cargo test -p ambition_workspace_policy game_policies
cargo test -p ambition_workspace_policy               # all scopes + self-tests
cargo test --workspace                                # handoff / merge gate
```
