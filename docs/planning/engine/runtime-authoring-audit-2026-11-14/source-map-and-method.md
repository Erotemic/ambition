# Source map and audit method

## Snapshot and reproducibility

This audit uses the supplied archive `ambition-agent-source-20261114T224830Z-34f03976b639.tar.gz`. Its SHA-256 is:

```text
449fcc645c30f3beec024895e0917d27cf8caf5fb4c06c95065e2e57a39fccf9
```

The archive identifies source commit `34f03976b639`. The generated navigation stamp records `2026-11-14T22:48:34Z`. The root manifest selects Bevy `0.19.1`. Current source, rather than older architectural notes or prior conversation context, supplies implementation facts in this review.

The audit read the root guidance and architecture/planning entry points, used `scripts/agent_query.py` and the regenerated ECS inventory to select owners, parsed every workspace-member manifest, and followed selected production paths. Three parallel passes covered build/dependency architecture, gameplay composition, and content/scripting. Their findings were reconciled against the integrated design and execution plan.

The existing query tool is runnable with Python 3.11 or later and the standard library. The audit successfully used its overview, crate, text-query, and tool-discovery paths. For example, from the repository root:

```bash
python3 scripts/agent_query.py overview
python3 scripts/agent_query.py crate ambition_extension_host
python3 scripts/agent_query.py "content generation scripting"
python3 scripts/agent_query.py tools "headless inspect capture"
```

Use its per-crate shards rather than loading the full flat symbol index. Follow every consequential generated result into source. The index is navigation, not proof of active schedule behavior or dependency cost.

The included census is reproducible after unpacking this overlay at the repository root:

```bash
python3 docs/planning/engine/runtime-authoring-audit-2026-11-14/reproduce_census.py \
  --root . \
  --output /tmp/ambition-audit-census.json \
  --snapshot 34f03976b639
```

For a different revision, omit or change `--snapshot`; it is a label, not a checkout operation or source verifier. The utility reads manifests and source files, writes one JSON report, and invokes no Cargo command. Compare only like denominators between snapshots.

## Exact census scope

| Item | Result | Meaning |
| --- | ---: | --- |
| Root workspace members | 112 | 92 under `crates`, 18 under `game`, one policy package, one extension fixture |
| Generated declared-index members | 111 | Omits `ambition_extension_fixture_modules` |
| Generated resolved-graph members | 112 | Includes that fixture; the graph is workspace-wide |
| Packages in generated workspace resolve | 916 | Includes external packages; not an external consumer's selected build closure |
| Rust files under member `src` | 2,179 | No attempt to exclude inline test modules |
| Raw source lines in those files | 964,837 | Includes blank lines, comments, and inline tests |
| Nonblank source lines | 906,805 | Still not production-only code |
| Separate member `tests` files | 475 | Counted separately from `src` |
| Raw lines in those test files | 182,231 | Does not count inline tests a second time |

Independent fixture workspaces, build scripts outside `src`, scripts, generated asset products, and the interiors of authoring-tool submodules are not part of the source-line denominator. The archive contains populated SFX, music, sprite, measurement, and map tooling; those submodules are not missing. Their complete implementations were outside the deep sample.

The ECS inventory covers 111 crates and lists 1,334 registered systems, 885 components, 626 resources, 432 message channels, 2,360 registrations, and 401 spawn sites. These are static extractor categories. A registration is not necessarily one system invocation, and no count here describes the active workload for a particular scene.

### Two graph views with different purposes

The census retains declared normal, build, and development dependencies, including optional and target-specific edges. Its **potential reverse reach** uses all declared normal internal edges. It is useful for finding shared foundations whose edits may have broad consequences. It overstates any particular selected consumer when optional/target edges are inactive.

The **mandatory internal closure** follows only nonoptional, target-independent normal edges, includes the starting package, and excludes external packages, build/development dependencies, feature expansion, and target-specific requirements. It is a lower bound, not a resolved minimal build or a timing estimate.

| Starting package | Mandatory internal closure, including self |
| --- | ---: |
| `ambition_platformer2d` | 70 |
| `ambition_content_cli` | 42 |
| `ambition_engine_schemas` | 40 |
| `ambition_content_modules` | 6 |
| `ambition_content_pack` | 2 |
| `ambition_extension_sdk` | 1 |

The module crate is ECS-free, but it directly uses `bevy_math`; calling its complete dependency set Bevy-free would be inaccurate. The extension SDK itself has no dependencies. A concrete dependency-removal implementation must use the independent consumer's actual feature resolution and then measure the selected build.

## High-level architecture map

| Responsibility | Principal source owners | Audit question |
| --- | --- | --- |
| Product shell and launch | `game/ambition_app`, `run_game.sh` | Does ordinary authoring enter Cargo or an existing runtime? |
| Native composition and SDK | `ambition_platformer2d`, provider, runtime | Can a project select and prepare installed services without a new Rust consumer? |
| Authored truth and preparation | content pack, engine schemas, content, publication | Which changes are data, and what admits a new generation? |
| Portable procedural contract | extension SDK, host, Wasm, domain port crates | Who owns execution, state, effects, phases, and replacement? |
| Physical simulation | core, characters, actor monolith, hosted world, portal2d | Do all producers use the same body facts and physical authority? |
| Gameplay domains | combat, damage, items, abilities, encounters, game modules | Are new algorithms composed from supported operations or added as engine-specific branches? |
| Presentation and observations | render, character sprites, sim view, audio, tools | Do views consume authoritative facts, and can an author inspect the result? |
| Persistence and rollback | persistence, session, rollback GGRS, extension state adapters | Which lifetimes are implemented, and who restores each record? |

