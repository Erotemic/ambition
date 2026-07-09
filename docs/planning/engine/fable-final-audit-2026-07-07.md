# FABLE FINAL AUDIT — 2026-07-07 (the last fable pass)

Whole-repo audit after the opus/codex decomposition landing (E1a–e, E2
projectiles, W3 world/LDtk split, W-queue step 3 lowering proof, asset-manager
carve, sprite-sheet absorb, encounter mint, boss tail, `gameplay_core` →
`ambition_actors`, `game/` re-home). **Findings are appended IN PRIORITY ORDER
as they land — treat every entry as a plan item even if the session cut off
before it was folded into the ledgers/cards.** Anything here that contradicts
an older card wins (it is the newer ruling).

Audit order (most valuable first):
1. Dep-graph / tier audit — does the crate DAG match architecture.md's arrows?
2. `ambition_actors` (68k) — the residual monolith's next decomposition line.
3. Facade/shim census — the E7/E8 dissolution checklist, made explicit.
4. Ruling-compliance spot checks (W3 zero-LDtk, [W-e] hard error, GeoId
   adoption, SweepSample adopters, Tier-0 purity).
5. Subtle-correctness greps (query order, time domains, pushout, Entity
   identity, seam races).
6. Full test gate.
7. Elegance directions newly visible in the post-carve structure.

## Findings

(appended below, newest last)
### F1 — Dep-graph audit: the DAG is sound; ELEVEN arrows need work (none design-hard)

The workspace DAG has no cycles (`actors → sim_view` is dev-dep only) and the
big shape is RIGHT: engine_core/entity_catalog at the bottom are dep-free,
characters/combat/primitives sit above engine_core, `game/` sits on top.
The arrows below are the full remaining cleanup list, each with its
prescription — log-once so E7/E8 executors don't re-derive:

1. ✅ **DONE (Codex 2026-07-08): `ambition_world` no longer depends on
   combat, interaction, or portal runtime crates.** The remaining legacy typed
   RoomSpec families were converted to world-owned plain authored payloads:
   hazards carry `HazardVolumeSpec`, interaction/pickup/chest/breakable rows
   carry `*Spec` data, and static portal authoring carries
   `PortalChannelColorSpec` + plain geometry/link fields. Runtime/presentation
   crates now lower those records at their own edge, while `ambition_world`'s
   manifest allow-list contains only engine/catalog/time foundations. The app
   architecture boundary test now forbids reintroducing `ambition_combat`,
   `ambition_interaction`, or `ambition_portal` in the world IR.
2. ✅ **DONE (Codex 2026-07-07): `ambition_actors::portal` facade deleted.**
   Consumers now import portal mechanics from `ambition_portal`, presentation
   resources/schedule labels from `ambition_portal_presentation`, and Ambition
   host adapter systems from `ambition_host::portal`; `ambition_actors` no
   longer depends on `ambition_portal_presentation`, and the boundary test
   ratchets against reintroducing the facade.
3. ✅ **DONE (Codex 2026-07-07): `ambition_vfx` no longer depends on
   `ambition_characters` for `ActorFaction`.** The effect vocabulary now owns
   `HitSide`, and emitters store that presentation-neutral side on
   `Hitbox`/`DamageBoxEffect`/`SummonSpec`. Combat maps `ActorFaction` ↔
   `HitSide` at spawn/resolution edges, summon execution maps back when it
   actually creates an actor, and the architecture boundary test now forbids
   both `ambition_actors` and `ambition_characters` in `ambition_vfx`.
4. ✅ **DONE (Codex 2026-07-07): `GameMode` moved down into
   `ambition_platformer_primitives::schedule`.** The coarse session-state enum
   now lives next to `PlatformerRuntimeSet`; `gameplay_allowed` /
   `gameplay_suspended` run conditions moved with it. Runtime, sim-view,
   content, and touch-input callers name the lower crate directly. **F2.1 follow-up
   burned down the actor-side `session::game_mode` compatibility facade as well**,
   so actor-internal code now names the primitive schedule vocabulary directly.
   This removes the session-state vocabulary reason for host/touch/render-side
   code to name `ambition_actors`; remaining
   arrows are concrete machinery/presentation seams and can be burned down
   independently.
5. ✅ **DONE (Codex 2026-07-08): `ambition_render` no longer depends on
   `ambition_actors`.** F1.5's first cut enumerated the true residue; the finish
   pass burned it down by moving render-facing asset vocabulary (`GameAssets`,
   entity sprites, parallax ids, and boss sprite animation/types) into
   `ambition_sprite_sheet`, moving camera/physics/shrine/dev/read-only session
   resources into lower vocabulary crates, and making controlled-body sprite
   rebinding use a render-owned `PlayerSpriteCharacter` marker supplied by the
   app seam. Render now reads lower model/view crates (`ambition_sim_view`,
   `ambition_world`, `ambition_sprite_sheet`, `ambition_platformer_primitives`,
   `ambition_dev_tools`) and `Cargo.toml` has no actor dependency. The render
   boundary test is upgraded from an allowed-residue count to a zero-dependency
   assertion.
