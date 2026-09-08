# Architecture review coverage and reproducibility

**Source:** `300004d601af1e633cfaee969f079cf9bb368ca8`, committed archive dated
2026-09-08. This is a bounded review receipt, not a second execution queue. The
[reassessment](architecture-reassessment.md), [map](architecture-responsibility-map.md),
[findings](architecture-review-findings.md) and [frontier](actor-monolith-work-frontier.md)
own the decisions and work. Retire this snapshot inventory when superseded;
do not maintain a parallel running architecture diary.

## Scope and evidence classes

The review inventoried all **79 workspace packages**, their manifests and Rust
source regions, and considered the disposition of all **102 original planning
Markdown files**. It traced selected state, caller, scheduling and lifetime paths
across the platformer spine and the concrete defects below. The resulting overlay
changes **67 existing planning files** and adds **4 planning files**;
the other 35 original files retain their focused product/owner contracts.
No source, manifest, test, asset or submodule change is part of the overlay.

This is **not** a line-by-line semantic audit of 679,785 physical Rust lines,
a security audit, a claim of complete call-graph recovery, or a fresh runtime
certification of every status statement retained in the planning tree. Peripheral
packages received inventory/context coverage; the table marks focused traces
separately. Recommendations and conditional counterexamples are not passing tests.

The archive contains the committed superproject and five populated submodules:
music, SFX and sprite renderers, developer measurements, and map assets. Uncommitted
workstation/submodule state is not included. Those submodules were context inputs,
not separately certified implementations. Epoch/shallow history limits older
commit-citation resolution; the review does not guess missing commit contents.

## Source inventory

There are **1,598 Rust files** below workspace package `src` directories, totaling
**679,785 physical lines**, including inline/separate tests and comments. The
actor-monolith count is **98,464**. Integration tests outside `src`, renderer
submodules, scripts, generated/binary assets and other non-workspace code are not
included in that line count. Counts are not production LOC or code-quality scores.

**T** means selected implementation/caller boundaries were traced for the stated
question, not every function in that package. **I** means manifest, source-region
and architectural-context inventory only; no full writer/lifetime verdict is
implied. Dominant source regions are locators, not ownership inferred from names.

