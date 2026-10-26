# Internal module cycle audit — Ambition

Snapshot: commit `5b47f840ac9f37c7a0a39d54fd296dceb804fc72` (archive generated 2026-10-26 02:54 UTC). Audit prepared 2026-10-25 in America/New_York.

## Executive result

- 107 Cargo workspace members scanned (all categories: `crates/`, `game/`, fixtures and tests).
- 29 members have **top-level module** reference cycles; 39 cyclic strongly connected components (SCCs).
- 10 additional **nested sibling-level** SCCs in 5 members.
- 30 members have a detected top-level or nested sibling cycle (the actor monolith is the one additional member with only nested cycles).
- The former `ambition_platformer2d_actor_monolith` is **a top-level DAG**, but still has two nested sibling cycles under `features`.

A module SCC is not a Cargo dependency cycle, compile failure, or correctness failure. It means the referenced module groups cannot each become independent crates **without** redesigning some references. It may be reasonable to keep a cohesive subsystem within one crate.

## How to use this baseline

This report records graph evidence. It does not select a work order. A single
edge cut can shrink an SCC without correcting ownership. It can also break
important Bevy or rollback behavior if done without review.

The semantic assessment, candidate packets and acceptance rules live in
[`docs/planning/engine/internal-module-cycle-breaking.md`](../planning/engine/internal-module-cycle-breaking.md).
Use that plan before changing a listed edge. Re-run the tool at the current
commit; this report is a snapshot, not a live gate.

Examples that need different treatment:

- `shared_tangle::lifecycle -> body` reaches `PrimaryBody` from room selection;
  this is a candidate ownership boundary.
- `characters::actor -> prepared` installs a preparation plugin while it
  registers catalog data; review the composition owner and preserve readiness.
- `input::bindings -> glyphs` reaches presentation from canonical input data;
  review where label formatting belongs.
- `core::body_clusters -> ability_projection` is a Bevy `#[require]` contract;
  keep spawn semantics and do not remove it just to shrink an SCC.
- `publication::custody_handoffs <-> transaction` can be one publication
  invariant; a smaller graph is not a reason to split the authority.

## All top-level cyclic components

`SCC size` is the number of *top-level modules* in the largest component; `SCC LOC` is the combined file-line count under those modules, including any inline test code in production files. `Cut` names a single reference direction whose removal shrinks that specific component (not necessarily makes it acyclic).

