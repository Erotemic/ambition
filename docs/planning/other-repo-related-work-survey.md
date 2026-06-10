# Related Work Survey for the Plugin Refactor

Reviewed: 2026-06-06

This document surveys Bevy crates, Rust game-engine projects, and reference games that may influence the next stages of the Ambition plugin/runtime refactor. The purpose is not to adopt every crate. The purpose is to keep the refactor honest: if the ecosystem already has a better implementation, pattern, or architecture, we should be willing to replace custom code or reshape our code to interoperate with it.

The survey focuses on these questions:

- What concepts should we borrow even if we do not adopt the crate?
- Which crates should we prototype or benchmark against Ambition's current code?
- Which crates are already in our dependency graph and should be periodically reevaluated?
- Which repos are useful as reference architecture for plugin topology, feature gating, data-driven content, mobile input, tooling, or production Bevy organization?

## Evaluation rubric

Use this rubric when converting an entry below into an implementation decision.

| Question | Why it matters |
|---|---|
| Bevy version and maintenance cadence | Ambition tracks modern Bevy; stale crates can be expensive to carry. |
| Feature-gate shape | The next refactor depends on headless, visible, portal, LDtk, audio, and devtools personas. |
| Runtime vs adapter split | Prefer crates that separate simulation/core from rendering/authoring/network backends. |
| Data-driven content support | We want named game content to move out of compiled constructors where possible. |
| Compile-time impact | A useful crate can still be wrong if it pulls render/audio/UI into headless builds. |
| API ownership | We want semantic messages/resources, not app-global singletons that hard-code Ambition concepts. |
| Replace vs borrow | Many repos are more useful as design examples than dependencies. |
| Migration cost | Replacing movement, save, AI, or networking can invalidate lots of gameplay assumptions. |

## Executive recommendations

### High-value near-term investigations

1. **bevy_asset_loader**: We already depend on it. Reevaluate whether its dynamic asset collections can replace more of Ambition's compiled asset registries and reduce asset-related recompiles.
2. **bevy_proto**: Prototype for entity/content definitions, especially for enemies, pickups, boss rosters, and scripted props. Do not adopt blindly; it may be stale relative to current Bevy, but the prototype/template model is exactly aligned with the content-as-data goal.
3. **bevy_save**: Evaluate for snapshot/checkpoint/save-migration patterns. Our current save system is game-specific; bevy_save's reflection/migration model may provide concepts even if we do not adopt it wholesale.
4. **bevy-tnua**: Compare movement architecture, especially its floating-controller model and Avian integration. It may not replace Ambition's custom platformer controller, but it is important prior art for generic body/controller abstractions.
5. **vleue_navigator**: Prototype pathing for NPCs or macro-navigation if we need route planning over rooms/levels. It may be less useful for tight platforming movement, but its dynamic obstacle/navmesh concepts are worth studying.
6. **dogoap and big-brain**: Compare GOAP vs Utility AI against the current brain/action system. Borrow componentized action/scorer/planner ideas before adding more bespoke AI modules.
7. **Lightyear, bevy_replicon, bevy_quinnet**: Decide early whether Ambition wants future rollback/prediction/multiplayer. Even if multiplayer is not near-term, the runtime crate should avoid assumptions that make deterministic/predictable networking impossible.
8. **scrcpy-mask and virtual_joystick**: Use as inspiration for a configurable mobile input mask/profile layer and external Android test harness, not as core runtime dependencies.
9. **bevy_tween / bevy_easings / smooth-bevy-cameras**: Reevaluate custom camera/easing/visual transition code. Borrow or adopt only behind presentation feature gates.
10. **Bones**: Study architecture seriously. Bones is a stronger architectural reference than a likely dependency: renderer-agnostic core, deterministic ECS, snapshot/restore, modding/scripting, and a separate Bevy renderer mirror many of Ambition's long-term goals.

### Likely keep, but clarify ownership