| Package / manifest | Physical src lines | Rust files | Coverage and selected source regions |
| --- | ---: | ---: | --- |
| [`ambition_abilities`](../../../crates/ambition_abilities/Cargo.toml) | 5,253 | 28 | I - ranged, traversal, thrown, test_support, ability_cooldown |
| [`ambition_app`](../../../game/ambition_app/Cargo.toml) | 32,011 | 67 | T - selected composition and acceptance tests; not every app/tool mode |
| [`ambition_app_tools`](../../../game/ambition_app_tools/Cargo.toml) | 9,110 | 16 | I - bin, probe_stage |
| [`ambition_asset_manager`](../../../crates/ambition_asset_manager/Cargo.toml) | 5,977 | 27 | I - image_stages, platformer_assets, asset_publish, resolver, manifest |
| [`ambition_audio`](../../../crates/ambition_audio/Cargo.toml) | 7,068 | 23 | I - music, selection, catalog, bank_asset, library |
| [`ambition_binding`](../../../crates/ambition_binding/Cargo.toml) | 821 | 1 | I - library entry point / public value surface |
| [`ambition_body_seed`](../../../crates/ambition_body_seed/Cargo.toml) | 1,759 | 4 | T - shared structural body construction contract |
| [`ambition_boss_encounter`](../../../crates/ambition_boss_encounter/Cargo.toml) | 15,075 | 49 | T - boss geometry, health/policy and actor integration |
| [`ambition_causal`](../../../crates/ambition_causal/Cargo.toml) | 1,647 | 8 | I - log, fact, unclaimed, sink, ecs |
| [`ambition_character_sprites`](../../../crates/ambition_character_sprites/Cargo.toml) | 3,243 | 7 | I - anim, posed_body, attack_hitbox |
| [`ambition_characters`](../../../crates/ambition_characters/Cargo.toml) | 40,226 | 90 | T - authored/prepared definitions versus live actor, brain and materialization vocabulary |
| [`ambition_combat`](../../../crates/ambition_combat/Cargo.toml) | 51,893 | 87 | T - contact/reaction, target publication, moveset interpreter, mixed noncombat modules |
| [`ambition_content`](../../../game/ambition_content/Cargo.toml) | 44,237 | 138 | T - selected world/character/rest-point authoring consumers; not every content definition |
| [`ambition_content_cli`](../../../crates/ambition_content_cli/Cargo.toml) | 311 | 2 | I - main |
| [`ambition_content_pack`](../../../crates/ambition_content_pack/Cargo.toml) | 3,187 | 8 | I - refs, schema, diagnostic, draft, prepared |
| [`ambition_conversation`](../../../crates/ambition_conversation/Cargo.toml) | 3,331 | 19 | I - dialog, ledger, instance, opening, authority |
| [`ambition_cutscene`](../../../crates/ambition_cutscene/Cargo.toml) | 913 | 2 | I - rollback_registration |
| [`ambition_damage`](../../../crates/ambition_damage/Cargo.toml) | 3,787 | 2 | T - hit vocabulary and damage/target integration |
| [`ambition_demo_mary_o`](../../../game/ambition_demo_mary_o/Cargo.toml) | 16,150 | 27 | I - powerups, snake, ldtk_vocabulary, movement, provider |
| [`ambition_demo_mary_o_app`](../../../game/ambition_demo_mary_o_app/Cargo.toml) | 988 | 3 | I - bin, main |
| [`ambition_demo_pocket`](../../../game/ambition_demo_pocket/Cargo.toml) | 261 | 1 | I - library entry point / public value surface |
| [`ambition_demo_sanic`](../../../game/ambition_demo_sanic/Cargo.toml) | 8,263 | 9 | I - ball_dash, smash_moveset, monitors, badnik, provider |
| [`ambition_demo_sanic_app`](../../../game/ambition_demo_sanic_app/Cargo.toml) | 795 | 3 | I - bin, main |
| [`ambition_demo_smash`](../../../game/ambition_demo_smash/Cargo.toml) | 25,518 | 39 | T - live flow customer, capture hydration and body/action integration |
| [`ambition_demo_smash_app`](../../../game/ambition_demo_smash_app/Cargo.toml) | 6,676 | 14 | I - tools, stage_diagram, bin, main |
| [`ambition_demo_twintrack`](../../../game/ambition_demo_twintrack/Cargo.toml) | 7,532 | 8 | I - observatory, spacetime_3d, split_screen, light_pulse, participants |
| [`ambition_demo_twintrack_app`](../../../game/ambition_demo_twintrack_app/Cargo.toml) | 407 | 3 | I - bin, main |
| [`ambition_dev_tools`](../../../crates/ambition_dev_tools/Cargo.toml) | 5,528 | 14 | I - runtime_census, dev_tools, profiling, hot_reload, persistence |
| [`ambition_dialog`](../../../crates/ambition_dialog/Cargo.toml) | 2,901 | 10 | I - systems, runtime, bridge, speech_sfx, context |
| [`ambition_encounter`](../../../crates/ambition_encounter/Cargo.toml) | 4,233 | 21 | I - switches, lifecycle, waves, content_schema, spec |
| [`ambition_encounter_features`](../../../crates/ambition_encounter_features/Cargo.toml) | 2,454 | 8 | I - systems, loading, conditions, conditions_tests, lock_walls |
| [`ambition_entity_catalog`](../../../crates/ambition_entity_catalog/Cargo.toml) | 6,369 | 5 | T - EffectRef, ParamSchemaRegistry, TechniqueFlow and validation/runtime representation |
| [`ambition_game_shell`](../../../crates/ambition_game_shell/Cargo.toml) | 8,947 | 20 | I - basic_presentation, pause_menu, session, plugin, scope |
| [`ambition_gameplay_trace`](../../../crates/ambition_gameplay_trace/Cargo.toml) | 1,896 | 6 | I - actor_trace, model, dump, buffer, policy |
| [`ambition_geometry`](../../../crates/ambition_geometry/Cargo.toml) | 3,311 | 7 | I - reference_frame, geometry, combat_volume, swing_shape, volume_shape |
| [`ambition_held_items`](../../../crates/ambition_held_items/Cargo.toml) | 3,510 | 4 | T - held/item conditions and installation boundary |
| [`ambition_input`](../../../crates/ambition_input/Cargo.toml) | 9,986 | 21 | I - local_seats, bindings, settings, semantic, presets |
| [`ambition_interaction`](../../../crates/ambition_interaction/Cargo.toml) | 345 | 1 | T - interaction model and authored breakable/object vocabulary |
| [`ambition_inventory_ui`](../../../crates/ambition_inventory_ui/Cargo.toml) | 257 | 3 | I - model, context |
| [`ambition_items`](../../../crates/ambition_items/Cargo.toml) | 2,095 | 8 | I - content_schema, equipment, shop, rollback_registration |
| [`ambition_load`](../../../crates/ambition_load/Cargo.toml) | 1,441 | 6 | I - coordinator, model, plugin, id |
| [`ambition_load_presentation`](../../../crates/ambition_load_presentation/Cargo.toml) | 1,871 | 7 | I - model, plugin, shell_adapter, basic_presentation, deterministic_activity |
| [`ambition_match`](../../../crates/ambition_match/Cargo.toml) | 2,628 | 7 | T - settlement ownership and source-move identity constraints |
| [`ambition_menu`](../../../crates/ambition_menu/Cargo.toml) | 4,827 | 12 | I - render, map, backend |
| [`ambition_menu_kaleidoscope`](../../../game/ambition_menu_kaleidoscope/Cargo.toml) | 2,919 | 9 | I - page, scrollbar_tests, per_face_rebuild_tests, fade_tests, rebuild_gate_tests |
| [`ambition_mount`](../../../crates/ambition_mount/Cargo.toml) | 1,886 | 1 | T - relation authority and control/materialization integration |
| [`ambition_persistence`](../../../crates/ambition_persistence/Cargo.toml) | 7,353 | 18 | I - settings, save_data, save, quest, store |
| [`ambition_platformer2d`](../../../crates/ambition_platformer2d/Cargo.toml) | 4,009 | 8 | T - public exports, PlatformerApp installation and dependency closure |
| [`ambition_platformer2d_actor_monolith`](../../../crates/ambition_platformer2d_actor_monolith/Cargo.toml) | 98,464 | 237 | T - live actor, checkpoint, construction/lowering, item installation, projectile and feature callers |
| [`ambition_platformer2d_actor_spawn`](../../../crates/ambition_platformer2d_actor_spawn/Cargo.toml) | 3,825 | 8 | T - builder-only contract and corrected live-query boundary |
| [`ambition_platformer2d_core`](../../../crates/ambition_platformer2d_core/Cargo.toml) | 47,097 | 76 | T - body/control vocabulary, movement/contact substrate and snapshot boundaries |
| [`ambition_platformer2d_host`](../../../crates/ambition_platformer2d_host/Cargo.toml) | 3,121 | 4 | T - host installers and mandatory render reachability |
| [`ambition_platformer2d_ldtk`](../../../crates/ambition_platformer2d_ldtk/Cargo.toml) | 7,220 | 20 | I - conversion, bevy_runtime, contract, fields, surfaces |
| [`ambition_platformer2d_provider`](../../../crates/ambition_platformer2d_provider/Cargo.toml) | 2,981 | 4 | T - provider registration, preparation and composition surface |
| [`ambition_platformer2d_rollback_ggrs`](../../../crates/ambition_platformer2d_rollback_ggrs/Cargo.toml) | 6,466 | 12 | T - backend adapter and confirmed lifecycle baseline boundary |
| [`ambition_platformer2d_runtime`](../../../crates/ambition_platformer2d_runtime/Cargo.toml) | 10,415 | 33 | T - phase composition, checkpoint horizon, content identity, rollback and external effects |
| [`ambition_platformer2d_shared_tangle`](../../../crates/ambition_platformer2d_shared_tangle/Cargo.toml) | 21,353 | 65 | T - typed construction protocol, raw Commands, verification and shared state |
| [`ambition_platformer2d_world`](../../../crates/ambition_platformer2d_world/Cargo.toml) | 5,680 | 19 | T - placement provider contracts, active collision services and room vocabulary |
| [`ambition_portal2d`](../../../crates/ambition_portal2d/Cargo.toml) | 6,589 | 25 | I - placement, view, transit, pieces, color |
| [`ambition_portal2d_presentation`](../../../crates/ambition_portal2d_presentation/Cargo.toml) | 9,573 | 18 | I - view_cones, far_side, visuals, compositing, gun_visuals |
| [`ambition_projectile_spec`](../../../crates/ambition_projectile_spec/Cargo.toml) | 50 | 1 | I - library entry point / public value surface |
| [`ambition_projectiles`](../../../crates/ambition_projectiles/Cargo.toml) | 2,533 | 14 | T - state, spawn/materialization, target/world adapters and rollback registration |
| [`ambition_registry_core`](../../../crates/ambition_registry_core/Cargo.toml) | 235 | 1 | T - canonical metadata and duplicate-policy protocol |
| [`ambition_relativity`](../../../crates/ambition_relativity/Cargo.toml) | 722 | 1 | I - library entry point / public value surface |
| [`ambition_relativity2d`](../../../crates/ambition_relativity2d/Cargo.toml) | 3,542 | 5 | I - signals, optics, targeting, telemetry |
| [`ambition_render`](../../../crates/ambition_render/Cargo.toml) | 31,773 | 60 | I - rendering, hud, fx, asset_census, runtime_census |
| [`ambition_settings_menu`](../../../crates/ambition_settings_menu/Cargo.toml) | 2,566 | 7 | I - settings, system |
| [`ambition_sfx`](../../../crates/ambition_sfx/Cargo.toml) | 1,147 | 3 | I - message, ids |
| [`ambition_sfx_bank`](../../../crates/ambition_sfx_bank/Cargo.toml) | 539 | 1 | I - library entry point / public value surface |
| [`ambition_sim_harness`](../../../crates/ambition_sim_harness/Cargo.toml) | 3,883 | 12 | I - move_exercise, runtime, combat_observation, capture, action |
| [`ambition_sim_view`](../../../crates/ambition_sim_view/Cargo.toml) | 11,801 | 17 | T - published simulation geometry/read-model boundary and presentation identity |
| [`ambition_sprite_fx`](../../../crates/ambition_sprite_fx/Cargo.toml) | 1,084 | 2 | I - library entry point / public value surface |
| [`ambition_sprite_sheet`](../../../crates/ambition_sprite_sheet/Cargo.toml) | 10,300 | 29 | I - character, game_assets, boss, portrait, pack |
| [`ambition_time`](../../../crates/ambition_time/Cargo.toml) | 1,012 | 5 | I - time_control, snapshot_impls, rollback_registration |
| [`ambition_touch_input`](../../../crates/ambition_touch_input/Cargo.toml) | 4,583 | 8 | I - bevy_plugin, placement, layout, virtual_device, state |
| [`ambition_ui_nav`](../../../crates/ambition_ui_nav/Cargo.toml) | 932 | 4 | I - list, pointer, drag |
| [`ambition_vfx`](../../../crates/ambition_vfx/Cargo.toml) | 781 | 4 | I - vfx, fx, rollback_registration |
| [`ambition_workspace_policy`](../../../tests/ambition_workspace_policy/Cargo.toml) | 2,985 | 18 | I - custom, workspace, rules, model, runner |
| [`ambition_world_items`](../../../crates/ambition_world_items/Cargo.toml) | 1,328 | 4 | T - ground-item state and motion versus collection/held custody |