| Workspace crate | SCCs | Largest SCC | Largest SCC LOC | First effective single-edge cut |
|---|---:|---:|---:|---|
| `crates/ambition_combat` | 1 | 14 | 14,918 | `death_rules -> moveset` (3 refs; 14 -> 10) |
| `game/ambition_demo_mary_o` | 2 | 14 | 9,497 | `cosmic -> powerups` (4 refs; 14 -> 11) |
| `crates/ambition_characters` | 1 | 10 | 31,138 | `actor -> prepared` (2 refs; 10 -> 7) |
| `crates/ambition_sprite_sheet` | 2 | 9 | 7,317 | `baked_body_rigs -> baked_sheet_rons` (1 ref; 9 -> 8) |
| `crates/ambition_platformer2d_shared_tangle` | 1 | 8 | 12,440 | `lifecycle -> body` (1 ref; 8 -> 4) |
| `crates/ambition_platformer2d_core` | 1 | 6 | 22,451 | `body_clusters -> ability_projection` (1 ref; 6 -> 5) |
| `crates/ambition_input` | 1 | 6 | 4,051 | `bindings -> glyphs` (1 ref; 6 -> 5) |
| `crates/ambition_platformer2d_ldtk` | 1 | 6 | 5,588 | `project -> fields` (3 refs; 6 -> 3) |
| `game/ambition_demo_smash` | 1 | 6 | 5,845 | `smash_pack -> rules` (1 ref; 6 -> 3) |
| `crates/ambition_render` | 1 | 5 | 36,839 | `folded_mesh -> rendering` (1 ref; 5 -> 4) |
| `crates/ambition_sim_view` | 2 | 5 | 4,945 | `anim_index -> passage_view` (1 ref; 5 -> 3) |
| `crates/ambition_platformer2d_rollback_ggrs` | 1 | 5 | 6,165 | `session -> first_run_witness` (1 ref; 5 -> 4) |
| `game/ambition_app` | 1 | 4 | 28,171 | `app -> headless` (1 ref; 4 -> 3) |
| `crates/ambition_boss_encounter` | 2 | 4 | 2,292 | `clusters -> ecs` (2 refs; 4 -> 2) |
| `crates/ambition_portal2d` | 2 | 4 | 1,233 | `color -> link` (1 ref; 4 -> 1) |
| `crates/ambition_touch_input` | 1 | 4 | 3,229 | `bevy_plugin -> menu_bridge` (1 ref; 4 -> 3) |
| `game/ambition_demo_sanic` | 1 | 4 | 1,011 | `pack -> rules` (1 ref; 4 -> 2) |
| `crates/ambition_platformer2d_world` | 1 | 3 | 8,546 | `platforms -> rooms` (2 refs; 3 -> 1) |
| `crates/ambition_portal2d_presentation` | 3 | 3 | 6,265 | `carry -> far_side` (1 ref; 3 -> 2) |
| `crates/ambition_platformer2d_provider` | 1 | 3 | 5,815 | `lifecycle -> authoring` (9 refs; 3 -> 1) |
| `crates/ambition_encounter` | 2 | 3 | 1,220 | `lifecycle -> entity` (1 ref; 3 -> 1) |
| `game/ambition_content` | 2 | 2 | 2,930 | `content_watch -> reload` (2 refs; 2 -> 1) |
| `crates/ambition_platformer2d_publication` | 1 | 2 | 2,633 | `custody_handoffs -> transaction` (3 refs; 2 -> 1) |
| `crates/ambition_dev_tools` | 1 | 2 | 3,768 | `perf_probe -> runtime_census` (1 ref; 2 -> 1) |
| `crates/ambition_platformer2d` | 2 | 2 | 2,791 | `rollback -> app` (3 refs; 2 -> 1) |
| `crates/ambition_match` | 1 | 2 | 1,628 | `staging -> prepared` (1 ref; 2 -> 1) |
| `crates/ambition_extension_sdk` | 1 | 2 | 678 | `invoke -> module` (1 ref; 2 -> 1) |
| `crates/ambition_world_items` | 1 | 2 | 1,011 | `item_motion -> world_item` (1 ref; 2 -> 1) |
| `crates/ambition_load` | 1 | 2 | 653 | `plugin -> coordinator` (1 ref; 2 -> 1) |

### Component membership and concrete source references

**`crates/ambition_combat`**

- SCC 1 (14 modules): `body_landmarks`, `body_rig`, `capture`, `clank`, `death_rules`, `guard`, `hand`, `held_items`, `hitbox`, `hurtbox_resolution`, `moveset`, `on_hit`, `rules`, `targeting`.
  - Candidate cut: `death_rules -> moveset` (3 qualified reference(s); largest SCC 14 -> 10) (`crates/ambition_combat/src/death_rules.rs:271`).
  - Candidate cut: `targeting -> death_rules` (3 qualified reference(s); largest SCC 14 -> 11) (`crates/ambition_combat/src/targeting.rs:289`).

**`game/ambition_demo_mary_o`**

- SCC 1 (14 modules): `bricks`, `castle_end`, `cosmic`, `cosmic_aura`, `cosmic_barrier`, `fire_shader`, `flag`, `powerups`, `provider`, `quasar_shader`, `snake`, `star`, `test_course`, `void_look`.
  - Candidate cut: `cosmic -> powerups` (4 qualified reference(s); largest SCC 14 -> 11) (`game/ambition_demo_mary_o/src/cosmic.rs:96`).
  - Candidate cut: `snake -> provider` (2 qualified reference(s); largest SCC 14 -> 12) (`game/ambition_demo_mary_o/src/snake.rs:429`).
