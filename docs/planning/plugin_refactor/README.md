# Plugin Refactor Roadmap

This folder captures the proposed next major refactor after the ECS refactor. The goal is to turn the current `ambition_sandbox` crate into a clearer composition of reusable platformer runtime code, optional Bevy mechanics plugins, optional authoring/rendering adapters, and Ambition-specific game content.

The intended direction is not a rewrite. It is a staged refactor that intentionally breaks implicit dependencies along seams we want to keep long term. The project should become easier to onboard into, easier for agents to navigate, faster to check in headless/persona builds, and less prone to bugs caused by implicit cross-module conventions.

## Why this refactor exists

The sandbox crate has grown beyond the point where implicit conventions are enough. Recent symptoms include:

- Room-local entities leaked across room transitions because `RoomScopedEntity` was a convention at spawn sites rather than a lifecycle API.
- Portal logic spans input, inventory, room transition, player/actor/item transit, authored gates, gravity interactions, presentation, debug overlay, and LDtk conversion.
- `app/plugins.rs` and similar hubs know too much about subsystem internals.
- Game-specific content such as named bosses, quests, worlds, items, intro routes, cut-the-rope logic, and music/dialogue IDs live close to reusable runtime code.
- Large files such as `portal.rs` are risky for agents to modify, especially by broad replacement.

The high-level inventory from the current sandbox shows the scale:

```text
Rust files:                 436
Total Rust lines:           143,833
Rust code lines:            106,972
Components:                 251
Resources:                  149
Plugins:                    39
Registered systems:         368
Spawn sites:                209
System-like functions:      565
Unique registration IDs:    638
```

These numbers are not inherently bad. The issue is that many important architectural policies are implicit, distributed, and re-decided locally.

## North star

The desired final shape is:

```text
ambition_sandbox
  = executable/game shell that composes plugins

platformer runtime crates
  = reusable Bevy-native 2D platformer primitives

optional mechanics plugins
  = portals, gravity, held items, combat, encounters, boss runtime, etc.

adapter plugins
  = rendering, LDtk, audio, UI, devtools

Ambition content pack
  = named worlds, quests, bosses, enemies, items, music/dialogue IDs, intro/cut-rope
```

The rule of thumb:

```text
Runtime plugins own generic verbs and invariants.
Mechanics plugins own reusable gameplay behavior.
Adapter plugins own integration with one external concern.
Game content owns nouns, rosters, tuning, story, maps, and asset bindings.
The app composes plugins but does not own subsystem internals.
```

## Recommended reading order

1. [01_goals_and_constraints.md](01_goals_and_constraints.md) - design goals and non-goals.
2. [02_architecture_principles.md](02_architecture_principles.md) - rules that should guide all branches.
3. [03_target_plugin_topology.md](03_target_plugin_topology.md) - final plugin ecosystem.
4. [04_crate_topology.md](04_crate_topology.md) - ideal crate set and dependencies.
5. [05_disk_layout.md](05_disk_layout.md) - target workspace layout and proto-crate layout.
6. [06_portal_plugin_design.md](06_portal_plugin_design.md) - detailed portal refactor design.
7. [07_staged_refactor_plan.md](07_staged_refactor_plan.md) - staged shotgun plan.
8. [08_build_personas_and_features.md](08_build_personas_and_features.md) - feature gates and compile-time goals.
9. [09_architecture_guardrails.md](09_architecture_guardrails.md) - tests and lint-like checks.
10. [10_risks_and_decisions.md](10_risks_and_decisions.md) - tradeoffs and unintended consequences.
11. [11_agent_task_breakdown.md](11_agent_task_breakdown.md) - agent-executable tasks.
12. [12_documentation_branch_plan.md](12_documentation_branch_plan.md) - docs branch work.
13. [13_inventory_notes.md](13_inventory_notes.md) - how to use ECS inventory data.
14. [14_action_plan.md](14_action_plan.md) - exact execution steps, validation commands, and agent task queue.

## Suggested branch split

Use two branches at first:

```bash
git switch -c docs/pluginized-platformer-runtime-roadmap
```

Use this branch to update docs, identify stale docs, and check in inventory snapshots.

```bash
git switch -c refactor/platformer-runtime-boundaries
```

Use this branch to begin code work: proto-runtime module, lifecycle helpers, plugin registration ownership, and portal plugin shell.

For the branch-by-branch execution sequence, use [14_action_plan.md](14_action_plan.md) as the authoritative checklist.

## Success criteria

The refactor is working when:

- New engineers and agents can identify the owner of portal, gravity, held item, room, and content behavior from disk layout alone.
- Headless/runtime checks do not compile rendering, audio, LDtk runtime, UI, or devtools unless a build persona requests them.
- `app/plugins.rs` mostly composes plugins instead of registering subsystem internals.
- Non-portal systems do not import `crate::portal` for generic helpers like raycasts.
- Ambition-specific content is grouped behind an Ambition content plugin.
- Portal can be disabled without breaking generic movement, world query, room transitions, gravity, or inventory infrastructure.
- Architecture tests fail when new code violates dependency direction or lifecycle policy.