## Planning disposition

Every original planning file appears once below. "Retained" means no change was
justified by this architecture review; it does **not** renew an old measured
receipt or certify all historical claims. Where a focused plan is amended rather
than rewritten, its product behavior and closed work remain intact and the new
section refines the affected boundary. The queue is still the only priority list.

| Original document | Disposition | Reason |
| --- | --- | --- |
| [README.md](../README.md) | Revised | Separate source facts from normative decisions; route current review and one queue. |
| [authoring-loop-program-2026-07-31.md](../authoring-loop-program-2026-07-31.md) | Revised | Use live flow customer and actual installed validation in the external authoring witness. |
| [awaiting-maintainer-decision.md](../awaiting-maintainer-decision.md) | Revised | Clarify technical evidence and unresolved product choices; record no new maintainer ruling. |
| [bevy-0.19-leverage-campaign.md](../bevy-0.19-leverage-campaign.md) | Retained | Closed campaign receipt; no new Bevy migration or historical replay justified. |
| [decision-principles.md](../decision-principles.md) | Revised | Correct the unwired-validator example without changing the decision principles. |
| [demos/README.md](../demos/README.md) | Retained | Serious secondary customers remain acceptance drivers; flagship priority unchanged. |
| [demos/campaigns/expressive-moves-2026-09-05.md](../demos/campaigns/expressive-moves-2026-09-05.md) | Retained | Retired receipt; current vocabulary decisions remain with inventory/engine owners. |
| [demos/campaigns/smash-fun-push-2026-08-22.md](../demos/campaigns/smash-fun-push-2026-08-22.md) | Retained | Retired product campaign; no architectural reason to reopen it. |
| [demos/hollow-lite.md](../demos/hollow-lite.md) | Retained | Customer-triggered scope remains; not a prerequisite for contact/lifecycle repairs. |
| [demos/moveset-reviews.md](../demos/moveset-reviews.md) | Retained | Maintainer-authored move intent; do not overwrite product semantics with architecture guesses. |
| [demos/sanic.md](../demos/sanic.md) | Retained | Distinct game/motion customer; no evidence here to change its product rules. |
| [demos/smash-parity-inventory.md](../demos/smash-parity-inventory.md) | Retained | Retain feature priority/status ownership; A11/A12 validate infrastructure without inventing parity. |
| [demos/super-mary-o.md](../demos/super-mary-o.md) | Retained | Distinct game/motion customer; keep authored mechanics and provider acceptance. |
| [demos/super-smash-siblings.md](../demos/super-smash-siblings.md) | Retained | Retain game rules and product direction; engine fixes go through shared owners. |
| [demos/twintrack.md](../demos/twintrack.md) | Retained | Distinct observer/multiview customer; no broad engine redesign inferred from its existence. |
| [demos/w8-playtest-2026-08-24.md](../demos/w8-playtest-2026-08-24.md) | Retained | Historical playtest receipt; this source-only review cannot update human observations. |
| [engine/actor-monolith-decomposition.md](actor-monolith-decomposition.md) | Revised | Authority-first migration and retained post-carve guards. |
| [engine/actor-monolith-hard-core-edge-ledger.md](actor-monolith-hard-core-edge-ledger.md) | Revised | Replace post-P4 TBD template with current semantic families and explicit holds. |
| [engine/actor-monolith-work-frontier.md](actor-monolith-work-frontier.md) | Revised | Twelve bounded packets with source/destination, invariants, acceptance and dependency holds. |
| [engine/agentic-character-runtime.md](agentic-character-runtime.md) | Revised | Runtime model intentions versus authoring mutation; stale-response and replay boundary. |
| [engine/architecture.md](architecture.md) | Revised | Retain policy-linked entry point without stale inbound counts; route planned versus implemented design. |
| [engine/asset-preparation-and-residency.md](asset-preparation-and-residency.md) | Revised | Mechanical prepared content versus visual/device revision; no invented residency budget. |
| [engine/authored-gameplay-logic-and-orchestration.md](authored-gameplay-logic-and-orchestration.md) | Revised | O4 installed support and A12 flow representation; preserve occurrence-latch semantics. |
| [engine/authoring-and-tools.md](authoring-and-tools.md) | Revised | Revision-aware agent workflow, installed-support admission and prepared program bounds. |
| [engine/binding-resolution-boundary.md](binding-resolution-boundary.md) | Revised | Installed versus known support and immutable prepared revision binding. |
| [engine/boss-design.md](boss-design.md) | Retained | Retain content/pattern design; boss geometry/ownership refinements are in boss-system. |
| [engine/boss-system.md](boss-system.md) | Revised | Boss content/reaction/lifecycle responsibilities; geometry fix does not decide rewards. |
| [engine/bounded-perception-and-attention.md](bounded-perception-and-attention.md) | Revised | Keep legitimate projectile-fact dependency and bounded evidence ownership. |
| [engine/capability-and-runtime-composition.md](capability-and-runtime-composition.md) | Revised | Mechanism-specific knowledge tests, phase ancestry, mandatory closure and explicit composition. |
| [engine/capability-progression-and-world-gating.md](capability-progression-and-world-gating.md) | Retained | Retain established gate semantics and unresolved product policy; no generic registry pivot. |
| [engine/character-authoring-package.md](character-authoring-package.md) | Revised | Separate authored/prepared/materialized/live responsibilities without duplicate fields. |
| [engine/collision-and-ccd.md](collision-and-ccd.md) | Revised | Preserve closed movement/hazard work; stage authored geometry and obstruction fixes. |
| [engine/combat-model.md](combat-model.md) | Revised | Combat versus world objects/rules/brain/presentation; accepted contact contract. |
| [engine/construction-and-reconstitution.md](construction-and-reconstitution.md) | Revised | Clarify checkpoint/lowering ownership and raw-Commands failure boundary. |
| [engine/control-authority-and-ai-policy.md](control-authority-and-ai-policy.md) | Revised | Separate proposal, accepted relation and body execution; A4 proof requirement. |
| [engine/controlled-character-actor-kernel.md](controlled-character-actor-kernel.md) | Revised | Control/writer census before splitting coherent body execution. |
| [engine/decomposition.md](decomposition.md) | Revised | Definition/state/behavior/lifetime ownership; separate four composition tests. |
| [engine/dialogue-continuity.md](dialogue-continuity.md) | Retained | Keep continuity owner; lifecycle/profile constraints flow through the shared owner documents. |
| [engine/engine-1.0-architecture-program.md](engine-1.0-architecture-program.md) | Revised | Integrate ownership, programmatic authoring, actual profiles and failure contracts. |
| [engine/expressive-move-capabilities.md](expressive-move-capabilities.md) | Revised | Admission/flow prerequisites; no universal ability service or duplicated contact query. |
| [engine/extension-model.md](extension-model.md) | Revised | Typed extension roles; bounded existing flow before any VM or dynamic ABI. |
| [engine/falling-sand.md](falling-sand.md) | Retained | Existing domain-specific mechanism; no evidence justifies absorbing it into generic features. |
| [engine/fighter-brain.md](fighter-brain.md) | Revised | Decision policy consumes combat facts; distinguish unselected from invalid moves. |
| [engine/frame-awareness.md](frame-awareness.md) | Revised | Record actual phase/visibility and movement-leg facts on every move. |
| [engine/godot-class-2d-capability.md](godot-class-2d-capability.md) | Revised | Independent shipping/profile/authoring acceptance, not graph or crate metrics. |
| [engine/headless-verification.md](headless-verification.md) | Revised | Real external profile and negative-fixture evidence; no empty/hidden-default pass. |
| [engine/immutable-content-and-transactional-construction.md](immutable-content-and-transactional-construction.md) | Revised | Explicit preflight/commit/verify/publication guarantee matrix; no generic undo promise. |
| [engine/inspection-diagnostics-and-workbench.md](inspection-diagnostics-and-workbench.md) | Revised | Read-only authoritative discovery and stage-local failures; moveset plan route. |
| [engine/instance-lifetime-provenance-and-persistence.md](instance-lifetime-provenance-and-persistence.md) | Revised | Scope-specific identity and retirement; no universal ID or undo assumption. |
| [engine/item-custody-and-accounting.md](item-custody-and-accounting.md) | Revised | Keep item semantics owned while session coordinates restore; define lifetime witness. |
| [engine/kinematic-world-objects.md](kinematic-world-objects.md) | Revised | World mechanism/destructible responsibility without a new feature catch-all. |
| [engine/ldtk-authoring-and-world-tools.md](ldtk-authoring-and-world-tools.md) | Revised | Typed bridge relocation and diagnostics for unsupported authored fields. |
| [engine/multiplayer-and-multiview.md](multiplayer-and-multiview.md) | Revised | Participants/views/worlds are independent axes; preserve current checkpoint product policy. |
| [engine/netcode.md](netcode.md) | Revised | Preserve same-build wire identities and confirmation; qualify durable-effect/instance promises. |
| [engine/open-world-runtime-and-residency.md](open-world-runtime-and-residency.md) | Revised | Repeated-room witness before instance generalization; publication failure limits. |
| [engine/participant-action-system.md](participant-action-system.md) | Revised | Separate action proposal, acceptance and move occurrence; validate installed techniques. |
| [engine/performance-and-iteration.md](performance-and-iteration.md) | Revised | Bounded source opportunities without unmeasured performance claims. |
| [engine/performer-up-b-the-wire.md](performer-up-b-the-wire.md) | Retained | Concrete move customer; reusable attachment/control contracts remain separately owned. |
| [engine/pickup-carve-checklist.md](pickup-carve-checklist.md) | Revised | Exclude checkpoint restoration from item carve; preserve custody and capture order. |
| [engine/platformer-navigation-and-reachability.md](platformer-navigation-and-reachability.md) | Revised | Navigation proposes movement through body authority; profile/instance constraints. |
| [engine/project-build-and-distribution.md](project-build-and-distribution.md) | Revised | Closure, compile fanout, bytes and runtime measurements are distinct. |
| [engine/public-sdk-1.0.md](public-sdk-1.0.md) | Revised | Separate public imports from compiler/runtime optionality; define real external profiles. |
| [engine/relativity.md](relativity.md) | Retained | Specialized physics/content direction; do not replace all movement with this capability. |
| [engine/render-animation-and-vfx.md](render-animation-and-vfx.md) | Revised | Keep consequences out of combat authority and mechanical data out of device residency. |
| [engine/reusable-authored-world-composition.md](reusable-authored-world-composition.md) | Revised | Provider-neutral lowering and repeated-instance identity proof. |
| [engine/room-transition-loading.md](room-transition-loading.md) | Revised | Admission latch and staged failure taxonomy; retain confirmed lifecycle boundary. |
| [engine/runtime-frame-history.md](runtime-frame-history.md) | Retained | Existing replay/inspection design; not a replacement snapshot engine or architecture queue. |
| [engine/shell-vanity-sequence.md](shell-vanity-sequence.md) | Retained | Narrow shell sequencing; no need to merge with gameplay lifecycle/encounters. |
| [engine/simulation-authority-and-determinism.md](simulation-authority-and-determinism.md) | Revised | Writer/scope/phase obligations, admission bounds, effect and build-identity limits. |
| [engine/slower-light.md](slower-light.md) | Retained | Customer-specific physics direction; broad spatial dimensional abstraction remains gated. |
| [engine/sprite-renderer.md](sprite-renderer.md) | Retained | Renderer authoring contract preserved; no renderer-submodule code was modified or certified. |
| [engine/svg-component-character-migration.md](svg-component-character-migration.md) | Retained | Incremental customer/performance-triggered authoring migration, not a roster rewrite. |
| [engine/ui-localization-and-accessibility.md](ui-localization-and-accessibility.md) | Revised | UI-absent profile and participant/view scope; display strings are not IDs. |
| [engine/unified-movement-kernel.md](unified-movement-kernel.md) | Revised | Repair two invalid abbreviated source-path citations; preserve movement policy. |
| [engine/world-facts-observations-and-memory.md](world-facts-observations-and-memory.md) | Revised | Scoped read facts reduce producer knowledge; no mutable world-facts bus. |
| [engine/world-geometry-and-spatial-semantics.md](world-geometry-and-spatial-semantics.md) | Revised | Keep broad unification deferred while allowing contact repair and adapter relocation. |
| [engine_rename_campaign.md](../engine_rename_campaign.md) | Revised | Make naming follow ownership; preserve warning labels and stable serialized identity. |
| [frontend-audio-is-per-experience.md](../frontend-audio-is-per-experience.md) | Retained | Retain route-specific audio policy; common registry protocol does not force replacement semantics. |
| [game/ambition.md](../game/ambition.md) | Retained | Flagship remains deep co-evolving engine customer, not a thin demo waiting for engine completion. |
| [game/bosses.md](../game/bosses.md) | Revised | Remove core-as-catch-all description; preserve boss content design. |
| [game/multiplayer.md](../game/multiplayer.md) | Retained | Co-op product scope and policy remain maintainer-owned; no policy invented from package structure. |
| [game/open-world-roadmap.md](../game/open-world-roadmap.md) | Retained | Traversal/interaction milestones and authored world order remain product-driven. |
| [game/reactive-characters-and-dialogue.md](../game/reactive-characters-and-dialogue.md) | Retained | Runtime agent product intent retained; trust/revision constraints refined in engine owner. |
| [game/systemic-progression.md](../game/systemic-progression.md) | Retained | Entitlement/economy choices remain distinct from item custody implementation. |
| [game/vision.md](../game/vision.md) | Retained | Game identity/pillars unchanged by engineering ownership review. |
| [maintainer-decisions.md](../maintainer-decisions.md) | Retained | Explicit rulings preserved verbatim; reviewer recommendations are not new rulings. |
| [modal-cli-binary-collapse.md](../modal-cli-binary-collapse.md) | Retained | Recorded modal-CLI direction retained; no source/binary consolidation performed. |
| [moveset-inspector.md](../moveset-inspector.md) | Revised | Remove contradictory no-inbound-link banner; connect discovery/contact facts to current owners. |
| [queue.md](../queue.md) | Revised | Replace graph-first sequence with ownership/correctness packets; preserve unrelated executable work. |
| [roadmap.md](../roadmap.md) | Revised | Replace SCC ordering with independent authority/profile/authoring streams. |
| [status.md](../status.md) | Revised | Remove false all-prerequisites-complete/no-correctness-gap posture; publish bounded evidence. |
| [tracks.md](../tracks.md) | Revised | Record evidence-gated future work without creating a second execution queue. |
| [triage/ambition-registry-core.md](../triage/ambition-registry-core.md) | Revised | Remove contradictory chronology and false function-equality argument; bound actual protocol. |
| [triage/ambition-test-support.md](../triage/ambition-test-support.md) | Revised | Small explicit fixtures; no hidden flagship installation in external-profile proof. |
| [triage/bevy-system-parameter-architecture.md](../triage/bevy-system-parameter-architecture.md) | Revised | Borrow grouping does not establish authority; no universal context wrapper. |
| [triage/character-dialogue-from-suggestions.md](../triage/character-dialogue-from-suggestions.md) | Retained | Shelved authoring alternative; no evidence to restart speculative IR migration. |
| [triage/declared-id-resolution-checks.md](../triage/declared-id-resolution-checks.md) | Revised | Actual installed technique admission joins existing resource validation. |
| [triage/gameplay-presentation-profiles.md](../triage/gameplay-presentation-profiles.md) | Retained | Existing profile policy retained; new compile-closure acceptance belongs to A9. |
| [triage/leafwing-clash-scan-patch-2026-07-23.md](../triage/leafwing-clash-scan-patch-2026-07-23.md) | Retained | Upstream/deferred patch receipt; no dependency-version or fork change in this review. |
| [triage/stable-identifier-centralization.md](../triage/stable-identifier-centralization.md) | Revised | Retire completed syntax consolidation; preserve per-domain identity classification. |
| [triage/unused-dependency-census.md](../triage/unused-dependency-census.md) | Revised | Historical compiler coverage is not current closure; retain doc-only ruling. |
| [vision.md](../vision.md) | Revised | Make the programmatic-engine target testable without editor/3D parity. |