- SCC 2 (4 modules): `forms`, `pack`, `rules`, `text`.
  - Candidate cut: `forms -> pack` (1 qualified reference(s); largest SCC 4 -> 3) (`game/ambition_demo_mary_o/src/forms.rs:234`).
  - Candidate cut: `pack -> forms` (1 qualified reference(s); largest SCC 4 -> 3) (`game/ambition_demo_mary_o/src/pack.rs:14`).

**`crates/ambition_characters`**

- SCC 1 (10 modules): `action_scheme`, `actor`, `brain`, `control`, `equipment`, `moveset_prefabs`, `perception`, `prepared`, `session_cast`, `smash_fighter`.
  - Candidate cut: `actor -> prepared` (2 qualified reference(s); largest SCC 10 -> 7) (`crates/ambition_characters/src/actor/character_catalog/registry.rs:713`).
  - Candidate cut: `perception -> actor` (1 qualified reference(s); largest SCC 10 -> 9) (`crates/ambition_characters/src/perception.rs:20`).

**`crates/ambition_sprite_sheet`**

- SCC 1 (9 modules): `baked_body_rigs`, `baked_landmarks`, `baked_part_flipbooks`, `baked_portrait_rons`, `baked_sheet_rons`, `character`, `fx`, `portrait`, `sprite_packs`.
  - Candidate cut: `baked_body_rigs -> baked_sheet_rons` (1 qualified reference(s); largest SCC 9 -> 8) (`crates/ambition_sprite_sheet/src/baked_body_rigs.rs:14`).
  - Candidate cut: `baked_portrait_rons -> baked_sheet_rons` (1 qualified reference(s); largest SCC 9 -> 8) (`crates/ambition_sprite_sheet/src/baked_portrait_rons.rs:15`).
- SCC 2 (2 modules): `boss`, `game_assets`.
  - No single edge reduces largest SCC; needs coordinated separation.

**`crates/ambition_platformer2d_shared_tangle`**

- SCC 1 (8 modules): `body`, `camera_layers`, `construction`, `feature_overlay`, `frame_env`, `gravity`, `lifecycle`, `orientation`.
  - Candidate cut: `lifecycle -> body` (1 qualified reference(s); largest SCC 8 -> 4) (`crates/ambition_platformer2d_shared_tangle/src/lifecycle/room_instance.rs:212`).
  - Candidate cut: `body -> frame_env` (1 qualified reference(s); largest SCC 8 -> 5) (`crates/ambition_platformer2d_shared_tangle/src/body.rs:331`).

**`crates/ambition_platformer2d_core`**

- SCC 1 (6 modules): `abilities`, `ability_projection`, `body_clusters`, `ledge_grab`, `movement`, `player_state`.
  - Candidate cut: `body_clusters -> ability_projection` (1 qualified reference(s); largest SCC 6 -> 5) (`crates/ambition_platformer2d_core/src/body_clusters.rs:121`).
  - Candidate cut: `abilities -> movement` (6 qualified reference(s); largest SCC 6 -> 5) (`crates/ambition_platformer2d_core/src/abilities.rs:1119`).

**`crates/ambition_input`**

- SCC 1 (6 modules): `active_input`, `bindings`, `glyphs`, `layout`, `presets`, `settings`.
  - Candidate cut: `bindings -> glyphs` (1 qualified reference(s); largest SCC 6 -> 5) (`crates/ambition_input/src/bindings.rs:351`).
  - Candidate cut: `presets -> bindings` (2 qualified reference(s); largest SCC 6 -> 5) (`crates/ambition_input/src/presets.rs:263`).

**`crates/ambition_platformer2d_ldtk`**