- `bevy_ecs_ldtk`: Keep as the main LDtk adapter for now, but Ambition-specific converters should move behind `ambition_content` or future `ambition_portal_ldtk`/`ambition_platformer_ldtk` adapter crates.
- `avian2d` and `parry2d`: Keep evaluating. Ambition still has a substantial custom movement/collision stack, so these should be treated as low-level physics/collision tools, not as a full platformer runtime replacement yet.
- `leafwing-input-manager`: Keep for keyboard/gamepad mapping, but layer Ambition semantic input profiles above it so mobile/replay/scripting can use the same intents.
- `bevy_kira_audio` and Yarn Spinner / `bevy_yarnspinner`: Keep as adapters. Do not let audio/dialogue crates leak into headless runtime or content validation builds unnecessarily.

### Watch only for now

- `dip`: Interesting composability/App framework ideas, but likely too broad and orthogonal to the current refactor.
- `bevy_retrograde`: Interesting 2D plugin-pack precedent, but Ambition is already beyond a plugin pack and needs narrower engine/mechanic/adaptor boundaries.
- `bevy_mod_scripting`: Powerful but expensive. Use only if we decide content scripting is truly needed; prefer data registries first.
- Lighting crates (`bevy-magic-light-2d`, `bevy_light_2d`): Consider for future visual polish, not architecture-critical now.
- Networking crates: Important for architecture constraints, but do not adopt until multiplayer/rollback goals are explicit.

## Current Ambition dependency snapshot

Based on the uploaded `portal-proper-physics` snapshot, Ambition currently uses or vendors these notable crates and systems.

| Area | Current dependency | Current use / review point |
|---|---|---|
| Engine | `bevy 0.18.1` | Core app/runtime. Keep explicit feature-gated dependency sets. |
| ECS/math | `bevy_ecs`, `bevy_math` | Runtime crate should prefer narrow Bevy crates where possible. |
| Physics/collision | `avian2d 0.5`, `parry2d 0.26` | Compare with `bevy-tnua`, Avian examples, and our custom platformer solver. |
| LDtk | `bevy_ecs_ldtk 0.14` | Keep, but move Ambition-specific converters out of generic runtime. |
| Assets | `bevy_asset_loader 0.26`, `bevy_common_assets 0.15`, custom `ambition_asset_manager` | Candidate for stronger dynamic asset manifests. |
| Audio | `bevy_kira_audio 0.25`, `ambition_sfx`, `ambition_sfx_bank` | Keep audio as adapter layer; evaluate tooling with `notation`. |
| Dialogue | `bevy_yarnspinner 0.8`, Yarn Spinner Rust | Keep as dialogue adapter; isolate from runtime/persona builds. |
| Input | `leafwing-input-manager 0.20`, `virtual_joystick 2.7.2` | Add semantic input profile/mask layer above physical inputs. |
| UI | vendored `bevy_lunex`, `bevy_material_ui` | Keep isolated from headless runtime. Reevaluate with plugin boundaries. |
| Debug | `bevy-inspector-egui` | Keep devtools optional. Consider `bevy-console` as a command UX. |
| Simulation | `bevy_falling_sand` | Treat as optional mechanic plugin, not runtime core. |
| Data | `ron`, `serde`, `serde_json`, `bevy_common_assets` | Continue RON for content, but evaluate prototypes/schema support. |
| Testing | `insta`, `proptest` | Keep; add architecture-related snapshot/inventory checks. |

## Asset, data, and content-authoring crates

### bevy_asset_loader

URL: https://github.com/NiklasEi/bevy_asset_loader

Status: **already used; high priority to reevaluate**.

What it offers:

- Loading states that wait until asset collections are ready.
- Derivable `AssetCollection` resources.
- Dynamic asset files using keys rather than compile-time paths.
- A clearer split between code and asset paths, which can reduce recompiles while iterating on content.

Applicability to Ambition:

- Strong fit for the move from compiled asset registries to content manifests.
- Useful for the proposed `AmbitionContentPlugin`, `PortalRenderPlugin`, music/dialogue manifests, and sprite catalog cleanup.
- Dynamic assets may help replace hard-coded `GameAssets` fields such as named boss/portal/item handles.

Risk / caution:

- Derive-driven collections can become another form of compiled content if every asset remains a struct field.
- Prefer dynamic asset keys for rosters and content packs.

Recommended next experiment:

- Make one content pack manifest for boss or portal assets using dynamic assets and compare against current `GameAssets` wiring.

### bevy_common_assets

URL: https://github.com/NiklasEi/bevy_common_assets

Status: **already used; keep**.

What it offers:

