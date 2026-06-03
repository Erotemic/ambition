# Capability matrix

Purpose: compact inventory of what Ambition can currently express. This is not a changelog, not a roadmap, and not implementation truth. Code, ADRs, and `docs/current/` win when there is a conflict.

**Review date:** 2026-05-27. Reviewed against source archive `ambition-source-2026-05-26T222032-5-3e93516618a5`.

For implementation details, start from `docs/current/state.md`, `docs/systems/index.md`, `docs/mechanics/expressibility-checklist.md`, and the generated indexes under `.agent/`.

## Movement and traversal

| Capability | Status | Where to read next |
|---|---:|---|
| Kinematic platformer controller, coyote/buffered jump, dash, wall cling/jump, fast fall | Available | `docs/concepts/movement-collision.md`, `docs/mechanics/expressibility-checklist.md` |
| Jump and dash input buffers | Available | `docs/mechanics/abilities.md`, `crates/ambition_sandbox/src/engine_core/movement/tuning.rs` |
| General action buffering for attack / pogo / projectile / tool / blink | Not yet unified | `docs/mechanics/abilities.md`, `docs/current/next.md` |
| Blink / short-range teleport with collision safety policy | Available | `docs/mechanics/blink.md`, `docs/systems/collision-geometry-and-secondary-physics.md` |
| Mark / Recall teleport ability (drop a mark, recall to it) | Available (landed 2026-06-03); first *wired* ability slot — a held item: `Attack` drops the mark, `Blink` recalls; equippable from the OoT menu; debug ground drop until authored placement; no beacon sprite yet | `crates/ambition_sandbox/src/mark_recall.rs` |
| Fireball ability (ranged shot that explodes on contact) | Available (landed 2026-06-03); wired ability — a ranged held item whose shot detonates a splash `HitEvent` (AOE) on hitting a body or wall, unlike the gun-sword's single-target bolt; equippable from the OoT menu; sold by the merchant (40g) | `crates/ambition_sandbox/src/item_pickup.rs` (`fire_held_ranged_system`, `held_projectile_step`) |
| Blink ability (short collision-clamped teleport + arrival shockwave) | Available (landed 2026-06-03); wired ability — `Attack` teleports the player up to a fixed distance along aim, `raycast_solids`-clamped to stop short of walls, and emits a small player-side shockwave `HitEvent` at the arrival point so you can blink *into* a cluster of enemies (composes with a gravity well). Equippable from the OoT menu; sold by the merchant (45g). Distinct from the `BlinkWall`/`blink.md` lore | `crates/ambition_sandbox/src/blink.rs` |
| Grapple ability (yank toward a grappled surface) | Available (landed 2026-06-03); wired ability — `Attack` casts along aim and, on a solid within range, yanks the player toward it with a burst velocity (collision settles them at the surface); a dry grapple fizzles; equippable from the OoT menu; sold by the merchant (45g) | `crates/ambition_sandbox/src/grapple.rs` |
| Ledge grab / mantle | Partial but implemented | `docs/mechanics/abilities.md`, `docs/mechanics/expressibility-checklist.md` |
| Body-mode traversal such as crouch, crawl, slide, compact/morph shapes | Available, needs more authored rooms | `docs/mechanics/body-modes.md` |
| Moving platforms and path motion | Available, needs more carry/edge validation | `docs/systems/collision-geometry-and-secondary-physics.md`, `docs/planning/tech-debt-log.md` |
| Climb-through-solid at ladder tops | Available; a climbing body passes through non-Hazard blocks intersecting the active climbable contact region, so authors needn't carve a gap where a ladder meets a floor | `crates/ambition_sandbox/src/engine_core/movement/collision.rs`, `crates/ambition_sandbox/src/engine_core/movement/tests/climbing.rs` |
| Sprint-jump / long-jump momentum tier | Not yet reusable backend | `docs/mechanics/abilities.md` |
| Grapple / tether / harpoon-dash constraints | Not yet reusable backend | `docs/mechanics/expressibility-checklist.md` |

## Combat, actors, and interactions

