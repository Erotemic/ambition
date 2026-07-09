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
     directly. The actual victim-routing/charge steppers still stay actor-side
     until the boss/player/world inputs are split.**
2. **RESIDUAL GLUE for already-minted crates** (audio/menu/dialog/items/
   encounter/persistence/music/dev modules, ~7k total): each is the actor-side
   wiring for a carved crate. Per ADR 0019, the plugin/schedule wiring belongs
   in `ambition_runtime`; actor-DOMAIN reactions stay. Treat each module as a
   two-way split, one commit each — do NOT move them wholesale into runtime
   (that would just relocate the god-hub). **F2 residual-glue slice (Codex
   2026-07-08) applied that rule to projectiles first: runtime owns the schedule
   facade for the remaining actor-side projectile steppers; the model stays in
   `ambition_projectiles`, and the actor-domain victim/charge logic has not been
   moved wholesale.** **A follow-on residual-glue slice (Codex 2026-07-08)
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
   sees the intentional ordering seam.
5. **The full app gate** re-ran clean after fix (1): all suites green except
   the two documented REDs (`unified_melee::a_hostile_actor` feel-RED;
   verify gnu_ton in the final run below).

### F5 — Elegance directions the new structure makes visible (NOT yet in any card)

1. ✅ **FIRST CUT (Codex 2026-07-09): `ambition` umbrella crate minted.**
   `game/ambition_app` still declares the old direct dependency wall for now,
   but the downstream engine surface now exists at `crates/ambition`: it
   re-exports the runtime, host, render, world/LDtk, actor/model, and lower
   vocabulary crates; exposes `engine::{add_headless_foundation,
   init_engine_states, PlatformerEnginePlugins}`; and has a curated prelude for
   new game/content crates. The facade deliberately does **not** depend on
   `ambition_app`, `ambition_content`, or the kaleidoscope app/backend crate.
   Remaining E9 work is app-manifest collapse: move app imports through the
   facade in mechanical clusters until `game/ambition_app/Cargo.toml` lists ≤ 4
   direct `ambition*` deps.
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

Full `cargo test -p ambition_app --features rl_sim` after the F4 fixes: **44
suites green; the only failure is the documented `unified_melee::
a_hostile_actor` feel-RED** (unchanged, feel-reserved for Jon). The
decomposition landing is behaviorally sound.

Ops note: the dev box hit 100% disk mid-audit; `~/ambition-target/debug/
incremental` (149G of regenerable build cache) was deleted to recover.
Consider `CARGO_INCREMENTAL=0` for CI-style full-gate runs, or a periodic
`cargo clean` cron, so a full disk doesn't silently kill background gates.

**Priority order for the next sessions (all opus-executable, most valuable
first):** finish the E9 app-manifest collapse through the new umbrella
surface → projectiles dedicated session (already carded). F2 is closed for
audit cleanup; F3.2, F4.3, and F4.4 are closed correctness/ruling seams; deeper
actor decomposition is tracked by the later world/plain-input, projectile, and
unified-actor cards.

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

After the F7 fixes: `ambition_ldtk_map` 25 green (contract tests restored),
`ambition_actors` lib 789 green, and the FULL app rl_sim gate = **44 suites
green with only the documented `unified_melee` feel-RED**. The boundary test
now explicitly allows the fixture's DATA path while still forbidding the
Cargo dep (the distinction is written into the test). The D2 template carries
the BINDING test-accounting rule so no future carve can silently drop pinned
contracts again.

This closes the fable audit. The repo is structurally sound, behaviorally
green, and every remaining item is enumerated with a prescription in F1–F7 —
the priority queue at F6 stands, with F7's lowering fixes already landed.