- Generic loaders for common file formats such as RON.

Applicability:

- Useful for data-driven content definitions, especially if we add `*.boss.ron`, `*.item.ron`, `*.ability.ron`, or `*.world_manifest.ron` assets.

Risk:

- It only loads data. It does not solve schema/versioning/registration policy by itself.

### bevy_proto

URL: https://github.com/MrGVSV/bevy_proto

Status: **prototype; do not adopt blindly**.

What it offers:

- Entity prototypes defined in config files.
- Template inheritance.
- Entity hierarchies.
- Asset references inside prototypes.
- Spawn by prototype key.

Why it matters for Ambition:

- Directly addresses the smell of compiled content rosters: named enemies, pickups, props, and maybe simple bosses could become data/prototypes instead of Rust constructors.
- Might help with `content/features` and `character_catalog` data modeling.

Caution:

- Check Bevy compatibility and maintenance before adopting. If not current, borrow the model instead.
- Reflection/prototype systems can become hard to debug if the schema is too dynamic.

Recommended next experiment:

- Build a small prototype branch for one non-critical entity class: e.g. chest, shrine, pickup, or simple NPC. Measure boilerplate, editor friendliness, and validation clarity.

### bevy_save

URL: https://github.com/hankjordan/bevy_save

Status: **prototype / borrow concepts**.

What it offers:

- Reflection-based snapshots of resources/entities/components.
- Save file location management.
- Checkpointing.
- Reflection-based migrations.
- WASM local-storage support.

Applicability:

- Useful prior art for save migration, reset/checkpoint semantics, and replay/fixture state capture.
- Could inform Ambition's save split between content state, runtime state, and debug checkpoints.

Caution:

- Ambition has game-specific persistence and feature-gated runtime needs. Replacing the save stack wholesale may be disruptive.
- Reflection-based snapshots are powerful but can hide ownership/lifetime mistakes if used as a blanket solution.

Recommended next experiment:

- Prototype save/load of a minimal headless state with version migration. Compare against current `save.rs` and `persistence` modules.

### bevy_mod_scripting

URL: https://github.com/makspll/bevy_mod_scripting

Status: **watch / prototype only if scripting becomes a real requirement**.

What it offers:

- Bevy scripting support, including Lua/Rhai topics.
- Useful for modding or hot-reloaded behavior.

Applicability:

- Could eventually support scripted dialogue actions, cutscene glue, or modded content.
- Relevant if Ambition decides data-only content is not expressive enough.

Caution:

- Heavy architecture commitment.
- WASM support may be limited depending on scripting backend.
- Prefer registries/data schemas before adding a scripting VM.

### Bones

URL: https://github.com/fishfolk/bones

Status: **major architecture reference; unlikely direct dependency**.

What it offers:

- A meta-engine for moddable, multiplayer 2D games.
- Renderer-agnostic core with Bevy renderer integration.
- Deterministic ECS.
- Snapshot/restore.
- Modding/scripting via schema and Lua.

Applicability:

- Very relevant to Ambition's long-term engine split. The layering resembles the desired shape: core game logic independent from rendering, optional Bevy integration, moddable data, and network-friendly state.
- Good reference for how far to push separation between engine/runtime, renderer, scripting, and game content.

Caution:

- Adopting Bones would be a huge rewrite. Treat as a design reference unless we intentionally reevaluate the entire engine architecture.

Recommended next experiment:

- Read Bones architecture docs and write a comparison note: `Ambition runtime split vs Bones lib/framework/bevy renderer`.

## World authoring, levels, rooms, and navigation

### bevy_ecs_ldtk

URL: https://github.com/Trouv/bevy_ecs_ldtk

Status: **already used; keep and isolate**.

What it offers:

- ECS-friendly LDtk loading.
- Level spawning and unloading.
- Entity and IntGrid bundle derivation.
- Hot reload and neighbor loading support.
- Bevy 0.18 compatible version is in use.

Applicability:

- Remains the best fit for LDtk-authored rooms.
- Ambition should keep using it, but the project-specific conversion layer should not live in generic runtime crates.

Caution:

- We have custom active-area composition, room graph, room-scoped lifecycle, portal gates, and validation. Those should remain ours or become Ambition-specific adapters.

### vleue_navigator

