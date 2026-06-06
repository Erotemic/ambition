# Ideal Crate Topology and Dependencies

This is the long-term workspace target. Do not create all crates immediately. First mirror this structure inside `ambition_sandbox/src/`, enforce dependency direction, then extract real crates.

## Recommended ideal crates

### Layer 0: leaf utilities

| Crate | Purpose | Depends on |
|---|---|---|
| `ambition_math` | AABB geometry, ray/rect intersection, numeric helpers | `bevy_math` or `glam`, optional `serde` |
| `ambition_data` | IDs, registries, validation result types | `serde`, optional `thiserror` |
| `ambition_schedule` | shared Bevy schedule sets and plugin conventions | `bevy_app`, `bevy_ecs` |
| `ambition_messages` | common messages/events | `bevy_ecs`, optional `serde` |

Keep this layer small. Avoid a giant `common` crate.

### Layer 1: reusable platformer runtime

| Crate | Purpose | Depends on |
|---|---|---|
| `ambition_platformer_core` | headless platformer data model | utility crates |
| `ambition_platformer_ecs` | Bevy components/systems for bodies, lifecycle, world query, rooms | core + Bevy ECS |
| `ambition_platformer_physics` | collision stepping, ledge grab, fit checks, solid-world queries | core/ecs, optional `avian2d` |
| `ambition_platformer_rooms` | room graph, loading zones, gate registry, room-scoped cleanup | core/ecs |
| `ambition_platformer_input` | input abstraction/action intents | core/ecs + Bevy input |
| `ambition_platformer_projectile` | generic projectile motion/collision | core/ecs/physics |

Pragmatic first merge option:

```text
ambition_platformer_core
ambition_platformer_ecs
ambition_platformer_projectile
```

Split more only when dependency pressure or compile-time wins justify it.

### Layer 2: optional mechanics

| Crate | Purpose | Depends on |
|---|---|---|
| `ambition_mechanics_portal` | portal gun, portal shots, placed portals, transit, authored gate state | platformer ecs/physics/rooms |
| `ambition_mechanics_gravity` | gravity zones, gravity field, base gravity, reset policy | platformer ecs/physics |
| `ambition_mechanics_held_items` | pickup/drop/throw/ground-item behavior | platformer ecs/physics |
| `ambition_mechanics_combat` | damage, hitboxes, health, teams, status | platformer ecs/projectile |
| `ambition_mechanics_interaction` | interactables, affordances, prompts | platformer ecs |
| `ambition_mechanics_encounter` | encounter state, lock walls, waves, rewards interface | rooms + combat |
| `ambition_mechanics_boss_runtime` | boss phase runtime and attack primitives | encounter + combat + projectile |
| `ambition_mechanics_inventory` | generic inventory model and item/ability binding | data + ecs |

Do not make a crate per Ambition ability initially. Abilities like beam, bomb, meteor, vortex, mark-recall, puppy slug gun, etc. can start in Ambition content or an Ambition-specific abilities module.

### Layer 3: adapters, rendering, authoring, devtools

| Crate | Purpose | Depends on |
|---|---|---|
| `ambition_platformer_render` | generic 2D presentation primitives | Bevy render/sprite + platformer ecs |
| `ambition_platformer_ldtk` | generic LDtk adapter for rooms/collision/loading zones | Bevy LDtk + platformer rooms/physics |
| `ambition_portal_render` | portal body pieces, rings, shots, disorientation | portal + render |
| `ambition_portal_ldtk` | portal LDtk entities/conversion/validation | portal + ldtk |
| `ambition_audio` | audio runtime, music director, cues | Bevy audio/Kira |
| `ambition_dialogue` | Yarn/dialogue runtime and bindings | Yarnspinner/Bevy bindings |
| `ambition_ui_core` | reusable UI navigation/pointer/list model | Bevy UI optional |
| `ambition_devtools` | debug overlay, trace, profiling, inspectors | dev dependencies + runtime |

### Layer 4: content and apps

| Crate | Purpose | Depends on |
|---|---|---|
| `ambition_content` | named Ambition content: worlds, quests, bosses, enemies, items, music/dialogue IDs | runtime + mechanics + adapters |
| `ambition_sandbox` | executable shell / playable sandbox app | selected plugins |
| `ambition_headless` | headless sim/replay/random-walker binary | runtime + selected content |
| `ambition_tools` | content validation, LDtk tools, asset probes | content + authoring adapters |

## Dependency graph

```text
ambition_math
ambition_data
ambition_schedule
ambition_messages
        |
        v
ambition_platformer_core
        |
        v
ambition_platformer_ecs
        |
        v
ambition_platformer_physics
ambition_platformer_rooms
ambition_platformer_projectile
ambition_platformer_input
        |
        v
mechanics crates
        |
        v
adapter/render/authoring crates
        |
        v
ambition_content
        |
        v
apps: ambition_sandbox, ambition_headless, ambition_tools
```

## Forbidden arrows

```text
platformer_*        -> ambition_content
platformer_*        -> ambition_sandbox
mechanics_portal    -> ambition_content
mechanics_portal    -> portal_render
mechanics_portal    -> portal_ldtk
mechanics_portal    -> ambition_sandbox
content             -> apps
render/adapters     -> apps
```

Adapters can depend on mechanics, but mechanics must not depend on adapters.

## Minimum useful first crate split

Do not start with all crates. The first real split should be closer to:

```text
ambition_math
ambition_data
ambition_platformer_core
ambition_platformer_ecs
ambition_mechanics_portal
ambition_content
ambition_sandbox
```

Then add render/LDtk adapters and additional mechanics once boundaries are proven.