- SCC 1 (6 modules): `conversion`, `fields`, `intgrid`, `project`, `surfaces`, `terrain`.
  - Candidate cut: `project -> fields` (3 qualified reference(s); largest SCC 6 -> 3) (`crates/ambition_platformer2d_ldtk/src/project.rs:11`).
  - Candidate cut: `fields -> conversion` (1 qualified reference(s); largest SCC 6 -> 4) (`crates/ambition_platformer2d_ldtk/src/fields.rs:25`).

**`game/ambition_demo_smash`**

- SCC 1 (6 modules): `rules`, `select`, `select_screen`, `smash_pack`, `stages`, `text`.
  - Candidate cut: `smash_pack -> rules` (1 qualified reference(s); largest SCC 6 -> 3) (`game/ambition_demo_smash/src/smash_pack.rs:22`).
  - Candidate cut: `smash_pack -> stages` (1 qualified reference(s); largest SCC 6 -> 5) (`game/ambition_demo_smash/src/smash_pack.rs:22`).

**`crates/ambition_render`**

- SCC 1 (5 modules): `bubble_shield_fx`, `folded_mesh`, `gate_portal_fx`, `lines`, `rendering`.
  - Candidate cut: `folded_mesh -> rendering` (1 qualified reference(s); largest SCC 5 -> 4) (`crates/ambition_render/src/folded_mesh.rs:204`).
  - Candidate cut: `rendering -> bubble_shield_fx` (1 qualified reference(s); largest SCC 5 -> 4) (`crates/ambition_render/src/rendering/mod.rs:557`).

**`crates/ambition_sim_view`**

- SCC 1 (5 modules): `anim_index`, `passage_view`, `pose_view`, `presented_pose`, `view_index`.
  - Candidate cut: `anim_index -> passage_view` (1 qualified reference(s); largest SCC 5 -> 3) (`crates/ambition_sim_view/src/anim_index.rs:133`).
  - Candidate cut: `passage_view -> pose_view` (1 qualified reference(s); largest SCC 5 -> 3) (`crates/ambition_sim_view/src/passage_view.rs:17`).
- SCC 2 (3 modules): `camera_snapshot`, `facts`, `local_view`.
  - No single edge reduces largest SCC; needs coordinated separation.

**`crates/ambition_platformer2d_rollback_ggrs`**

- SCC 1 (5 modules): `first_run_witness`, `lifecycle_commit`, `local_session`, `peer`, `session`.
  - Candidate cut: `session -> first_run_witness` (1 qualified reference(s); largest SCC 5 -> 4) (`crates/ambition_platformer2d_rollback_ggrs/src/session.rs:997`).
  - Candidate cut: `first_run_witness -> session` (2 qualified reference(s); largest SCC 5 -> 4) (`crates/ambition_platformer2d_rollback_ggrs/src/first_run_witness.rs:48`).

**`game/ambition_app`**

- SCC 1 (4 modules): `app`, `dev`, `headless`, `menu`.
  - Candidate cut: `app -> headless` (1 qualified reference(s); largest SCC 4 -> 3) (`game/ambition_app/src/app/cli.rs:241`).
  - Candidate cut: `dev -> app` (2 qualified reference(s); largest SCC 4 -> 3) (`game/ambition_app/src/dev/debug_overlay.rs:376`).

**`crates/ambition_boss_encounter`**

- SCC 1 (4 modules): `anim`, `attack_geometry`, `clusters`, `ecs`.
  - Candidate cut: `clusters -> ecs` (2 qualified reference(s); largest SCC 4 -> 2) (`crates/ambition_boss_encounter/src/clusters.rs:286`).
  - Candidate cut: `attack_geometry -> clusters` (1 qualified reference(s); largest SCC 4 -> 3) (`crates/ambition_boss_encounter/src/attack_geometry/mod.rs:93`).
- SCC 2 (2 modules): `behavior`, `catalog`.
  - Candidate cut: `catalog -> behavior` (2 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_boss_encounter/src/catalog.rs:16`).
  - Candidate cut: `behavior -> catalog` (11 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_boss_encounter/src/behavior.rs:36`).