URL: https://github.com/vleue/vleue_navigator

Status: **prototype if NPC navigation needs grow**.

What it offers:

- NavMesh pathfinding for Bevy.
- NavMeshes from glTF or obstacle components.
- Dynamic navmesh updating.
- Avian integration compatibility table.

Applicability:

- Could help NPCs reason about room-scale movement, enemy patrols, and non-platforming agents.
- May be useful for topological navigation layered above platformer-specific movement.

Caution:

- Tight 2D platforming pathing is often not a simple navmesh problem. Jump arcs, ledges, one-way platforms, gravity zones, portals, and room transitions need custom reachability logic.

Recommended next experiment:

- Prototype a nav graph for one simple room or hub area. Do not integrate into combat AI until we know how it handles platform-specific constraints.

## Movement, physics, and camera

### Avian / avian2d

URL: https://github.com/avianphysics/avian

Status: **already used; keep evaluating**.

What it offers:

- ECS-driven 2D/3D physics for Bevy.
- Current Ambition dependency is `avian2d 0.5`.

Applicability:

- Good low-level physics substrate.
- Compare with custom collision and movement modules as runtime extraction continues.

Caution:

- Ambition's movement has strong authored platformer semantics. Do not assume generic physics replaces kinematic solver behavior.

### bevy-tnua

URL: https://github.com/idanarye/bevy-tnua

Status: **important comparison / possible partial adoption**.

What it offers:

- Floating character controller for Bevy.
- Supports Rapier and Avian, 2D and 3D via integration crates.
- Maintained through Bevy 0.18-era releases.

Applicability:

- Compare architecture, body API, schedule integration, and physics integration against Ambition's platformer movement.
- Good source for designing generic body/controller traits even if we keep Ambition's custom controller.

Caution:

- The floating-controller model may not match all precision-platformer requirements, especially portals, gravity zones, ledge-grab specifics, and custom affordances.

Recommended next experiment:

- Build a minimal prototype with Avian + Tnua for a small platformer scene. Compare feel and integration complexity; do not replace movement without replay comparisons.

### smooth-bevy-cameras

URL: https://github.com/bonsairobo/smooth-bevy-cameras

Status: **borrow/adopt if camera code remains custom-heavy**.

What it offers:

- Smooth camera controllers with exponential smoothing.

Applicability:

- Compare with Ambition's camera easing and follow logic.
- Could simplify camera behavior if our needs align.

Caution:

- Check Bevy version and maintenance; there may be no formal releases.

### bevy_framepace

URL: https://github.com/aevyrie/bevy_framepace

Status: **dev/performance tool candidate**.

What it offers:

- Frame pacing and frame limiting for Bevy.

Applicability:

- Useful for mobile battery/thermal control, deterministic-ish test harnesses, and dev performance stability.

## Input, mobile, and controller abstraction

### leafwing-input-manager

URL: https://github.com/Leafwing-Studios/leafwing-input-manager

Status: **already used; keep, but layer above it**.

What it offers:

- Stateful input manager for Bevy.
- Good action mapping vocabulary.

Applicability:

- Keep for physical keyboard/gamepad mapping.
- Build Ambition's semantic input profile layer above it so touch, replay, scripts, and external test harnesses can emit the same intents.

### bevy_enhanced_input

URL: https://github.com/simgine/bevy_enhanced_input

Status: **compare with Leafwing and our desired semantic layer**.

What it offers:

- Input manager for Bevy.

Applicability:

- Worth comparing if we decide Leafwing does not match profile/mask needs.

Caution:

- Do not switch input libraries without a clear limitation in the current stack.

### virtual_joystick

URL: https://github.com/SergioRibera/virtual_joystick

Status: **already used; keep / wrap**.

What it offers:

- Bevy virtual joystick UI for mobile games, usable with mouse on desktop.

Applicability:

- Continue using as a physical/mobile input source.
- Wrap it behind an Ambition `InputMappingProfile`/mask layer so layout and output intents become data-driven.

### scrcpy-mask

URL: https://github.com/AkiChase/scrcpy-mask

Status: **tooling/reference; not runtime dependency**.

What it offers:

- Scrcpy client in Rust/Bevy/React for mouse/key mapping to Android device controls.
- Visual key-mapping and scripting concepts.

