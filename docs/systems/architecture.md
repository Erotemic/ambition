# Architecture

Ambition is a Rust/Bevy workspace, Bevy-native and ECS-first (old backend-neutral
constraints superseded by ADR 0002). Post-bisection (Stage 20) it is a layered
crate graph; lower layers must never import higher ones. Current remaining
module-boundary work is tracked in `docs/planning/tracks.md` and
`docs/planning/tech-debt-log.md`.

## Crate layers

| Layer | Crates | Responsibility |
|---|---|---|
| foundations | `ambition_engine_core`, `ambition_characters`, `ambition_platformer_primitives`, `ambition_portal`, `ambition_time`, `ambition_input`, `ambition_menu`, `ambition_audio`, `ambition_sfx[_bank]`, `ambition_asset_manager`, `ambition_gameplay_trace`, `ambition_cutscene`, `ambition_interaction`, `ambition_sprite_sheet`, `ambition_ui_nav`, `ambition_vfx` | Reusable/content-free vocabulary, data models, and low-level systems. |
| machinery | `ambition_gameplay_core` (lib) | Content-free simulation systems, runtime state, world/LDtk integration, player/session systems, combat/items/encounter machinery, persistence, schedules, and compatibility facade re-exports. |
| presentation | `ambition_render`, `ambition_portal_presentation` | Bevy presentation: sprite/world sync, camera, parallax, HUD, screen-space effects, dialog/cutscene UI, fonts, and render-only visual systems. Reads gameplay state; does not own app entrypoints. |
| content | `ambition_content` | Named game content: quests, bosses, enemy/item rosters, dialogue, intro, banter, portal adapters. |
| app | `ambition_app` | Bevy assembly, host glue, ALL binaries (`ambition_game_bin`, `headless`, `trace_replay`, `rl_*`), menu host stack + `DevToolsPlugin`, full-stack integration tests. |

## Machinery (`ambition_gameplay_core`) module shape

```text
src/abilities/        player ability and weapon-kit systems
src/combat/           combat buses, hazards, damage, hitboxes, pickups, variation
src/features/         room-authored ECS entity runtime and spawn/view sync
src/world/            LDtk world, room building, physics, platforms
src/player/           player ECS components, systems, affordances, trail scaffold
src/portal/           gameplay-side portal host adapter and schedule facade
src/persistence/      save data and settings (control settings facade re-exports ambition_input)
src/dev/              dev state, trace recorder, profiling
src/menu/             settings/menu IR and map model (host stack is in ambition_app)
src/schedule/         schedule vocabulary (`SandboxSet`, input population, ordering)
src/session/          game mode, reset, and session state
```

Presentation modules that used to live under
`ambition_gameplay_core::presentation` now live in `ambition_render`
(`rendering/`, `hud`, `fx`, `dialog_ui`, `cutscene`, `screen_effects`,
`ui_fonts`). App assembly and binaries live in `ambition_app`.

`lib.rs` re-exports the foundation crates under facade paths (`engine_core`,
`kinematic`, `input`, `time`, `portal`, `actor`, `brain`) plus a few historical shims (`features`, `ldtk_world`, `rooms`, `game_mode`, `trace`). Edit the crate, not a facade.

## Boundary rules

- Reusable, dependency-clean mechanics → a foundation crate. Still-coupled
  machinery stays in `ambition_gameplay_core` until its outward deps are inverted.
- Named game content → `ambition_content`. App assembly / bins / host glue →
  `ambition_app`. The machinery lib must import neither (guard-enforced).
- New gameplay subsystems are self-owning `Plugin`s (components-as-plugins).
- Use components/resources/messages at runtime seams; don't add abstraction
  layers only to avoid Bevy. Keep presentation/packaging out of the engine core.

## Refactor rules for agents

1. Search `dev/` for prior module-split or stale-component traps before broad refactors.
2. Preserve public compatibility shims until downstream imports are updated in the same patch.
3. Keep tests and test helper visibility working during moves.
4. Prefer one focused module move at a time.
5. Regenerate `.agent/` indexes after moving docs, tests, or code symbols.

## Validation anchors

```bash
cargo fmt --check
cargo test -p ambition_engine_core            # engine core unit tests (now its own crate)
cargo test -p ambition_gameplay_core --lib          # machinery
cargo test -p ambition_content --all-features  # named content
cargo test -p ambition_app                    # assembly + integration suites (replay, boundaries, …)
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Related decisions: ADR 0002, ADR 0003, ADR 0012, ADR 0016.