6. ✅ **DONE (Codex 2026-07-08): `inventory_ui` split out of
   `ambition_items`.** The reusable item model now owns only catalog + shop
   primitives and no longer depends on `ambition_ui_nav`; the menu-navigation
   resource (`InventoryUiState`, `InventoryTab`, `MenuFocusState` ownership) now
   lives in `ambition_inventory_ui`. App/menu callers import the UI-state crate
   directly, while `ambition_items` remains the lower item-model crate.
7. ✅ **DONE (Codex 2026-07-08): `ControlFrame` moved down into
   `ambition_engine_core`.** The brain-facing control vocabulary now lives with
   the body/input-state contract, beside `InputState` and reference-frame
   helpers. `ambition_characters` consumes `ambition_engine_core::ControlFrame`
   and no longer depends on `ambition_input`; `ambition_input` is now the device
   adapter that builds engine-owned frames from Leafwing/settings and keeps
   `ambition_input::ControlFrame` only as a compatibility re-export.
8. ✅ **DONE (Codex 2026-07-08): `ambition_asset_manager` no longer
   depends on `ambition_sfx`.** The unused SFX-bank provider adapter was
   deleted instead of feature-gated: the asset manager resolves the logical
   `audio.sfx_bank` id to an `AssetLocation`, and the audio/app layer constructs
   `BankProvider` from that location. The architecture boundary test now
   forbids `ambition_sfx`/`ambition_audio` deps, the old `sfx` feature, and the
   removed `sfx_integration.rs` module.
9. ✅ **DONE (Codex 2026-07-08): `ambition_runtime`'s actor/combat/
   projectile edges are intentional.** F1.9 is a no-move ruling, now ratcheted
   by an architecture boundary test: `ambition_runtime` is the headless sim
   composition tier and may directly name sim/mechanic/model crates such as
   `ambition_actors`, `ambition_combat`, `ambition_projectiles`, and the
   foundational `ambition_dev_tools` state seam used by headless sim/dev wiring,
   but it must not drift upward into app/content/host/render/touch/menu/backend
   ownership.
10. ✅ **DONE (Codex 2026-07-08): `ambition_host` no longer depends on
    `ambition_actors`.** The host now schedules actor-owned input bridges
    through the `ambition_runtime::host_input` facade, reads camera-shake and
    controlled-subject vocabulary from lower runtime primitives, and uses
    runtime demo-fixture seams in its smoke test. Its remaining direct deps are
    host/presentation/runtime seams (`input`, `render`, `runtime`, `sim_view`,
    plus optional portal presentation), not the actor-systems crate.
11. ✅ **DONE (Codex 2026-07-08): `ambition_touch_input` → render is an
    accepted presentation-adapter edge.** The crate owns the on-screen touch HUD
    overlay and folds those virtual controls into input frames, so its direct
    render dependency is not a dep-flip blocker. The remaining issue is naming /
    placement only: keep the current crate name for now, document it as a
    presentation/input adapter, and leave a future optional rename/re-home under
    a `presentation/` grouping as LOW priority. The architecture boundary test
    now ratchets this ruling by allowing `ambition_render` specifically while
    forbidding app/content/host/backend drift.

### F2 — `ambition_actors` (68k): what it still smuggles + the residual's true shape

Module census (top): features 24.7k (ecs 19.8k — the REAL actor domain:
actors/mount/damage/bosses/spawn/perception/attack/damage_apply), boss_encounter
6.9k, player 6.6k, abilities 4.1k, character_sprites 2.8k, projectile 2.3k +
enemy_projectile 0.7k, world 2.0k, dev 1.6k, assets 1.6k, encounter 1.6k,
items 1.5k, time 1.4k, persistence 1.3k, session 1.3k, audio 1.1k, menu 1.0k,
body_mode 0.8k, portal 0.76k, schedule 0.6k, dialog 0.5k, music 0.4k.

**The actor DOMAIN itself (features+player+abilities+boss_encounter+body_mode
≈ 43k) is legitimately here.** The rest divides into three disposition
classes — log-once so the next sessions don't re-derive:

1. **MISPLACED (move whole, mechanical):**
   - `assets/` (GameAssets + sandbox_assets + loading) — an asset catalog in
     the actor crate; it was also render's biggest reason to dep actors (F1.5).
     Destination: `ambition_asset_manager` (catalog machinery) +
     `ambition_sprite_sheet` (character-sprite-specific lookups). **F2
     misplaced-pass slice (Codex 2026-07-08) repointed external consumers of
     the pure asset vocabulary (`GameAssetConfig`, `GameAssets`,
     `EntitySprite`, `SandboxAssetCatalog`, and catalog `ids`) to those lower
     crates directly. The remaining actor-side asset surface is now the
     adapter that joins content registries, embedded world rows, and the
     character/boss sprite loaders (`load_game_assets`,
     `build_sandbox_catalog_with`, `AmbitionAssetSourcePlugin`).**
   - `character_sprites/` remainder (2.8k) — sheets/anim/animator modules are
     ~50% facade re-exports of `ambition_sprite_sheet` already (8 facade
     files); finish the absorb, delete the tree. **F2 misplaced-pass slice
     (Codex 2026-07-08) moved the `SheetRegistryPlugin` home into
     `ambition_sprite_sheet` and deleted the pure facade modules
     (`animator`, `baked_sheet_rons`, `registry`, `sheets`, `sprite_packs`);
     the remaining actor-side `character_sprites` code is now the real
     actor/content join: animation fact adapters, authored hitbox resolver,
     and character-catalog-aware sprite loading/body collision. **A follow-up
     slice (Codex 2026-07-08) repointed SimView's pure `CharacterAnim` read-model
     fields to `ambition_sprite_sheet::character::CharacterAnim`; only the
     actor-state animation pickers still route through the actor-side adapter.**
   - `world/physics.rs` (avian adapter) + `world/overlay{,_rebuild}.rs` —
     these stayed actors-side because the overlay REBUILD reads live feature
     components. Correct interim home, but name the end-state: after W-queue
     step-3 dissolution the rebuild's inputs become plain solids and the pair
     joins `ambition_world`; physics.rs (debris/avian) is presentation-adjacent
     and can join render/host side whenever.
   - `projectile/` + `enemy_projectile/` ECS half (3k) — joins
     `ambition_projectiles` in the carded dedicated session. **F2 residual-glue
     slice (Codex 2026-07-08) centralized the remaining actor-side projectile
     steppers behind `ambition_runtime::projectile_schedule`; app/content
     production code no longer schedules through `ambition_actors::projectile`
     directly. A projectile-dedicated follow-up (Codex 2026-07-09) moved the
     enemy/boss projectile effect-request spawn executor into
     `ambition_projectiles::enemy`; runtime still schedules it through
     `ambition_runtime::projectile_schedule`, but no longer reaches through
     `ambition_actors::enemy_projectile` for that substrate-only spawn step.
     The next follow-up moved projectile-kind-specific expiry VFX
     (`ProjectileVisualKind::expiry_vfx`, currently the lasersword detonation)
     into `ambition_projectiles::visual_kind`. A test-travel follow-up moved the
     pure projectile primitive tests (motion gestures, spawner gates, kind
     tuning, and projectile-body collision) out of the actor facade and into
     `ambition_projectiles`; actor-side projectile tests now cover only the
     woven sim steppers. The actual victim-routing, charge-input, and
     world-collision steppers still stay actor-side until the boss/player/world
     inputs are split.**
2. **RESIDUAL GLUE for already-minted crates** (audio/menu/dialog/items/
   encounter/persistence/music/dev modules, ~7k total): each is the actor-side
   wiring for a carved crate. Per ADR 0019, the plugin/schedule wiring belongs
   in `ambition_runtime`; actor-DOMAIN reactions stay. Treat each module as a
   two-way split, one commit each — do NOT move them wholesale into runtime
   (that would just relocate the god-hub). **F2 residual-glue slice (Codex
   2026-07-08) applied that rule to projectiles first: runtime owns the schedule
   facade for the remaining actor-side projectile steppers; the model stays in
   `ambition_projectiles`, and the actor-domain victim/charge logic has not been
   moved wholesale.** **A projectile-dedicated follow-up (Codex 2026-07-09)
   identified the enemy-pool `Effect::Projectiles` drain as substrate-only and
   moved its canonical implementation into `ambition_projectiles::enemy`, leaving
   runtime as the schedule facade and actor-side projectile code focused on the
   still-woven charge/victim/world stepper. A follow-up moved the kind-specific
   projectile expiry VFX cue (`ProjectileVisualKind::expiry_vfx`, currently the
   lasersword detonation) next to the visual-kind art policy in
   `ambition_projectiles`, so the actor stepper decides when a projectile expires
   but no longer owns projectile-kind-specific presentation policy. A test-travel
   follow-up moved the pure projectile primitive tests into `ambition_projectiles`,
   leaving `ambition_actors::projectile` with only actor-woven stepper coverage.**
   **A follow-on residual-glue slice (Codex 2026-07-08)
   also burned down the developer-tools facade: app/runtime/sim-view now import
   `ambition_dev_tools::{dev_tools,profiling,sync_live_player_dev_edits_system}`
   directly, while `ambition_actors::dev` keeps only the sim-coupled trace
   recorder.** **A later audio-facade slice (Codex 2026-07-08) repointed app
   radio/menu/android consumers of pure playback vocabulary (`AudioLibrary`,
   `MusicPlaybackState`, `RadioStationState`, `MusicChannel`, `SfxChannel`,
   and `set_radio_track`) to `ambition_audio::library`; the actor-side
   `audio` residue is now the sandbox `SandboxAudioPlugin` and environment
   detector that still joins actor contacts, settings, schedule, and music
   intent.** **A menu-backend slice (Codex 2026-07-08) moved
   `InventoryUiBackend` and backend-availability constants to
   `ambition_menu::backend`; follow-on closeout slices moved the
   renderer-agnostic map/minimap state (`MapMenuState`, `MapRoomNode`, and zoom
   constants) to `ambition_menu::map`, repointed the runtime resource init to
   that canonical home, and deleted the actor-side map model facade. `ambition_actors::menu` now owns only the
   room/save hydration, hotkeys, and Bevy-UI map adapter systems, not
   presentation-backend or reusable map-state vocabulary.**
   **A settings/menu-IR facade slice (Codex 2026-07-08) repointed app menu
   hosts/tests to import stored settings from `ambition_persistence::settings`
   and renderer-agnostic menu IR from `ambition_settings_menu` directly; the
   final closeout repointed actor-local settings compatibility helpers to
   `ambition_settings_menu` directly and deleted the actor-side `menu::ir`
   facade. The actor-side `persistence::settings` module now keeps only the
   pause-menu compatibility controller/model that still reads actor/dev/window
   state.** **An encounter-vocabulary slice (Codex
   2026-07-08) repointed app/content/runtime/sim-view consumers of pure
   encounter state, music-request, registry, phase, and reward helper vocabulary
   to `ambition_encounter`; the actor-side encounter module now remains the
   LDtk/ECS/schedule adapter surface (`load_encounter_specs_from_ldtk`,
   `populate_encounter_registry`, switch queues, and lock-wall contribution);
   the content-installed encounter wave book now lives in `ambition_encounter`
   with the rest of the pure encounter vocabulary.** **A dialog/developer-persistence tail slice (Codex
   2026-07-08) repointed app/content/runtime users of reusable dialog
   vocabulary (`DialogState`, reveal/input systems, and Yarn binding/mirror
   types) to `ambition_dialog`, while keeping Ambition's game-specific Yarn
   bindings and `GameMode` sync plugins actor-side. The same slice moved the
   `DeveloperPersistenceSchedulePlugin` home to `ambition_dev_tools`; the F2
   closeout removed the actor-side dev-persistence alias after consumers named
   `ambition_dev_tools` directly. Actor persistence now keeps only the real
   save/runtime persistence surface plus actor-local settings compatibility.**