The four new documents are the reassessment, responsibility map, source findings
and this coverage receipt. The existing frontier and semantic edge-ledger paths
were rewritten in place, so their established inbound links remain valid.

## Reproduce the inventory and conservative facade closure

Run from the repository root with Python 3.11 or later. This is a measurement
example in documentation, not a new repository tool or required pre-commit gate.
It reads files only.

```bash
python3 - <<'PYCODE'
from pathlib import Path
from collections import deque
import tomllib

root = Path.cwd()
workspace = tomllib.loads((root / "Cargo.toml").read_text())
packages = {}
by_directory = {}
for member in workspace["workspace"]["members"]:
    directory = (root / member).resolve()
    manifest = tomllib.loads((directory / "Cargo.toml").read_text())
    name = manifest["package"]["name"]
    packages[name] = (directory, manifest)
    by_directory[directory] = name

files = [f for directory, _ in packages.values()
         for f in (directory / "src").rglob("*.rs")]
print("packages:", len(packages), "src Rust files:", len(files))
print("physical lines:", sum(len(f.read_text().splitlines()) for f in files))

edges = {}
for name, (directory, manifest) in packages.items():
    edges[name] = []
    for dep in manifest.get("dependencies", {}).values():
        if not isinstance(dep, dict) or dep.get("optional", False):
            continue
        if "path" in dep:
            target = by_directory.get((directory / dep["path"]).resolve())
            if target:
                edges[name].append(target)

start = "ambition_platformer2d"
paths = {start: [start]}
queue = deque([start])
while queue:
    source = queue.popleft()
    for target in sorted(edges[source]):
        if target not in paths:
            paths[target] = paths[source] + [target]
            queue.append(target)
print("other mandatory internal packages:", len(paths) - 1)
print("render path:", " -> ".join(paths.get("ambition_render", [])))
PYCODE
```