**`crates/ambition_portal2d`**

- SCC 1 (4 modules): `color`, `link`, `rooms`, `types`.
  - Candidate cut: `color -> link` (1 qualified reference(s); largest SCC 4 -> 1) (`crates/ambition_portal2d/src/color.rs:214`).
  - Candidate cut: `types -> color` (1 qualified reference(s); largest SCC 4 -> 3) (`crates/ambition_portal2d/src/types.rs:9`).
- SCC 2 (3 modules): `placement`, `snag`, `transit`.
  - No single edge reduces largest SCC; needs coordinated separation.

**`crates/ambition_touch_input`**

- SCC 1 (4 modules): `bevy_plugin`, `menu_bridge`, `placement`, `virtual_device`.
  - Candidate cut: `bevy_plugin -> menu_bridge` (1 qualified reference(s); largest SCC 4 -> 3) (`crates/ambition_touch_input/src/bevy_plugin.rs:22`).
  - Candidate cut: `virtual_device -> bevy_plugin` (1 qualified reference(s); largest SCC 4 -> 3) (`crates/ambition_touch_input/src/virtual_device.rs:31`).

**`game/ambition_demo_sanic`**

- SCC 1 (4 modules): `ball_dash`, `pack`, `rules`, `text`.
  - Candidate cut: `pack -> rules` (1 qualified reference(s); largest SCC 4 -> 2) (`game/ambition_demo_sanic/src/pack.rs:14`).
  - Candidate cut: `rules -> pack` (1 qualified reference(s); largest SCC 4 -> 2) (`game/ambition_demo_sanic/src/rules.rs:414`).

**`crates/ambition_platformer2d_world`**

- SCC 1 (3 modules): `collision`, `platforms`, `rooms`.
  - Candidate cut: `platforms -> rooms` (2 qualified reference(s); largest SCC 3 -> 1) (`crates/ambition_platformer2d_world/src/platforms/mod.rs:10`).
  - Candidate cut: `rooms -> collision` (2 qualified reference(s); largest SCC 3 -> 2) (`crates/ambition_platformer2d_world/src/rooms/rollback.rs:66`).

**`crates/ambition_portal2d_presentation`**

- SCC 1 (3 modules): `carry`, `far_side`, `view_cones`.
  - Candidate cut: `carry -> far_side` (1 qualified reference(s); largest SCC 3 -> 2) (`crates/ambition_portal2d_presentation/src/carry.rs:313`).
  - Candidate cut: `far_side -> view_cones` (1 qualified reference(s); largest SCC 3 -> 2) (`crates/ambition_portal2d_presentation/src/far_side.rs:40`).
- SCC 2 (2 modules): `glow`, `limb_seams`.
  - Candidate cut: `limb_seams -> glow` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_portal2d_presentation/src/limb_seams.rs:26`).
  - Candidate cut: `glow -> limb_seams` (3 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_portal2d_presentation/src/glow.rs:446`).
- SCC 3 (2 modules): `gun_visuals`, `visuals`.
  - Candidate cut: `gun_visuals -> visuals` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_portal2d_presentation/src/gun_visuals.rs:18`).
  - Candidate cut: `visuals -> gun_visuals` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_portal2d_presentation/src/visuals.rs:19`).

**`crates/ambition_platformer2d_provider`**

- SCC 1 (3 modules): `authoring`, `lifecycle`, `world_source`.
  - Candidate cut: `lifecycle -> authoring` (9 qualified reference(s); largest SCC 3 -> 1) (`crates/ambition_platformer2d_provider/src/lifecycle.rs:40`).
  - Candidate cut: `world_source -> lifecycle` (1 qualified reference(s); largest SCC 3 -> 2) (`crates/ambition_platformer2d_provider/src/world_source.rs:14`).

**`crates/ambition_encounter`**

