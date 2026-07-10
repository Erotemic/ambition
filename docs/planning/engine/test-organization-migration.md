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

### Task 7 — custom determinism scanner (LANDED)

`crates/ambition_runtime/tests/determinism_lints.rs` → `custom:determinism`
(scanner `src/custom/determinism.rs`, config `policies/determinism.toml`). The
semantic analysis (bare-vs-FQ `HashMap`, Bevy-hasher discrimination, binding
tracking) stays Rust; only configuration moved to data: sim roots (each SCOPED so
`engine_policies` scans `crates/*` and `game_policies` scans `game/*` content +
demo rules — independently runnable), excluded subpaths, the
`AMBITION_REVIEW(determinism)` marker, forbidden RNG crates/calls, wall-clock
reads, and the source doc.

| old test fn (determinism_lints.rs) | destination |
| --- | --- |
| `sim_crates_pull_in_no_ambient_rng` | custom:`determinism` (rule 1a, manifest read directly from workspace-relative root — no doubled `crates/`) |
| `sim_sources_call_no_global_rng` | custom:`determinism` (rule 1b) |
| `sim_sources_read_no_wall_clock` | custom:`determinism` (rule 2) |
| `sim_sources_never_iterate_a_std_hash_container` | custom:`determinism` (rule 3) |
| `sim_sources_never_sort_by_entity` | custom:`determinism` (rule 4) |
| `rule_three_sees_a_bare_hashmap_and_not_a_bevy_one` | custom:`determinism::poison_self_tests` (bare-std detected, Bevy not a false positive) |
| `reviewed_determinism_exceptions_are_listed` | **removed** — a pure `println!` inventory, not a guard; the marker itself is exercised by the poison self-test |

Improved over the original: poison self-tests now cover EVERY rule (global RNG,
wall-clock, std-hash iteration, entity sort, review-marker suppression), not just
rule 3; a real-config poison (`thread_rng()` injected into `ambition_engine_core`)
reddens `engine.determinism` with file:line, confirming the shape real code uses.
Old file deleted after parity.

### Task 8 — custom ControlFrame scanner (LANDED)