| Capability | Status | Where to read next |
|---|---:|---|
| Directional melee, upward slash, downward slash / pogo | Available | `docs/mechanics/abilities.md`, `docs/systems/brain-driver.md` |
| Player attack start via `ActorActionMessage::Melee` | Available, pogo still has a raw player-specific path | `docs/systems/brain-driver.md` |
| Enemy melee windup start via `ActorActionMessage::Melee` plus hitbox-entity lifecycle | Available | `docs/systems/brain-driver.md`, `crates/ambition_sandbox/src/content/features/ecs/brain_effects.rs` |
| Enemy ranged and boss specials via `ActorActionMessage` consumers | Available for current ranged enemies and authored boss specials | `docs/systems/brain-driver.md`, `docs/systems/boss-encounter-architecture.md` |
| Smash brain verb-selection by range (ranged mid-range, dash-to-close, melee up close) | Available (landed 2026-06-03); the enemy/player unification flex — a ranged-capable Smash actor fires at mid-range on a brain cadence (`RANGED_COOLDOWN_S`), bursts a dash to close a *large* gap (`dash_to_close`, `DASH_COOLDOWN_S`), then melees up close; goblins (`MediumStriker`) now throw rocks + dash, a melee+ranged+dash+jump kit. No throw animation / balance pass yet | `crates/ambition_sandbox/src/brain/smash/mod.rs` (`maybe_substitute_ranged`/`maybe_substitute_dash`) |
| Player wields boss attacks — shockwave (AOE) / volley (ranged fan) / focus beam (line) | Available (landed 2026-06-03); three gauntlet held items the player wields via the faction-tagged `Hitbox` + faction-aware projectile pool (player faction → damages enemies); each is a defeated boss's signature drop (T-Rex→shockwave, mockingbird→volley, smirking_behemoth→beam) + a debug ground item. Cost mana | `crates/ambition_sandbox/src/shockwave.rs`, `volley.rs`, `beam.rs` |
| Possession — take over a non-boss actor (Down+Interact hold) | Available (landed 2026-06-03); drive an enemy/actor from player input through its own `ActorControlFrame`; it flips to your faction (melee+ranged redirect at its former allies), the camera follows it, and on release you **vacate** — step out where it stood. Boss case + NPC-driving + body-hide visual are handoffs | `crates/ambition_sandbox/src/possession.rs` |
| Projectiles and motion-input upgrades | Available | `docs/mechanics/projectiles-and-motion-inputs.md` |
| Shield/parry/bubble-shield vocabulary | Available | `docs/mechanics/abilities.md` |
| Player-allied summon (puppy-slug gun) | Available (landed 2026-06-03); a held item summons capped, player-faction passive puppy slugs that harm enemies but not the player via `is_player_side`; generalized `spawn_runtime_minion(faction, aggression)` | `crates/ambition_sandbox/src/puppy_slug_gun.rs` |
| Actor, faction, damage, interactable, pickup, breakable vocabulary | Available | `docs/systems/progression-systems.md`, `docs/systems/factions.md` |
| Canonical `HitSpec` / `HitInstance` / `HitResult` pipeline | Missing; damage is functional but fragmented | `docs/systems/gameplay-effects.md`, `docs/current/next.md` |
| Enemy archetypes, boss profiles, encounter lock walls/rewards | Partial but playable | `docs/systems/boss-behavior-profiles.md`, `docs/systems/boss-encounter-architecture.md`, `docs/planning/tech-debt-log.md` |
| Authored bosses (data-driven) | Available; Gradient Sentinel, Mockingbird, GNU-ton, Smirking Behemoth, **Flying Spaghetti Monster** (aerial), **T-Rex** (grounded melee) — each a `boss_profiles.ron` row + `boss_encounters/<id>.ron` + arena. New boss reusing existing attacks = data + an LDtk BossSpawn (no new attack code) | `crates/ambition_sandbox/assets/data/boss_profiles.ron`, `crates/ambition_sandbox/src/boss_encounter/` |
| Universal Brain interface: `Brain` enum, state-machine templates, per-entity `ActionSet`, `ActorControl`, and `ActorActionMessage` resolver | Available and live for player movement, player melee start, enemy ranged, enemy melee start, and current boss specials | `crates/ambition_sandbox/src/brain/`, `docs/systems/brain-driver.md`, `docs/recipes/extending-brains-and-action-sets.md` |
| Character catalog (RON-driven authoring) | Available; 97 catalog entries in the reviewed archive, keyed by content rows rather than Rust-only spawns | `crates/ambition_sandbox/assets/data/character_catalog.ron`, `docs/systems/character-catalog.md`, `docs/recipes/adding-a-character.md`, `docs/adr/0017-rust-behavior-ron-content-ldtk-space.md` |
| Hall of Characters room | Available; auto-generated catalog showcase / sprite audit surface | `tools/ambition_ldtk_tools/specs/hall_of_characters_area.ron`, `tools/ambition_ldtk_tools/ambition_ldtk_tools/generate_hall_of_characters.py`, `tools/ambition_ldtk_tools/ambition_ldtk_tools/inspect_hall_sprites.py` |
| LDtk authoring toolkit | Available; includes IntGrid paint/erase, debug labels, spawn-overlap checks, and wall/door validation | `tools/ambition_ldtk_tools/`, `docs/systems/sprite-rendering-surface.md` |
| Boss-encounter spec as RON content | Available; 3 authored boss specs in the reviewed archive, with Rust profiles still owning behavior wiring | `crates/ambition_sandbox/assets/data/boss_encounters/`, `crates/ambition_sandbox/src/boss_encounter/specs.rs`, `docs/systems/boss-encounter-architecture.md` |
| Gradient Sentinel boss fight | Available in current sandbox code; multi-special boss using focused `ActorActionMessage::Special` consumers | `crates/ambition_sandbox/src/content/features/bosses.rs`, `crates/ambition_sandbox/src/content/features/ecs/brain_effects.rs` |
| Smirking Behemoth "cut the rope" boss | Available; environmental boss in a dedicated `you_have_to_cut_the_rope.ldtk` world — cut the suspended rope prop to drop a heavy object (rope/anvil/piano variants, per-variant victory dialogue) on the boss, which attacks with `EyeBeam`; includes a room-replay command | `crates/ambition_sandbox/src/boss_encounter/cut_rope.rs`, `crates/ambition_sandbox/assets/data/boss_profiles.ron`, `crates/ambition_sandbox/assets/ambition/worlds/you_have_to_cut_the_rope.ldtk` |
| GNU-ton boss apple-rain attack | Available; gravity-driven apples drop from the ceiling on a golden-ratio x-spread during the scheduled `GnuAppleRain` strike window (phase1), self-dodging the boss body, reusable via `SpecialActionSpec::GnuAppleRain` | `crates/ambition_sandbox/src/content/features/ecs/brain_effects.rs`, `crates/ambition_sandbox/assets/data/boss_profiles.ron` |
| Per-boss phase attack schedules in data | Available; each boss's `attack_pattern: Scripted(intro/phase1/transition/phase2/enrage)` Telegraph/Strike/Rest steps are authored in `boss_profiles.ron`, not hardcoded in Rust | `crates/ambition_sandbox/assets/data/boss_profiles.ron`, `crates/ambition_sandbox/src/brain/boss_pattern.rs` |
| Dialogue/commerce hooks | Scaffolded | `docs/systems/progression-systems.md`, `docs/adr/0008-dialogue-and-commerce-architecture.md` |
| Dialogue ↔ inventory read+grant bridge | Available; Yarn `inventory_has(item)` reads the live catalog (via the per-frame `YarnStateMirror`) and `<<give_item kind count>>` adds to it (24-item catalog + legacy aliases) | `crates/ambition_sandbox/src/dialog/yarn_bindings.rs`, `crates/ambition_sandbox/src/items.rs` |
| Gravity grenade (thrown well that lifts enemies) | Available (landed 2026-06-03); a thrown held item whose fuse opens a temporary up-gravity `TemporaryZone` well — enemies/items caught in it float up (crowd control). Emergent: the grenade only spawns a `GravityZone`; the localized-gravity system does the lifting. Held-item only (no catalog slot), debug-spawned | `crates/ambition_sandbox/src/gravity_grenade.rs`, `crates/ambition_sandbox/src/physics.rs` (`TemporaryZone`) |
| Directional shield-block (defensive verb) | Available (landed 2026-06-03); holding the shield fully negates a hit coming from the side the player faces (you can't guard your back) — a short guard i-frame, a "blocked" banner + clang. Pure `shield_blocks_hit` decision, applied in the player-damage path | `crates/ambition_sandbox/src/app/world_flow.rs` (`shield_blocks_hit`, `handle_player_damage_events`) |
| Bosses drop the ability they embody (learn powers from combat) | Available (landed 2026-06-03); a defeated boss drops a `PickupKind::Ability` pickup (FSM→Blink, T-Rex→Grapple) that `collect_ecs_pickups` grants into `OwnedItems` on overlap, so you can equip it from the OoT menu. The north-star "every boss a failed objective function, every upgrade a theorem" loop | `crates/ambition_sandbox/src/content/features/ecs/damage.rs`, `.../pickups.rs` |
| Enemies drop their weapon on defeat (steal weapons) | Available (landed 2026-06-03); a defeated enemy that was wielding a held item (e.g. a pirate's gun-sword) drops it as a `GroundItem` the player can grab + wield via the existing pickup path | `crates/ambition_sandbox/src/content/features/ecs/damage.rs` |
| Economy earn-side: enemies drop currency on defeat | Available (landed 2026-06-03); a defeated standard enemy drops a collectible `PickupKind::Currency` coin at the death site (`drop_currency_coin`), reusing the existing `collect_ecs_pickups` -> wallet path. Closes kill -> coin -> wallet -> merchant/ability shop. Flat amount (working loop, not balanced) | `crates/ambition_sandbox/src/content/features/ecs/damage.rs` |
| Merchant buy/sell economy (dialogue-driven shop) | Available (landed 2026-06-03); `<<buy_item>>`/`<<sell_item>>` + `wallet_balance()`/`can_afford()` over `PlayerWallet` + `OwnedItems`; `merchant_seed` node is a working shop, NPC placed in `sandbox.ldtk`; stocks axe/health/mana + the gun-sword, puppy-slug gun, and bomb (buy → equip-from-menu → use). Dedicated shop overlay UI is a follow-up | `crates/ambition_sandbox/src/shop.rs`, `crates/ambition_sandbox/assets/dialogue/sandbox/kernel.yarn` |

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
| Per-player `PlayerInputFrame` mirror of the primary `ControlFrame` | Available | `docs/systems/input-and-control-frame.md` |
| Player `Brain::Player` translating input to `ActorControlFrame` | Available | `docs/systems/brain-driver.md` |
| Menu navigation, pause mode, inventory/map/pause UI routing | Available | `docs/systems/ui-navigation-and-pause.md` |
| OoT-style 6×4 item-grid inventory menu | Available (landed 2026-06-03, feature `oot_inventory`); native Bevy-UI grid of all 24 catalog items (owned/dim/equipped/cursor), confirm equips a weapon (shared HeldItem seam) or uses a consumable; keyboard/gamepad/touch nav; easy-cut seam (legacy 3-tab menu when off). 3D OoT "cube" renderer + item icons are follow-ups | `crates/ambition_sandbox/src/oot_menu/`, `crates/ambition_sandbox/src/items.rs`, `submodules/ambition_inventory_ui/DESIGN-OOT-DEMO.md` |
| Finite 24-item pickup catalog (`Item` + `OwnedItems`) | Available; 24 = OoT item-subscreen slot count, the game's complete pickup set; pickups + `<<give_item>>` + Yarn mirror read/write it; legacy 3-kind `PlayerInventory` bridged | `crates/ambition_sandbox/src/items.rs` |
| Settings persistence across audio/video/gameplay/controls | Available | `docs/systems/settings-and-persistence.md` |
| Mobile touch controls | Available, platform-sensitive | `docs/systems/mobile-touch-controls.md` |
| Context-sensitive control HUD (verb + per-device glyph + pressed-state) | Available; gameplay still owns its own execution branches | `docs/systems/control-affordances.md` |
| Camera screen shake on a hard fall | Available (landed 2026-05-28) | `crates/ambition_sandbox/src/time/camera_ease.rs` — `CameraShakeState`, `tick_camera_shake`, `hard_fall_shake_amplitude` |
| Desktop, web, Android/mobile, controller, Steam Deck paths | Current targets | `docs/concepts/platform-targets.md`, `docs/recipes/index.md` |

## Validation and agent support

| Capability | Status | Where to read next |
|---|---:|---|
| Engine + sandbox unified into a single crate | Available (landed 2026-05-28) | `crates/ambition_sandbox/src/engine_core/` (formerly the `ambition_engine` crate); `docs/adr/0002-engine-must-be-bevy-native.md` |
| Headless simulation entry point | Available | `docs/systems/headless-simulation.md` |
| RL reward-shaping examples (survival / exploration / health terms + `SandboxSim::step_with_reward`) | Available | `crates/ambition_sandbox/src/rl_sim/reward.rs`, `docs/systems/headless-simulation.md` |
| Headless room verification (no GPU): geometry-debug PNG renderer + spatial-anomaly report + CI guard | Available; renders collision, authored entities, moving platforms, kinematic paths, camera zones, and live boss hurtboxes; `-- report` scans every room for out-of-bounds entities / spawn-in-solid; `room_spatial_integrity` fails the build on a regression | `cargo run -p ambition_sandbox --example render_room_geometry -- <ROOM_ID\|all\|report>`; `docs/recipes/headless-room-verification.md` |
| Trace recording / replay for movement bugs | Available | `docs/systems/gameplay-trace-recorder.md` |
| Agent-readable indexes | Available, generated | `.agent/manifest.yaml`, `.agent/index/` |
| Documentation health checks | Available | `scripts/check_agent_kb.py`, `scripts/check_doc_links.py` |
| LDtk area-spec coordinate drift check | Available | `tools/ambition_ldtk_tools` `level diff-specs --all`; `dev/benchmark-candidates/ldtk-area-spec-drift-2026-05-21.md` |
| LDtk read-only spatial queries for LLM authoring (`intgrid query` / `entity measure` / `gates audit`) | Available | `tools/ambition_ldtk_tools`; `docs/concepts/llm-spatial-authoring-discipline.md` |
| Music cue loudness/level report + clipping gate | Available | `tools/ambition_music_renderer/level_report.py` (`--check`); `docs/recipes/generated-music-workflow.md` |
| EnemySpawn `Patrol:<id>` brain reference validation | Available as warnings | `crates/ambition_sandbox/src/content/content_validation.rs::validate_patrol_brain_paths` |

## Intro vertical slice (intro-v1)

| Capability | Status | Where to read next |
|---|---:|---|
| Intro spine wake -> raid -> vertical shaft -> drain market -> gate stack -> combat lab -> first boss | Available, math-checked, unplaytested | `dev/vertical-slices/intro-v1/playtest-handoff.md`, `dev/vertical-slices/intro-v1/map-contract.md` |
| Alice/Bob cartography route | Available | `dev/vertical-slices/intro-v1/task-04-under-town-trust-route.md` |
| Cartography route durable state | Available; 10+ route flags and auto-quest hooks in the reviewed archive | `crates/ambition_sandbox/src/intro/route_state.rs`, `crates/ambition_sandbox/src/content/quest.rs::default_quest_specs` |
| Flag-gated LockWalls | Available | `crates/ambition_sandbox/src/intro/route_state.rs::sync_intro_flag_gated_lock_walls` |
| Conditional intro dialogue | Available; Oiler/Alice/Bob route-state variants remain documented in code | `crates/ambition_sandbox/src/intro/route_state.rs::redirect_post_intro_dialog`, `crates/ambition_sandbox/src/intro/dialog.rs` |
| Evil/lawful "Submit private route" Switch | Available | `tools/ambition_ldtk_tools/specs/gate_stack_lower_area.yaml` |

## Rules for updating this file

- Keep it short and status-shaped.
- Do not add line-number links.
- Do not record commit history here.
- If a capability is speculative, put it in `docs/brainstorms/` or `docs/planning/` instead.
- If a capability needs a procedure, write or update a recipe and link that recipe.