- SCC 1 (3 modules): `entity`, `lifecycle`, `waves`.
  - Candidate cut: `lifecycle -> entity` (1 qualified reference(s); largest SCC 3 -> 1) (`crates/ambition_encounter/src/lifecycle.rs:17`).
  - Candidate cut: `entity -> waves` (1 qualified reference(s); largest SCC 3 -> 2) (`crates/ambition_encounter/src/entity.rs:81`).
- SCC 2 (2 modules): `registry`, `switches`.
  - No single edge reduces largest SCC; needs coordinated separation.

**`game/ambition_content`**

- SCC 1 (2 modules): `content_watch`, `reload`.
  - Candidate cut: `content_watch -> reload` (2 qualified reference(s); largest SCC 2 -> 1) (`game/ambition_content/src/content_watch.rs:131`).
  - Candidate cut: `reload -> content_watch` (4 qualified reference(s); largest SCC 2 -> 1) (`game/ambition_content/src/reload.rs:2072`).
- SCC 2 (2 modules): `provider`, `world_preparation`.
  - No single edge reduces largest SCC; needs coordinated separation.

**`crates/ambition_platformer2d_publication`**

- SCC 1 (2 modules): `custody_handoffs`, `transaction`.
  - Candidate cut: `custody_handoffs -> transaction` (3 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_platformer2d_publication/src/custody_handoffs.rs:76`).
  - Candidate cut: `transaction -> custody_handoffs` (5 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_platformer2d_publication/src/transaction.rs:311`).

**`crates/ambition_dev_tools`**

- SCC 1 (2 modules): `perf_probe`, `runtime_census`.
  - Candidate cut: `perf_probe -> runtime_census` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_dev_tools/src/perf_probe/mod.rs:44`).
  - Candidate cut: `runtime_census -> perf_probe` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_dev_tools/src/runtime_census.rs:1842`).

**`crates/ambition_platformer2d`**

- SCC 1 (2 modules): `app`, `rollback`.
  - Candidate cut: `rollback -> app` (3 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_platformer2d/src/rollback.rs:11`).
  - Candidate cut: `app -> rollback` (6 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_platformer2d/src/app.rs:521`).
- SCC 2 (2 modules): `autopilot`, `scripted_input`.
  - Candidate cut: `autopilot -> scripted_input` (2 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_platformer2d/src/autopilot.rs:47`).
  - Candidate cut: `scripted_input -> autopilot` (2 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_platformer2d/src/scripted_input.rs:155`).

**`crates/ambition_match`**

- SCC 1 (2 modules): `prepared`, `staging`.
  - Candidate cut: `staging -> prepared` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_match/src/staging.rs:275`).
  - Candidate cut: `prepared -> staging` (6 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_match/src/prepared.rs:13`).

**`crates/ambition_extension_sdk`**

- SCC 1 (2 modules): `invoke`, `module`.
  - Candidate cut: `invoke -> module` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_extension_sdk/src/invoke.rs:15`).
  - Candidate cut: `module -> invoke` (3 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_extension_sdk/src/module.rs:5`).

**`crates/ambition_world_items`**

- SCC 1 (2 modules): `item_motion`, `world_item`.
  - Candidate cut: `item_motion -> world_item` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_world_items/src/item_motion.rs:10`).
  - Candidate cut: `world_item -> item_motion` (2 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_world_items/src/world_item.rs:147`).

**`crates/ambition_load`**