Applicability:

- Good inspiration for mobile control masks, external Android testing, and input scripting.
- Could support desktop-driven Android QA workflows.

Caution:

- It is host-side tooling, not an in-game input abstraction.
- Do not make the game runtime depend on scrcpy, ADB, or screen-coordinate injection.

Recommended next experiment:

- Create an Ambition `InputMappingProfile` doc/schema with touch regions, virtual sticks, buttons, gestures, physical bindings, and semantic outputs.

## NPC AI and planning

### big-brain

URLs:

- https://codeberg.org/zkat/big-brain
- https://github.com/zkat/big-brain
- https://docs.rs/big-brain/latest/big_brain/

Status: **study / possible borrow; check current Codeberg state**.

What it offers:

- Utility AI library for Bevy.
- Data-driven AI definitions using scorers and actions.
- Parallel/ECS-friendly evaluation.

Applicability:

- Very relevant to the current `brain` and `content/features/ecs/brain_effects` complexity.
- Could replace some bespoke AI scoring/action selection or at least inform a cleaner scorer/action split.

Caution:

- Development moved from GitHub to Codeberg; the GitHub archive is read-only.
- Verify current Bevy compatibility and maintenance before adopting.

### dogoap

URL: https://github.com/victorb/dogoap

Status: **prototype for goal-driven NPC behavior**.

What it offers:

- Data-oriented GOAP with Bevy integration.
- Dynamic setup of states/actions/goals.
- Useful when NPCs need multi-step task planning.

Applicability:

- Good for non-combat NPCs, quest behaviors, and world simulation tasks.
- Could complement Utility AI: GOAP for deliberate plans, utility scoring for tactical choices.

Caution:

- It may be overkill for tight enemy combat behaviors that are more state-machine or behavior-tree-like.

Recommended next experiment:

- Model one simple NPC task: move to item, pick it up, move to target, drop/use item. Compare dogoap complexity to current state-machine code.

## Networking and multiplayer architecture

### Lightyear

URL: https://github.com/cBournhonesque/lightyear

Status: **strategic reference / future prototype**.

What it offers:

- Bevy multiplayer networking with prediction/interpolation-oriented features.
- Configurable input buffers, interpolation delay, send rate, bandwidth management, lag compensation, observability, and Avian examples.

Applicability:

- Important if Ambition may eventually support multiplayer, rollback, or deterministic replay.
- Even without adopting, it should influence runtime design: semantic inputs, deterministic state boundaries, snapshot/restore, clear component ownership.

Caution:

- Adopting network prediction late can require huge refactors. Decide architecture constraints early, adoption later.

### bevy_replicon

URL: https://github.com/simgine/bevy_replicon

Status: **strategic reference / possible replication layer**.

What it offers:

- Server-authoritative replication for Bevy.
- Automatic world replication, remote events/triggers, authorization, visibility control, backend-agnostic transport integration, and active Bevy 0.18 support.

Applicability:

- Strong architecture model: replication core separate from messaging backends.
- Its backend separation is a good pattern for Ambition adapters.

Caution:

- Server-authoritative replication is different from rollback/prediction-first action games. Compare with Lightyear before choosing.

### bevy_quinnet

URL: https://github.com/Henauxg/bevy_quinnet

Status: **transport candidate, not gameplay networking by itself**.

What it offers:

- QUIC-based client/server networking for Bevy.
- Reliable/unreliable ordered/unordered channels.
- Synchronous Bevy-facing API over async internals.

Applicability:

- Useful as a transport or for tooling/server control.
- Less directly useful than Lightyear/Replicon for gameplay replication/prediction.

### extreme_bevy

URL: https://github.com/johanhelsing/extreme_bevy

Status: **reference game**.

What it offers:

- Low-latency multiplayer action game reference, including P2P and rollback networking in a browser.

Applicability:

- Reference for browser/network constraints, rollback architecture, and action-game input/state management.

## Presentation, animation, lighting, effects, and UI polish

### bevy_easings

URL: https://github.com/vleue/bevy_easings

Status: **possible adoption for simple component easing**.

What it offers:

- Easing components to target values.
- Ease functions, chaining, custom component support.
- Bevy version support through 0.18.

Applicability:

- Good for simple UI and presentation transitions.
- Could reduce custom one-off easing systems.

