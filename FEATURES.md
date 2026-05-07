# Ambition FEATURES

> Catalog of landed gameplay/system features. Pair with `TODO.md` (what's
> outstanding) and `docs/CURRENT_STATE.md` (architectural snapshot).
>
> Source of truth: working tree + git history. When a feature lands, add it
> here in the same commit as its docs.
>
> Last regenerated: 2026-05-07 (full re-audit against `git log --all`)

## How to read this
Each entry: feature name — what it does — where it lives — landing commits.
Status badges:
- `[stable]` — feature-complete, has tests, used in sandbox
- `[playable]` — works end-to-end but rough edges
- `[scaffolded]` — module + types exist; not yet expressive

---

## Sim → presentation message channels

- **`PlayerDiedMessage` (Bevy 0.18 Message API)** `[stable]` — replaces the previous `SandboxRuntime::player_died_pending` bool. `death_respawn_player` pushes into `FrameFeedback.died`; `flush_feedback` drains into `MessageWriter`; `update_encounters_from_world` reads `MessageReader<PlayerDiedMessage>`. Keeps the runtime resource a pure state store. [lib.rs:67](crates/ambition_sandbox/src/lib.rs#L67), [app.rs:2172](crates/ambition_sandbox/src/app.rs#L2172).
- **`SfxMessage` / `VfxMessage` / `DebrisBurstMessage`** `[stable]` — Vec-collector → MessageWriter pattern documented in `docs/events_refactor_plan.md`. [audio.rs:53](crates/ambition_sandbox/src/audio.rs#L53), [fx.rs](crates/ambition_sandbox/src/fx.rs).

## Engine — physics & player state

- **Wall-jump OOB fix via `body_is_side_contact`** `[stable]` — guards `sweep_player_y` from snap-direction tunneling on edge-touching walls. [movement.rs:1321](crates/ambition_engine/src/movement.rs#L1321). Commit `4002b4d` (Catch edge-touching wall side-contact in y-sweep).
- **`BodyMode` state machine** `[stable]` — Standing / Crouching / Crawling / Sliding / MorphBall drivers with per-mode `BodyShape`. [player_state.rs:145](crates/ambition_engine/src/player_state.rs#L145). Commits `dafa5a4`, `5899c2c`.
- **`ResourceMeter`** `[stable]` — generic stamina/mana/ammo/charge meter; used by projectile fireball mana. [player_state.rs:330](crates/ambition_engine/src/player_state.rs#L330).
- **Ledge grab + swim as opt-in abilities** `[playable]` — `Ability` flags gate behavior; sandbox-side state still being promoted. Commit `70fc567`.
- **Glide / slow-fall ability** `[stable]` — held-jump while airborne caps fall speed at `glide_fall_speed` (220, vs `max_fall_speed` 1040) and boosts air accel to `glide_air_accel` (6200, vs `air_accel` 4700). Cancels on land / fast-fall / blink-hang / water. `Player::gliding` flag drives sandbox sprite/sfx hooks. [movement.rs](crates/ambition_engine/src/movement.rs).
- **`evaluate_character_ai` vocabulary** `[stable]` — engine fn returning `CharacterAiMode`; per-brain knobs (`chase_speed`, `aggro_radius`) on archetypes. [character_ai.rs:98](crates/ambition_engine/src/character_ai.rs#L98), [features.rs:1233](crates/ambition_sandbox/src/features.rs#L1233). Commit `93b4e05`.
- **`Player::damage_multiplier` + `Player::invincible`** `[stable]` — promoted from `SandboxRuntime` to engine-side per-player state. Outgoing slash damage scales by `damage_multiplier`; incoming damage drops if `invincible`. F3 stats editor writes these directly on the player. Survives reset (settings preserved per tester intent). [movement.rs](crates/ambition_engine/src/movement.rs).
- **`Player::mana: ResourceMeter`** `[stable]` — promoted from sandbox `SandboxRuntime::mana_current/max` (i32) to the existing engine `ResourceMeter` type (f32 with regen/decay/try_spend). F3 inspector still surfaces i32; conversion happens at the editor boundary. Reset refills via `refill_full`. [movement.rs](crates/ambition_engine/src/movement.rs), [player_state.rs:330](crates/ambition_engine/src/player_state.rs#L330).

## Combat & projectiles

- **Player damage flow** `[stable]` — HP drain, entity-hazard knockback, max HP 20. Commits `bc85740`, `cc208ee`.
- **Charge fireballs** `[stable]` — hold-to-charge, multi-tier release. Commit `e71afd8`.
- **Hadouken motion-input** `[stable]` — quarter-circle + fire upgrades fireball; grace-input window; super Hadouken via full motion. Commits `ba1ed49`, `e71afd8`, `577aaa8`.
- **Unified damage path + projectile bounce + projectile-actor damage** `[stable]` — single damage funnel for melee/projectile/hazards. Commit `11ae11d`.
- **Fireball split off attack on gamepad** `[stable]` — separate inputs. Commit `643fd63`.

## Enemies, NPCs, AI

- **Patrolling NPCs with gravity** `[playable]` — uses shared `character_ai` vocabulary. Commit `93b4e05`.
- **Hostile NPCs** `[playable]` — NPCs convertible to enemies; death persisted; semantic boss IDs. Commits `75ebfcb`, `70fc567`, `f8a75b6`.
- **Boss phase machine** `[stable]` — `BossPhase` enum (intro / phase1 / phase2 / Stagger / Enraged); stagger threshold + window; per-phase tunables. [boss_encounter.rs:35](crates/ambition_engine/src/boss_encounter.rs#L35). Commit `06ea438`.
- **Boss encounter integration test** `[stable]`. Commit `07df47b`.

## Encounter / wave system (mob_lab)

- **mob_lab room** `[playable]` — LDtk area with EncounterTrigger / Switch defs; hallway threshold → lock-wall slam → camera zoom → music swap → 3 waves with delayed sub-spawns → switch toggles green/red on victory. [sandbox.ldtk](crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk). Commits `701e144`, `c6d390c`, `2c5eafd`, `138e575`, `da30fc9`, `3745ed4`, `3699c6e`.
- **Encounter reward chest** `[stable]` — ground placement, sprite, switch reset on encounter reset. Commit `e21abc3`.
- **Encounter resets on death + waves wait for kills** `[stable]`. Commits `1a49239`, `3745ed4`.

## Movement primitives & rooms

- **Swim mechanic + water_world room** `[playable]` — Mario-style swim, source-agnostic IntGrid `BlockKind::Water`, two-pool lab with overlapping-zone fix. Commits `8b3bec6`, `fe7cef5`.
- **Crouch / morph-ball as `BodyMode`-driven mechanics** `[stable]` — double-tap-down edge routes through `SandboxRuntime` to fire morph ball. Commits `dafa5a4`, `5899c2c`, `da1151a`.
- **One-way platforms + hazard tiles (32×16)** `[stable]` — fixes vertical squish; HazardBlock entities migrated to IntGrid. Commits `b59529a`, `8c8cf30`.
- **Smooth camera ease + victory chest** `[stable]`. Commit `1a49239`.
- **`CameraEaseTuning` resource** `[stable]` — replaces the hardcoded `CAMERA_ZOOM_{IN,OUT}_RATE` consts with an editable Bevy resource (`zoom_out_rate`, `zoom_in_rate`, `snap_epsilon`). Defaults match prior values; sandbox / tests can override without recompiling. [lib.rs](crates/ambition_sandbox/src/lib.rs).

## LDtk authoring (runtime spine)

- **OneWayPlatform + DamageVolume promotion** `[stable]` — typed LDtk runtime-spine components with parity overlay; JSON authority retained until parity tests pass. Commit `d5e0f52`.
- **LDtk biome metadata → runtime resource** `[stable]` — populates on every embedded active area. Commits `3ca2e2f`, `08afafb`.
- **Room music driven by biome metadata seam** `[stable]` — replaces ad-hoc per-room overrides. Commits `fdbbb02`, `458d902`.
- **Programmatic LDtk authoring + room builder + layout lints** `[stable]` — agent-friendlier authoring path. Commits `9db48f7`, `f8a75b6`, `fcda12d`.
- **`list_ldtk_metadata.py` tool + body_mode reset tests** `[stable]`. Commit `7ae5dd6`.
- **LDtk validator: blank `activeArea` rejection** `[stable]`. [ldtk_world.rs](crates/ambition_sandbox/src/ldtk_world.rs).
- **LDtk × audio cross-validation: unknown `music_track` warnings** `[stable]` — `LdtkProject::music_track_warnings` returns one warning per (level, unknown_id); `init_sandbox_resources` runs it at startup against `SandboxDataSpec::audio.music_tracks`. Regression-pinned by `embedded_ldtk_music_tracks_match_audio_catalog`. [ldtk_world.rs](crates/ambition_sandbox/src/ldtk_world.rs).
- **`RoomSet::layout_warnings` branch tests** `[stable]`. [rooms.rs:351](crates/ambition_sandbox/src/rooms.rs#L351).

## Cutscenes & quests

- **Engine + sandbox cutscene spine** `[playable]` — room-triggered cutscenes, lazy boss specs. Commits `06ea438`, `55dc1a5`.
- **Hold-Reset cutscene skip with HUD progress bar** `[stable]`. Commit `322d9f2`.
- **Per-cutscene "always skip if seen" flag** `[stable]` — `CutsceneScript::seen_flag: Option<String>` is set in save data when a cutscene completes (or is skipped via hold-Reset); future triggers consult `SandboxSaveData::flag` and skip silently. Cutscenes without a `seen_flag` always replay (intentional for test rooms / dev cycles). [cutscene.rs:62](crates/ambition_engine/src/cutscene.rs#L62), [cutscene.rs:180](crates/ambition_sandbox/src/cutscene.rs#L180).
- **Quest system + quest hooks** `[playable]` — engine + sandbox modules. Commits `06ea438`, `55dc1a5`.

## UI / menus

- **Map UI panel + minimap** `[playable]` — full-screen map + toggleable corner minimap; visited tracking. [map_menu.rs](crates/ambition_sandbox/src/map_menu.rs). Commit `75ebfcb`.
- **Reset Sandbox pause-menu item + reset processor** `[stable]`. Commit `45631d4`.
- **HUD wiring** `[playable]`. Commit `55dc1a5`.
- **Dedicated quest panel** `[playable]` — `QuestPanelText` UI surface anchored top-right, written by `update_quest_panel`. Decouples the quest log from the debug HUD's stats dump; collapses (empty string) when no active quests. [app.rs](crates/ambition_sandbox/src/app.rs).
- **F3 stats editor** `[stable]`. Commit `70fc567`.

## Settings (full system)

- **Settings architecture + menu input + persistence** `[playable]` — `settings/` module. Commits `00b2536`, `239b138`.
  - Controls subsection: deadzone (left + right stick), trigger thresholds, hysteresis, dash repeat. [settings/controls.rs](crates/ambition_sandbox/src/settings/controls.rs).
  - Gameplay subsection: difficulty (Easy/Medium/Hard with damage multipliers). [settings/gameplay.rs](crates/ambition_sandbox/src/settings/gameplay.rs).
  - Audio: volume + difficulty/assist multipliers wired from `UserSettings`. Commit `3be81f3`.

## Save / persistence

- **Sandbox save game + encounter state vocabulary** `[playable]`. Commit `239b138`.
- **Hostile-NPC death persisted across save** `[playable]`. Commit `75ebfcb`.

## Audio / music

- **Music backend split: production renderer (pretty-midi) + fallback** `[stable]` — large refactor pass. Commits `619d08f`, `1d7bd3f`, `7843010`, `d7459c6`.
- **Renderer overhaul** `[stable]` — compressor, Schroeder reverb, constraints, humanize, bends. Commit `b8d1a18`.
- **Adaptive music cues** `[playable]` — fast_paced_violin_boss, dinosaur_liberators (Freebird-length 9:08), violin_boss_relentless, crooked_ascent_boss (5/4 klezmer/Bartok), 3 faction-area themes. Commits `dddf505`, `fdfd4a5`, `a03c1d7`, `d4b25ca`, `207294e`, `ec5a3d1`.
- **Audio validation tests + `music_track_report` binary** `[stable]`. Commit `fb175f5`.
- **LFO automation curves + per-cue dynamics tuning** `[stable]`. Commits `a03c1d7`, `533c47d`.

## Input

- **`SandboxAction::Interact`** `[playable]` — bound to gamepad RightTrigger; covers every action on keyboard + gamepad. [input.rs:41](crates/ambition_sandbox/src/input.rs#L41). Commit `e503c0d`.
- **`SandboxAction::MenuSelect`** `[stable]` — Enter / NumpadEnter / Space / GamepadButton::South / configured Jump key all confirm in menus. [input.rs:342](crates/ambition_sandbox/src/input.rs#L342). Pause menu reads `actions.just_pressed(&SandboxAction::MenuSelect)`.
- **`SandboxAction::MenuNavigate{Up,Down,Left,Right}`** `[stable]` — D-pad + arrow keys + analog stick navigate menus. Toggleable via `controls.dpad_menu_navigation`. [input.rs:331](crates/ambition_sandbox/src/input.rs#L331).
- **Dash trigger hysteresis** `[stable]` — analog right trigger goes through `update_trigger_edge` with configurable release / press thresholds (defaults 0.30 / 0.55). Three input modes: Trigger (analog only) / Button (RB/R1) / Both. Prevents jitter spam on aged controllers. [input.rs:488](crates/ambition_sandbox/src/input.rs#L488), [settings/controls.rs:202](crates/ambition_sandbox/src/settings/controls.rs#L202).

## Tooling / generators

- **Sprite passes** `[stable]` — tight-crop entity art, tile sprites, IntGrid-blocked tiles, 11 entity sprites via `gen2d`, procedural morph-ball sprite. Commits `e634d39`, `bd95097`, `efdb8fb`.
- **Programmatic LDtk authoring tool + smoke test** `[stable]`. Commit `9db48f7`.
- **Sprite generator + component sheet tooling** `[stable]`. Commits `4016e2e`, `95f7fd9`, `553186a`, `e3ca24e`.
- **AI-driven enemy update tool** `[stable]`. Commit `f8a75b6`.

## Continuous integration

- **GitHub Actions test workflow** `[stable]` — `.github/workflows/test.yml` runs three jobs on push / PR: (1) `cargo test -p ambition_engine` (engine library + proptest fuzzers), (2) `cargo test -p ambition_sandbox --lib` + `repro_walls` integration test + a `cargo run --bin headless -- 60` smoke pass, (3) `cargo fmt --check` + `cargo clippy` (informational, non-blocking).

## Tests / observability

- **Wall-jump repro test** `[stable]` — [tests/repro_walls.rs](crates/ambition_sandbox/tests/repro_walls.rs).
- **`RoomSet::layout_warnings` branch tests** `[stable]`.
- **Boss encounter integration test** `[stable]`.
- **Body-mode reset tests** `[stable]`.
- **`BodyShape::fits_at` proptest** `[stable]` — three proptest properties pin the geometric contract: empty world fits any finite shape at any finite center; one-block world fits iff the shape AABB and block AABB don't strictly intersect; MorphBall fits wherever Standing fits (strict shape shrink). [body_shape_fits_at.rs](crates/ambition_engine/tests/body_shape_fits_at.rs).
- **Wall-jump start-position fuzz** `[stable]` — proptest randomizes (x_offset, y_in_wall, vel_y) along the square_arena left wall and asserts no >100 px y-snap and no out-of-world / through-wall teleport on a single wall-jump frame. Regression guard around the `body_is_side_contact` predicate that closes the historical OOB-teleport bug class. [wall_jump_fuzz.rs](crates/ambition_engine/tests/wall_jump_fuzz.rs).
- **Audio validation tests** `[stable]`.

---

## Notes
- This catalog supersedes piecemeal "what's done" claims scattered in handoff docs. When closing a `TODO.md` item, add the corresponding entry here in the same commit.
- For a continuous narrative of what changed when, `git log --oneline` is authoritative — this file is the "current capabilities" view.
- "Recently completed" entries in `TODO.md` should be migrated here once they've stabilized (~2 weeks).