3. **FACADES (60 `pub use ambition_*` re-export sites in actors).** These are
   the deliberate hub-continuity aliases. The dissolution ratchet: **a facade
   may be deleted the moment `grep -rn "ambition_actors::<mod>"` outside
   actors returns zero** — put that one-liner in the E7/E8 card as the
   per-facade exit test, and burn them down opportunistically (each is a
   5-minute repoint+delete). **F2 first burn-down (Codex 2026-07-08) removed
   the F1-era compatibility facades for `GameMode`, camera layers/ease/shake,
   `SandboxDevState`, `ControlledSubject`, and external `FeatureEcsWorldOverlay`
   reads; consumers now name `ambition_platformer_primitives` /
   `ambition_dev_tools` directly. A follow-on schedule-label pass repointed
   runtime/content/app/sim-view consumers of `SandboxSet`, `CombatSet`,
   `BossSteerSlot`, `PresentationSetupSet`, and `SimulationSetupSet` to
   `ambition_platformer_primitives::schedule`; `ambition_actors::schedule` now
   remains only for the concrete actor-owned schedule installer and input bridge
   systems.** **F2 closeout (Codex 2026-07-08) removed the remaining safe
   pure-vocabulary facades found in this pass: map/minimap state is owned by
   `ambition_menu::map` with no actor-side model facade, actor menu no longer
   has a settings/System IR facade, actor dialog no longer re-exports reusable
   `ambition_dialog` state/input/Yarn-binding vocabulary, and actor persistence
   no longer aliases dev-tools persistence. The remaining external
   `ambition_actors` references found by the closeout sweep are documented
   adapter seams: asset catalog/loader assembly, LDtk encounter loading and ECS
   registry update, Ambition-specific Yarn binding refresh/plugins, item pickup
   and held-projectile simulation components, concrete schedule/resource
   installers, and map UI/hydration systems. Treat F2 as closed for audit
   cleanup; deeper decomposition of those actor-domain systems belongs to the
   later world/plain-input, projectile, and unified-actor cards.**

**North star for the residual (fold into unified-actors.md):** `player/`
(6.6k) existing as a SIBLING of `features/ecs` is the last structural
player-centrism — the fighter-unification S5/S6 endgame folds the player's
remaining special-cased systems into the one actor pipeline (the single
control seam already made the player "an actor wearing Brain::Player"). The
right long-term shape is ONE `actors` module tree where player-ness is a
brain + a slot, not a directory. Do not force this before S5/S6; DO stop
adding new player-only systems (new work lands body-generic or brain-side).

### F3 — Ruling-compliance spot checks: four green, THREE corrections

Verified green:
- **[W-e]/[W-b] lowering registry** — `ambition_world::placements` has the
  duplicate-registration panic AND the unknown-kind hard error, both pinned by
  `#[should_panic]` tests. Exactly as ruled.
- **§3.6 GeoId stamping survived the W3 move** — `ambition_ldtk_map::intgrid`
  still stamps level-scoped `TileLayer` ids with the merge ordinal.
- **Tier-0 purity** — `ambition_entity_catalog` still deps NOTHING. ✓
- **W4/ADR 0021 first cut recorded**; `ambition_world` has no LDtk dep. ✓

Corrections (log-once, all small):
1. ✅ **DONE (Codex 2026-07-07): `ron_room` moved to the world side.** The serialized-IR
   loader (`RonRoomDoc`, `room_doc_from_ron`, `load_manifest_ron_rooms`) and
   the `WorldManifest.ron_rooms` rows lived in `ambition_ldtk_map` — the LDtk
   BACKEND crate. Its entire purpose is "a room enters the graph with no LDtk
   anywhere in the path"; the parser/serializer/source row and pure generated-
   room fixture now live in `ambition_world::ron_room`, while LDtk composition
   calls `load_ron_rooms` as a backend → IR consumer.