Caution:

- For complex animation graphs, `bevy_tween` may be more flexible.

### bevy_tween

URL: https://github.com/Multirious/bevy_tween

Status: **prototype for richer animation/tween workflows**.

What it offers:

- Procedural/keyframe animation library.
- Functional/declarative composition model.
- Parallel/sequential/overlapping typed tweens.
- Event emission from animation timelines.

Applicability:

- Relevant for portal visuals, boss presentation, UI transitions, and scripted visual beats.

Caution:

- README describes it as young with breaking changes expected. Keep behind presentation feature gates if adopted.

### bevy_spritesheet_animation

URL: https://github.com/merwaaan/bevy_spritesheet_animation

Status: **compare with current animation code**.

What it offers:

- Bevy plugin for animating 2D and 3D sprites.

Applicability:

- Could simplify character sprite animation if current custom animation sheets/animator code remains hard to maintain.

### bevy-magic-light-2d and bevy_light_2d

URLs:

- https://github.com/zaycev/bevy-magic-light-2d
- https://github.com/jgayfer/bevy_light_2d

Status: **future visual polish / prototype only**.

Applicability:

- Consider if Ambition wants 2D lighting/shadow effects, especially in caves/labs/boss rooms.

Caution:

- Lighting can pull render dependencies and shader maintenance burden. Keep as presentation-only experiments.

### bevy_mod_billboard

URL: https://github.com/kulkalkul/bevy_mod_billboard

Status: **niche presentation candidate**.

Applicability:

- Useful if 3D text/labels/billboards remain part of debug or presentation layers.

### bevy_plot

URL: https://github.com/eliotbo/bevy_plot

Status: **debug/effects exploration**.

Applicability:

- Potentially useful for math-based effects, tuning curves, debug visualizations, and in-game inspector graphs.

### notation

URL: https://github.com/notation-fun/notation

Status: **tooling reference for audio/music generation**.

Applicability:

- Could inspire tooling for generated music/SFX visualization or validation.
- More likely a sidecar tool than runtime dependency.

## Dialogue, localization, and text

### Yarn Spinner Rust / bevy_yarnspinner

URLs:

- https://github.com/YarnSpinnerTool/YarnSpinner-Rust
- https://docs.rs/bevy_yarnspinner/latest/index.html
- https://docs.yarnspinner.dev/2.5/api/rust

Status: **already used; keep as adapter**.

Applicability:

- Continue using for authored dialogue.
- Keep dialogue plugins separate from runtime/headless checks unless content validation needs them.

### bevy_fluent

URL: https://github.com/kgv/bevy_fluent

Status: **evaluate if localization becomes near-term**.

Applicability:

- Fluent can be a stronger localization layer than ad hoc strings.
- Relevant once dialogue/UI localization is a real goal.

## Devtools, console, and workflow

### bevy-console

URL: https://github.com/makspll/bevy-console

Status: **prototype for devtools**.

Applicability:

- Useful for runtime debug commands: teleport, give item, set room, spawn entity, toggle feature, inspect quest flags, run replay, trigger music cue.
- Could reduce custom debug UI clutter.

Caution:

- Keep behind `devtools`; do not let command parsing leak into gameplay code.

### bevy-inspector-egui

URL: https://github.com/jakobhellermann/bevy-inspector-egui

Status: **already used**.

Applicability:

- Keep as dev-only inspector.
- Consider schedule/resource/entity inspector conventions in architecture docs.

### bevy_framepace

URL: https://github.com/aevyrie/bevy_framepace

Status: **dev/mobile performance candidate**.

Applicability:

- Frame pacing, mobile thermal control, debugging jank, and test determinism.

## Reference repositories and production architecture

### awesome-bevy

URL: https://github.com/nolantait/awesome-bevy

Status: **ongoing discovery index**.

Applicability:

- Use as a periodic survey source. It explicitly tracks up-to-date Bevy resources and categories such as assets, code organization, workflow, networking, pathfinding, physics, UI, and testing.

### bevy_best_practices

URL: https://github.com/tbillington/bevy_best_practices

Status: **reference for code organization conventions**.

Applicability:

- Compare against Ambition's plugin grouping, schedule sets, state management, and module ownership.

### bevy_awesome_prod

