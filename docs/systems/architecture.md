# Architecture

Ambition is a Rust/Bevy workspace, Bevy-native and ECS-first (old backend-neutral
constraints superseded by ADR 0002). Post-bisection (Stage 20) it is a **4-layer
crate graph**; lower layers must never import higher ones. Survey + remaining work:
`docs/planning/plugin_refactor/22_monolith_breaker_survey.md`.

## Crate layers

| Layer | Crates | Responsibility |
|---|---|---|
| foundations | `ambition_engine_core` (movement/collision/body/geometry/world/player clusters), `ambition_platformer_runtime` (kinematic, gravity, rooms, projectile), `ambition_portal`, `ambition_time`, `ambition_input`, `ambition_menu` (reusable renderers), `ambition_audio`, `ambition_sfx[_bank]`, `ambition_asset_manager` | Reusable, content-free, no dep on the layers below |
| machinery | `ambition_sandbox` (lib) | brain, actor, mechanics, `features` (named actor/boss ECS world), presentation, world/LDtk, items, encounter, persistence, dev STATE, menu IR/map. Content-free (guard-enforced). Re-exports foundations under facade paths (`crate::engine_core`, `crate::input`, …). |
| content | `ambition_content` | Named game content: quests, bosses, items roster, dialogue, intro, banter, portal adapters |
| app | `ambition_app` | Bevy assembly, host glue, ALL binaries (playable `ambition_sandbox` bin, headless, rl_*), menu host stack + `DevToolsPlugin`, full-stack integration tests |

## Machinery (`ambition_sandbox`) module shape

```text
src/brain|actor/      universal brain + actor control (bosses are actors, ADR 0016)
src/mechanics/        combat kit + gravity (content-free)
src/features/         named actor/boss ECS world (still lib; B3-tracked)
src/world/            LDtk world, room building, physics, platforms
src/player/           player ECS components and systems
src/presentation/     sprites, camera, parallax, rendering, UI fonts
src/persistence/      save data and settings
src/dev/              dev STATE (dev_tools), trace recorder, profiling
src/menu/             settings IR, Map tab, backend selector (host stack is in ambition_app)
src/app/              SCHEDULE VOCABULARY only (SandboxSet, input populate) — assembly is in ambition_app
```

`lib.rs` re-exports the foundation crates under facade paths (`engine_core`,
`kinematic`, `input`, `time`, `portal`) plus a few historical shims (`features`,
`ldtk_world`, `rooms`, `game_mode`, `trace`). Edit the crate, not a facade.

## Boundary rules

- Reusable, dependency-clean mechanics → a foundation crate. Still-coupled
  machinery stays in `ambition_sandbox` until its outward deps are inverted.
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
cargo test -p ambition_sandbox --lib          # machinery
cargo test -p ambition_content --all-features  # named content
cargo test -p ambition_app                    # assembly + integration suites (replay, boundaries, …)
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Related decisions: ADR 0002, ADR 0003, ADR 0012, ADR 0016.