2. ✅ **DONE (Codex 2026-07-09): `SweepSample` adoption / portal anchor cleanup.**
   Runtime actor/boss ECS queries now require the shared `SweepSample` component
   spawned by `AncillaryMovementBundle`, so `integrate_boss_bodies` cannot
   silently run without the canonical §3.1 motion record. Portal transit now
   consumes that same kernel-written sample for CCD and retired the portal-local
   `PortalSweepAnchor`; the adapter only uses a sample whose `curr` still
   matches the live body position, so a post-sim teleport cannot become fake
   swept travel.
3. ✅ **DONE (Codex 2026-07-07): `ambition_world` now has a dep-direction
   regression test.** It asserts the explicit allow-list and deliberately
   leaves combat/interaction/portal as named legacy family residue to delete
   one at a time as W step-3 branch conversions land.

### F4 — Correctness findings: two REAL regressions FIXED in-session + three logged hazards

**Fixed in this audit (commits on main):**
1. **The `game/` re-home broke desktop asset-root resolution** —
   `desktop_asset_root()` + `capture_scene` hopped `../ambition_actors/assets`
   from `game/ambition_app` (lands in `game/`, not `crates/`); the silent
   fallback to exe-relative `assets` reproduces "game runs but nothing
   renders / no music". Fixed to `../../crates/…`; caught by the
   (well-written) cli test. Every other `CARGO_MANIFEST_DIR` hop audited —
   correct.
2. **The `gameplay_core → ambition_actors` rename broke the music tools** —
   `_paths.py` repo-root probe + cli/bundle/audit registry paths pointed at
   the dead crate dir (submodule commit + bump). The regen shell scripts were
   already updated.

**Logged hazards (small, opus-executable):**
3. ✅ **DONE (Codex 2026-07-09): clock-reset seam.** Respawn, room
   transition, sandbox reset, and replay reset code now emit
   `ClockResetRequest` instead of writing `ClockState.time_scale = 1.0`
   directly. `time/time_control` owns the handler and snaps both the live
   sim clock and `RequestedClockScale` back to neutral, preserving the old
   reset behavior without bypassing the ADR 0010/0011 authority seam.
4. ✅ **DONE (Codex 2026-07-09): deterministic `PlayerSlot` fallback.**
   The save-load hostile-grudge fallback and actor slot-board anchor fallback
   no longer use raw Bevy query order. Both sites now choose the lowest
   available `PlayerSlot` when a primary entity cannot be resolved, and both
   carry `AMBITION_REVIEW(determinism)` comments so future multiplayer/RL work
   sees the intentional ordering seam. A compile-fallout top-up kept the
   `PlayerSlot` read optional at the query seam so legacy/fixture primary-player
   entities still anchor hostile brains while the non-primary fallback remains
   slot-ordered.
5. **The full app gate** treats `unified_melee::a_hostile_actor` as a
   convergence/read-model test, not a second enemy-AI regression oracle. The
   green `enemy_attacks_player` test still pins target acquisition and brain
   commitment. `unified_melee` now re-resolves the spawned hostile across the
   observation window, serializes the two in-file sandbox sims, and accepts the
   modern actor swing authority (`MovePlayback` for moveset-backed melee) as
   the lifecycle source while still accepting the `BodyMelee` projection when
   present. This matches the post-E9/fable structure: player flat read-models
   and actor movesets share combat ownership without requiring the test to
   depend on transient projection timing.

### F5 — Elegance directions the new structure makes visible (NOT yet in any card)

1. ✅ **DONE (Codex 2026-07-09): `ambition` umbrella crate + app-manifest
   collapse.** The downstream engine surface exists at `crates/ambition`: it
   re-exports the runtime, host, render, world/LDtk, actor/model, and lower
   vocabulary crates; exposes `engine::{add_headless_foundation,
   init_engine_states, PlatformerEnginePlugins}`; and has a curated prelude for
   new game/content crates. The facade deliberately does **not** depend on
   `ambition_app`, `ambition_content`, or the kaleidoscope app/backend crate.
   `game/ambition_app` now exercises that same surface: its manifest keeps only
   `ambition`, app-local `ambition_content`, and the kaleidoscope renderer
   extension as direct `ambition*` deps, while reusable engine/model/render
   imports route through `ambition::{actors, runtime, render, ...}`.
2. ✅ **FIRST CUT (Codex 2026-07-09): Sanic/SMB1 demo homes created as oracle
   crates.** `game/ambition_demo_sanic/` and `game/ambition_demo_smb1/` are
   registered workspace members whose manifests depend only on `ambition`.
   Their first content plugins are intentionally empty: the value is the
   boundary ratchet that a second platformer starts from the umbrella surface,
   not by copying `game/ambition_app`'s dependency wall. Later demo work fills
   actual movement/content data without changing the engine crates.
3. **At the S5/S6 player-fold, rename `features/` away.** The module name is
   pre-decomposition residue ("content features" that are now just the actor
   ECS). When player/ folds in (F2 north star), the tree becomes
   `ambition_actors::{bodies, brains, spawn, damage, mount, perception,
   bosses}` — names that say what they are. Do not rename before the fold
   (one churn, not two).