These are responsibility groupings, not a proposed reorganization into eight new crates. The detailed reports identify individual edges and customers before recommending a move.

## Deep samples and source anchors

All paths below are relative to the supplied source root. Line anchors refer to this snapshot. Recheck them at an implementation checkout's current HEAD.

| Evidence | Path and entry anchor | What was followed |
| --- | --- | --- |
| Build invalidation | `game/ambition_app/build.rs:18`; `game/ambition_app/src/menu/build_label.rs:62` | Workspace watches, Git watches, timestamp, compiled label consumer |
| Normal launch | `run_game.sh:129`, `:552`, `:658`, `:766` | Profile default, command construction, launch plan, final invocation |
| Module builds/watching | `scripts/regen/build_extension_modules.sh:29`, `:38`, `:51` | Selected package, explicit Wasm compilation, fixed watch roots |
| Runtime installation versus build graph | `crates/ambition_platformer2d_runtime/src/profile.rs:16`; facade and actor manifests | Plugin omission compared with mandatory manifest edges |
| Low-layer policy | `crates/ambition_platformer2d_shared_tangle/src/developer_hotkeys.rs:12`; `src/camera_ease.rs` in the same crate | Semantic contracts mixed with host key policy and view implementation |
| Content compiler and embedding | `crates/ambition_content_pack/src/embed.rs:26`; `crates/ambition_engine_schemas/src/lib.rs:20` | Pure compiler, conditional embedding, schema composition dependencies |
| Language boundary | `crates/ambition_extension_host/src/admission.rs:43`; `crates/ambition_extension_wasm/src/lib.rs:108` | Backend interface, checked execution, fresh Wasmi instances |
| Port and state vocabulary | `crates/ambition_extension_sdk/src/port.rs:63`; `src/schema.rs:62` in the same crate | Rust value/codec contract, attachment and field kinds |
| Request outcomes | `crates/ambition_extension_host/src/exec.rs:349`; `src/lib.rs:400` in the same crate | Record commit, request staging, settlement, domain lowering |
| Loading and discovery | `crates/ambition_platformer2d_runtime/src/extension_composition.rs:273`, `:376`, `:502` | Opt-in loading, known-file polling, initial directory enumeration |
| Reload ownership | `crates/ambition_platformer2d/src/lib.rs:130`, `:440`; `game/ambition_content/src/pack.rs:377` | Static SDK/demo borrows versus main-game App-local `Arc` ownership |
| Control and body authority | `crates/ambition_characters/src/actor/control.rs:160`; `crates/ambition_platformer2d_actor_monolith/src/control/authority.rs:15` | Intent frames and controller/body separation |
| Forms and resize | `crates/ambition_platformer2d_core/src/player_state.rs:136`, `:217`, `:374` | Fixed forms, shared dimensions, existing safe transition |
| Effect composition | `crates/ambition_platformer2d_core/src/ability_projection.rs:20`; `src/movement/model.rs:631` in the same crate | Source-owned grants, single momentum boost slot, numeric composition limits |
| Controller-bound hooks | `crates/ambition_combat/src/extension/think.rs:42`; `src/extension/contact.rs:75` in the same crate | AI ownership versus independent body behavior |
| Native portal services | `crates/ambition_portal2d/src/plugin.rs:71`; `crates/ambition_platformer2d_runtime/src/portal_schedule.rs:24` | Geometry, physical admission, and schedule ordering |
| Small stateful script customer | `game/ambition_content_modules/src/sentry.rs:38` | Spend, spawn, per-turret cadence, projectile and sound requests |
| Rich composition customer | `game/ambition_content/src/primeval.rs:139`, `:370`, `:460` | Coordination, reservation, custody, common control, rollback |
| External native SDK proof | `fixtures/external_consumer/src/lib.rs:116`, `:279` | Independent game definitions and custom native state |

The detailed chapters add narrower evidence and explain counterexamples. For example, the current AI hook's exclusion of driven bodies is appropriate for control ownership; it does not prove that all body transitions require a player. Main Ambition's generation ownership differs from the SDK/demo leak. The review preserves those distinctions.

## Evidence classes and limits

**Observed static fact** means the supplied source or parsed manifest establishes the claim. It does not imply that the game was run. **Historical evidence** refers to timing receipts already in repository documents, from their stated machines and earlier revisions. **Proposed design** is a recommendation whose current implementation is not claimed. **Acceptance target** is work that a subsequent implementation must demonstrate.

No native compilation, game execution, current timing benchmark, multiplayer run, browser/mobile package run, or visual quality comparison was performed. `compile_cost.py --help` was read; its build-producing measurement was not run. The audit cannot establish the dominant current compiler phase or compare interpreter throughput. Large files, broad declared reach, allocations, and instance creation identify places to measure; they do not establish measured bottlenecks.

The recommendations favor concrete source-backed repairs and one external game workflow. They do not certify general migration correctness, arbitrary save compatibility, all target loaders, or parity with Godot. Those are acceptance questions assigned to the existing owner plans in [the execution plan](execution-plan.md).