URL: https://github.com/ThierryBerger/bevy_awesome_prod

Status: **production reference index**.

Applicability:

- Use to find examples of feature gating, release pipelines, mobile/web deployment, and production-ready Bevy organization.

### bevy-basics

URL: https://github.com/marcelchampagne/bevy-basics

Status: **onboarding reference**.

Applicability:

- Good for onboarding docs and basic examples, not architecture-critical.

### bevy_mod_inverse_kinematics

URL: https://github.com/Kurble/bevy_mod_inverse_kinematics

Status: **niche mechanic candidate**.

Applicability:

- Could matter for grapples, tentacles, arms, boss appendages, or procedural animation.

### Celeste Bevy remake/reference

URL: https://github.com/NightsWatchGames/celeste

Status: **platformer reference**.

Applicability:

- Useful for comparing platformer movement organization, state machine design, camera, and level data assumptions.

### Additional use-case repos from the candidate list

These should be inspected more deeply if their area becomes active:

| Repo | URL | Why inspect |
|---|---|---|
| Astratomic | https://github.com/spicylobstergames/astratomic | Production-ish Bevy game structure and content organization. |
| rust-game-ports | https://github.com/rust-gamedev/rust-game-ports | Survey of porting strategies and idioms. |
| TheSeeker | https://github.com/TheSeekerGame/TheSeeker | Reference game architecture. |
| bevy-2d-shooter | https://github.com/bones-ai/bevy-2d-shooter | 2D shooter architecture; may overlap with projectiles/combat. |
| extreme_bevy | https://github.com/johanhelsing/extreme_bevy | Browser rollback/networking/action-game reference. |

## Other relevant repos to add to the watchlist

These were not all in the original candidate list, but they are relevant to the architecture goals.

| Area | Repo | Why it matters |
|---|---|---|
| Particles/VFX | https://github.com/djeedai/bevy_hanabi | Mature Bevy GPU particle system; potential VFX replacement/reference. |
| Shapes/vector rendering | https://github.com/rparrett/bevy_prototype_lyon | Could simplify 2D debug/visual primitives. |
| Picking/pointer UX | https://github.com/aevyrie/bevy_mod_picking | Useful for editors/debug overlays if Bevy version aligns. |
| Schedule graph debugging | https://github.com/jakobhellermann/bevy_mod_debugdump | Useful for architecture guardrails and plugin schedule inspection. |
| Bevy templates | https://github.com/NiklasEi/bevy_game_template | CI/mobile/web packaging reference. |
| Blender/authoring workflow | https://github.com/kaosat-dev/Blenvy | Useful if asset authoring expands beyond LDtk/sprites. |
| Sprite/vector shapes | https://github.com/james-j-obrien/bevy_vector_shapes | Alternative for debug and visual primitives. |
| Large-world support | https://github.com/aevyrie/big_space | Probably not needed now, but relevant if world scale grows. |

## Domain-specific conclusions

### Asset/content direction

Best path:

1. Keep `bevy_asset_loader` and `bevy_common_assets`.
2. Expand dynamic asset manifests for content packs.
3. Prototype `bevy_proto` or a smaller internal prototype system for non-behavioral entity definitions.
4. Keep Rust for behavior kernels; move named rosters/tuning/rewards/assets into data.

Avoid:

- Replacing all content with a scripting VM before data schemas are exhausted.
- Encoding every asset path as a compiled `GameAssets` field.

### Movement/physics direction

Best path:

1. Keep Ambition's custom movement until replay tests prove an alternative matches feel.
2. Compare body vocabulary and controller architecture with `bevy-tnua`.
3. Continue using Avian/Parry as low-level substrates where useful.
4. Extract generic body/world-query/transit runtime vocabulary before attempting a movement rewrite.

Avoid:

- Replacing the controller just because a crate exists.
- Moving player-specific kinematics into the runtime crate.

### AI direction

Best path:

1. Split current AI into generic scorer/action/planner vocabulary where possible.
2. Prototype big-brain-style Utility AI for tactical choices.
3. Prototype dogoap-style planning for NPC/world tasks.
4. Keep boss attack execution kernels in Rust; move rosters/tuning to data.

Avoid:

- Adding a second AI framework without retiring or isolating current `brain` modules.