This lower-bound traversal uses only direct nonoptional path entries in normal
`[dependencies]`. It excludes external libraries, optional feature activation,
build/dev/target-specific edges and inherited dependency features. Cargo's actual
resolved profile can require more. The baseline is **51 other workspace packages**;
the mandatory path includes facade -> host -> render. This is not a size or speed
benchmark. A9 requires real Cargo metadata/tree and external behavior fixtures.

## Executed and unavailable verification

The exact delivery receipt is also supplied beside the overlay. The docs-only
validation set includes local links, Markdown structure, live planning pointers,
SDK module names, foreign-ordering instrumentation and the actor-spawn boundary.
The final selected Python suite passed **42 tests**. The link checker passed
**287 documents / 1,193 local links**. All **6,553 checked nonplanning files**
remained byte-identical. None of these checks compiles the changed architecture
proposals or reproduces F1-F8 in Rust.

Baseline local links passed: 283 documents / 949 links. The original strict
citation audit found 210 unresolved citations: 208 missing historical commits
and two abbreviated source paths. The two source paths were repaired. Redundant
rewritten historical prose was removed for clarity, not to fabricate green
history. The final remaining unresolved citations are unavailable historical
commits: **192 unresolved historical references**, **zero newly unresolved
references**, and **zero current source-path or qualified-symbol failures**
reported by this checker. Bare identifiers and every semantic use of a citation
are outside that instrument's coverage.