`crates/ambition_runtime/tests/control_frame_lint.rs` → `custom:control_frame`
(scanner `src/custom/control_frame.rs`, config `policies/control_frame.toml`). The
holder detection (`Res<…ControlFrame>`/`ResMut`/`World` access, enclosing-fn
attribution, whole-word matching, `#[cfg(test)]` skipping) stays Rust; the
justified allowlist (file + fn + bridge category + reason), scoped roots, excluded
subpaths, and review marker moved to data. Bidirectional (unlisted holder AND
stale allowlist entry fail) and scope-split (engine crates/* + game
ambition_content run independently). A failure names the policy ID, owning crate,
source path:line, enclosing fn, and the review mechanism.

| old test fn (control_frame_lint.rs) | destination |
| --- | --- |
| `control_frame_holders_match_the_allowlist` | custom:`control_frame::run` (bidirectional) |
| `every_allowlist_entry_is_justified` | custom:`control_frame::allowlist_is_justified` (why>40, Slot0Gesture ⇒ MULTIPLAYER TODO) |
| `the_lint_catches_an_injected_sim_reader` | custom:`control_frame::poison_self_tests` |
| `the_lint_catches_a_reader_written_through_any_import_path` | custom:`control_frame::poison_self_tests` |
| `the_lint_ignores_near_misses` | custom:`control_frame::poison_self_tests` |
| `the_lint_skips_cfg_test_modules` | custom:`control_frame::poison_self_tests` |
| `the_review_marker_suppresses_a_holder` | custom:`control_frame::poison_self_tests` |
| `reviewed_control_frame_exceptions_are_listed` | **removed** — println inventory of markers + Slot0Gesture entries; the Slot0Gesture-must-say-TODO check survives in `allowlist_is_justified` |

Real-config poison: an unlisted `Res<ControlFrame>` holder injected into
`ambition_actors` reddens `engine.control-frame` with fn + file:line + the fix.
Determinism + control-frame are kept SEPARATE scanners (not merged into one generic
parser) — the merge would not reduce code or preserve clarity. Old file deleted
after parity.

### Task 9 — finish decomposing architecture_boundaries.rs (COMPLETE — file DELETED)

All 67 original `architecture_boundaries.rs` tests migrated: **65 declarative** (in
`policies/engine.toml` + `policies/game.toml`) and **2 custom** scanners
(`custom/lifecycle.rs` raw-spawn allowlist, `custom/content_ownership.rs`
archetype-free enemy config). Two new declarative rule kinds — `file-contains` /
`file-omits` — carry the facade re-export + burned-down-facade shape. Delivered in
3 batches (batch 1 in Task 6; batches 2 + 3 here), each ran old + new green together
before removing the fns; the file was deleted only after the matrix reached zero
`legacy-pending`. The migration-matrix self-test forced honesty at every step.

Three originals were silently VACUOUS (scanned dirs that had moved to their own
crates): `combat_kit_stays_content_free` → re-pointed at `ambition_combat/src`;
`presentation_does_not_use_the_archetype_enum` → `ambition_render/src`;
`portal_core_*` gravity/roster scans of the deleted `ambition_actors/src/portal` →
`ambition_portal/src`. Re-pointing at the real homes made them meaningful (an
improvement demanded by "a green empty scan is a failure").

`cargo test -p ambition_app --test architecture_boundaries` references removed from
`docs/architecture/architecture-boundaries.md` and ADR 0019; guard-location
pointers in ADR 0023, netcode.md, unified-actors.md updated to the policy package.
The policy suite runs without compiling or linking `ambition_app`.

**Migration matrix: 65 declarative + 2 custom + 0 legacy-pending = 67. Complete.**

<!-- MIGRATION-MATRIX-END -->

## Task 10 — compile-impact measurements

| metric | before | after |
| --- | --- | --- |
| dedicated policy test binaries | 6 (+1 inline lib test) | **1** (`tests/policy.rs`) |
| edit-test loop: touch policy **Rust** → recompile | 13.6 s (architecture_boundaries, links all of ambition_app) / 8.0 s (determinism) | **0.47 s** (policy pkg only) |
| edit-test loop: touch policy **data** (`policies/*.toml`) → recompile | n/a (was Rust literals: 8–13.6 s) | **0.14 s** (data read at runtime; nothing recompiles) |
| touch a watched **production** file → policy pkg rebuild | — | **0.14 s** (no rebuild — policy pkg has no production dep) |
| running policy suite compiles `ambition_app`? | yes (architecture_boundaries) | **no** — building the policy pkg compiles **zero** production crates |
| warm policy execution (all scopes + self-tests) | — | **0.95 s** |
| policy pkg cold compile (one-time) | — | 10.8 s (incl. the new `toml` dep) |
| policy pkg dependencies | (each binary linked its production crate) | `serde` + `toml` (+ `walkdir`, `serde_spanned`, `toml_datetime`, `toml_parser`, `winnow`) — no `cargo_metadata`, no production crate |

Repository-policy LOC: **before** 4,769 in `crates/**` + `game/**` test dirs (7
files, all deleted) + ~47 inline in `ambition_world/src/lib.rs`. **After** ~6,463 in
the sequestered package (2,477 Rust scanner/machinery compiled ONCE + 3,057 TOML
policy DATA + 444 `tests/policy.rs` + 485 fixtures/frozen-list/matrix). Total LOC
rose (data-driven TOML is more verbose than dense Rust literals, and the honesty
infrastructure — frozen list + migration matrix + poison fixtures — is new), but
**production source navigation is 4,769 test-LOC lighter** and the shared scanner
compiles once instead of being re-implemented per test file.

Result vs. the intended outcomes: fewer policy binaries (6→1) ✓; shared scanner
compiled once ✓; policy-only edits don't rebuild production crates ✓; running policy
never compiles `ambition_app` ✓; targeted policy execution materially cheaper (13.6 s
→ 0.47 s Rust, 0.14 s data) ✓. Full-workspace `--no-run` is neutral-to-better: the
single most expensive test binary (`architecture_boundaries`, which linked all of
`ambition_app`) plus five others are gone, replaced by one tiny data-driven package.

## Commands (target model)

```bash
cargo test -p ambition_workspace_policy repository_policies
cargo test -p ambition_workspace_policy engine_policies
cargo test -p ambition_workspace_policy game_policies
cargo test -p ambition_workspace_policy               # all scopes + self-tests
cargo test --workspace                                # handoff / merge gate
```