### Input/mobile direction

Best path:

1. Keep Leafwing and virtual joystick as physical input sources.
2. Introduce an Ambition semantic `InputMappingProfile` / mask layer.
3. Use scrcpy-mask as external tooling inspiration for Android QA and visual mapping.
4. Make replay/test/script inputs emit the same semantic intents as keyboard/touch/gamepad.

Avoid:

- Binding mobile runtime directly to screen-coordinate injection or scrcpy tooling.

### Networking/determinism direction

Best path:

1. Do not adopt networking yet unless multiplayer becomes a real goal.
2. Study Lightyear/Replicon now to avoid runtime decisions that block networking later.
3. Prefer semantic input messages, snapshot-friendly state, and deterministic-ish boundaries.

Avoid:

- Mixing networking libraries into gameplay before runtime boundaries are stable.

### Presentation/effects direction

Best path:

1. Keep presentation crates optional.
2. Compare simple easing with `bevy_easings`, rich animation with `bevy_tween`, and custom camera code with `smooth-bevy-cameras`.
3. Move portal/gravity visual systems into separate render/presentation modules before adopting more visual crates.

Avoid:

- Pulling render-heavy crates into headless/runtime builds.

## Proposed survey follow-up tasks

### Task RW-1: Cargo dependency audit

Create a generated file:

```text
docs/generated/current_cargo_dependencies.md
```

Include crate, version, feature flags, current owner module, and reevaluation status.

### Task RW-2: Asset/content prototype comparison

Prototype one simple content family in three forms:

1. Current Rust constructors.
2. Internal RON schema loaded by `bevy_common_assets`.
3. `bevy_proto` if compatible.

Suggested content family: pickups, shrines, or simple enemies.

### Task RW-3: Save/checkpoint spike

Implement a tiny `bevy_save` spike in a throwaway branch:

- capture a small world snapshot,
- reload it,
- add a versioned migration,
- test WASM/local-storage story if relevant.

### Task RW-4: Movement comparison spike

Build a minimal Avian + Tnua scene matching one Ambition movement fixture. Compare feel, runtime dependencies, and testability.

### Task RW-5: AI planning spike

Model the same NPC behavior in big-brain style and dogoap style, then decide whether current `brain` should borrow a Utility AI, GOAP, or behavior-tree vocabulary.

### Task RW-6: Mobile input profile design

Write an `InputMappingProfile` schema inspired by virtual joystick + scrcpy-mask. Do not implement full UI yet.

### Task RW-7: Portal/render split survey application

Before adopting animation/lighting crates, move portal presentation out of portal core and define the API it would consume. Then compare `bevy_tween`, `bevy_easings`, and custom systems for that API.

## Decision matrix

| Decision | Recommended next move | Do not do yet |
|---|---|---|
| Asset manifests | Expand dynamic asset use with `bevy_asset_loader` | Full asset system rewrite |
| Entity prototypes | Prototype `bevy_proto` or internal equivalent | Move all gameplay to reflection immediately |
| Save system | Spike `bevy_save` concepts | Replace current save before migration story exists |
| Movement | Compare with Tnua | Replace custom controller without replay proof |
| AI | Prototype big-brain/dogoap patterns | Add multiple AI frameworks to production path |
| Input | Design semantic profile/mask layer | Make scrcpy-mask a runtime dependency |
| Networking | Study Lightyear/Replicon constraints | Adopt networking before deterministic boundaries exist |
| Visual effects | Split presentation first | Pull visual crates into core runtime |
| Scripting | Keep on watchlist | Add scripting VM before content schemas are insufficient |

## Notes on adoption posture

The healthiest default is: **borrow architecture first, adopt dependency second**.

A crate should become a dependency only when it passes at least one of these thresholds:

- It replaces a substantial custom subsystem with better behavior and lower maintenance.
- It gives us a data/editor workflow that would be expensive to build internally.
- It has a clean feature-gated core/adapter split that matches our build personas.
- It is already a dependency and we can make better use of it rather than writing parallel systems.

A crate should remain a reference if:

- It solves a related problem but brings the wrong runtime assumptions.
- It is too broad for the immediate goal.
- It is behind Bevy version compatibility.
- It would force us to rewrite gameplay semantics before the runtime boundaries are stable.