Executed source instruments at this baseline report:

| Instrument | Result | Limit |
| --- | --- | --- |
| `measure_kernel_module_graph.py --scc --cuts --edges 200` | SCCs of 9 and 2 | Textual paths, heuristic test exclusion; no semantic Rust/Cargo/state graph |
| `measure_foreign_system_ordering.py` | 0 capability/ruleset private orderings; 73 composition; 174 foreign installs | Locator classification, not phase visibility or ownership acceptance |
| `measure_carveable_installations.py` | 3 reducible / 38 irreducible | Current crate-identity heuristic, not permission to move every reducible block |
| `measure_registry_core_adoption.py` | Reference/prose classification captured | A JUSTIFIED label can still contain an incorrect rationale, as F7 demonstrates |
| `check_declared_system_packages.py` | 33 declared; 21 missing | Nonstrict exit status did not establish a provisioned Rust/GPU/audio environment |

Rust and Cargo were unavailable. No Rust build/test, rendered game, rollback
execution, P2P transport, hardware benchmark or packaged-platform acceptance was
performed. Existing suggested Rust commands in the frontier are instructions for
the implementation agent, not results from this review. A zero-test filter or an
unavailable prerequisite must not be recorded as pass.

## Review-specific source checks

The package/module inventory and caller traces are reproducible from the named
snapshot. The final overlay is checked for paths restricted to `docs/planning`,
no changed nonplanning source bytes, valid current source-path citations, no new
citation-audit failures, valid local links and clean patch application to the
exact base. These establish delivery integrity, not semantic correctness of the
future source implementation. Each packet still needs its behavioral evidence.