4. **Tests should travel with their subject:** `features/conversion_tests.rs`
   (849 lines) tests LDtk conversion that now lives in `ambition_ldtk_map`;
   actors' `world/rooms/tests.rs` (602) tests the room graph that lives in
   `ambition_world`. Moving them tightens both crates' change-detection
   (a conversion regression should fail IN the backend crate).
5. **Anti-goal (Jon's tiny-crate skepticism, restated for the tail):** the
   remaining wins are MOVES and DELETIONS, not new crates. Beyond E9 + the
   demo crates + possibly `ambition_session_state` (F1.4), no new crate
   should be minted without a consumer that exists today. The crate count
   (38) is already at the top of the comfortable range; the value now is
   thinning `ambition_actors` and deleting facades, not adding boxes.

### F6 — Final gate + close

Full `cargo test -p ambition_app --features rl_sim` after the F4/E9 fixes had
left only `unified_melee::a_hostile_actor`; follow-up analysis found that the
behavioral chain was already pinned by `enemy_attacks_player`, while
`unified_melee` was still assuming the old flat `BodyMelee` read-model was the
only valid hostile swing observation. The test now re-resolves the body for the
authored id across the observation window, serializes the in-file sandbox sims,
and treats attack `MovePlayback` as the authoritative moveset-backed hostile
lifecycle while retaining `BodyMelee` projection/owned-hitbox checks when they
are visible.

Ops note: the dev box hit 100% disk mid-audit; `~/ambition-target/debug/
incremental` (149G of regenerable build cache) was deleted to recover.
Consider `CARGO_INCREMENTAL=0` for CI-style full-gate runs, or a periodic
`cargo clean` cron, so a full disk doesn't silently kill background gates.

**Priority order for the next sessions (all opus-executable, most valuable
first):** continue the projectiles dedicated session by splitting the remaining
actor-side charge/victim/world inputs only when their dependencies are explicit.
The first projectile follow-ups are now landed: enemy-pool
`Effect::Projectiles` spawn requests live in `ambition_projectiles::enemy`,
projectile-kind-specific expiry VFX lives in `ProjectileVisualKind::expiry_vfx`,
and pure projectile primitive tests live with `ambition_projectiles` rather than
under the actor facade. F2 is closed for audit cleanup; F3.2, F4.3, F4.4, and E9
are closed correctness/ruling/facade seams; deeper actor decomposition is tracked
by the later world/plain-input, projectile, and unified-actor cards.

### F7 — Deep pass: the lowering seam had three real defects (FIXED); test-loss lesson

1. **Lowered hazards lost their authored display names** — the record carried
   `{id, schema, aabb}` only, so the interpreter labeled hazards by LDtk iid
   (live cosmetic regression in basement_hazards et al). **RULING:
   `PlacementRecord` gains a record-level `name: String`** (serde-default =
   the id; the `PropSpec.name` precedent). Every future placement family gets
   display names for free — do NOT put names inside schemas.
2. **The inline-motion trap was armed:** the converter dual-emitted a record
   for EVERY DamageVolume, the spawn guard skips lowered ids, and the hazard
   interpreter hardcodes `motion: None` — so a legacy inline-`motion` hazard
   would silently become STATIC. No live map trips it (audited all four
   .ldtk files), but the fix is in: inline-motion hazards stay legacy-only
   (no record) until dissolution lifts the path to a room-level
   `KinematicPath`, and a test pins it.
3. **The W3 carve dropped a cfg(test) fixture and, with it, FOUR ruled
   contract tests** (the [W-b] dual-emission pin, the §3.6 tile-GeoId
   determinism pin, the W2 sanic ron-room IR proof, the converter-registry
   coverage). All restored in `ambition_ldtk_map`. **LESSON for every future
   carve — add to the D2 template: `git log --stat` the source module's test
   files and account for every `#[test]` by name in the moved crate; a carve
   that can't run a test must MOVE its fixture, not delete the test.**

Clean-verified in the same pass (worth recording as sound): hash-order
iteration in sim is order-insensitive at both live sites; zero
`partial_cmp().unwrap()` NaN-sorts; sim-path `unwrap()`s concentrate in
tests; the engine-for-other-games oracle HOLDS (zero live core→content
references — the one `include_str!` is the sanctioned cfg(test) fixture
pattern); live-doc drift is nil (dead crate names appear only in
planning/history docs).

### F8 — Deep-pass certification

After the F7 fixes: `ambition_ldtk_map` 25 green (contract tests restored)
and `ambition_actors` lib 789 green. The later F4/E9 gate fallout has been
tracked in the execution log: the former `unified_melee` RED was a stale
read-model assumption in the test, not the hostile brain/melee chain. The
boundary test now explicitly allows the fixture's DATA path while still
forbidding the Cargo dep (the distinction is written into the test). The D2
template carries the BINDING test-accounting rule so no future carve can
silently drop pinned contracts again.

This closes the fable audit. The repo is structurally sound, behaviorally
green, and every remaining item is enumerated with a prescription in F1–F7 —
the priority queue at F6 stands, with F7's lowering fixes already landed.


## Follow-up checklist after the fable audit

Completed in the projectile tail after F8:

- [x] Move the substrate-only enemy/boss `Effect::Projectiles` spawn executor
  from `ambition_actors::enemy_projectile` to `ambition_projectiles::enemy`.
- [x] Move projectile-kind-specific expiry VFX policy into
  `ProjectileVisualKind::expiry_vfx`.
- [x] Move pure projectile primitive tests to `ambition_projectiles`; leave the
  actor crate with only actor-woven projectile stepper tests.

Still open, in priority order:

- [ ] Split another projectile seam only when the dependency boundary is
  explicit. Current blockers are: charge input still reads brain action
  messages, `UserSettings`, gravity, and optional player animation facts;
  victim routing still emits `HitEvent`/player heal/SFX/VFX and queries bosses,
  actors, breakables, shields, and owner combat; world collision still needs the
  live feature overlay and portal-carve snapshot.
- [ ] Continue the world/plain-input follow-up before moving
  `ProjectileCollisionWorld`; the carved solids are not a plain world input yet.
- [ ] Audit test-travel opportunities individually before moving them. The
  doc-visible candidates are `features/conversion_tests.rs` and
  `world/rooms/tests.rs`, but the current contents should be checked first so
  only pure backend/world tests move; actor-simulation tests should stay with
  `ambition_actors`.
- [ ] Defer the S5/S6 player fold and the eventual `features/` rename until the
  unified-actor work is ready; do not churn the module tree first.
- [ ] Keep the E9 umbrella narrow. New app/demo/content code should import
  engine/model vocabulary through `ambition::*`, while app-local extensions such
  as kaleidoscope stay direct.
- [ ] Avoid new crates unless there is a present consumer and a real dependency
  inversion; the remaining fable work should mostly be moves, deletions, and
  ratchets.

### F9 — 2026-07-09 fable verification of the executed queue + the next-phase ruling

The F1–F6 priority queue was executed by follow-up agents (~40 commits,
2026-07-08/09). This section is an INDEPENDENT verification — checked against
manifests/source, not the execution log:

- **F1 arrows: ALL closed.** render→actors = 0; host deps = exactly its
  charter (engine_core/input/primitives/portal*/render/runtime/sim_view);
  vfx→characters gone (vfx owns its hit-side vocabulary); items→ui_nav gone;
  the actors::portal facade deleted; `GameMode` lives in
  platformer_primitives; `ControlFrame` moved to engine_core (the F1.7
  split-brain resolved ahead of schedule).
- **World purity: DONE, by a different (accepted) route** — see the ruling
  below. `ambition_world` deps = engine_core + entity_catalog + time ONLY,
  and the allow-list ratchet test
  (`ambition_world_dependency_allowlist_ratchets_world_ir_purity`) exists
  with dissolution instructions in its failure message. `ron_room` was
  re-sided into `ambition_world` (F3.1) with ldtk_map re-exporting — the
  legal direction.
- **F3.2 closed:** ECS actors/bosses REQUIRE `SweepSample`;
  `PortalSweepAnchor` retired; portal CCD feeds from the kernel sample with
  a live-endpoint guard against teleport-as-crossing.
- **F4.3/F4.4 closed:** `ClockResetRequest` through the one owner;
  lowest-`PlayerSlot` fallbacks, ratcheted.
- **E9 exit MET:** app manifest = `ambition` + ambition_content +
  ambition_menu_kaleidoscope (3 ≤ 4); `crates/ambition` umbrella + prelude
  exist; `game/ambition_demo_sanic` / `game/ambition_demo_smb1` are
  registered workspace members depping ONLY the umbrella, oracle-ratcheted.
  (They are intentionally empty content plugins — filling them is a BUILD
  item, below.)
- **Gate: 44/44 suites green, ZERO failures.** The former `unified_melee`
  feel-RED was diagnosed as a stale read-model assumption in the TEST (it
  now follows the hostile by `FeatureId + BodyMelee` and accepts both swing
  authorities); the enemy-AI oracle (`enemy_attacks_player`) was already
  green. `ambition_actors` is down to 63.4k.

**RULING on the F1.1 route (accept + follow through):** purity was achieved
by making the family spec types IR-NATIVE (`HazardVolumeSpec`,
`InteractableSpec`, `PickupSpec`, `ChestSpec`, `BreakableSpec`, IR
`PortalSpec`) with actor-side lowering to runtime components — rather than
by dissolving the typed lists into `PlacementRecord`. This is a legitimate,
arguably better sequencing: the dep payoff arrived without waiting for six
branch conversions. ACCEPTED. But the two-channel IR (typed lists +
placements) is now an INTERNAL split-brain with a real tax — the dual-emit
guard, the F7 name-loss class of bug, two spawn paths. The record-over-schema
consolidation therefore CONTINUES, now as pure IR-internal cleanup: convert
one family per session (interactables → pickups → chests → breakables →
portals; hazards are done), each ending with that family's `Vec` deleted
from `RoomSpec` and its lowering registered. Exit for the whole arc: the
dual-emit guard in `spawn_room_feature_entities_with_registry` DELETES
(placements become the only channel).

✅ **DONE (Opus 2026-07-09): interactables consolidated to placements-only.**
The first branch conversion landed and went FURTHER than the hazard exemplar:
because `InteractableSpec`/`InteractionKindSpec` are fully Tier-0-pure (no
kernel `Vec2`, unlike `HazardVolumeSpec`'s inline `KinematicPath`), they were
MOVED DOWN into `ambition_entity_catalog::placements` and reused directly as
the `PlacementSchema::Interactable` payload — so there is ONE pure type, not a
schema/world mirror with a mapping. `RoomSpec.interactables`,
`RoomEmission.interactables`, and the `RoomEmission::interactable` helper are
DELETED; the LDtk npc/switch converters emit a `PlacementRecord` only; the
actor sim path lowers via a registered `lower_interactable_placement`; and the
render authored-visual path reads the same records (both consumers now share
the single channel). No dual-emit guard was added for interactables — the
family flows through placements exclusively. `ambition_world::rooms` re-exports
the moved types so `rooms::InteractableSpec` paths stayed stable. Gate green
(`ambition_app --features rl_sim`, actors lib 748, ldtk_map 24, world 25).
Pattern to reuse for pickups/chests/breakables: same Tier-0 move (all four are
Vec2-free); portals stay LAST because `PortalSpec` carries `Vec2` and cannot go
Tier-0 without a plain-pair mirror. When all families are placements-only, the
hazard dual-emit guard becomes the arc's final deletion (hazards keeps its Vec
+ inline-motion legacy path until its `KinematicPath` lifts to room level).

✅ **DONE (Opus 2026-07-09): pickups consolidated to placements-only.** Family 2
followed the interactables pattern exactly: `PickupSpec`/`PickupKindSpec` moved
into `ambition_entity_catalog::placements` as the `PlacementSchema::Pickup`
payload; `RoomSpec.pickups`/`RoomEmission.pickups`/the `pickup` helper deleted;
the LDtk `PickupSpawn` converter emits a `PlacementRecord` only; actor sim
lowers via `lower_pickup_placement`; render reads pickups off `spec.placements`.
The spatial-integrity test and the geometry-debug example gained a shared
`placement_aabbs(room, kind)` helper (using `PlacementRecord::kind()`) so
family lookups stay one-liners. Gate green. Remaining: chests → breakables →
portals.

✅ **DONE (Opus 2026-07-09): chests consolidated to placements-only.** Family 3,
identical pattern: `ChestSpec`/`ChestStateSpec` moved into
`ambition_entity_catalog::placements` (reward reuses the already-moved
`PickupKindSpec`) as `PlacementSchema::Chest`; `RoomSpec.chests`,
`RoomEmission.chests`, the `chest` helper deleted; the LDtk `ChestSpawn`
converter emits a `PlacementRecord`; actor sim lowers via
`lower_chest_placement`; render reads chests off `spec.placements`. Gate green.
Remaining: breakables → portals.

✅ **DONE (Opus 2026-07-09): breakables consolidated to placements-only.** Family
4 — all four types (`BreakableSpec`, `BreakableStateSpec`, `BreakableTriggerSpec`,
`BreakableCollisionSpec`, with their impls) moved into
`ambition_entity_catalog::placements` as `PlacementSchema::Breakable`. The one
twist vs. the other families: breakables enter via the SURFACE-compile pipeline
(`BreakablePlatform`/`BreakablePogoOrb` → `compile_surface` →
`SurfaceCompiled.breakables`), not a dedicated converter — so the placement
conversion happens in `RoomEmission::from_compiled`, which now maps each typed
`Authored<BreakableSpec>` into a `PlacementRecord` at the emission edge
(`SurfaceCompiled` keeps its internal typed field). `RoomSpec.breakables`,
`RoomEmission.breakables` deleted; actor sim lowers via
`lower_breakable_placement`; render reads breakables off `spec.placements`. Gate
green. **Only portals remain** — and portals are the deliberate exception:
`PortalSpec` carries `ae::Vec2` (`pos`, `normal`), which cannot live in the
Tier-0 catalog, so it needs a plain-pair (`[f32;2]`) mirror rather than a move.
That is the one family where the split-brain END STATE (Vec deletion) requires
new mirror types instead of a move — assess separately.

**The next-phase queue (in order):**
1. **Demo content** — fill `ambition_demo_sanic` (movement identity showcase)
   and `ambition_demo_smb1` (level 1-1 style slice) with real rooms +
   profiles. This is the umbrella's real test and the first BUILD (not
   restructure) item; it will surface every remaining engine leak.
2. **IR consolidation branch conversions** (the ruling above) — opus-sized,
   one family each. **interactables + pickups + chests + breakables: DONE (2026-07-09).**
   Only portals remain (see the ruling above; `PortalSpec` carries `Vec2`, so
   it needs a plain-pair mirror rather than a Tier-0 move).
3. **Projectile remaining steppers** — stay actor-side until their inputs are
   plain (the blockers are correctly enumerated in the follow-up checklist);
   do NOT force this seam.
4. **S5/S6 player fold + the features/ rename** — deferred by design until
   unified-actor work is ready.
5. Standing: no new crates without a present consumer; facade re-adds are
   ratcheted.
