# Capability matrix

Purpose: compact inventory of what Ambition can currently express. This is not a changelog, not a roadmap, and not implementation truth. Code, ADRs, and `docs/current/` win when there is a conflict.

For implementation details, start from `docs/current/state.md`, `docs/systems/index.md`, `docs/mechanics/expressibility-checklist.md`, and the generated indexes under `.agent/`.

## Movement and traversal

| Capability | Status | Where to read next |
|---|---:|---|
| Kinematic platformer controller, coyote/buffered jump, dash, wall cling/jump, fast fall | Available | `docs/concepts/movement-collision.md`, `docs/mechanics/expressibility-checklist.md` |
| Blink / short-range teleport with collision safety policy | Available | `docs/mechanics/blink.md`, `docs/systems/collision-geometry-and-secondary-physics.md` |
| Ledge grab / mantle | Partial | `docs/mechanics/abilities.md`, `docs/mechanics/expressibility-checklist.md` |
| Body-mode traversal such as crouch, crawl, slide, compact/morph shapes | Available, needs more authored rooms | `docs/mechanics/body-modes.md` |
| Moving platforms and path motion | Available, needs more carry/edge validation | `docs/systems/collision-geometry-and-secondary-physics.md`, `docs/planning/tech-debt-log.md` |
| Grapple/tether constraints | Not yet reusable backend | `docs/mechanics/expressibility-checklist.md` |

## Combat, actors, and interactions

| Capability | Status | Where to read next |
|---|---:|---|
| Directional melee, upward slash, downward slash / pogo | Available | `docs/mechanics/abilities.md`, `docs/mechanics/projectiles-and-motion-inputs.md` |
| Projectiles and motion-input upgrades | Available | `docs/mechanics/projectiles-and-motion-inputs.md` |
| Shield/parry/bubble-shield vocabulary | Available | `docs/mechanics/abilities.md` |
| Actor, faction, damage, interactable, pickup, breakable vocabulary | Available | `docs/systems/progression-systems.md`, `docs/systems/factions.md` |
| Enemy archetypes, boss profiles, encounter lock walls/rewards | Partial but playable | `docs/systems/boss-behavior-profiles.md`, `docs/systems/boss-encounter-architecture.md`, `docs/planning/tech-debt-log.md` |
| Dialogue/commerce hooks | Scaffolded | `docs/systems/progression-systems.md`, `docs/adr/0008-dialogue-and-commerce-architecture.md` |

## World authoring and runtime projection

| Capability | Status | Where to read next |
|---|---:|---|
| LDtk-authored sandbox world | Current authority | `docs/concepts/ldtk-world-composition.md`, `docs/recipes/ldtk-authoring.md` |
| Collision IntGrid lowering to runtime collision rectangles | Available | `docs/systems/ldtk-world-composition.md`, `docs/tools/ldtk-tools.md` |
| Loading zones, transition validation, safe spawn checks | Available, high-risk | `docs/systems/transition-spawn-validation.md`, `docs/current/risks.md` |
| Hot reload / explicit validate-apply loop | Available for dev builds | `docs/systems/ldtk-hot-reload.md` |
| Generated music, SFX, sprites, backgrounds, parallax assets | Available through tools | `docs/tools/index.md`, `docs/recipes/generated-music-workflow.md` |

## Input, platform, and UI

| Capability | Status | Where to read next |
|---|---:|---|
| Keyboard/controller action mapping and control-frame normalization | Available | `docs/systems/input-and-control-frame.md` |
| Menu navigation, pause mode, inventory/map/pause UI routing | Available | `docs/systems/ui-navigation-and-pause.md` |
| Settings persistence across audio/video/gameplay/controls | Available | `docs/systems/settings-and-persistence.md` |
| Mobile touch controls | Available, platform-sensitive | `docs/systems/mobile-touch-controls.md` |
| Desktop, web, Android/mobile, controller, Steam Deck paths | Current targets | `docs/concepts/platform-targets.md`, `docs/recipes/index.md` |

## Validation and agent support

| Capability | Status | Where to read next |
|---|---:|---|
| Headless simulation entry point | Available | `docs/systems/headless-simulation.md` |
| Trace recording / replay for movement bugs | Available | `docs/systems/gameplay-trace-recorder.md` |
| Agent-readable indexes | Available, generated | `.agent/manifest.yaml`, `.agent/index/` |
| Documentation health checks | Available | `scripts/check_agent_kb.py`, `scripts/check_doc_links.py` |

## Rules for updating this file

- Keep it short and status-shaped.
- Do not add line-number links.
- Do not record commit history here.
- If a capability is speculative, put it in `docs/brainstorms/` or `docs/planning/` instead.
- If a capability needs a procedure, write or update a recipe and link that recipe.