- SCC 1 (2 modules): `coordinator`, `plugin`.
  - Candidate cut: `plugin -> coordinator` (1 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_load/src/plugin.rs:7`).
  - Candidate cut: `coordinator -> plugin` (3 qualified reference(s); largest SCC 2 -> 1) (`crates/ambition_load/src/coordinator.rs:7`).

## Nested sibling cycles not visible in a top-level scan

| Crate | Parent module | Sibling SCC | File lines across affected siblings |
|---|---|---|---:|
| `game/ambition_app` | `app` | `cli`, `plugins`, `resources`, `visible_composition` | 2,648 |
| `game/ambition_app` | `app` | `host_pack`, `startup_loading`, `title_screen` | 1,567 |
| `game/ambition_app` | `app` | `versus`, `versus_fighters`, `versus_rules` | 3,001 |
| `game/ambition_app` | `menu` | `dispatch`, `kaleidoscope_app` | 3,658 |
| `crates/ambition_dev_tools` | `perf_probe` | `alloc`, `traces` | 537 |
| `crates/ambition_platformer2d_core` | `movement` | `abilities`, `integration`, `knockdown`, `model`, `tuning` | 7,140 |
| `crates/ambition_render` | `rendering` | `actors`, `chain_skin`, `gate_portal_visuals`, `nameplates`, `terrain_skin`, `world` | 12,179 |
| `crates/ambition_render` | `rendering` | `control_legend`, `label_layout`, `primitives` | 1,933 |
| `crates/ambition_platformer2d_actor_monolith` | `features` | `ecs`, `npcs` | 14,552 |
| `crates/ambition_platformer2d_actor_monolith` | `features::ecs` | `errand`, `navigation` | 752 |

Two nested SCCs persist in `ambition_platformer2d_actor_monolith`: `features::{ecs,npcs}` and `features::ecs::{errand,navigation}`. This refines rather than contradicts the top-level DAG measurement.

## Method and limits

The accompanying `scripts/audit_internal_module_cycles.py` has no dependencies beyond Python 3.11+. It reads `[workspace].members` from the root `Cargo.toml`, traces `mod foo;` declarations from `src/lib.rs` or `src/main.rs`, ignores undeclared source files and `#[cfg(test)]` modules/functions, masks comments and quoted literals, and resolves explicit qualified paths to current modules. `use crate::{a, b}` trees and unambiguous crate-root `pub use` aliases are expanded. Tarjan SCC decomposition identifies cycles; re-running after deletion of each individual directed edge finds high-leverage cut candidates. A second pass resolves qualified references to the deepest file modules and computes sibling-level SCCs.

**Limitations that can change the numbers:** this is static **qualified-path** evidence, not `rustc` name resolution. Unqualified paths, glob reexports, macros generating modules/references, inline `mod foo { ... }` module bodies, custom `#[path]` module layouts, and feature-gated code are not modeled completely. The scan is effectively the union of feature branches visible in source, so some inferred edges may never coexist in one compiled configuration. A root `lib.rs` that itself owns business logic is not modeled as a cyclic top-level sibling. Generated/indirect crate modules require targeted review. References that happen to be present in expanded macros require semantic inspection before making changes.

This is intentionally different from the existing `scripts/module_graph.py` explorer: that tool models `use` declarations; the audit also counts explicit qualified paths in production source and filters test-only items. The two counts should not be compared as if they were identical graph definitions.

Reference counts are occurrences of syntactically qualified paths, **not** performance measures or degrees of runtime coupling. Removing one graph edge means *eliminating all references in that direction*, not deleting one line or necessarily moving one type.

**Suggested second wave:** `ambition_platformer2d_world` (room/collision/platform loop), `ambition_platformer2d_provider` (authoring/lifecycle/world source), `ambition_platformer2d_rollback_ggrs` (session authority), and nested `ambition_platformer2d_core::movement`. They are worth boundary review if planned work needs those responsibilities separated.

## Re-run

```bash
cd ~/code/ambition
python3 scripts/audit_internal_module_cycles.py --top 30
python3 scripts/audit_internal_module_cycles.py --crate ambition_platformer2d_shared_tangle --details
python3 scripts/audit_internal_module_cycles.py --json .agent/reports/module-cycles.json
python3 -m unittest scripts.tests.test_audit_internal_module_cycles -v
```

Validation used for this report: scanned the provided source snapshot; ran the four fixture tests for braced imports, root aliases, test filtering, and nested cycles. No Cargo build was performed in the analysis environment (Cargo/Rust tree-sitter unavailable).
