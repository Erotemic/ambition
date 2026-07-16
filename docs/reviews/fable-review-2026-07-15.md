Interesting - I see references to ambition_actors, ambition_portal, ambition_runtime which are NOT declared deps of render. Let me locate those and classify the seam-relevant deps.

Bash Find quoted proper-noun string literals suggesting named content
IN
cd /home/joncrall/code/ambition/crates/ambition_characters && echo "=== string literals that look like content names (quoted PascalCase/proper nouns) ===" && grep -rhoE '"[A-Z][a-z]+[A-Za-z ]*"' --include=*.rs src | grep -v tests | sort -u | head -60

OUT
=== string literals that look like content names (quoted PascalCase/proper nouns) ===
"ActorActionMessage registered"
"Alpha"
"Back"
"Beta"
"Brain attached"
"Brain still attached"
"BrainActionCounter registered"
"Chase"
"Cornered"
"Dash IS demanded"
"Dash"
"Distinct verbs"
"Front"
"Jump"
"NaN in trace"
"Phase model"
"PlayerRobot"
"Robot"
"Shared"
"Sniper must not fire at dead target"
"Sniper out of aggro should not fire"
"SpecialActionSpec should deserialize from RON"
"SpecialActionSpec should serialize to RON"
"Standard melee striker"
"Still"
"Test"
Now let me read the facade provider.rs (the logic-carrying file) and the app's key wiring/entry files.

Read provider.rs
Let me characterize the platformer_primitives modules and the smaller crates.

Now let me read the lib.rs files and Cargo.toml dependencies for the main crates to understand identities and dependencies.

Read crate-remap-idea.md
Read lib.rs
Bash Locate undeclared-dep references in render
IN
cd /home/joncrall/code/ambition/crates/ambition_render && echo "=== ambition_actors refs (NOT a declared dep) ===" && grep -rn "ambition_actors" src && echo "=== ambition_portal (non-presentation) refs ===" && grep -rn "ambition_portal::" src && echo "=== ambition_runtime refs ===" && grep -rn "ambition_runtime" src && echo "=== ambition_interaction refs (0 counted) ===" && grep -rn "ambition_interaction" src

OUT
=== ambition_actors refs (NOT a declared dep) ===
src/rendering/item_visuals.rs:191:/// Marks a sprite entity visualizing a [`WorldItem`] (ambition_actors::items::world_item::WorldItem).
src/rendering/item_visuals.rs:196:/// id a [`WorldItem`] (ambition_actors::items::world_item::WorldItem) carries →
=== ambition_portal (non-presentation) refs ===
src/rendering/mod.rs:187:        // tag, gun art — see `ambition_portal::host_adapter`). Gravity visuals
src/rendering/gravity_visuals.rs:2://! the presentation rendering plugin). Extracted from `ambition_portal::presentation`
=== ambition_runtime refs ===
src/platformer_presentation.rs:36://! app.add_plugins(ambition_runtime::PlatformerEnginePlugins::fixed_tick());
=== ambition_interaction refs (0 counted) ===
Bash Extract platformer_primitives module docs
IN
cd /home/joncrall/code/ambition/crates/ambition_platformer_primitives/src
for f in body.rs gravity.rs kinematic.rs class_b.rs frame_env.rs schedule.rs camera_ease.rs camera_layers.rs world_item_art.rs math.rs orientation.rs physics.rs transit.rs feature_overlay.rs markers.rs sim_id.rs time.rs projectile/mod.rs lifecycle/mod.rs; do echo "##### $f #####"; awk '/^\/\/!/{print; next} {c++} c>0 && !/^\/\/!/{exit}' "$f" | head -8; done

OUT
##### body.rs #####
//! Unified body kinematics for every controllable platformer body.
//!
//! Systems that hold multiple mutable [`BodyKinematics`] queries must prove
//! them disjoint with marker filters (`With<PlayerEntity>`, `With<ActorConfig>`,
//! `With<BossConfig>`, plus `Without<...>` guards where needed). Do that with
//! filters, never by re-splitting the component.
##### gravity.rs #####
//! Shared world physics applied to every actor body.
//!
//! [`GravityField`] is the world's gravity state (a redirectable down). The
//! goal of this module is that adding a new *global* force later — wind, a
//! tractor field, a gravity well — is a one-place change that reaches **every**
//! actor automatically:
//!
//! - **Free bodies** (thrown / ground items, projectiles) integrate through
##### kinematic.rs #####
//! Generic kinematic body — gravity + axis-separated sweep against a `World`.
//!
//! Why this exists: the player's `movement` module owns a sophisticated
//! sweep with jump-buffer, dash, blink, climb, and other player-only
//! affordances. Enemies and NPCs only need a small subset (gravity,
//! ground/wall collision, optional drop-through), but the sandbox used
//! to ship hand-rolled `blocked` / `blocked_y` predicates that diverged
//! from player physics in subtle ways — most visibly, hostile NPCs and
##### class_b.rs #####
//! **Class-B transit authority** — the per-frame remap ledger.
//!
//! `docs/planning/engine/collision-and-ccd.md` §3.2 splits every consumer of a
//! frame's motion path into three classes. Class A is the movement kernel (one
//! per body, resolves solid contacts at TOI). Class C are observers that read
//! the path and never move the body. **Class B is the transit authority**: the
//! small set of writers allowed to *remap* a body's position discontinuously —
//! portal transit, loading-zone room transitions, death/respawn, and scripted
##### frame_env.rs #####
//! The authoritative per-body movement frame: resolved once, consumed everywhere.
//!
//! ADR 0024's frame law says every body-relative operation in a movement tick —
//! controller interpretation, the active movement policy, jumps/dashes/blinks,
//! knockback and launch directions, support publication — must consume ONE
//! environment-resolved reference/acceleration frame for that body and tick. This
//! module owns the ECS half of that law:
//!
##### schedule.rs #####
//! Runtime schedule vocabulary that is independent of Ambition content.
//!
//! `SandboxSet` remains the concrete app schedule for now. These labels document
//! the future crate-level concepts and give new runtime modules names that do
//! not depend on app assembly details.
##### camera_ease.rs #####
//! Smoothed camera scale + world-target state with tunable ease rates.
//!
//! Moved below `ambition_actors` during F1.5 so render/host can share camera
//! presentation timing state without depending on the actor-domain crate.
##### camera_layers.rs #####
//! Presentation camera markers shared by host, render, and app wiring.
//!
//! These are Bevy presentation vocabulary, not actor-domain state. Keeping them
//! below the actor crate lets render/host systems agree on camera identity
//! without depending on `ambition_actors`.
##### world_item_art.rs #####
//! Provider-contributed art declarations for walk-into world items.
//!
//! A `WorldItem` (in `ambition_actors`) carries a presentation `sprite` id (an art
//! key, deliberately separate from the equipment it grants).
//! The render layer draws that id as a real sprite through its `WorldItemArt`
//! handle map — but a gameplay PROVIDER crate (which owns the asset knowledge:
//! "the milk pickup is `sprites/props/super_mary_o_milk_carton.png`") must not
//! depend on the renderer. So the contribution is split, exactly like the audio /
##### math.rs #####
//! Pure portal-map vector math for platformer mechanics.
//!
//! These are the orientation-between-two-normals transforms at the heart of the
//! portal system, factored out as plain [`Vec2`] math with no ECS, no Ambition
//! content, and no AABB types. They are portal-shaped in origin but are reusable
//! reflect/rotate-between-two-normals primitives, so the sandbox's
//! `portal_pieces` (AABB / piece geometry) and `transit` (velocity transit) both
//! build on them while this crate stays content-free.
##### orientation.rs #####
//! Actor orientation under gravity (the "which way is down" upright reflex).
//!
//! The shared body-orientation component ([`ActorRoll`]) and its righting system
//! ([`update_actor_roll`]). Any body that can be reoriented (by a portal flip, a
//! gravity zone, a knockback) eases back toward "feet along gravity" here. The
//! component and systems are gravity-driven and actor-generic — they operate on
//! the unified [`crate::body::BodyKinematics`] body and the in-crate
//! [`crate::gravity::GravityCtx`], with no sandbox / content dependency.
##### physics.rs #####
//! Shared secondary-physics settings resource.
//!
//! The Avian adapter stays in `ambition_actors::world::physics`; this tiny
//! resource lives below render/app so presentation systems can receive the same
//! settings value without depending on actor machinery.
##### transit.rs #####
//! Generic body-transit velocity math for platformer mechanics.
//!
//! This is the proto-runtime home for the pure velocity transform that maps a
//! body's velocity from one oriented surface to another. It is portal-shaped in
//! origin but is plain reflect/rotate-between-two-normals math with no portal
//! dependency, so non-portal mechanics can reuse it.
##### feature_overlay.rs #####
//! Shared read resource for transient ECS-derived world collision overlays.
##### markers.rs #####
//! Generic entity-marker components shared by reusable mechanics.
//!
//! These are pure `Component` markers — query filters with no fields and no
//! sandbox-internal dependencies. They live in the runtime crate so portal,
//! gravity, and other extracted mechanics can query the player / simulated
//! feature entities without depending on the sandbox's `player` or `features`
//! modules. The host (`ambition_actors`) re-exports them from their original
//! paths so existing call sites compile unchanged.
##### sim_id.rs #####
//! **`SimId` — the one identity vocabulary for snapshot, replay, and netcode.**
//!
//! `docs/planning/engine/netcode.md` N3.1, *Identity & scope* (pinned 2026-07-06):
//!
//! > *"One identity vocabulary, shared with SimView. Every snapshot-registered
//! > entity carries a `SimId` — the EXISTING stable ids, not a new system: actors
//! > use `ActorConfig.id` (== LDtk iid; placement identity), player bodies use
//! > their slot, dynamically-spawned sim entities (projectiles, dropped items,
##### time.rs #####
//! Neutral simulation-time resource for the platformer runtime.
//!
//! The runtime's generic systems (gravity integration, oscillating /
//! temporary zones, the orient-to-gravity roll) advance world-anchored state and
//! must scale with bullet-time / hitstop / pause. In the sandbox that scaled dt
//! is `ambition_time::WorldTime::sim_dt`, but the runtime crate cannot
//! depend on the sandbox. [`SimDt`] is the inversion seam: a content-free
//! resource the runtime reads, which the **host** mirrors from its own clock
##### projectile/mod.rs #####
//! Reusable, game-agnostic projectile physics primitive.
//!
//! This module is the brain-free physics core for projectiles in a 2D
//! platformer: authored-intent specs, a per-frame kinematic body, and a
//! world-vs-body collision resolver. It depends only on
//! `ambition_engine_core` (geometry + world) — no spawn logic, no damage
//! routing, no actor roster, and no Ambition-specific content. Any
//! platformer (or an agent building one) can drop it in and feed it a
##### lifecycle/mod.rs #####
//! Lifecycle vocabulary for entities spawned by reusable platformer systems.
//!
//! The public API is the helper verb (`spawn_room_scoped`, `spawn_run_scoped`,
//! `spawn_mode_scoped`, `spawn_persistent`) rather than the marker component
//! convention. Marker components remain public because existing cleanup queries
//! and tests need to name them, but new spawn sites should prefer
//! [`SpawnScopedExt`].
Let me examine what's game-specific content vs. generic machinery. Reading boss catalog, enemies archetypes, character roster, and checking the sibling ambition_content crate.

Bash Locate mid-frequency dep references in render
IN
cd /home/joncrall/code/ambition/crates/ambition_render && echo "=== combat refs ===" && grep -rn "ambition_combat" src && echo "=== characters refs ===" && grep -rn "ambition_characters" src && echo "=== persistence refs ===" && grep -rn "ambition_persistence" src && echo "=== entity_catalog refs ===" && grep -rn "ambition_entity_catalog" src && echo "=== dialog refs ===" && grep -rn "ambition_dialog" src && echo "=== input refs ===" && grep -rn "ambition_input" src && echo "=== sfx refs ===" && grep -rn "ambition_sfx" src

OUT
=== combat refs ===
src/rendering/primitives.rs:7:use ambition_combat::events::FeatureVisualKind;
src/rendering/health.rs:13:use ambition_combat::events::FeatureVisualKind;
src/rendering/mod.rs:60:pub use ambition_combat::events::BoundFeatureKind;
src/rendering/actors/overlays.rs:10:use ambition_combat::events::FeatureVisualKind;
src/rendering/world.rs:17:use ambition_combat::events::FeatureVisualKind;
src/rendering/actors/mod.rs:15:use ambition_combat::events::{BoundFeatureKind, FeatureVisualKind};
src/rendering/actors/mod.rs:262:fn state_aware_entity_sprite(view: &ambition_combat::events::FeatureView) -> Option<EntitySprite> {
=== characters refs ===
src/rendering/health.rs:12:use ambition_characters::actor::Health;
src/rendering/hit_flash.rs:17://! - **Player**: [`ambition_characters::actor::BodyCombat::hit_flash`].
src/rendering/actors/mod.rs:48:            Without<ambition_characters::actor::WornCharacter>,
src/rendering/actors/mod.rs:87:            &ambition_characters::actor::WornCharacter,
src/rendering/actors/worn_binder_tests.rs:10:use ambition_characters::actor::WornCharacter;
=== persistence refs ===
src/quality.rs:8:use ambition_persistence::settings::{UserSettings, VisualQualityBudget, VisualQualityProfile};
src/quality.rs:18:        let settings = ambition_persistence::settings::VisualQualitySettings::default();
src/screen_effects.rs:34:use ambition_persistence::settings::{ScreenShaderSettings, UserSettings};
src/rendering/parallax.rs:18:use ambition_persistence::settings::ParallaxBudget;
src/rendering/actors/mod.rs:17:use ambition_persistence::settings::TextureResolutionScale;
=== entity_catalog refs ===
src/rendering/world.rs:108:        if let ambition_entity_catalog::placements::PlacementSchema::Hazard(hazard) = &record.schema
src/rendering/world.rs:121:        if let ambition_entity_catalog::placements::PlacementSchema::Pickup(pickup) = &record.schema
src/rendering/world.rs:138:        if let ambition_entity_catalog::placements::PlacementSchema::Chest(chest) = &record.schema {
src/rendering/world.rs:150:        if let ambition_entity_catalog::placements::PlacementSchema::Breakable(breakable) =
src/rendering/world.rs:201:        if let ambition_entity_catalog::placements::PlacementSchema::Interactable(spec_i) =
=== dialog refs ===
src/lib.rs:17:/// `ambition_dialog`.
src/dialog_ui.rs:3://! Render-only. [`sync_dialog_ui`] mirrors `ambition_dialog::DialogState`
src/dialog_ui.rs:15:use ambition_dialog::{DialogChoiceSlot, DialogState};
=== input refs ===
src/cutscene/mod.rs:176:                        // `ambition_input::presets::ControlPreset::input_map`
=== sfx refs ===
src/fx.rs:15:use ambition_sfx::{SfxMessage, SfxWriter};
Bash Extract dependencies from Cargo.toml for target crates
IN
for c in ambition_combat ambition_projectiles ambition_portal ambition_portal_presentation ambition_encounter ambition_interaction ambition_cutscene ambition_dialog ambition_dev_tools ambition_persistence ambition_load ambition_load_presentation; do
  echo "=== $c/Cargo.toml [dependencies] ==="
  awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&NF' /home/joncrall/code/ambition/crates/$c/Cargo.toml 2>/dev/null | grep -oE '^ambition_[a-z_]+|^[a-z_]+ ' | head -40
done

OUT
=== ambition_combat/Cargo.toml [dependencies] ===
ambition_characters 
ambition_engine_core 
ambition_entity_catalog 
ambition_interaction 
ambition_platformer_primitives 
ambition_sfx 
ambition_time 
ambition_vfx 
serde 
bevy 
=== ambition_projectiles/Cargo.toml [dependencies] ===
ambition_engine_core 
ambition_platformer_primitives 
ambition_portal 
ambition_world 
ambition_gameplay_trace 
ambition_input 
ambition_vfx 
serde 
bevy 
=== ambition_portal/Cargo.toml [dependencies] ===
serde 
bevy 
ambition_engine_core 
ambition_platformer_primitives 
=== ambition_portal_presentation/Cargo.toml [dependencies] ===
bevy 
ambition_engine_core 
ambition_platformer_primitives 
ambition_portal 
=== ambition_encounter/Cargo.toml [dependencies] ===
ambition_engine_core 
ambition_interaction 
ambition_persistence 
bevy 
ron 
serde 
=== ambition_interaction/Cargo.toml [dependencies] ===
ambition_characters 
ambition_engine_core 
ambition_entity_catalog 
serde 
=== ambition_cutscene/Cargo.toml [dependencies] ===
serde 
bevy 
=== ambition_dialog/Cargo.toml [dependencies] ===
ambition_engine_core 
ambition_ui_nav 
ambition_input 
ambition_sfx 
ambition_persistence 
bevy 
bevy_yarnspinner 
=== ambition_dev_tools/Cargo.toml [dependencies] ===
ambition_engine_core 
ambition_characters 
ambition_platformer_primitives 
ambition_persistence 
bevy 
serde 
ron 
=== ambition_persistence/Cargo.toml [dependencies] ===
ambition_engine_core 
ambition_input 
bevy 
ron 
serde 
=== ambition_load/Cargo.toml [dependencies] ===
bevy 
=== ambition_load_presentation/Cargo.toml [dependencies] ===
ambition_load 
ambition_game_shell 
bevy
Bash Read ambition_time header
IN
cd /home/joncrall/code/ambition
echo "=== ambition_time lib.rs ==="; wc -l crates/ambition_time/src/lib.rs; awk '/^\/\/!/{print} NR<=40{next}' crates/ambition_time/src/lib.rs | head -0
sed -n '1,45p' crates/ambition_time/src/lib.rs

OUT
=== ambition_time lib.rs ===
326 crates/ambition_time/src/lib.rs
//! Reusable time vocabulary + Bevy producer for the named-clock dt model
//! (ADR 0010 / 0011).
//!
//! Host games write [`ClockState::time_scale`]; [`TimePlugin`] converts
//! Bevy `Time` into [`WorldTime`] once per frame. Read dt through typed
//! accessors instead of `Res<Time>::delta_secs()`:
//!
//! - [`WorldTime::sim_dt`] for gameplay state and world-anchored animation.
//! - [`WorldTime::wall_dt`] for UI, audio, hot reload, and debug overlays.
//! - [`WorldTime::player_dt`] for observer cognitive time.
//! - [`WorldTime::entity_dt`] for per-entity proper time.

use bevy::prelude::*;

/// A clock observer — the seam for per-player (or per-agent) cognitive
/// clocks (ADR 0011). A game maps its own player-slot type onto this
/// generic index; in single-player every observer collapses to
/// [`ClockObserver::PRIMARY`].
///
/// Kept a plain `u8` newtype (not coupled to any game's player type) so
/// this crate stays content-free. Convert from a host slot type with a
/// `From`/`Into` impl on the host side.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ClockObserver(pub u8);

impl ClockObserver {
    /// The local primary observer (player 1 / the single-player observer).
    pub const PRIMARY: ClockObserver = ClockObserver(0);

    /// The raw observer index.
    pub fn index(self) -> u8 {
        self.0
    }
}

/// ADR 0010 vocabulary — sim time, per-observer cognitive time, and
/// unscaled host wall time.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ClockDomain {
    SimClock,
    PlayerClock(ClockObserver),
    WallClock,
}

/// The current sim-clock scale — the single mutable `f32` the time
Bash Find ambition_content crate and references
IN
cd /home/joncrall/code/ambition
echo "=== workspace crates list ===" && ls crates/ | head -60
echo "=== game/ dir ===" && ls game/ 2>/dev/null
echo "=== ambition_content exists? ===" && find . -maxdepth 3 -type d -name 'ambition_content' 2>/dev/null
echo "=== references to ambition_content in ambition_actors ==="
grep -rn 'ambition_content' crates/ambition_actors/src --include='*.rs' | head

OUT
=== workspace crates list ===
ambition
ambition_actors
ambition_asset_manager
ambition_audio
ambition_characters
ambition_combat
ambition_cutscene
ambition_dev_tools
ambition_dialog
ambition_encounter
ambition_engine_core
ambition_entity_catalog
ambition_game_shell
ambition_gameplay_trace
ambition_host
ambition_input
ambition_interaction
ambition_inventory_ui
ambition_items
ambition_ldtk_map
ambition_load
ambition_load_presentation
ambition_menu
ambition_persistence
ambition_platformer_primitives
ambition_portal
ambition_portal_presentation
ambition_projectiles
ambition_render
ambition_runtime
ambition_settings_menu
ambition_sfx
ambition_sfx_bank
ambition_sim_view
ambition_sprite_sheet
ambition_time
ambition_touch_input
ambition_ui_nav
ambition_vfx
ambition_world
=== game/ dir ===
ambition_app
ambition_content
ambition_demo_mary_o
ambition_demo_mary_o_app
ambition_demo_pocket
ambition_demo_sanic
ambition_demo_sanic_app
ambition_menu_kaleidoscope
=== ambition_content exists? ===
./game/ambition_content
./.tmp-ambition-agent-source-20260612T232700Z-75cb34e1798e-stage/crates/ambition_content
=== references to ambition_content in ambition_actors ===
crates/ambition_actors/src/character_roster.rs:13:        "../../../game/ambition_content/assets/data/character_catalog.ron"
crates/ambition_actors/src/lib.rs:11://! - above it, `ambition_content` provides the named game DATA (rooms, bosses,
crates/ambition_actors/src/lib.rs:15://! content has been migrated out to `ambition_content`. Foundation crates
crates/ambition_actors/src/audio/tests.rs:23:            .join("../../game/ambition_content/assets/worlds");
crates/ambition_actors/src/audio/tests.rs:32:                embedded_bevy_path: Some("ambition_content/worlds/sandbox.ldtk"),
crates/ambition_actors/src/audio/tests.rs:443:                "../../../../game/ambition_content/assets/data/character_catalog.ron"
crates/ambition_actors/src/boss_encounter/roster.rs:5://! content: they live in `ambition_content/assets/data/boss_encounters/*.ron`
crates/ambition_actors/src/boss_encounter/roster.rs:6://! and are contributed through `ambition_content::bosses::register` into the
crates/ambition_actors/src/boss_encounter/systems.rs:38:    // `ambition_content/assets/data/boss_encounters/<id>.ron` and assembled
crates/ambition_actors/src/boss_encounter/catalog.rs:676:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
This crate appears genuinely content-free (machinery). Data lives in game/ambition_content/assets/data/. Let me verify that and inspect the equipment/catalog/smash machinery.

Read catalog.rs
The facade's provider.rs clearly carries substantial logic. Now let me map content, demos, runtime, host, and the policy tests in parallel.

Now let me dig into combat's moveset system and check for parallel paths, plus read the smaller crates' identities.

Read code_smells.md
Bash Explore game/ambition_content data location
IN
echo "=== top-level dirs ===" && ls -d */ && echo "=== game/ dir ===" && ls game/ 2>/dev/null && echo "=== ambition_content assets? ===" && find game -maxdepth 4 -type d 2>/dev/null | head -40

OUT
=== top-level dirs ===
assets/
crates/
debug_traces/
dev/
docs/
game/
scripts/
target/
tests/
tmp/
tools/
untracked/
=== game/ dir ===
ambition_app
ambition_content
ambition_demo_mary_o
ambition_demo_mary_o_app
ambition_demo_pocket
ambition_demo_sanic
ambition_demo_sanic_app
ambition_menu_kaleidoscope
=== ambition_content assets? ===
game
game/ambition_app
game/ambition_demo_mary_o_app
game/ambition_demo_sanic
game/ambition_content
game/ambition_demo_sanic_app
game/ambition_demo_mary_o
game/ambition_menu_kaleidoscope
game/ambition_demo_pocket
game/ambition_app/src
game/ambition_app/debug_traces
game/ambition_app/web
game/ambition_app/tests
game/ambition_demo_mary_o_app/src
game/ambition_demo_mary_o_app/tests
game/ambition_demo_sanic/src
game/ambition_content/assets
game/ambition_content/src
game/ambition_content/tests
game/ambition_demo_sanic_app/src
game/ambition_demo_sanic_app/debug_traces
game/ambition_demo_sanic_app/tests
game/ambition_demo_mary_o/src
game/ambition_menu_kaleidoscope/src
game/ambition_demo_pocket/src
game/ambition_content/src/dialogue
game/ambition_content/src/items
game/ambition_content/src/intro
game/ambition_content/src/falling_sand
game/ambition_content/src/portal
game/ambition_content/src/bosses
game/ambition_content/src/quests
game/ambition_app/src/bin
game/ambition_app/src/menu
game/ambition_app/src/rl_sim
game/ambition_app/src/headless
game/ambition_app/src/dev
game/ambition_app/src/app
game/ambition_app/src/host
game/ambition_app/web/pkg
Bash content module map + deps
IN
echo "=== ambition_content module map ==="; find game/ambition_content/src -type f -name "*.rs" | xargs wc -l | sort -rn | head -50; echo; echo "=== content Cargo deps ==="; sed -n '/\[dependencies\]/,/\[\[/p' game/ambition_content/Cargo.toml | head -60

OUT
=== ambition_content module map ===
 14311 total
  1551 game/ambition_content/src/portal/tests.rs
  1354 game/ambition_content/src/falling_sand.rs
   934 game/ambition_content/src/bosses/specials/gradient_sentinel.rs
   621 game/ambition_content/src/content_validation.rs
   532 game/ambition_content/src/portal/plugin.rs
   482 game/ambition_content/src/bosses/gnu_ton/tests.rs
   404 game/ambition_content/src/bosses/cut_rope/arena.rs
   356 game/ambition_content/src/quest.rs
   335 game/ambition_content/src/intro/route_state/tests.rs
   323 game/ambition_content/src/bosses/cut_rope/mod.rs
   310 game/ambition_content/src/intro/plugin.rs
   308 game/ambition_content/src/bosses/mod.rs
   302 game/ambition_content/src/portal/ability_adapter/tests.rs
   291 game/ambition_content/src/portal/transit_body_adapter.rs
   287 game/ambition_content/src/intro/sprites.rs
   238 game/ambition_content/src/falling_sand/tests.rs
   234 game/ambition_content/src/bosses/banter.rs
   214 game/ambition_content/src/bosses/specials/gradient_nova.rs
   199 game/ambition_content/src/portal/ability_adapter.rs
   194 game/ambition_content/src/provider.rs
   186 game/ambition_content/src/music.rs
   186 game/ambition_content/src/intro/route_state.rs
   183 game/ambition_content/src/intro/cutscene.rs
   181 game/ambition_content/src/bosses/specials/mode_collapse.rs
   178 game/ambition_content/src/portal/transit_body_adapter/projectile_transit_tests.rs
   174 game/ambition_content/src/bosses/specials/overflow_flood.rs
   172 game/ambition_content/src/portal/inventory_adapter.rs
   170 game/ambition_content/src/bosses/specials/snapshot.rs
   165 game/ambition_content/src/bosses/specials/seismic_stomp.rs
   162 game/ambition_content/src/bosses/specials/eye_beam.rs
   158 game/ambition_content/src/bosses/specials/echo_fan.rs
   157 game/ambition_content/src/bosses/cut_rope/victory.rs
   148 game/ambition_content/src/duel_arena.rs
   137 game/ambition_content/src/bosses/yarn.rs
   136 game/ambition_content/src/portal/inventory_adapter/tests.rs
   133 game/ambition_content/src/portal/fire_adapter.rs
   133 game/ambition_content/src/dialogue/cutscene_defaults.rs
   126 game/ambition_content/src/bosses/specials/mod.rs
   123 game/ambition_content/src/portal/host_adapter.rs
   122 game/ambition_content/src/worlds.rs
   122 game/ambition_content/src/dialogue/yarn/tests.rs
   121 game/ambition_content/src/character_catalog.rs
   120 game/ambition_content/src/banter.rs
   114 game/ambition_content/src/portal/input_adapter.rs
   111 game/ambition_content/src/plugin.rs
   110 game/ambition_content/src/bosses/gnu_ton.rs
    96 game/ambition_content/src/dialogue/yarn.rs
    92 game/ambition_content/src/portal/transit_adapter.rs
    87 game/ambition_content/src/portal/shot_adapter.rs

=== content Cargo deps ===
[dependencies]
ambition = { path = "../../crates/ambition", default-features = false }
# Baseline machinery features: the sandbox lib was never built fully bare
# (ldtk/input/portal code is not end-to-end cfg-gated), so the content crate
# pins the same always-on baseline every persona used pre-split.
ambition_dialog = { path = "../../crates/ambition_dialog", default-features = false }
ambition_actors = { path = "../../crates/ambition_actors", default-features = false, features = [
    "ldtk_runtime",
    "input",
    "portal",
] }
ambition_sprite_sheet = { path = "../../crates/ambition_sprite_sheet" }
ambition_projectiles = { path = "../../crates/ambition_projectiles" }
ambition_render = { path = "../../crates/ambition_render" }
ambition_combat = { path = "../../crates/ambition_combat" }
ambition_encounter = { path = "../../crates/ambition_encounter" }
# N3.1: this crate's boss specials hold SIM STATE, so they register their own
# snapshot codecs (`SnapshotRegistry` is a resource). netcode.md: "each sim crate
# registers its components' serialization."
ambition_runtime = { path = "../../crates/ambition_runtime" }
ambition_cutscene = { path = "../../crates/ambition_cutscene" }
ambition_asset_manager = { path = "../../crates/ambition_asset_manager", features = ["bevy"] }
ambition_sfx = { path = "../../crates/ambition_sfx" }
ambition_vfx = { path = "../../crates/ambition_vfx" }
ambition_time = { path = "../../crates/ambition_time" }
ambition_audio = { path = "../../crates/ambition_audio" }
ambition_platformer_primitives = { path = "../../crates/ambition_platformer_primitives" }
# `MovingPlatformSet` + `world_with_sandbox_solids` — the composited collision
# world the portal host adapter and the arena-gate tests read (R3).
ambition_world = { path = "../../crates/ambition_world" }
ambition_portal = { path = "../../crates/ambition_portal" }
ambition_portal_presentation = { path = "../../crates/ambition_portal_presentation", optional = true }
ambition_input = { path = "../../crates/ambition_input" }
ambition_engine_core = { path = "../../crates/ambition_engine_core" }
ambition_entity_catalog = { path = "../../crates/ambition_entity_catalog" }
ambition_characters = { path = "../../crates/ambition_characters" }
ambition_persistence = { path = "../../crates/ambition_persistence" }
ambition_interaction = { path = "../../crates/ambition_interaction" }
ambition_items = { path = "../../crates/ambition_items" }
bevy = { version = "0.18.1", default-features = false, features = ["ui_api"] }
bevy_yarnspinner = { version = "0.8", optional = true }
# Drop bevy_falling_sand's default `physics` feature: it bundles its OWN
# avian2d `PhysicsPlugins`, which collides with Ambition's `AmbitionPhysicsPlugin`
# ("plugin already added" panic when the falling_sand feature is on). Ambition
# bridges particles into its own movement world (project_particles_to_movement_world),
# not avian rigid bodies, so it only needs render/movement/reactions.
bevy_falling_sand = { version = "0.7.0", optional = true, default-features = false, features = [
    "render",
    "movement",
    "reactions",
] }
serde = { version = "1", features = ["derive"] }
ron = "0.11"
serde_json = "1"

[dev-dependencies]
# Self-reference so THIS crate's own test build always enables `portal` — the
# module (and its ~36 transit-conservation tests) is otherwise compiled out under
# the bare `default = []`, so `cargo test -p ambition_content` silently skipped it
# (the F13 momentum-drift regression shipped "green" that way). Downstream crates
Read mod.rs
Read mod.rs
Now let me examine the remaining smaller crates and the reference-frame/relativity theme.

Read lib.rs
Bash demo crate maps + deps
IN
for d in ambition_demo_mary_o ambition_demo_mary_o_app ambition_demo_sanic ambition_demo_sanic_app ambition_demo_pocket; do echo "===== game/$d ====="; find game/$d/src -type f -name "*.rs" | xargs wc -l | sort -rn; echo "--- Cargo deps ---"; sed -n '/\[dependencies\]/,/\[\[/p' game/$d/Cargo.toml | grep -E 'path|=' | head -30; echo; done

OUT
===== game/ambition_demo_mary_o =====
  2639 total
  1019 game/ambition_demo_mary_o/src/lib.rs
   430 game/ambition_demo_mary_o/src/flag.rs
   419 game/ambition_demo_mary_o/src/powerups.rs
   311 game/ambition_demo_mary_o/src/crony.rs
   272 game/ambition_demo_mary_o/src/bricks.rs
   188 game/ambition_demo_mary_o/src/provider.rs
--- Cargo deps ---
ambition = { path = "../../crates/ambition", default-features = false }
bevy = { version = "0.18.1", default-features = false }

===== game/ambition_demo_mary_o_app =====
  452 total
  330 game/ambition_demo_mary_o_app/src/lib.rs
  122 game/ambition_demo_mary_o_app/src/main.rs
--- Cargo deps ---
ambition = { path = "../../crates/ambition", default-features = false }
ambition_demo_mary_o = { path = "../ambition_demo_mary_o" }
bevy = { version = "0.18.1", default-features = false }
bevy_kira_audio = { version = "0.25", default-features = false, features = ["ogg"], optional = true }

===== game/ambition_demo_sanic =====
  2575 total
   802 game/ambition_demo_sanic/src/tests.rs
   782 game/ambition_demo_sanic/src/lib.rs
   470 game/ambition_demo_sanic/src/ball_dash/tests.rs
   389 game/ambition_demo_sanic/src/ball_dash.rs
   132 game/ambition_demo_sanic/src/provider.rs
--- Cargo deps ---
ambition = { path = "../../crates/ambition", default-features = false }
bevy = { version = "0.18.1", default-features = false }

===== game/ambition_demo_sanic_app =====
  549 total
  429 game/ambition_demo_sanic_app/src/lib.rs
  120 game/ambition_demo_sanic_app/src/main.rs
--- Cargo deps ---
ambition = { path = "../../crates/ambition", default-features = false }
ambition_demo_sanic = { path = "../ambition_demo_sanic" }
bevy = { version = "0.18.1", default-features = false }
bevy_kira_audio = { version = "0.25", default-features = false, features = ["ogg"], optional = true }

===== game/ambition_demo_pocket =====
266 game/ambition_demo_pocket/src/lib.rs
--- Cargo deps ---
ambition = { path = "../../crates/ambition", default-features = false }
bevy = { version = "0.18.1", default-features = false }
Read lib.rs
Let me look at the baked-in content and hardcoded named IDs across the crate.

Bash Module docs for world and input crates
IN
cd /home/joncrall/code/ambition
echo "########## ambition_world modules ##########"
for f in lib.rs collision.rs debug_label.rs placements.rs ron_room.rs platforms/mod.rs rooms/mod.rs rooms/graph.rs rooms/room_graph.rs rooms/metadata.rs rooms/spawn.rs rooms/specs.rs rooms/camera.rs rooms/gate_portal.rs rooms/loading_zone.rs; do echo "##### world/$f #####"; awk '/^\/\/!/{print; next} {c++} c>0 && !/^\/\/!/{exit}' "crates/ambition_world/src/$f" | head -6; done
echo; echo "########## ambition_input modules ##########"
for f in lib.rs actions.rs active_input.rs control.rs menu.rs motion_input.rs presets.rs settings.rs; do echo "##### input/$f #####"; awk '/^\/\/!/{print; next} {c++} c>0 && !/^\/\/!/{exit}' "crates/ambition_input/src/$f" | head -6; done

OUT
########## ambition_world modules ##########
##### world/lib.rs #####
//! Backend-agnostic authored world IR.
//!
//! This crate owns the room graph, authored placement records, room metadata,
//! moving-platform math, and the composited [`collision`] world every sweep and
//! raycast reads. Backend adapters such as LDtk convert into these types;
//! simulation crates interpret them through explicit lowering seams.
##### world/collision.rs #####
//! The composited collision world: the authored room folded together with the
//! per-frame dynamic contributions a running sim adds to it.
//!
//! [`CollisionWorld`] is the single collision read-API every actor sweep/raycast
//! should reach for instead of `ambition_platformer_primitives::lifecycle::SessionWorldRef<RoomGeometry>`: it composites the authored
//! room with moving platforms and the ECS overlay so player, NPC, enemy, and
##### world/debug_label.rs #####
//! Generic room-object label for debug overlays and editor selection.
//!
//! DebugLabel is a sandbox concern — rendering of labels lives in the
//! Bevy adapter, and the meaning is authored with the room data so
//! debug overlays, inspectors, and future editor tools share the same
//! source of truth. (Moved here from the engine when the
##### world/placements.rs #####
//! Authored placement RECORDS on the room IR — the [W-b] shape
//! (decomposition.md, W-track ruling; architecture.md §4b).
//!
//! A record joins the spatial footprint (record-level: `id` + `aabb`, owned
//! by the space IR) to the CLOSED Tier-0 authored schema
//! ([`ambition_entity_catalog::placements::PlacementSchema`] — what the
##### world/ron_room.rs #####
//! The `ron-room` loader: rooms as serialized world IR.
//!
//! A `ron-room` is a [`RoomSpec`] plus its graph links serialized as RON.
//! It is a backend-neutral path for generated rooms and fixtures: a bake
//! tool emits room IR, and the loader appends it beside rooms produced by
//! an authoring backend such as LDtk.
##### world/platforms/mod.rs #####
//! LDtk-authored moving-platform runtime helpers.
//!
//! Moving platforms remain sandbox-side as a design experiment, but they now
//! contribute temporary solid blocks to the engine collision world each frame.
//! That gives us rideable/collidable behavior without committing moving-solid
//! semantics to `ambition_engine_core` before we have tests for carrying, crushing,
##### world/rooms/mod.rs #####
//! Room graph and authored room IR.
##### world/rooms/graph.rs #####
//! `RoomSet` graph assembly + queries (petgraph-backed transition graph).
//!
//! `impl RoomSet` block: builds the node/edge graph from runtime rooms
//! (`from_parts`), exposes active-room accessors (`active_spec`/`active_world`/
//! `active_loading_zones`/…), and resolves player transitions
//! (`transition_for_player`, `nearby_zone_hints`, `layout_warnings`). The
##### world/rooms/room_graph.rs #####
//! RoomSpec + the transition graph types.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.
##### world/rooms/metadata.rs #####
//! Room metadata, music request, and visual profile.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.
##### world/rooms/spawn.rs #####
//! Spawn-point resolution: where the player lands after a room transition.
//!
//! `validated_spawn` clamps/repairs a proposed spawn out of walls and solids;
//! `arrival_from_target_zone` picks the landing point per `LoadingZoneActivation`
//! (edge inset for `EdgeExit`, target-zone center for `Door`/`Walk`). Helpers
//! and the player-half/inset constants are `pub(super)` for sibling `graph`.
##### world/rooms/specs.rs #####
//! Authored room content specs (props, items, portals, shrines, gravity zones).
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.
##### world/rooms/camera.rs #####
//! Camera zones, clamp modes, and kinematic path specs.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.
##### world/rooms/gate_portal.rs #####
//! Gate portals — phase state machine + registry.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.
##### world/rooms/loading_zone.rs #####
//! Loading zones — activation rules + readiness.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

########## ambition_input modules ##########
##### input/lib.rs #####
//! Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.
//!
//! Physical inputs are bound to `SandboxAction` with Leafwing Input Manager.
//! The engine-owned compact `ControlFrame` keeps movement physics independent
//! from keyboards, gamepads, UI rebinding, or replay input.
//!
##### input/actions.rs #####
//! The `SandboxAction` leafwing action enum — the logical-input vocabulary the
//! device-binding layer maps physical keys/sticks onto, before it is folded into
//! the device-agnostic `ControlFrame`/`MenuInputFrame`. Gated behind the `input`
//! feature (pulls in leafwing's `Actionlike`).
##### input/active_input.rs #####
//! Which input source is CURRENTLY active — the last one to produce
//! GENUINE input.
//!
//! This is a *marker*, not a mode switch. No source is ever disabled:
//! a player may steer with the arrow keys while clicking buttons with
//! the mouse (mouse-as-touchpad), or push a gamepad stick while tapping
##### input/control.rs #####
//! Device adapters that build the engine-owned `ControlFrame` resource.
//!
//! The pure, brain-facing [`ControlFrame`] vocabulary lives in
//! `ambition_engine_core`; this module is the input adapter that translates
//! Leafwing `SandboxAction`s, control settings, and trigger hysteresis into that
//! frame. Headless/replay/netcode callers can construct `ControlFrame` directly
##### input/menu.rs #####
//! Menu-side input vocabulary: the device-agnostic `MenuInputFrame` /
//! `MenuControlFrame` / `MenuInputState` resources and the `MenuDir` / `analog_to_dir`
//! helpers. The menu companion to `control` — keyboard/gamepad/touch fold into one
//! semantic UI intent here so menu systems never read leafwing or raw touch events.
##### input/motion_input.rs #####
//! Quarter-circle / half-circle motion-input recognition. Used by
//! the sandbox to upgrade a plain Fireball press into Hadouken /
//! HadoukenSuper when the player buffered the right gesture.
##### input/presets.rs #####
//! Default binding presets: the selectable keyboard layouts (`PresetId` /
//! `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad map
//! (`GAMEPAD_MAP`) that seed leafwing's input map for `SandboxAction`.
##### input/settings.rs #####
//! Controls / input settings.
//!
//! Holds controller deadzones, trigger thresholds, hysteresis, dash
//! input behavior, and which keyboard / controller profile is active.
//! The values flow into input filtering before the engine-owned `ControlFrame`
//! is built so gameplay sees clean edges instead of analog jitter.
Bash Find baked content and hardcoded IDs
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== include_str!/include_bytes! referencing content ==="
grep -rnE 'include_str!|include_bytes!' --include='*.rs' . | grep -vi test | head -40
echo
echo "=== hardcoded named boss/enemy string IDs (non-test) ==="
grep -rnoiE '"(clockwork_warden|gnu_?ton|goblin[a-z_]*|puppy_?slug|parrot|blorp|cultist|warden)"' --include='*.rs' . | grep -vi test | head -40

OUT
=== include_str!/include_bytes! referencing content ===
character_roster.rs:12:    CharacterCatalog::from_data(parse_catalog(include_str!(
boss_encounter/catalog.rs:676:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
boss_encounter/catalog.rs:677:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
boss_encounter/catalog.rs:678:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
boss_encounter/catalog.rs:679:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
boss_encounter/catalog.rs:680:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
boss_encounter/catalog.rs:681:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
boss_encounter/catalog.rs:682:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
boss_encounter/catalog.rs:683:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
boss_encounter/catalog.rs:684:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
boss_encounter/catalog.rs:714:            include_str!("../../../../game/ambition_content/assets/data/boss_profiles.ron"),
boss_encounter/catalog.rs:716:            include_str!("../../../../game/ambition_content/assets/data/boss_sheets.ron"),
boss_encounter/catalog.rs:734:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
boss_encounter/catalog.rs:735:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
boss_encounter/catalog.rs:736:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
boss_encounter/catalog.rs:737:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
boss_encounter/catalog.rs:738:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
boss_encounter/catalog.rs:739:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
boss_encounter/catalog.rs:740:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
boss_encounter/catalog.rs:741:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
boss_encounter/catalog.rs:742:            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
boss_encounter/catalog.rs:748:            include_str!("../../../../game/ambition_content/assets/data/boss_profiles.ron"),
boss_encounter/catalog.rs:750:            include_str!("../../../../game/ambition_content/assets/data/boss_sheets.ron"),
encounter/loading.rs:24:        ron::from_str(include_str!(
features/enemies/mod.rs:937:    CharacterRoster::from_ron(include_str!(
avatar/bundles.rs:298:            ambition_characters::actor::character_catalog::parse_catalog(include_str!(
session/data.rs:35:        ron::from_str(include_str!("../../assets/ambition/sandbox.ron"))
session/data.rs:67:        ron::from_str(include_str!(
session/data.rs:76:        ron::from_str(include_str!(

=== hardcoded named boss/enemy string IDs (non-test) ===
boss_encounter/systems.rs:430:"clockwork_warden"
boss_encounter/ids.rs:2:"clockwork_warden"
boss_encounter/ids.rs:9:"clockwork_warden"
boss_encounter/behavior.rs:359:"clockwork_warden"
boss_encounter/behavior.rs:406:"clockwork_warden"
boss_encounter/profile.rs:89:"clockwork_warden"
boss_encounter/profile.rs:176:"clockwork_warden"
boss_encounter/profile.rs:209:"clockwork_warden"
boss_encounter/catalog.rs:712:"clockwork_warden"
boss_encounter/catalog.rs:746:"clockwork_warden"
boss_encounter/catalog.rs:761:"clockwork_warden"
boss_encounter/catalog.rs:767:"clockwork_warden"
boss_encounter/catalog.rs:792:"clockwork_warden"
boss_encounter/catalog.rs:842:"clockwork_warden"
encounter/loading.rs:151:"goblin_encounter"
character_sprites/assets.rs:333:"goblin"
features/ecs/effect_bus.rs:94:"goblin_encounter"
features/ecs/effect_bus.rs:96:"goblin_encounter"
features/ecs/spawn_actors.rs:933:"puppy_slug"
features/enemies/mod.rs:308:"puppy_slug"
features/enemies/mod.rs:331:"puppy_slug"
abilities/thrown/puppy_slug_gun.rs:37:"puppy_slug"
Let me investigate combat's parallel paths and overlaps with actors/projectiles.

Read roadmap.md
Read decision-principles.md
The game/ directory holds content crates (ambition_content + demos mary_o, sanic, pocket). Now let me examine the catalog/equipment machinery in ambition_characters and confirm no embedded roster.

Now let me read the demo provider/app files to understand the required hooks and boilerplate, plus runtime/host maps.

Bash Find ranged/special/boss handling in combat moveset
IN
cd /home/joncrall/code/ambition
echo "=== moveset: ranged/special handling ==="
grep -rn "RangedActionSpec\|SpecialActionSpec\|MeleeActionSpec\|ActionRequest\|RANGED_VERB\|SPECIAL" crates/ambition_combat/src/moveset/ | grep -v tests.rs | head -40
echo
echo "=== 'special' / 'boss' occurrences in combat ==="
grep -rin "special\|boss" crates/ambition_combat/src --include=*.rs -l | grep -v tests

OUT
=== moveset: ranged/special handling ===
crates/ambition_combat/src/moveset/prefabs.rs:2://! turn authored specs (`MeleeActionSpec`/`RangedActionSpec`), tunable params
crates/ambition_combat/src/moveset/prefabs.rs:12:/// Convert an authored [`MeleeActionSpec`] into a data-driven `"attack"`
crates/ambition_combat/src/moveset/prefabs.rs:26:pub fn attack_move_from_melee(spec: &MeleeActionSpec) -> MoveSpec {
crates/ambition_combat/src/moveset/prefabs.rs:29:    // engine prefab (A2): the MeleeActionSpec timeline becomes prefab params.
crates/ambition_combat/src/moveset/prefabs.rs:203:/// Convert an authored [`RangedActionSpec`] into a data-driven `"ranged"`
crates/ambition_combat/src/moveset/prefabs.rs:214:pub fn fire_move_from_ranged(spec: &RangedActionSpec) -> MoveSpec {
crates/ambition_combat/src/moveset/prefabs.rs:218:        RangedActionSpec::Pistol { .. } => (0.08, 0.15),
crates/ambition_combat/src/moveset/prefabs.rs:219:        RangedActionSpec::Rock { .. } => (0.12, 0.18),
crates/ambition_combat/src/moveset/prefabs.rs:220:        RangedActionSpec::Bolt { .. } => (0.18, 0.20),
crates/ambition_combat/src/moveset/prefabs.rs:221:        RangedActionSpec::Arrow { .. } => (0.28, 0.22),
crates/ambition_combat/src/moveset/prefabs.rs:265:        id: RANGED_VERB.to_string(),
crates/ambition_combat/src/moveset/prefabs.rs:629:    melee: Option<&MeleeActionSpec>,
crates/ambition_combat/src/moveset/prefabs.rs:630:    ranged: Option<&RangedActionSpec>,
crates/ambition_combat/src/moveset/prefabs.rs:658:            .insert(RANGED_VERB.to_string(), fire.id.clone());
crates/ambition_combat/src/moveset/mod.rs:42:    ActionRequest, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
crates/ambition_combat/src/moveset/mod.rs:67:pub const RANGED_VERB: &str = "ranged";
crates/ambition_combat/src/moveset/mod.rs:608:            (moveset.0.move_for_verb(RANGED_VERB), &[RANGED_VERB])
crates/ambition_combat/src/moveset/mod.rs:771:                    request: ActionRequest::Special {
crates/ambition_combat/src/moveset/mod.rs:772:                        spec: SpecialActionSpec::Special(effect.key.clone()),
crates/ambition_combat/src/moveset/mod.rs:794:                        RANGED_VERB,
crates/ambition_combat/src/moveset/mod.rs:811:                    request: ActionRequest::Ranged {

=== 'special' / 'boss' occurrences in combat ===
crates/ambition_combat/src/hazards.rs
crates/ambition_combat/src/targeting.rs
crates/ambition_combat/src/falling_chest.rs
crates/ambition_combat/src/lib.rs
crates/ambition_combat/src/util.rs
crates/ambition_combat/src/events.rs
crates/ambition_combat/src/components/features.rs
crates/ambition_combat/src/components/mod.rs
crates/ambition_combat/src/components/actors.rs
crates/ambition_combat/src/hitbox/mod.rs
crates/ambition_combat/src/moveset/prefabs.rs
crates/ambition_combat/src/moveset/mod.rs
Read mod.rs
Read mod.rs
Read equipment.rs
Read provider.rs
Read provider.rs
Bash Module docs for gameplay_trace and asset_manager
IN
cd /home/joncrall/code/ambition
echo "########## ambition_gameplay_trace modules ##########"
for f in lib.rs actor_trace.rs buffer.rs dump.rs model.rs; do echo "##### trace/$f #####"; awk '/^\/\/!/{print; next} {c++} c>0 && !/^\/\/!/{exit}' "crates/ambition_gameplay_trace/src/$f" | head -7; done
echo; echo "########## ambition_asset_manager modules ##########"
for f in lib.rs id.rs kind.rs location.rs manifest.rs policy.rs preload.rs profile.rs resolver.rs bevy_integration.rs asset_publish/mod.rs sandbox_assets/mod.rs; do echo "##### am/$f #####"; awk '/^\/\/!/{print; next} {c++} c>0 && !/^\/\/!/{exit}' "crates/ambition_asset_manager/src/$f" | head -6; done

OUT
########## ambition_gameplay_trace modules ##########
##### trace/lib.rs #####
//! Gameplay flight-recorder format — the reusable, content-free core of the
//! trace recorder.
//!
//! A rolling ring buffer of per-frame player snapshots ([`GameplayTraceFrame`])
//! and discrete gameplay events ([`GameplayTraceEvent`]), plus the markdown/JSON
//! [`write_dump`] writers. The game's simulation systems (the recorder /
//! OOB-detector, which read live player + world state) fill the buffer; this
##### trace/actor_trace.rs #####
//! Non-player-centric body trace: a rolling timeline of EVERY simulated
//! body's kinematic state (player, boss, enemy, NPC — no privileged
//! observer) plus a per-body out-of-bounds classifier and a dump-on-OOB
//! writer.
//!
//! This is the relativity-respecting sibling of [`GameplayTraceBuffer`].
//! That recorder captures the rich, input-driven PLAYER feel timeline
##### trace/buffer.rs #####
//! The `GameplayTraceBuffer` resource: a rolling ring buffer of per-frame
//! snapshots and discrete events that the game's recorder systems push into. Owns
//! capacity/tick/sequence bookkeeping; the dump writers live in `dump`.
##### trace/dump.rs #####
//! Dump writers: serialize a `GameplayTraceBuffer` to a timestamped markdown +
//! JSON pair (`write_dump`, path/label helpers). The markdown is a human-readable
//! tail summary; the JSON is the full payload the replay harness reads back.
##### trace/model.rs #####
//! Serializable trace data shapes: the per-frame `GameplayTraceFrame` (player +
//! platform + control state) and the discrete `GameplayTraceEvent` / `DumpReason`
//! / `OobReason` enums, plus serde-friendly geometry mirrors (`TracePoint`,
//! `TraceAabb`) that avoid leaking `bevy_math`/engine types into the JSON shape.

########## ambition_asset_manager modules ##########
##### am/lib.rs #####
//! Ambition asset catalog + source/profile policy.
//!
//! This crate owns **Ambition's logical asset layer**: stable [`AssetId`]s,
//! a [`manifest`] of [`AssetEntry`] records, [`profile::AssetProfile`]
//! personas, [`policy::MissingAssetPolicy`] / [`policy::CachePolicy`]
//! rules, [`preload::PreloadGroup`] tagging, and a [`resolver`] that
##### am/id.rs #####
//! [`AssetId`] — stable logical identifier for an asset entry.
//!
//! The wire form is a dotted lowercase string, e.g.
//! `sprite.entity.chest_closed`, `world.sandbox_ldtk`, or
//! `audio.sfx_bank`. The string is the canonical id: equality, hashing,
//! manifest lookup, and serialization all key off the string.
##### am/kind.rs #####
//! [`AssetKind`] — coarse Ambition-side classification of an asset.
//!
//! `AssetKind` is the *type the catalog thinks the asset is*, not the
//! Rust type Bevy will hand back. Bevy keeps its own typed
//! `AssetServer::load::<T: Asset>` API — the catalog stores the kind so
//! resolver code can pick reasonable defaults (cache policy, fallback
##### am/location.rs #####
//! [`AssetLocation`] — where the bytes for a logical [`crate::AssetId`]
//! live for a given [`crate::profile::AssetProfile`].
//!
//! The resolver picks one variant per `(id, profile)` pair. Bevy
//! consumers only ever see the string returned by [`AssetLocation::bevy_asset_path`];
//! non-Bevy byte adapters (SFX bank, manifest preload) consult the
##### am/manifest.rs #####
//! [`AssetManifest`] — the catalog of [`AssetEntry`] records.
//!
//! Each entry binds one [`crate::AssetId`] to:
//! - an [`crate::kind::AssetKind`],
//! - a *logical* path (the canonical relative path used by the loose-fs
//!   profile and as a default for embedded / installed sources),
##### am/policy.rs #####
//! Policy enums for handling missing assets and caching.
//!
//! These are decided per-[`crate::manifest::AssetEntry`], not per-profile.
//! The active [`crate::profile::AssetProfile`] influences whether a
//! location resolves at all; this layer governs what to do *after* a
//! resolution failure ("the location returned `Disabled` / the bytes
##### am/preload.rs #####
//! [`PreloadGroup`] — coarse "load-this-set-up-front" tag.
//!
//! Asset entries declare which group they belong to (or `None` for
//! lazy/on-demand). The resolver groups entries by tag so the consumer
//! can drive preloads with [`bevy_asset_loader`] or a hand-rolled state
//! machine. The catalog itself does not block on loads.
##### am/profile.rs #####
//! [`AssetProfile`] — the active platform/runtime persona that drives
//! which [`crate::location::AssetLocation`] the resolver returns for a
//! given [`crate::AssetId`].
//!
//! The profile is set once per app session (typically from a CLI flag or
//! compile-time `cfg`) and lives behind the
##### am/resolver.rs #####
//! Resolve `(AssetId, AssetProfile) -> ResolvedAsset`.
//!
//! The resolver is the seam between Ambition's logical catalog
//! ([`crate::manifest`]) and the runtime asset backend. Bevy callers feed
//! resolved locations to `AssetServer`; non-Bevy consumers feed them to their
//! owning subsystem's byte/provider loader.
##### am/bevy_integration.rs #####
//! Bevy plugin / resource / helper layer.
//!
//! Wraps [`crate::manifest::AssetManifest`] + [`crate::profile::AssetProfile`]
//! in two Bevy resources and adds load helpers that route through
//! Bevy's `AssetServer` and `AssetPath` machinery. The crate does NOT
//! re-implement async loading, handles, dependencies, or hot reload —
##### am/asset_publish/mod.rs #####
//! Publish/install boundary for generated sprite assets.
//!
//! Generators (the `ambition_sprite2d_renderer` toolchain and the quality-
//! variant scripts) emit many kinds of file: runtime sheet records and page
//! images, transitional actor sidecars, throwaway YAML/JSON intermediates, and
//! human-only diagnostics (canonical poses, labeled previews, debug overlays).
##### am/sandbox_assets/mod.rs #####
//! Sandbox-side aggregator for the [`ambition_asset_manager`] catalog.
//!
//! This module builds the single [`SandboxAssetCatalog`] resource used by visible
//! sandbox systems to resolve Bevy asset paths: sprites, parallax, character and
//! boss sheets, fonts, LDtk world/data, SFX bank, and music tracks.
//!
Bash runtime, host, policy maps
IN
echo "=== runtime map ==="; find crates/ambition_runtime/src -type f -name "*.rs" | xargs wc -l | sort -rn | head -40; echo; echo "=== host map ==="; find crates/ambition_host/src -type f -name "*.rs" | xargs wc -l | sort -rn; echo; echo "=== policy tests map ==="; find tests/ambition_workspace_policy -type f | xargs wc -l 2>/dev/null | sort -rn | head -40

OUT
=== runtime map ===
  7702 total
  1912 crates/ambition_runtime/src/snapshot/tests.rs
  1402 crates/ambition_runtime/src/snapshot/codecs.rs
  1192 crates/ambition_runtime/src/snapshot/mod.rs
   709 crates/ambition_runtime/src/snapshot/registry.rs
   492 crates/ambition_runtime/src/snapshot/restore.rs
   368 crates/ambition_runtime/src/snapshot/motion_codec.rs
   339 crates/ambition_runtime/src/lib.rs
   285 crates/ambition_runtime/src/combat_schedule.rs
   222 crates/ambition_runtime/src/player_schedule.rs
   154 crates/ambition_runtime/src/sim_core_resources.rs
   137 crates/ambition_runtime/src/input_stream.rs
   131 crates/ambition_runtime/src/progression_schedule.rs
   114 crates/ambition_runtime/src/mode_scope.rs
   101 crates/ambition_runtime/src/portal_schedule.rs
    80 crates/ambition_runtime/src/session_world.rs
    49 crates/ambition_runtime/src/room_schedule.rs
    15 crates/ambition_runtime/src/projectile_schedule.rs

=== host map ===
  997 total
  743 crates/ambition_host/src/portal.rs
  254 crates/ambition_host/src/lib.rs

=== policy tests map ===
  7418 total
  2508 tests/ambition_workspace_policy/policies/engine.toml
   690 tests/ambition_workspace_policy/tests/policy.rs
   560 tests/ambition_workspace_policy/policies/game.toml
   526 tests/ambition_workspace_policy/src/custom/determinism.rs
   451 tests/ambition_workspace_policy/src/custom/control_frame.rs
   351 tests/ambition_workspace_policy/migration_matrix.toml
   305 tests/ambition_workspace_policy/src/workspace.rs
   299 tests/ambition_workspace_policy/src/model.rs
   224 tests/ambition_workspace_policy/src/custom/module_size.rs
   210 tests/ambition_workspace_policy/src/custom/session_world.rs
   193 tests/ambition_workspace_policy/src/custom/migration_matrix.rs
   131 tests/ambition_workspace_policy/policies/control_frame.toml
   102 tests/ambition_workspace_policy/src/custom/lifecycle.rs
   101 tests/ambition_workspace_policy/src/rules/dependency.rs
    99 tests/ambition_workspace_policy/policies/determinism.toml
    94 tests/ambition_workspace_policy/src/rules/source_reference.rs
    79 tests/ambition_workspace_policy/src/custom/content_ownership.rs
    71 tests/ambition_workspace_policy/fixtures/architecture_boundaries_source_tests.txt
    55 tests/ambition_workspace_policy/src/rules/file_content.rs
    48 tests/ambition_workspace_policy/src/lib.rs
    41 tests/ambition_workspace_policy/policies/repository.toml
    36 tests/ambition_workspace_policy/src/custom/mod.rs
    34 tests/ambition_workspace_policy/src/rules/paths.rs
    34 tests/ambition_workspace_policy/Cargo.toml
    24 tests/ambition_workspace_policy/src/rules/workspace_member.rs
    22 tests/ambition_workspace_policy/fixtures/poison/legacy/legacy_runtime.rs
    21 tests/ambition_workspace_policy/src/runner.rs
    21 tests/ambition_workspace_policy/policies/module_size.toml
    21 tests/ambition_workspace_policy/fixtures/poison/manifest.toml
    20 tests/ambition_workspace_policy/fixtures/poison/src/leak.rs
    14 tests/ambition_workspace_policy/fixtures/poison/pose_writes/bare_pose_write.rs
    13 tests/ambition_workspace_policy/fixtures/poison/frame_reads/reconstructed_frame.rs
    10 tests/ambition_workspace_policy/src/rules/mod.rs
    10 tests/ambition_workspace_policy/fixtures/poison/motion_model/optional_model.rs
Let me read the moveset runtime's action-dispatch section and the events/on_hit files to understand parallel paths.

Bash Sizes of decomposition-relevant docs
IN
wc -l docs/planning/engine/decomposition.md docs/planning/engine/refactor-chain.md docs/planning/engine/architecture.md docs/planning/tracks.md

OUT
  564 docs/planning/engine/decomposition.md
  106 docs/planning/engine/refactor-chain.md
  260 docs/planning/engine/architecture.md
  191 docs/planning/tracks.md
 1121 total
Read mod.rs (lines 561-860)
Let me read the content-baking section and several more subsystem entry points.

Read catalog.rs (lines 656-815)
Read mod.rs (lines 291-350)
ambition_characters is confirmed machinery. Now let me pivot to ambition_sprite_sheet's suspicious dependencies.

Read refactor-chain.md
Let me examine the reference-frame/relativity theme, the sandbox_assets content, and the movement modules that had no doc header.

Read architecture.md
Now let me examine ambition_actors (boss_encounter, brain paths) and the projectiles crate to find overlaps and parallel paths.

Bash ambition_sprite_sheet Cargo + tree
IN
cd /home/joncrall/code/ambition/crates/ambition_sprite_sheet && echo "=== Cargo.toml ===" && cat Cargo.toml && echo "" && echo "=== FILE TREE + LOC ===" && find . -name '*.rs' -not -path '*/target/*' | xargs wc -l | sort -rn | head -40 && echo "" && echo "=== non-rs files ===" && find . -type f -not -name '*.rs' -not -path '*/target/*' | head -30

OUT
=== Cargo.toml ===
[package]
name = "ambition_sprite_sheet"
version = "0.1.0"
edition.workspace = true
license.workspace = true
authors.workspace = true
description = "Reusable, content-free sprite-sheet metadata vocabulary: the SheetRecord / AnimationMetrics / PixelRect / FrameRect schema + the SheetRegistry resource that parses a baked `(filename_root, ron_text)` table. The host game owns where its sprite assets live and passes the baked table in. Extracted from ambition_actors."

[lib]
crate-type = ["rlib"]

[features]
default = []
# Select WebServedAssets as the wasm default in GameAssetConfig. Forwarded by
# the app/actor composition features that serve loose assets over HTTP.
web_served = []
# Enable authored embedded-core image candidates in the game-asset manifest.
static_core_assets = ["ambition_asset_manager/static_core_assets"]

[dependencies]
# ECS surface only (Resource / Component / App via prelude) + logging macros.
# No render/audio/windowing — this crate stays reusable and headless.
bevy = { version = "0.18.1", default-features = false, features = [
    "2d_bevy_render",
] }

# RON manifest schema + parsing of the baked sheet table.
serde = { version = "1", features = ["derive"] }
ron = "0.11"
# The ultrapack SpritePackCatalog is emitted as JSON by the Python packer
# (drift-free interchange — Python-authored RON parses looser than Rust's ron).
serde_json = "1"


# Render-facing asset resource/model helpers moved below ambition_render.
ambition_asset_manager = { path = "../ambition_asset_manager", features = ["bevy"] }
ambition_combat = { path = "../ambition_combat" }
ambition_engine_core = { path = "../ambition_engine_core" }
ambition_entity_catalog = { path = "../ambition_entity_catalog" }
ambition_interaction = { path = "../ambition_interaction" }
ambition_persistence = { path = "../ambition_persistence" }
ambition_world = { path = "../ambition_world" }

# Logging facade (bevy logs through tracing); used for the load diagnostics.
tracing = "0.1"

=== FILE TREE + LOC ===
  5989 total
  1269 ./src/boss.rs
   566 ./src/character/sheets/mod.rs
   529 ./src/pack.rs
   515 ./src/lib.rs
   468 ./src/game_assets/mod.rs
   363 ./src/character/anim.rs
   358 ./src/game_assets/entity_sprite.rs
   280 ./src/frames.rs
   278 ./src/tests.rs
   250 ./src/game_assets/resolvers.rs
   192 ./src/sprite_packs.rs
   185 ./src/frames/tests.rs
   149 ./build.rs
   143 ./src/character/sheets/atlas.rs
   142 ./src/character/animator.rs
    89 ./src/character/sheets/geometry.rs
    84 ./src/character/sheets/tests.rs
    65 ./src/character/assets.rs
    54 ./src/character/mod.rs
    10 ./src/baked_sheet_rons.rs

=== non-rs files ===
./MODULES.md
./Cargo.toml
Let me verify whether the flagged types (Health, WornCharacter, FeatureView, DialogState) are sim components read directly vs read-models. Let me also read render's lib.rs and check sim_view's public API.

Bash Map ambition_actors and boss_encounter
IN
cd /home/joncrall/code/ambition
echo "=== ambition_actors module tree (top-level dirs + LOC) ==="
find crates/ambition_actors/src -maxdepth 2 -name '*.rs' | while read f; do lc=$(wc -l < "$f"); printf "%6d  %s\n" "$lc" "${f#crates/ambition_actors/}"; done | sort -rn | head -50
echo
echo "=== boss_encounter dir ==="
find crates/ambition_actors/src/boss_encounter -name '*.rs' 2>/dev/null | while read f; do lc=$(wc -l < "$f"); printf "%6d  %s\n" "$lc" "${f#crates/ambition_actors/}"; done | sort -rn
echo
echo "=== ambition_actors total LOC ==="
find crates/ambition_actors/src -name '*.rs' | xargs wc -l | tail -1

OUT
=== ambition_actors module tree (top-level dirs + LOC) ===
  1049  src/avatar/trail.rs
   893  src/boss_encounter/catalog.rs
   854  src/features/actor_movement_tests.rs
   832  src/projectile/systems.rs
   717  src/character_sprites/assets.rs
   705  src/encounter/tests.rs
   667  src/boss_encounter/behavior.rs
   587  src/boss_encounter/systems.rs
   515  src/dialog/yarn_bindings.rs
   509  src/features/mod.rs
   495  src/character_sprites/tests.rs
   471  src/encounter/systems.rs
   468  src/audio/tests.rs
   440  src/avatar/starting_character.rs
   412  src/world/physics.rs
   402  src/schedule/input_systems.rs
   400  src/features/movement_fx.rs
   391  src/affordances/devices.rs
   379  src/avatar/bundles.rs
   378  src/control/multiplayer_smoke_tests.rs
   373  src/affordances/variants.rs
   357  src/character_roster/tests.rs
   337  src/audio/plugin.rs
   305  src/lib.rs
   279  src/features/npcs.rs
   278  src/affordances/tests.rs
   274  src/items/world_item.rs
   267  src/control/input_systems.rs
   257  src/gravity/resolve.rs
   256  src/boss_encounter/encounter_entity.rs
   252  src/affordances/intent.rs
   251  src/avatar/body_integration.rs
   238  src/character_sprites/attack_hitbox.rs
   238  src/audio/environment.rs
   232  src/safe_pos_tests.rs
   227  src/affordances/mod.rs
   226  src/features/bosses.rs
   220  src/boss_encounter/encounter_script.rs
   211  src/music/tests.rs
   211  src/boss_encounter/profile.rs
   211  src/avatar/systems.rs
   206  src/music/intent.rs
   201  src/time/feel.rs
   199  src/session/setup.rs
   197  src/cutscene.rs
   189  src/encounter/loading.rs
   175  src/control/components.rs
   172  src/actor.rs
   170  src/encounter/switches.rs
   167  src/avatar/clone_probe_tests.rs

=== boss_encounter dir ===
   964  src/boss_encounter/attack_geometry/sprite_metadata_derivation_tests.rs
   893  src/boss_encounter/catalog.rs
   667  src/boss_encounter/behavior.rs
   590  src/boss_encounter/attack_geometry/mod.rs
   587  src/boss_encounter/systems.rs
   477  src/boss_encounter/sprites/tests.rs
   256  src/boss_encounter/encounter_entity.rs
   225  src/boss_encounter/encounter_script/tests.rs
   220  src/boss_encounter/encounter_script.rs
   211  src/boss_encounter/profile.rs
   173  src/boss_encounter/attack_geometry/strike_geometry_data_tests.rs
   161  src/boss_encounter/attack_geometry/frame.rs
   154  src/boss_encounter/encounter_entity/tests.rs
   150  src/boss_encounter/attack_geometry/aabb.rs
   111  src/boss_encounter/mod.rs
    75  src/boss_encounter/tests.rs
    51  src/boss_encounter/roster.rs
    46  src/boss_encounter/events.rs
    45  src/boss_encounter/specs.rs
    38  src/boss_encounter/attack_geometry/simple_geometry_tests.rs
    32  src/boss_encounter/registry.rs
    30  src/boss_encounter/ids.rs
    12  src/boss_encounter/sprites/mod.rs
     5  src/boss_encounter/rewards.rs

=== ambition_actors total LOC ===
  68709 total
Bash Reference-frame theme spread
IN
cd /home/joncrall/code/ambition
echo "=== reference-frame / relativity theme across target crates ==="
grep -rniE 'proper.?time|reference.?frame|gravity.?relative|motion.?authorit|AccelerationFrame|MotionFrame|cognitive.?time|relativi|redshift|time.?dilation' crates/ambition_engine_core/src crates/ambition_platformer_primitives/src crates/ambition_time/src crates/ambition_world/src crates/ambition_input/src crates/ambition_gameplay_trace/src --include='*.rs' -l
echo
echo "=== count of AccelerationFrame/MotionFrame/reference_frame refs by file (engine_core) ==="
grep -rncE 'AccelerationFrame|MotionFrame|ReferenceFrame|reference_frame|LocalAxes|WorldVec2|MotionAuthority|carry_body|transit_body' crates/ambition_engine_core/src --include='*.rs' | grep -v ':0' | sort -t: -k2 -rn | head -30

OUT
=== reference-frame / relativity theme across target crates ===
crates/ambition_engine_core/src/test_support.rs
crates/ambition_engine_core/src/geometry.rs
crates/ambition_engine_core/src/player_state.rs
crates/ambition_engine_core/src/body_clusters.rs
crates/ambition_engine_core/src/control_frame.rs
crates/ambition_engine_core/src/volume_shape.rs
crates/ambition_engine_core/src/reference_frame.rs
crates/ambition_engine_core/src/lib.rs
crates/ambition_engine_core/src/collision_semantics.rs
crates/ambition_engine_core/src/movement/blink.rs
crates/ambition_engine_core/src/movement/integration.rs
crates/ambition_engine_core/src/movement/control.rs
crates/ambition_engine_core/src/movement/input.rs
crates/ambition_engine_core/src/movement/authority.rs
crates/ambition_engine_core/src/movement/collision.rs
crates/ambition_engine_core/src/movement/tuning.rs
crates/ambition_engine_core/src/movement/mod.rs
crates/ambition_engine_core/src/movement/simulation.rs
crates/ambition_engine_core/src/movement/surface_momentum/tests.rs
crates/ambition_engine_core/src/movement/adhesive_crawler.rs
crates/ambition_engine_core/src/movement/kernel.rs
crates/ambition_engine_core/src/movement/kernel/tests.rs
crates/ambition_engine_core/src/movement/abilities.rs
crates/ambition_engine_core/src/ledge_grab/runtime.rs
crates/ambition_engine_core/src/movement/surface_momentum/mod.rs
crates/ambition_engine_core/src/movement/tests/c4_reaction_seams.rs
crates/ambition_engine_core/src/reference_frame/tests.rs
crates/ambition_engine_core/src/ledge_grab/mod.rs
crates/ambition_engine_core/src/collision_semantics/tests.rs
crates/ambition_platformer_primitives/src/frame_env.rs
crates/ambition_engine_core/src/movement/tests/wall_collision.rs
crates/ambition_platformer_primitives/src/gravity.rs
crates/ambition_platformer_primitives/src/projectile/body.rs
crates/ambition_platformer_primitives/src/kinematic/tests.rs
crates/ambition_world/src/collision.rs
crates/ambition_time/src/lib.rs
crates/ambition_platformer_primitives/src/projectile/collision/tests.rs
crates/ambition_world/src/platforms/mod.rs
crates/ambition_world/src/platforms/tests.rs
crates/ambition_world/src/rooms/camera.rs
crates/ambition_gameplay_trace/src/actor_trace.rs

=== count of AccelerationFrame/MotionFrame/reference_frame refs by file (engine_core) ===
crates/ambition_engine_core/src/movement/surface_momentum/tests.rs:59
crates/ambition_engine_core/src/reference_frame.rs:43
crates/ambition_engine_core/src/reference_frame/tests.rs:27
crates/ambition_engine_core/src/movement/kernel/tests.rs:23
crates/ambition_engine_core/src/movement/tests/wall_collision.rs:16
crates/ambition_engine_core/src/ledge_grab/tests.rs:13
crates/ambition_engine_core/src/movement/surface_momentum/mod.rs:11
crates/ambition_engine_core/src/movement/input.rs:11
crates/ambition_engine_core/src/movement/authority.rs:11
crates/ambition_engine_core/src/movement/tests/c4_reaction_seams.rs:10
crates/ambition_engine_core/src/movement/integration.rs:10
crates/ambition_engine_core/src/movement/tests/glide_and_air.rs:8
crates/ambition_engine_core/src/movement/tests/blink.rs:8
crates/ambition_engine_core/src/ledge_grab/runtime.rs:8
crates/ambition_engine_core/src/movement/mod.rs:7
crates/ambition_engine_core/src/movement/tests/climbing.rs:6
crates/ambition_engine_core/src/movement/kernel.rs:5
crates/ambition_engine_core/src/movement/adhesive_crawler.rs:5
crates/ambition_engine_core/src/lib.rs:5
crates/ambition_engine_core/src/movement/abilities.rs:4
crates/ambition_engine_core/src/test_support.rs:3
crates/ambition_engine_core/src/movement/control.rs:3
crates/ambition_engine_core/src/body_clusters.rs:3
crates/ambition_engine_core/src/volume_shape.rs:2
crates/ambition_engine_core/src/player_state.rs:2
crates/ambition_engine_core/src/movement/tuning.rs:2
crates/ambition_engine_core/src/movement/tests/ledge_grab.rs:2
crates/ambition_engine_core/src/movement/simulation.rs:2
crates/ambition_engine_core/src/movement/blink.rs:2
crates/ambition_engine_core/src/movement/tests/sweep_sample.rs:1
Bash Read render lib.rs and rendering mod head
IN
echo "=== render lib.rs ===" && cat crates/ambition_render/src/lib.rs && echo "=== render rendering/mod.rs head ===" && head -70 crates/ambition_render/src/rendering/mod.rs

OUT
=== render lib.rs ===
//! Ambition's Bevy presentation layer — the sandbox's default renderer.
//!
//! Everything here is downstream of the sim: no module on the gameplay critical
//! path lives in this crate. It reads lower read-model crates (`ambition_sim_view`, `ambition_world`,
//! `ambition_sprite_sheet`, `ambition_platformer_primitives`) and mirrors them
//! into Bevy sprites / UI; it never mutates the sim. The sim/render seam is now
//! a CRATE boundary in both directions: render does not depend on
//! actor machinery, and actor machinery cannot import render (enforced by
//! `architecture_boundaries`).
//!
//! Modules are migrated here incrementally from the old
//! the old actor-side presentation umbrella; consumers (content, app) import
//! `ambition_render::*` directly.

pub mod cutscene;
/// The dialog-box overlay UI. Render-only; reads the reusable dialog state in
/// `ambition_dialog`.
pub mod dialog_ui;
pub mod fx;
/// The in-world HUD overlay: health/mana bars, ability pips, banner text.
pub mod hud;
/// The presentation face a demo can add (oracle-violation OV1). See its module docs.
pub mod platformer_presentation;
pub mod quality;
pub mod rendering;
pub mod screen_effects;
pub mod ui_fonts;
=== render rendering/mod.rs head ===
//! Bevy visual synchronization for engine state.
//!
//! Render-only component tags and visual sync systems. This module mirrors
//! player and world state from ECS components into Bevy transforms / sprites.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`primitives`] — marker components ([`SceneEntities`],
//!   [`PlayerVisual`], [`HudText`], [`QuestPanelText`], [`RoomVisual`],
//!   [`FeatureVisual`], [`HealthOverlayVisual`]) plus color / Z /
//!   feature-kind helpers and `spawn_world_label`.
//! - [`actors`] — per-frame sync of player + enemy + boss sprites
//!   and animation. Owns [`sync_visuals`], [`animate_player`],
//!   [`animate_characters`], [`animate_bosses`], [`upgrade_actor_sprites`],
//!   [`upgrade_boss_sprites`].
//! - [`world`] — static room visuals. Owns [`spawn_room_visuals`],
//!   [`spawn_block`], [`spawn_loading_zone`], [`spawn_grid`],
//!   [`spawn_room_object`].
//! - [`features`] — runtime-spawned feature visuals via
//!   [`spawn_dynamic_feature_visuals`].
//! - [`health`] — debug health-bar overlay
//!   ([`sync_health_overlays`]).
//! - [`nameplates`] — player-facing actor/door labels
//!   ([`sync_actor_nameplates`]).
//! - [`parallax`] — optional generated sky/background/atmosphere layers
//!   ([`spawn_parallax_layers`], [`sync_parallax_layers`]).
//! - [`camera`] — player-following camera with eased zoom around
//!   encounter transitions ([`camera_follow`]).

pub mod actors;
pub mod bubble_shield;
mod camera;
mod deep_dream;
mod features;
pub mod gate_portal_visuals;
pub mod gravity_visuals;
mod health;
mod hit_flash;
mod item_visuals;
pub mod mark_beacon;
pub mod morph_ball;
mod nameplates;
mod parallax;
mod pirate_weapon;
mod primitives;
pub mod projectile_visuals;
pub(crate) mod sheet_atlas;
pub mod shrine_visuals;
pub(crate) mod slash_visuals;
mod world;

pub use actors::{
    actor_sprite_path_owns, animate_bosses, animate_characters, animate_player,
    apply_hide_sprites_override, apply_placeholder_sprites_override,
    refresh_player_sprites_on_game_assets_change, refresh_prop_sprites_on_game_assets_change,
    sync_visuals, upgrade_actor_sprites, upgrade_boss_sprites, PlayerSpriteCharacter,
};
// `BoundFeatureKind` lives with the combat event vocabulary; re-exported here
// so existing render call sites resolve unchanged.
pub use ambition_combat::events::BoundFeatureKind;
// `manage_gradient_lane_visual` + `GradientLaneVisual` stay
// module-private; the schedule registration uses
// `actors::manage_gradient_lane_visual` directly so no outside
// callers need a re-export.
pub use ambition_sim_view::camera_snapshot::{CameraSnapshot2d, SceneCaptureRequest};
#[cfg(feature = "portal_render")]
pub use camera::publish_portal_camera_clamp;
pub use camera::{camera_follow, publish_camera_viewport, CameraViewState};
pub use health::{sync_boss_health_bar_overlay, sync_health_overlays};
pub use nameplates::{
A 1269-LOC boss.rs in a "content-free" sprite crate is a red flag. Let me grep the actual dependency use-sites and inspect boss.rs and build.rs.

Bash Check if characters types are Components
IN
echo "=== characters actor: Health / WornCharacter / BodyCombat definitions ===" && grep -rn "pub struct Health\b\|pub struct WornCharacter\|pub struct BodyCombat\|#\[derive.*Component" crates/ambition_characters/src/actor.rs | head -40

OUT
=== characters actor: Health / WornCharacter / BodyCombat definitions ===
ugrep: warning: crates/ambition_characters/src/actor.rs: No such file or directory
The provider boilerplate is nearly verbatim between demos. Let me read the standalone app boilerplate, pocket, runtime/host libs, and how the host wires providers.

Let me check the non-test hardcoded IDs and the enemy roster include.

Bash Public API of undocumented movement modules
IN
cd /home/joncrall/code/ambition
echo "=== movement modules without doc header - first 3 non-empty lines ==="
for f in simulation.rs control.rs input.rs ops.rs events.rs integration.rs collision.rs blink.rs; do echo "##### movement/$f #####"; grep -nE 'pub (fn|struct|enum|trait|const)' crates/ambition_engine_core/src/movement/$f | head -5; done

OUT
=== movement modules without doc header - first 3 non-empty lines ===
##### movement/simulation.rs #####
18:pub fn handle_jump_buffer_clusters(
##### movement/control.rs #####
16:pub fn handle_blink_clusters(
130:pub fn handle_attacks_clusters(
##### movement/input.rs #####
18:pub struct InputState {
74:    pub const fn local_axis(self) -> Vec2 {
79:    pub const fn with_axes(x: f32, y: f32) -> Self {
##### movement/ops.rs #####
8:pub enum MovementOp {
36:    pub fn symbol(self) -> &'static str {
65:    pub fn name(self) -> &'static str {
103:pub struct ComboMark {
##### movement/events.rs #####
7:pub struct BlinkEvent {
15:pub struct FrameEvents {
31:    pub fn op_clusters(
48:    pub fn extend(&mut self, other: FrameEvents) {
##### movement/integration.rs #####
28:pub fn set_jump_velocity(vel: &mut crate::Vec2, gravity_dir: crate::Vec2, speed: f32) {
45:pub fn gravity_descend(axis_y: f32, gravity_dir: crate::Vec2) -> f32 {
337:pub struct NormalSpineCtx {
351:    pub fn bare(on_ground: bool) -> Self {
369:pub fn integrate_normal_spine(
##### movement/collision.rs #####
487:pub fn standing_on_one_way_aabb(world: &World, body: Aabb, gravity_dir: Vec2) -> bool {
497:pub fn touching_hazard_aabb(world: &World, aabb: crate::Aabb) -> bool {
505:pub fn touching_rebound_aabb(world: &World, aabb: crate::Aabb) -> Option<Vec2> {
##### movement/blink.rs #####
17:pub fn complete_blink_clusters(
81:pub fn blink_destination_clusters(
99:pub fn blink_destination_to_point_clusters(
Read ids.rs
Bash Grep all ambition_ use-sites in sprite_sheet
IN
cd /home/joncrall/code/ambition/crates/ambition_sprite_sheet && echo "=== use ambition_ sites ===" && grep -rn "ambition_combat\|ambition_interaction\|ambition_persistence\|ambition_world\|ambition_entity_catalog\|ambition_engine_core\|ambition_asset_manager" --include=*.rs src build.rs | grep -v "^.*://" | head -80

OUT
=== use ambition_ sites ===
src/boss.rs:17:use ambition_persistence::settings::VisualQualityBudget;
src/boss.rs:843:    catalog: &ambition_asset_manager::sandbox_assets::SandboxAssetCatalog,
src/boss.rs:910:    catalog: &ambition_asset_manager::sandbox_assets::SandboxAssetCatalog,
src/boss.rs:917:    let id = ambition_asset_manager::sandbox_assets::ids::boss_sprite(label);
src/boss.rs:927:        .filter(|scale| *scale != ambition_persistence::settings::TextureResolutionScale::Full)
src/boss.rs:929:            let variant_id = ambition_asset_manager::sandbox_assets::scaled_asset_id(
src/game_assets/entity_sprite.rs:7:use ambition_asset_manager::{
src/game_assets/entity_sprite.rs:13:use ambition_persistence::settings::TextureResolutionScale;
src/game_assets/entity_sprite.rs:226:                ambition_asset_manager::AssetSourceProfile::EmbeddedBinary,
src/game_assets/entity_sprite.rs:227:                ambition_asset_manager::AssetLocation::embedded(embedded_url.to_string()),
src/game_assets/entity_sprite.rs:284:        ambition_asset_manager::sandbox_assets::scaled_asset_id(base_id, scale.asset_id_suffix())
src/game_assets/entity_sprite.rs:307:    use ambition_asset_manager::sandbox_assets::embedded_core;
src/game_assets/resolvers.rs:4:use ambition_engine_core as ae;
src/game_assets/resolvers.rs:8:use ambition_combat::events::FeatureVisualKind;
src/game_assets/resolvers.rs:9:use ambition_world::rooms::LoadingZoneActivation;
src/game_assets/resolvers.rs:51:    _volume: &ambition_world::rooms::HazardVolumeSpec,
src/game_assets/resolvers.rs:57:    pickup: &ambition_world::rooms::PickupSpec,
src/game_assets/resolvers.rs:62:pub fn entity_sprite_for_chest(_chest: &ambition_world::rooms::ChestSpec) -> Option<EntitySprite> {
src/game_assets/resolvers.rs:71:    _chest: &ambition_interaction::Chest,
src/game_assets/resolvers.rs:77:    _breakable: &ambition_world::rooms::BreakableSpec,
src/game_assets/resolvers.rs:83:    interactable: &ambition_world::rooms::InteractableSpec,
src/game_assets/resolvers.rs:87:        ambition_world::rooms::InteractionKindSpec::Npc { .. }
src/game_assets/resolvers.rs:100:    interactable: &ambition_interaction::Interactable,
src/game_assets/resolvers.rs:104:        ambition_interaction::InteractionKind::Npc { .. }
src/game_assets/resolvers.rs:113:    brain: &ambition_entity_catalog::placements::CharacterBrain,
src/game_assets/resolvers.rs:127:fn character_brain_is_sandbag(brain: &ambition_entity_catalog::placements::CharacterBrain) -> bool {
src/game_assets/resolvers.rs:130:        ambition_entity_catalog::placements::CharacterBrain::Custom(key)
src/game_assets/resolvers.rs:136:    _brain: &ambition_entity_catalog::placements::BossBrain,
src/game_assets/resolvers.rs:141:fn pickup_sprite(kind: &ambition_world::rooms::PickupKindSpec) -> EntitySprite {
src/game_assets/resolvers.rs:143:        ambition_world::rooms::PickupKindSpec::Health { .. } => EntitySprite::PickupHealth,
src/game_assets/resolvers.rs:144:        ambition_world::rooms::PickupKindSpec::Currency { .. } => EntitySprite::PickupCurrency,
src/game_assets/resolvers.rs:145:        ambition_world::rooms::PickupKindSpec::Ability { .. } => EntitySprite::PickupAbility,
src/game_assets/resolvers.rs:153:pub fn breakable_state_sprite(state: ambition_interaction::BreakableState) -> EntitySprite {
src/game_assets/resolvers.rs:155:        ambition_interaction::BreakableState::Intact => EntitySprite::BreakableIntact,
src/game_assets/resolvers.rs:156:        ambition_interaction::BreakableState::Cracking => EntitySprite::BreakableCracked,
src/game_assets/resolvers.rs:157:        ambition_interaction::BreakableState::Broken
src/game_assets/resolvers.rs:158:        | ambition_interaction::BreakableState::Respawning => EntitySprite::BreakableBroken,
src/game_assets/mod.rs:34:use ambition_asset_manager::AssetProfile;
src/game_assets/mod.rs:38:use ambition_persistence::settings::VisualQualityBudget;
src/game_assets/mod.rs:39:use ambition_world::rooms::RoomMetadata;
src/game_assets/mod.rs:307:        catalog: &ambition_asset_manager::sandbox_assets::SandboxAssetCatalog,
src/game_assets/mod.rs:388:    catalog: &ambition_asset_manager::sandbox_assets::SandboxAssetCatalog,
src/game_assets/mod.rs:413:    catalog: &ambition_asset_manager::sandbox_assets::SandboxAssetCatalog,
src/game_assets/mod.rs:436:    catalog: &ambition_asset_manager::sandbox_assets::SandboxAssetCatalog,
Read ids.rs
Read lib.rs
Read lib.rs
Bash Check combat FeatureView type and render queries
IN
echo "=== combat events::FeatureView / FeatureVisualKind ===" && grep -rn "pub struct FeatureView\b\|pub enum FeatureVisualKind\|pub struct FeatureView {\|Component" crates/ambition_combat/src/events.rs | head -30 && echo "=== how render queries WornCharacter/Health (component query?) ===" && grep -rn "Query<\|Without<\|With<\|&Health\|&WornCharacter" crates/ambition_render/src/rendering/actors/mod.rs | head -20

OUT
=== combat events::FeatureView / FeatureVisualKind ===
crates/ambition_combat/src/events.rs:14:pub enum FeatureVisualKind {
crates/ambition_combat/src/events.rs:32:#[derive(Component, Clone, Copy, Debug, PartialEq)]
crates/ambition_combat/src/events.rs:52:pub struct FeatureView {
=== how render queries WornCharacter/Health (component query?) ===
crates/ambition_render/src/rendering/actors/mod.rs:39:/// `Sprite` to query. The `Without<WornCharacter>` filter (a spawn-time fact, no
crates/ambition_render/src/rendering/actors/mod.rs:43:    players: Query<
crates/ambition_render/src/rendering/actors/mod.rs:46:            With<PlayerVisual>,
crates/ambition_render/src/rendering/actors/mod.rs:47:            Without<Sprite>,
crates/ambition_render/src/rendering/actors/mod.rs:48:            Without<ambition_characters::actor::WornCharacter>,
crates/ambition_render/src/rendering/actors/mod.rs:84:    players: Query<
crates/ambition_render/src/rendering/actors/mod.rs:91:        With<PlayerVisual>,
crates/ambition_render/src/rendering/actors/mod.rs:157:    mut player_query: Query<
crates/ambition_render/src/rendering/actors/mod.rs:164:        With<PlayerVisual>,
crates/ambition_render/src/rendering/actors/mod.rs:166:    mut feature_query: Query<
crates/ambition_render/src/rendering/actors/mod.rs:168:        Without<PlayerVisual>,
crates/ambition_render/src/rendering/actors/mod.rs:321:/// `Without<CharacterAnimator>`, so it then skipped that boss forever and its
crates/ambition_render/src/rendering/actors/mod.rs:325:/// `Without<BossAnimator>` filter cannot either (the boss upgrader legitimately
crates/ambition_render/src/rendering/actors/mod.rs:339:    features: Query<(
crates/ambition_render/src/rendering/actors/mod.rs:496:    players: Query<
crates/ambition_render/src/rendering/actors/mod.rs:503:        With<PlayerVisual>,
crates/ambition_render/src/rendering/actors/mod.rs:552:    props: Query<(Entity, &PropVisual, Option<&BoundSpriteQuality>)>,
Read mod.rs
Read profile.rs
Bash Inspect non-test hardcoded IDs and roster include
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== enemies/mod.rs 925-945 ==="; sed -n '925,945p' features/enemies/mod.rs
echo "=== encounter/loading.rs 140-160 ==="; sed -n '140,160p' encounter/loading.rs
echo "=== effect_bus.rs 85-100 ==="; sed -n '85,100p' features/ecs/effect_bus.rs
echo "=== spawn_actors.rs 925-940 ==="; sed -n '925,940p' features/ecs/spawn_actors.rs
echo "=== boss systems.rs 420-435 ==="; sed -n '420,435p' boss_encounter/systems.rs

OUT
=== enemies/mod.rs 925-945 ===
                .unwrap_or_default();
            candidate.register(fragment)?;
            let roster = candidate.assemble()?;
            (candidate, roster)
        };
        self.insert_resource(registry).insert_resource(roster);
        Ok(self)
    }
}

#[cfg(test)]
pub(crate) fn test_roster() -> CharacterRoster {
    CharacterRoster::from_ron(include_str!(
        "../../../../../game/ambition_content/assets/data/character_archetypes.ron"
    ))
}

/// Resolve a spec by its spawn brain key against the checked-in Ambition test
/// fixture. Production callers always receive an explicit App-local roster.
#[cfg(test)]
pub(crate) fn test_spec(brain_key: &str) -> CharacterArchetypeSpec {
=== encounter/loading.rs 140-160 ===
            mobs: wave_mobs,
        }]
    }
}

#[cfg(test)]
mod loading_tests {
    use super::*;

    #[test]
    fn goblin_waves_escalate_and_spawn_past_the_trigger() {
        let waves = authored_encounter_waves("goblin_encounter")
            .expect("goblin_encounter has an authored wave book entry");
        assert_eq!(waves.len(), 3, "three authored waves");

        // Documented spatial invariant: every wave mob sits past the
        // encounter trigger's right edge (~1160) so it is on-screen after
        // the camera zooms out and the player has entered the arena.
        const TRIGGER_RIGHT: f32 = 1160.0;
        for wave in &waves {
            assert!(!wave.mobs.is_empty(), "wave '{}' has no mobs", wave.label);
=== effect_bus.rs 85-100 ===
        let flag = SetFlagRequested {
            id: "flag".into(),
            on: true,
        };
        let quest = QuestAdvanceRequested(
            ambition_persistence::quest::QuestAdvanceEvent::NpcTalked("guide".into()),
        );
        let switch = SwitchActivated {
            activation: crate::encounter::SwitchActivation {
                id: "goblin_encounter".into(),
                action: "ResetEncounter".into(),
                target_encounter: "goblin_encounter".into(),
            },
            pos: ae::Vec2::new(1.0, 2.0),
        };
        let sfx = GameplaySfxRequested {
=== spawn_actors.rs 925-940 ===
/// so callers don't have to wrap them in an `Authored<CharacterBrain>`.
/// The resulting entity carries the same component set as authored
/// encounter mobs — crucially including the `EncounterMob` marker
/// so `spawn_dynamic_feature_visuals` picks it up next frame and
/// attaches the right sprite. Without that marker the minion would
/// spawn invisibly (ECS-only).
///
/// `archetype_id` matches one of the strings in `BRAIN_NAME_TO_ARCHETYPE`
/// (`"puppy_slug"`, `"small_lurker"`, …); unknown strings fall back
/// to `Combatant` via `spec_for_brain`. `half_size` is
/// the spawn AABB half-extent (the archetype spec's `default_size`
/// usually overrides this anyway). `id` should be unique per spawn
/// so per-entity systems don't collide on identity. `encounter_id`
/// scopes the minion to a parent encounter so room reset / boss
/// despawn cleans it up alongside the boss.
#[allow(clippy::too_many_arguments)]
=== boss systems.rs 420-435 ===
#[cfg(test)]
mod phase_feedback_tests {
    use super::*;
    use crate::boss_encounter::BossEncounterPhase;
    use crate::features::ecs::boss_clusters::test_support::{test_boss_config, test_boss_status};
    use crate::features::ecs::boss_clusters::BossEncounter;
    use crate::features::{BodyKinematics, CenteredAabb, FeatureId};
    use ambition_platformer_primitives::camera_ease::CameraShakeState;

    fn spawn_boss(app: &mut App, phase: BossEncounterPhase) -> Entity {
        let config = test_boss_config("gradient_sentinel", "Gradient Sentinel", "clockwork_warden");
        let status = test_boss_status(100, phase);
        app.world_mut()
            .spawn((
                FeatureId::new("gradient_sentinel"),
                BodyKinematics {
Let me measure the content-heavy subtrees and check a few remaining content-leak candidates.

Now let me read the ambition_encounter crate to compare its vocabulary vs boss_encounter.

The _app duplication is stark. Now let me see how the host app wires the demo providers, plus runtime/host libs and pocket.

Read lib.rs
Bash Find characters module structure and type defs
IN
echo "=== characters src tree ===" && find crates/ambition_characters/src -name '*.rs' | head && echo "=== find Health/WornCharacter/BodyCombat defs ===" && grep -rn "pub struct Health\b\|pub struct WornCharacter\|pub struct BodyCombat\|struct WornCharacter" crates/ambition_characters/src/ | head

OUT
=== characters src tree ===
crates/ambition_characters/src/perception.rs
crates/ambition_characters/src/lib.rs
crates/ambition_characters/src/equipment.rs
crates/ambition_characters/src/boss_encounter.rs
crates/ambition_characters/src/actor/body.rs
crates/ambition_characters/src/actor/worn.rs
crates/ambition_characters/src/actor/mod.rs
crates/ambition_characters/src/actor/pose.rs
crates/ambition_characters/src/actor/control.rs
crates/ambition_characters/src/actor/ai.rs
=== find Health/WornCharacter/BodyCombat defs ===
crates/ambition_characters/src/actor/worn.rs:33:pub struct WornCharacter(pub String);
crates/ambition_characters/src/actor/body.rs:109:pub struct BodyCombat {
crates/ambition_characters/src/actor/mod.rs:64:pub struct Health {
Read plugins.rs
Read spec.rs
Bash Read combat FeatureView/FeatureVisualKind context
IN
echo "=== combat events.rs FeatureView area ===" && sed -n '10,90p' crates/ambition_combat/src/events.rs

OUT
=== combat events.rs FeatureView area ===

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureVisualKind {
    /// Any brain-carrying body — enemy, NPC, boss, sandbag. There is ONE actor
    /// kind: "enemy vs NPC vs boss vs training-dummy" was never a render *type*,
    /// only a STATE of one actor (see `FeatureView::fighting` for the combat
    /// state and the sandbag/name fallback in the actor sprite-upgrade system for
    /// the depiction). The taxonomy is actors|props; this is the actor arm.
    Actor,
    Hazard,
    Breakable,
    Chest,
    Pickup,
    /// Latched switch. Renders as a colored block whose color depends
    /// on `FeatureView::switch_on` (red = off, green = on).
    Switch,
}

/// Marker binding a feature visual to its kind + collision size (moved here from
/// the render layer so the mount gameplay can remove it without importing presentation).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BoundFeatureKind {
    pub kind: FeatureVisualKind,
    pub collision_size: ae::Vec2,
}

impl BoundFeatureKind {
    pub fn new(kind: FeatureVisualKind, collision: bevy::math::Vec2) -> Self {
        Self {
            kind,
            collision_size: ae::Vec2::new(collision.x, collision.y),
        }
    }

    pub fn matches(&self, kind: FeatureVisualKind, collision_size: ae::Vec2) -> bool {
        self.kind == kind && (self.collision_size - collision_size).length_squared() <= 0.25
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FeatureView {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub kind: FeatureVisualKind,
    pub visible: bool,
    pub flash: bool,
    /// For `FeatureVisualKind::Breakable`: the current authored breakable
    /// state, so presentation can select intact/cracked/broken art without
    /// querying live ECS feature components. `None` for every other kind.
    pub breakable_state: Option<ambition_interaction::BreakableState>,
    /// For `FeatureVisualKind::Chest`: true once the chest has been opened.
    /// Ignored for every other kind.
    pub chest_opened: bool,
    /// For `FeatureVisualKind::Actor`: true when the actor is in the FIGHTING
    /// state (a fact about the actor itself — NOT "hostile to the player";
    /// relativity principle). A STATE flag exactly like `flash`: a provoked NPC
    /// enters it, an at-rest enemy hasn't engaged yet. Stamped at the rebuild
    /// site from the disposition signal until the fighting-state machinery moves
    /// onto a `FightingAble` capability component. Ignored for non-actor kinds.
    pub fighting: bool,
    /// For `FeatureVisualKind::Switch`: true when the switch reads as
    /// "on" (encounter cleared / reset path armed). Renders green when
    /// true, red when false. Ignored for other kinds.
    pub switch_on: bool,
    /// Z-axis rotation to apply to the rendered sprite, in radians
    /// (Bevy frame; +π/2 is CCW). Non-zero for surface-walking
    /// archetypes that crawl on walls/ceilings; everyone else
    /// reports 0.0 and renders axis-aligned. Uses the engine → Bevy
    /// rotation mapping shared by actor rendering.
    pub rotation_rad: f32,
    /// Liveness fact (E4 slice 5): actors/bosses read their combat/health
    /// clusters (+ boss defeat), breakables `!broken`, hazards `active`;
    /// state-less kinds (pickup/chest/switch) report `true`. Presentation
    /// (nameplates, debug bars) reads THIS, never the live clusters.
    pub alive: bool,
    /// Seconds remaining on the damage flash (actors + live bosses; `0.0`
    /// for everything else, including a boss corpse — death rows are
    /// authored sprites and must not read as a lit silhouette).
    pub hit_flash_secs: f32,
Bash Find demo provider wiring in app
IN
echo "=== where are demos referenced in ambition_app ==="; grep -rn "demo_sanic\|demo_mary_o\|demo_pocket\|SanicExperiencePlugin\|MaryOExperiencePlugin\|PocketExperiencePlugin\|register_gameplay_experience\|ExperiencePlugin" game/ambition_app/src | head -40

OUT
=== where are demos referenced in ambition_app ===
game/ambition_app/src/app/shell_host.rs:12://! The Ambition GAME lives behind [`AmbitionExperiencePlugin`] — the same
game/ambition_app/src/app/shell_host.rs:55:    AmbitionExperienceConfig, AmbitionExperiencePlugin, AmbitionPreparedWorld, AMBITION_EXPERIENCE,
game/ambition_app/src/app/shell_host.rs:91:        AmbitionExperiencePlugin::new(AmbitionExperienceConfig::default()),
game/ambition_app/src/app/shell_host.rs:92:        ambition_demo_sanic::SanicExperiencePlugin,
game/ambition_app/src/app/shell_host.rs:93:        ambition_demo_mary_o::MaryOExperiencePlugin,
game/ambition_app/src/app/shell_host.rs:94:        ambition_demo_pocket::PocketExperiencePlugin,
Read decomposition.md
Bash Measure content subtrees and config constants
IN
cd /home/joncrall/code/ambition
echo "=== asset_manager subtree LOC ==="
echo "sandbox_assets:"; find crates/ambition_asset_manager/src/sandbox_assets -name '*.rs' | xargs wc -l | tail -1
echo "asset_publish:"; find crates/ambition_asset_manager/src/asset_publish -name '*.rs' | xargs wc -l | tail -1
echo "core (rest):"; find crates/ambition_asset_manager/src -maxdepth 1 -name '*.rs' | xargs wc -l | tail -1
echo
echo "=== engine_core: total vs test LOC ==="
echo "test files:"; find crates/ambition_engine_core/src -name '*.rs' \( -path '*/tests*' -o -name 'tests.rs' -o -name 'test_support.rs' \) | xargs wc -l | tail -1
echo "non-test:"; find crates/ambition_engine_core/src -name '*.rs' ! -path '*/tests*' ! -name 'tests.rs' ! -name 'test_support.rs' ! -name '*_tests.rs' | xargs wc -l | tail -1
echo
echo "=== config.rs constants ==="
grep -nE 'pub const|pub fn' crates/ambition_engine_core/src/config.rs | head -40

OUT
=== asset_manager subtree LOC ===
sandbox_assets:
 1129 total
asset_publish:
  889 total
core (rest):
 1729 total

=== engine_core: total vs test LOC ===
test files:
  9409 total
non-test:
 13782 total

=== config.rs constants ===
20:pub const WINDOW_W: u32 = 1600;
23:pub const WINDOW_H: u32 = 900;
25:pub const WORLD_Z_BLOCK: f32 = 0.0;
26:pub const WORLD_Z_DUMMY: f32 = 10.0;
27:pub const WORLD_Z_PLAYER: f32 = 20.0;
28:pub const WORLD_Z_FX: f32 = 30.0;
30:pub const GRID_STEP: f32 = 80.0;
38:pub fn world_to_bevy(world: &World, p: Vec2, z: f32) -> Vec3 {
46:pub fn world_size_to_bevy(size: Vec2, p: Vec2, z: f32) -> Vec3 {
Bash Inspect FeatureView usage in render
IN
echo "=== render actors/mod.rs lines 335-360 FeatureView query ===" && sed -n '335,360p' crates/ambition_render/src/rendering/actors/mod.rs && echo "=== where does render get FeatureView? via sim_view or direct query ===" && grep -rn "FeatureView\|FeatureViewIndex\|DynamicFeatureViews" crates/ambition_render/src/rendering/actors/mod.rs | head

OUT
=== render actors/mod.rs lines 335-360 FeatureView query ===
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(
        Entity,
        &FeatureVisual,
        Option<&BoundFeatureKind>,
        Option<&BoundSpriteQuality>,
    )>,
    // Materialized actor identity read-model (name / sprite-override / sandbag /
    // authored render size) — the renderer binds a sprite from this snapshot
    // WITHOUT borrowing gameplay_core's live actor clusters. Built by
    // `rebuild_actor_render_index` in the sim's `FeatureViewSync` set.
    actor_render: Res<ambition_sim_view::ActorRenderIndex>,
    // A boss is ALSO an actor (post-unification), so its id appears in BOTH render
    // read-models. This one is read to YIELD, never to bind — see
    // `actor_sprite_path_owns`.
    boss_render: Res<ambition_sim_view::BossRenderIndex>,
    // Names we've already warned about resolving no sprite, so the warning fires
    // once per offending name instead of every frame the actor is unbound.
    mut warned_sprite_names: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
=== where does render get FeatureView? via sim_view or direct query ===
crates/ambition_render/src/rendering/actors/mod.rs:18:use ambition_sim_view::FeatureViewIndex;
crates/ambition_render/src/rendering/actors/mod.rs:153:    feature_views: Res<FeatureViewIndex>,
crates/ambition_render/src/rendering/actors/mod.rs:155:    // facts resolved in `FeatureViewSync`; render never touches the live
crates/ambition_render/src/rendering/actors/mod.rs:262:fn state_aware_entity_sprite(view: &ambition_combat::events::FeatureView) -> Option<EntitySprite> {
crates/ambition_render/src/rendering/actors/mod.rs:338:    feature_views: Res<FeatureViewIndex>,
crates/ambition_render/src/rendering/actors/mod.rs:348:    // `rebuild_actor_render_index` in the sim's `FeatureViewSync` set.
Found the leak: game_assets/resolvers.rs maps gameplay entity specs to sprites. Let me inspect boss.rs and the build.rs, and check for named-boss content.

Read timeline.rs
Read boss.rs
Read build.rs
Let me read the abilities module, projectile modules, and items to assess overlap and content-specificity.

Let me read the host shell wiring, runtime/host libs, pocket, and content plugin/provider.

Read mod.rs
Read shell_host.rs
Read lib.rs
Bash Read encounter timeline and boss encounter script
IN
cd /home/joncrall/code/ambition
echo "=== ambition_characters/boss_encounter files ==="
find crates/ambition_characters/src -path '*boss*' -name '*.rs' | while read f; do lc=$(wc -l < "$f"); printf "%6d  %s\n" "$lc" "${f#crates/ambition_characters/}"; done | sort -rn
echo
echo "=== boss_encounter/encounter_script.rs head ==="
head -60 crates/ambition_actors/src/boss_encounter/encounter_script.rs

OUT
=== ambition_characters/boss_encounter files ===
  1352  src/brain/boss_pattern/tests.rs
  1213  src/brain/boss_pattern/mod.rs
   714  src/brain/boss_pattern/tick.rs
   486  src/brain/boss_pattern/control_flow/tests.rs
   468  src/brain/boss_pattern/validator.rs
   422  src/boss_encounter.rs
   402  src/brain/boss_pattern/validator/tests.rs
   302  src/brain/boss_pattern/seeds.rs
   251  src/brain/boss_pattern/control_flow.rs
   232  src/boss_encounter/phase_mechanism_tests.rs

=== boss_encounter/encounter_script.rs head ===
//! Encounter-script EXECUTION + its actor-specific mechanics.
//!
//! The generic timeline vocabulary — [`EncounterGate`], [`EncounterTrigger`],
//! [`EncounterEffect`], [`EncounterBeat`], [`EncounterScript`] — and the generic
//! beat-advance (`EncounterScript::advance`) live in `ambition_encounter` (the
//! one timeline authority). This module owns only what TOUCHES actor bodies: it
//! reads each script's `advance`d effects and EXECUTES them (defeat a member,
//! command a member's brain, drop a hazard, banner, music), plus the two generic
//! mechanics an effect spawns — [`CommandedMove`] (a "walk the boss to a spot"
//! brain override) and [`FallingHazard`] (a "hang, wait for alignment, fall,
//! fire the impact gate" hazard). Member indices address the encounter's generic
//! [`EncounterParticipants`].
//!
//! The cut-rope fight is expressed entirely as a script: `Gate("rope_cut")` →
//! [`EncounterEffect::CommandMoveTo`] (lure the behemoth under the drop) +
//! [`EncounterEffect::DropHazard`] (a [`FallingHazard`]) → `ForceKill`. The
//! swallowed-NPC release falls out of the generic
//! [`ReleaseOnDeath`] (super::encounter_entity::ReleaseOnDeath).

use bevy::prelude::*;

use crate::features::ecs::boss_clusters::{BossClusterRef, BossEncounter};
use crate::features::CenteredAabb;
use ambition_encounter::{EncounterEffect, EncounterGate, EncounterParticipants, EncounterScript};
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_platformer_primitives::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Advance every encounter script and EXECUTE the effects it yields this tick.
/// The trigger evaluation + cursor logic is generic (`EncounterScript::advance`,
/// reading fired gates + participant deadness); this system supplies the
/// actor-touching execution. Runs in the Progression set after
/// `update_encounter_progress` (which refreshes participant `alive`).
pub fn tick_encounter_scripts(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    mut gates: MessageReader<EncounterGate>,
    mut scripts: Query<(&EncounterParticipants, &mut EncounterScript)>,
    mut members: Query<(
        &mut BossEncounter,
        &mut ambition_characters::actor::BodyHealth,
    )>,
    session_owners: Query<&SessionScopedEntity>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    mut music: ambition_platformer_primitives::lifecycle::SessionWorldMut<crate::encounter::EncounterMusicRequest>,
) {
    let dt = world_time.sim_dt();
    let fired: Vec<String> = gates.read().map(|g| g.gate.clone()).collect();

    for (participants, mut script) in &mut scripts {
        let effects = script.advance(dt, participants, &fired);
        let member_entity = |i: usize| participants.members.get(i).and_then(|p| p.entity);
        for effect in &effects {
            match effect {
                EncounterEffect::ForceKill(i) => {
                    if let Some(m) = member_entity(*i) {
                        if let Ok((mut status, mut health)) = members.get_mut(m) {
                            health.health.current = 0;
Read puppy_slug_gun.rs
Now the runtime/host libs and content plugin/provider.

The build.rs reaches into ../ambition_actors/assets/sprites*/. Let me check the EntitySprite enum, game_assets header, and hunt named content across sprite_sheet.

Read lib.rs
Read lib.rs
Bash Inspect projectile and enemy_projectile modules
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== projectile/ files ==="; find projectile -name '*.rs' | xargs wc -l | sort -rn
echo "=== projectile/mod.rs head ==="; sed -n '1,40p' projectile/mod.rs 2>/dev/null || sed -n '1,40p' projectile.rs
echo "=== enemy_projectile/ files ==="; find enemy_projectile -name '*.rs' | xargs wc -l | sort -rn
echo "=== enemy_projectile/mod.rs head ==="; sed -n '1,35p' enemy_projectile/mod.rs 2>/dev/null || find enemy_projectile -name 'mod.rs' -exec sed -n '1,35p' {} \;

OUT
=== projectile/ files ===
 1776 total
  832 projectile/systems.rs
  374 projectile/tests/charging.rs
  277 projectile/tests/mod.rs
  269 projectile/tests/collision.rs
   24 projectile/mod.rs
=== projectile/mod.rs head ===
//! Sandbox PLAYER-faction projectile glue.
//!
//! The reusable projectile MODEL — shot vocabulary (`ProjectileKind` / visual
//! kinds), the ECS components, `PlayerProjectileState`, the `SpawnProjectile`
//! pool + player-pool spawner, and pure portal transit — now lives in the
//! [`ambition_projectiles`] crate (E2 carve) and is re-exported below so
//! `crate::projectile::*` paths resolve unchanged for every sandbox consumer.
//!
//! What STAYS here is the victim/world/anim-woven sim STEPPERS that cannot leave
//! until the boss/actor/player domains carve (E6/E7) and the world overlay lands
//! in `ambition_world` (W3): the unified [`systems::step_projectiles`] (queries
//! bosses/breakables/actors, emits `HitEvent`, parry-heals the player), the
//! [`systems::charge_projectile_input`] player-input/anim driver, and the
//! `ambition_projectiles::collision_world::ProjectileCollisionWorld` param, which
//! reads the ECS world overlay (it came home in R4).
//! They CONSUME the model crate — the legal sim → model direction.

pub use ambition_projectiles::*;

pub mod systems;
pub use systems::{charge_projectile_input, step_projectiles};

#[cfg(test)]
mod tests;
=== enemy_projectile/ files ===
  687 total
  542 enemy_projectile/systems/tests.rs
  108 enemy_projectile/test_support.rs
   20 enemy_projectile/systems.rs
   17 enemy_projectile/mod.rs
=== enemy_projectile/mod.rs head ===
//! Enemy-fired projectile glue (pirate volleys etc).
//!
//! The enemy-shot MODEL — the `EnemyProjectile` marker + `EnemyProjectileState`/
//! `EnemyProjectileSpawn` — now lives in [`ambition_projectiles::enemy`] (E2
//! carve) and is re-exported below so `crate::enemy_projectile::*` paths resolve
//! unchanged. The canonical effect-request spawn executor now lives in
//! [`ambition_projectiles::enemy::apply_enemy_projectile_effect_requests`]. This
//! module keeps the legacy system name for actor-internal tests and transitional
//! call sites only; runtime scheduling goes through `ambition_runtime`.

pub use ambition_projectiles::enemy::*;

pub mod systems;
pub use systems::apply_projectile_effects;

#[cfg(test)]
pub(crate) mod test_support;
Bash Read EntitySprite enum
IN
cd /home/joncrall/code/ambition/crates/ambition_sprite_sheet && echo "=== EntitySprite enum variants ===" && grep -n "enum EntitySprite" -A 60 src/game_assets/entity_sprite.rs | head -70

OUT
=== EntitySprite enum variants ===
21:pub enum EntitySprite {
22-    // Features
23-    ChestClosed,
24-    ChestOpen,
25-    BreakableIntact,
26-    BreakableCracked,
27-    BreakableBroken,
28-    PickupHealth,
29-    PickupCurrency,
30-    PickupAbility,
31-    HazardSpikes,
32-    NpcTerminal,
33-    BossCore,
34-    SandbagDummy,
35-    // Switch on/off — chosen state-aware in `state_aware_entity_sprite` by the
36-    // `FeatureView::switch_on` flag (armed = on, disabled = off).
37-    SwitchArmed,
38-    SwitchDisabled,
39-    // Blocks / surfaces
40-    SolidBlock,
41-    OneWayPlatform,
42-    SoftBlinkWall,
43-    HardBlinkWall,
44-    PogoOrb,
45-    ReboundPad,
46-    MovingPlatform,
47-    // Loading zones
48-    DoorZone,
49-    EdgeExit,
50-    // Player projectiles (Fireball + Hadouken share the same sprite)
51-    ProjectileEnergy,
52-    // 32×32 tile sprites for IntGrid-derived block surfaces. Rendered
53-    // via `Sprite::image_mode = Tiled` so they REPEAT across the
54-    // arbitrary aspect ratios that long floors / tall walls produce,
55-    // instead of stretching one sprite across the whole footprint.
56-    SolidTile,
57-    OneWayTile,
58-    HazardTile,
59-    SoftBlinkTile,
60-    HardBlinkTile,
61-    /// Visual for the encounter-driven "lock wall" block that
62-    /// `sync_lock_walls` inserts into `world.blocks` while an
63-    /// encounter is in flight. Uses a dedicated tile so it reads as
64-    /// "this just appeared" rather than blending into adjacent walls.
65-    LockWallTile,
66-}
67-
68-impl EntitySprite {
69-    /// Path relative to the configured sprite folder.
70-    pub const fn relative_path(self) -> &'static str {
71-        match self {
72-            Self::ChestClosed => "entities/chest_closed.png",
73-            Self::ChestOpen => "entities/chest_open.png",
74-            Self::BreakableIntact => "entities/breakable_intact.png",
75-            Self::BreakableCracked => "entities/breakable_cracked.png",
76-            Self::BreakableBroken => "entities/breakable_broken.png",
77-            Self::PickupHealth => "entities/pickup_health.png",
78-            Self::PickupCurrency => "entities/pickup_currency.png",
79-            Self::PickupAbility => "entities/pickup_ability.png",
80-            Self::HazardSpikes => "entities/hazard_spikes.png",
81-            Self::NpcTerminal => "entities/npc_terminal.png",
Let me confirm the engine_core/platformer_primitives boundary details and check remaining content-constant candidates.

Let me check how render uses Health (health.rs) and WornCharacter, plus verify the architecture boundary enforcement and the workspace policy test.

Bash Inspect Health and WornCharacter usage/definitions
IN
echo "=== health.rs Health usage ===" && grep -n "Health\|Query\|Res<\|BoundFeatureKind\|FeatureView" crates/ambition_render/src/rendering/health.rs | head -30 && echo "=== worn.rs WornCharacter context ===" && sed -n '25,45p' crates/ambition_characters/src/actor/worn.rs

OUT
=== health.rs Health usage ===
2://! a `Health` resource. Toggled via
10:use super::primitives::HealthOverlayVisual;
12:use ambition_characters::actor::Health;
18:use ambition_sim_view::{ActorRenderIndex, BossRenderIndex, FeatureViewIndex};
21:pub struct BossHealthBarOverlayVisual;
31:/// `BossRenderIndex`, liveness + hp ride the boss's `FeatureView` row.
34:    overlays: Query<Entity, With<BossHealthBarOverlayVisual>>,
35:    boss_render: Res<BossRenderIndex>,
36:    feature_views: Res<FeatureViewIndex>,
37:    ui_fonts: Option<Res<UiFonts>>,
38:    active_session: Option<Res<ActiveSessionScope>>,
54:                Health {
97:                Name::new("Boss Health Overlay Root"),
98:                BossHealthBarOverlayVisual,
115:                Name::new(format!("Boss Health Panel: {boss_name}")),
135:                        Name::new(format!("Boss Health Track: {boss_name}")),
145:                            Name::new(format!("Boss Health Fill: {boss_name}")),
160:    dev_state: Res<ambition_dev_tools::SandboxDevState>,
161:    active_session: Option<Res<ActiveSessionScope>>,
162:    developer_tools: Res<ambition_dev_tools::dev_tools::DeveloperTools>,
163:    overlays: Query<Entity, With<HealthOverlayVisual>>,
165:    // `BodyPoseView`; actors/bosses/breakables ride their `FeatureView`
167:    player: Query<
171:    feature_views: Res<FeatureViewIndex>,
172:    actor_render: Res<ActorRenderIndex>,
173:    boss_render: Res<BossRenderIndex>,
174:    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
197:            Health {
207:        let hp = Health {
271:    health: Health,
=== worn.rs WornCharacter context ===

/// The catalog `character_id` a body currently wears.
///
/// Simulation-owned and set at spawn from the selected character; changing it
/// (a re-wear / transformation) is the supported runtime path, and downstream
/// gameplay + presentation systems observe the change through Bevy's
/// `Changed<WornCharacter>` filter.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct WornCharacter(pub String);

impl WornCharacter {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The worn catalog id.
    pub fn id(&self) -> &str {
        &self.0
    }
}
Let me compare the two phase-state machines: boss (characters) vs wave (encounter crate).

Bash Named content search + game_assets header
IN
cd /home/joncrall/code/ambition/crates/ambition_sprite_sheet && echo "=== named content (bosses/chars) mentioned in src ===" && grep -rhoiE "gradient.sentinel|mockingbird|goblin|sandbag|sanic|mary.?o|prism|sentinel|\bboss\b" --include=*.rs src | sort | uniq -c | sort -rn | head -30 && echo "" && echo "=== game_assets/mod.rs header ===" && head -35 src/game_assets/mod.rs

OUT
=== named content (bosses/chars) mentioned in src ===
     84 boss
     19 goblin
     15 sandbag
     10 mockingbird
      8 Boss
      5 Sandbag
      3 gradient sentinel
      3 MOCKINGBIRD
      2 Goblin
      1 gradient_sentinel
      1 gradient-sentinel
      1 Mockingbird

=== game_assets/mod.rs header ===
//! Game asset wiring with fallback-friendly loading.
//!
//! The sandbox loads two layers of art on top of its colored-rectangle
//! placeholders:
//!
//! - **Character spritesheets** (robot/goblin/boss) — animated, owned by
//!   `character_sprites`. Loaded here to keep all asset config in one place.
//! - **Entity sprites** — single-image placeholders for chests, pickups,
//!   breakables, hazards, NPCs, blocks, loading zones, and so on. Their
//!   identity/path is emitted by
//!   `python -m ambition_sprite2d_renderer draw-entities` as
//!   `entity_manifest.yaml` next to the generated PNGs.
//! - **Background/parallax layers** — biome-specific sky and atmosphere
//!   plates generated by `python -m ambition_parallax_renderer`, intentionally
//!   outside the sprite asset folder.
//!
//! Two design rules:
//!
//! 1. **The game must always run.** Every asset is `Option<Handle<Image>>`;
//!    if a PNG is missing on disk the rendering layer falls back to its
//!    legacy colored rectangle. The `--no-assets` CLI flag forces every
//!    handle to `None` regardless of disk state, so designers can sanity-
//!    check the placeholder visuals at any moment.
//! 2. **Asset *source* is pluggable.** Today's loader walks the
//!    `assets/<sprite_folder>/` directory; a future loader can synthesize
//!    `Image` assets at runtime and insert them into `Assets<Image>` —
//!    callers only see `GameAssets` and don't care where handles came from.
//!    To make that swap painless we go through the high-level `GameAssets`
//!    struct rather than baking specific paths into call sites.

use bevy::prelude::*;
use std::collections::HashMap;

use ambition_asset_manager::AssetProfile;
Now the content plugin/provider and its render/runtime usage.

Read boss_encounter.rs
Read plugin.rs
Read state.rs
Read provider.rs
Bash Find architecture boundary enforcement rules
IN
echo "=== workspace policy test dir ===" && find /home/joncrall/code/ambition/tests/ambition_workspace_policy -name '*.rs' | head && echo "=== grep architecture boundary rules mentioning render/actors ===" && grep -rn "render\|actors\|sim_view\|forbid\|deny\|boundary" /home/joncrall/code/ambition/tests/ambition_workspace_policy/src 2>/dev/null | head -40

OUT
=== workspace policy test dir ===
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/model.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/runner.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/lib.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/workspace.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/tests/policy.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/session_world.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/module_size.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/mod.rs
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/migration_matrix.rs
=== grep architecture boundary rules mentioning render/actors ===
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/runner.rs:13:        RuleKind::ForbiddenPath => rules::paths::forbidden(ws, policy, report),
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/runner.rs:16:        RuleKind::DependencyDenylist => rules::dependency::denylist(ws, policy, report),
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/workspace.rs:217:/// forbidden identifier must never trip a source scan.
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/workspace.rs:225:/// forbidden names are type/crate identifiers, and the reviewed string-data
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/session_world.rs:150:                    rationale: "live platformer world state exists only as components on the exact SessionRoot; process resources and synchronization bridges are forbidden"
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/module_size.rs:23:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/module_size.rs:39:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/content_ownership.rs:12:const FILE: &str = "crates/ambition_actors/src/features/ecs/actor_clusters.rs";
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/content_ownership.rs:38:        owners: vec!["ambition_actors".to_string()],
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/content_ownership.rs:53:                owners: vec!["ambition_actors".to_string()],
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:1://! **The `ControlFrame` allowlist lint** — custom scanner (unified-actors step 5,
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:33:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:46:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:70:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:214:                    let boundary = line[..at]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:220:                    if boundary {
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:389:        |_marker: &str, t: &str| control_frame_holders(m, "crates/ambition_actors/src/x.rs", t);
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/control_frame.rs:394:        "crates/ambition_actors/src/features/ecs/actors/update.rs",
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/migration_matrix.rs:33:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:11://! CONFIG (sim roots + scope, excluded subpaths, review marker, forbidden RNG
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:26:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:41:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:48:#[serde(deny_unknown_fields)]
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:480:        "use std::collections::HashMap;\nstruct S {\n    actors: HashMap<String, u8>,\n}\nfn f(s: &S) {\n    for a in s.actors.values() {}\n}".to_string(),
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:487:        std_hash_bindings("use std::collections::HashMap;\nlet actors: HashMap<u8, u8> = q();")
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:488:            .contains("actors"),
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:493:            "use bevy::platform::collections::HashMap;\nlet actors: HashMap<u8, u8> = q();"
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/determinism.rs:495:        .contains("actors"),
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/lifecycle.rs:4://! The allowlist (path=count) is `docs/architecture/architecture-boundary-allowlist.txt`.
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/lifecycle.rs:11:const SPAWN_DIR: &str = "crates/ambition_actors/src/features/ecs";
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/lifecycle.rs:12:const CRATE_SRC: &str = "crates/ambition_actors/src";
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/lifecycle.rs:13:const ALLOWLIST: &str = "docs/architecture/architecture-boundary-allowlist.txt";
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/lifecycle.rs:45:        owners: vec!["ambition_actors".to_string()],
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/custom/lifecycle.rs:83:                owners: vec!["ambition_actors".to_string()],
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/rules/source_reference.rs:1://! `forbidden-source-reference`: no `.rs` file under `roots` may name any
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/rules/source_reference.rs:2://! identifier in `forbid`. The workhorse of the architecture guards — it powers
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/rules/source_reference.rs:12://!                         reviewed cross-boundary / string-data exceptions).
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/rules/source_reference.rs:16://! that names a forbidden type never trips the scan.
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/rules/source_reference.rs:24:        "policy `{}` (forbidden-source-reference) lists no roots — vacuous",
/home/joncrall/code/ambition/tests/ambition_workspace_policy/src/rules/source_reference.rs:28:        !policy.forbid.is_empty(),
Bash Confirm boundary details and constant locations
IN
cd /home/joncrall/code/ambition
echo "=== how platformer_primitives::kinematic uses engine_core ==="
grep -nE 'use ambition_engine_core|collision_semantics|engine_core::' crates/ambition_platformer_primitives/src/kinematic.rs | head -20
echo
echo "=== gravity.rs: does it duplicate reference_frame concepts? ==="
grep -nE 'AccelerationFrame|MotionFrame|gravity_dir|GravityField|reference|pub struct|pub fn' crates/ambition_platformer_primitives/src/gravity.rs | head -25
echo
echo "=== movement tuning constants (GRAVITY/JUMP_SPEED etc) — are they hardcoded consts? ==="
grep -nE 'pub const (GRAVITY|JUMP_SPEED|DASH_SPEED|MAX_RUN_SPEED|BLINK_DISTANCE|WALL_JUMP_X)' crates/ambition_engine_core/src/movement/tuning.rs
echo
echo "=== world.rs BlinkWallTier / content vocabulary ==="
grep -nE 'pub enum|pub struct|BlinkWallTier|Sanic|Tier' crates/ambition_engine_core/src/world.rs | head -30

OUT
=== how platformer_primitives::kinematic uses engine_core ===
34:use ambition_engine_core::collision_semantics::{
41:use ambition_engine_core::Vec2;
42:use ambition_engine_core::{Aabb, AabbExt};
43:use ambition_engine_core::{BlockKind, World};
279:    if let Some(hit) = ambition_engine_core::cast::body_sweep(world, start_body, delta, |block| {

=== gravity.rs: does it duplicate reference_frame concepts? ===
3://! [`GravityField`] is the world's gravity state (a redirectable down). The
10://!   velocity" call. Add a force to `GravityField` + one line here and they all
14://!   [`GravityField::vertical_sign`] (which way is "down" along Y). They read the
15://!   same `GravityField`, so a gravity flip moves them too.
32:pub struct GravityField {
36:impl Default for GravityField {
45:impl GravityField {
48:    pub fn gravity_accel(&self, magnitude: f32) -> Vec2 {
54:    pub fn motion_frame(&self, magnitude: f32) -> ambition_engine_core::MotionFrame {
55:        ambition_engine_core::MotionFrame::from_direction(self.dir, magnitude)
60:    pub fn vertical_sign(&self) -> f32 {
72:/// overlapping zone's direction) into the live [`GravityField`] each frame, so
75:pub struct BaseGravity {
92:    pub fn cycle(&mut self) {
103:    pub fn direction_label(&self) -> &'static str {
119:/// elsewhere — see [`gravity_dir_at`].
121:pub struct GravityZone {
134:pub struct GravityZones {
141:pub fn collect_gravity_zones(mut snapshot: ResMut<GravityZones>, zones: Query<&GravityZone>) {
151:pub struct OscillatingZone {
168:pub struct TemporaryZone {
175:pub fn tick_temporary_zones(
191:pub fn oscillate_gravity_zones(
209:/// [`GravityField`].)
210:pub fn gravity_dir_at(pos: Vec2, zones: &GravityZones, base_dir: Vec2) -> Vec2 {

=== movement tuning constants (GRAVITY/JUMP_SPEED etc) — are they hardcoded consts? ===
32:pub const GRAVITY: f32 = 2250.0;
45:pub const MAX_RUN_SPEED: f32 = 270.0;
50:pub const JUMP_SPEED: f32 = 630.0;
52:pub const WALL_JUMP_X: f32 = 430.0;
55:pub const DASH_SPEED: f32 = 760.0;
60:pub const BLINK_DISTANCE: f32 = 190.0;

=== world.rs BlinkWallTier / content vocabulary ===
11:pub enum BlinkWallTier {
20:pub enum BlockKind {
26:    BlinkWall { tier: BlinkWallTier },
50:pub struct Block {
101:    pub fn blink_wall(name: impl Into<String>, min: Vec2, size: Vec2, tier: BlinkWallTier) -> Self {
203:pub struct WaterVolumeSpec {
234:pub enum WaterKind {
245:pub struct WaterRegion {
262:pub struct WaterContact {
279:pub enum ClimbableKind {
294:pub struct ClimbableSpec {
317:pub struct ClimbableRegion {
338:pub struct ClimbableContact {
355:pub enum SurfaceKind {
369:pub enum SurfacePort {
394:pub struct SurfaceJunction {
415:pub struct SurfaceFrame {
446:pub struct SurfaceChain {
782:pub struct World {
805:pub struct SweepHit<'a> {
817:pub struct FaceAnchor {
1146:/// to cover dash / Sanic-momentum bursts. Blink is a discrete teleport handled
1170:pub struct RoomGeometry(pub World);
Bash content render/runtime usages and lib top
IN
echo "=== content: ambition_render usages ==="; grep -rn "ambition_render\|render::" game/ambition_content/src | grep -v "//" | head -30; echo; echo "=== content: ambition_runtime usages ==="; grep -rn "ambition_runtime\|runtime::" game/ambition_content/src | grep -v "^\s*//" | head -30; echo "=== content lib.rs top ==="; sed -n '1,60p' game/ambition_content/src/lib.rs

OUT
=== content: ambition_render usages ===
game/ambition_content/src/intro/plugin.rs:30:use ambition_render::quality::ResolvedVisualQuality;
game/ambition_content/src/portal/tests.rs:908:    use ambition_render::rendering::PlayerVisual;
game/ambition_content/src/portal/tests.rs:932:    use ambition_render::rendering::PlayerVisual;
game/ambition_content/src/bosses/cut_rope/mod.rs:31:use ambition_render::rendering::PropVisual;

=== content: ambition_runtime usages ===
game/ambition_content/src/provider.rs:16:use ambition_runtime::PlatformerSessionWorld;
game/ambition_content/src/falling_sand.rs:600:            ambition_actors::platformer_runtime::lifecycle::RoomVisual,
game/ambition_content/src/falling_sand.rs:1290:            ambition_actors::platformer_runtime::lifecycle::RoomVisual,
game/ambition_content/src/portal/shot_adapter.rs:4://! [`SolidWorldQuery`] (ambition_actors::platformer_runtime::collision::SolidWorldQuery)
game/ambition_content/src/portal/shot_adapter.rs:13:use ambition_actors::platformer_runtime::prelude::SpawnScopedExt;
game/ambition_content/src/portal/inventory_adapter.rs:25:use ambition_actors::platformer_runtime::prelude::SpawnScopedExt;
game/ambition_content/src/portal/tests.rs:8:use ambition_actors::platformer_runtime::gravity::{gravity_upright_angle, GravityField};
game/ambition_content/src/portal/tests.rs:9:use ambition_actors::platformer_runtime::orientation::{update_actor_roll, ActorRoll};
game/ambition_content/src/portal/tests.rs:10:use ambition_actors::platformer_runtime::transit::rotate_velocity_between_normals as portal_transform_velocity;
game/ambition_content/src/portal/tests.rs:18:use ambition_actors::platformer_runtime::collision::raycast_solids;
game/ambition_content/src/portal/tests.rs:1132:            With<ambition_actors::platformer_runtime::lifecycle::RoomScopedEntity>,
game/ambition_content/src/portal/transit_body_adapter.rs:11://! [`BodyKinematics`]: ambition_actors::platformer_runtime::body::BodyKinematics
game/ambition_content/src/portal/transit_body_adapter.rs:167:    gravity: Option<Res<ambition_actors::platformer_runtime::gravity::GravityField>>,
game/ambition_content/src/portal/transit_body_adapter.rs:176:        ambition_actors::platformer_runtime::gravity::gravity_dir_or_default(gravity.as_deref());
game/ambition_content/src/portal/transit_body_adapter/projectile_transit_tests.rs:22:use ambition_actors::platformer_runtime::body::BodyKinematics;
game/ambition_content/src/bosses/specials/mod.rs:93:        app.init_resource::<ambition_runtime::snapshot::SnapshotRegistry>();
game/ambition_content/src/bosses/specials/mod.rs:96:            .resource_mut::<ambition_runtime::snapshot::SnapshotRegistry>();
game/ambition_content/src/bosses/specials/snapshot.rs:10://! trait must live where `ambition_runtime` can implement it for `ambition_time` and
game/ambition_content/src/bosses/specials/snapshot.rs:15:use ambition_runtime::snapshot::{
game/ambition_content/src/bosses/specials/snapshot.rs:18:use ambition_runtime::{snapshot_pod, snapshot_unit_enum};
=== content lib.rs top ===
//! THE named Ambition game content — everything that names this game's
//! specific world: quests, bosses, items, dialogue, banter, the intro,
//! the enemy roster, music cues, and the cross-content validator.
//!
//! This is the content crate, distinct from the reusable machinery crate
//! `ambition_actors` it depends on. The dependency direction is strict and
//! one-way — content → machinery, never the reverse — so the named cast and
//! data installed here build on top of the generic schemas/pipelines that
//! live machinery-side. Registration flows through one seam,
//! [`AmbitionContentPlugin`].
//!
//! Most top-level modules are thin install plugins ([`plugin`], [`quests`],
//! [`bosses`], [`dialogue`], [`items`]) that seed named rosters into
//! machinery resources, alongside the authored data/content itself
//! ([`quest`], [`enemy_roster`], [`banter`], [`music`], [`intro`]) and the
//! [`content_validation`] cross-reference checker. Several names re-export
//! their machinery half (e.g. [`data`], [`features`]) so historical
//! `crate::…` paths keep resolving.

/// The authored audio registries (music/SFX RON), registered as an App-local
/// provider fragment.
pub mod provider;
pub mod audio_registries;
pub mod banter;
pub mod bosses;
/// The character catalog data and curated playable cast, contributed as an
/// immutable provider fragment to the App-local catalog assembly.
pub mod character_catalog;
pub mod content_validation;
pub mod dialogue;
/// The spectator-duel exhibition fight (RoomLoaded consumer + `<<duel>>`).
pub mod duel_arena;
/// The falling-sand prototype room (self-gating content plugin).
#[cfg(feature = "falling_sand")]
pub mod falling_sand;
// `features` (the feature-ECS actor/boss world) was promoted to
// `ambition_actors::features` (lib root): machinery presentation/dev still read
// its named bits (doc 20 B3/B4), so it stays in the sandbox lib when
// the rest of this content module becomes the `ambition_content`
// crate. Re-exported here so `content::features` paths keep working.
pub use ambition_actors::features;
/// The named hostile-archetype data, contributed as an immutable provider
/// fragment to the App-local roster assembly.
pub mod enemy_roster;
pub mod intro;
pub mod items;
#[cfg(feature = "audio")]
pub mod music;
pub mod plugin;
pub mod quest;
pub mod quests;
/// The LDtk world payload + Ambition's `WorldManifest` (install seam:
/// `ambition_actors::ldtk_world`).
pub mod worlds;

#[cfg(feature = "portal")]
pub mod portal;

pub use plugin::AmbitionContentPlugin;
Bash Read headers of items/world/session/schedule/control/affordances/encounter/body_mode
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
for f in items/mod.rs world/mod.rs session/mod.rs schedule/mod.rs control/mod.rs affordances/mod.rs encounter/mod.rs body_mode/mod.rs; do
  echo "===== $f ====="; sed -n '1,28p' "$f" 2>/dev/null
done

OUT
===== items/mod.rs =====
//! Actor-sim item adapters.
//!
//! The reusable item catalog, shop primitives, and inventory UI state live in
//! `ambition_items` (E8). The pickup/throw/projectile steppers stay here because
//! they mutate actor bodies, gravity, portals, abilities, and hit events.

pub use ambition_items::*;

pub mod persist;
pub mod pickup;
pub mod world_item;

pub use world_item::{spawn_world_item, WorldItem, WorldItemPayload};
===== world/mod.rs =====
//! World / level authoring runtime: room graph + spawning, the code-first
//! room builder, the LDtk hot-reloadable project loader, the Avian2D
//! physics adapter, and LDtk-authored moving platforms.
//!
//! Long-term shape: this umbrella is the spine the future `ambition`
//! framework crate wraps. Module-internal `crate::rooms::…` paths still
//! resolve via re-exports at the crate root so this reorg is a pure
//! relocation.

pub mod ldtk_world;
pub mod overlay;
pub mod physics;
pub mod placements;
pub mod platforms;
pub mod rooms;
===== session/mod.rs =====
//! Sandbox SESSION lifecycle: startup setup ([`setup`]), full
//! reset/respawn ([`reset`]), RON data manifests ([`data`]), and setup glue.
//! Coarse `GameMode` state and camera layer markers now live in
//! `ambition_platformer_primitives`.
//!
//! Name overlap warning: this is the in-crate session runtime, NOT the
//! separate `ambition_platformer_primitives` crate (re-exported here as
//! `crate::platformer_runtime`), which holds content-free engine
//! primitives. This module is sim-side session glue that `app/` (the
//! schedule wiring) calls into.

pub mod data;
pub mod reset;
pub mod setup;

pub use ambition_world::rooms::RespawnRoomVisualsRequested;
===== schedule/mod.rs =====
//! Schedule + input-frame vocabulary shared by the machinery lib, the
//! content crate, and the app crate.
//!
//! The Bevy app ASSEMBLY (plugins, resources, sim systems, HUD, CLI)
//! moved to the `ambition_app` crate (Stage 20 / A3). What stays here
//! is the vocabulary other layers order against: the `SandboxSet`
//! schedule labels (+ the content/machinery slot sets) and the
//! device -> ControlFrame populate systems the menu/host layers anchor
//! to.

mod input_systems;
mod schedule;

pub use ambition_platformer_primitives::schedule::{
    BossSteerSlot, CombatSet, PresentationSetupSet, SandboxSet, SimulationSetupSet,
};
#[cfg(feature = "input")]
pub use input_systems::{
    apply_menu_frame_to_cutscene_request, attach_player_input_components,
    populate_control_frame_from_actions, populate_menu_control_frame_from_actions,
    toggle_player_trail_emission_from_actions, MenuNavConsume,
};
pub use schedule::configure_sandbox_sets;
===== control/mod.rs =====
//! **The local control seam** — device frame → slot → the body carrying that
//! slot's player brain.
//!
//! This is not player-centrism; it is the wire between a human and a body, and
//! naming it is most of what "player-ness is a brain and a slot, not a directory"
//! means. Read it in order:
//!
//! 1. [`components`] — the state. `LocalPlayer` (this slot's input is local),
//!    `PlayerInputFrame` (a body's own frame), `SlotGestures` /
//!    [`SlotInteractionState`] (a CONTROLLER's gestures, which follow it onto
//!    whatever body it drives).
//! 2. [`input_systems`] — the device layer: edge/timer derivation and gesture
//!    recognition off the raw `ControlFrame`.
//! 3. [`slots`] — the two bridges: device→slot, then slot→body.
//! 4. [`queries`] — slot-explicit player lookups, so a call site says whether it
//!    means "the primary player" or "every player".
//!
//! **Downstream of this module, nothing holds `Res<ControlFrame>`.** A body reads
//! its own `PlayerInputFrame`, or its brain's `ActorControl`
//! (`ambition_characters::actor::control` — the brain→body contract, the far end
//! of this same wire). `ambition_runtime/tests/control_frame_lint.rs` enforces it,
//! and its allowlist is almost exactly this module's contents.
//!
//! Extracted from `crate::avatar` in the S5/S6 fold (refactor-chain R6c): the
//! slot machinery was never player-only state, and keeping it under `player/`
//! was one of the reasons that module read as a universal dependency sink.

pub mod components;
===== affordances/mod.rs =====
//! Player affordances: "what would each button do right now?"
//!
//! The affordance table is the single source of truth bridging player
//! input + player state + world state to the verb each input would
//! invoke this frame. The HUD reads it to label each on-screen button;
//! gameplay code (today: nothing; future: the attack / jump / shield
//! subsystems) will read the same table so the HUD can never disagree
//! with what actually fires when a button is pressed.
//!
//! ## Shape
//!
//! - [`intent::PlayerIntent`] — pure player-driven input intent
//!   (directional aim today, motion-input history later).
//! - Per-verb variant enums in [`variants`] — closed sets describing
//!   every label/outcome a verb can take (`AttackVariant::DAir`,
//!   `JumpVariant::Climb`, …).
//! - Per-verb pure [`resolvers`] — `(intent, body, world) -> variant`,
//!   trivially unit-testable, callable by gameplay or HUD.
//! - [`interactable_proximity::NearestInteractable`] — frame-snapshot
//!   resource describing the nearest interactable's classification.
//! - [`PlayerAffordances`] resource (this module) — the denormalized
//!   table of variants for every verb, computed once per frame.
//! - [`AffordancesPlugin`] — wires the three compute systems
//!   (intent → proximity → affordances) into the schedule.
//!
//! ## What this replaces
//!
//! The previous design had a flat `PlayerActionContext` POD struct +
===== encounter/mod.rs =====
//! Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not
//! scripted) — distinct from `crate::boss_encounter`, which is one specific
//! scripted boss fight with hand-authored phases.
//!
//! An "encounter" is a sequence of mob waves with explicit lock / unlock
//! semantics: entering the trigger zone starts it, exits seal until all waves
//! are defeated, player death resets/unlocks, all-defeated → cleared + exits
//! unlock. Any number of encounters coexist via `EncounterRegistry`.
//!
//! Facade module. Authored data, registry resources, event vocabulary, music
//! request resources, reward math, and the headless state machine live in
//! `ambition_encounter`. Gameplay-core keeps the adapters that still touch LDtk,
//! ECS spawning, player/body queries, feature overlays, banners, save/quest
//! plumbing, and schedule sets.

use ambition_platformer_primitives::schedule::SimScheduleExt;
mod events;
mod loading;
mod lock_walls;
mod music;
mod registry;
mod rewards;
mod spec;
mod state;
mod switches;
mod systems;

pub use ambition_encounter::{
===== body_mode/mod.rs =====
//! Sandbox-side body-mode driver: facade re-exporting [`update_body_mode`].
//!
//! [`mechanics`] owns the whole driver — the crouch / climb / morph-ball
//! / stand-up state transitions read from input + contact state and ask
//! the engine to flip the player's `BodyMode`. (Morph-ball sprite visuals
//! live elsewhere, not in this module.)

mod mechanics;

pub use mechanics::update_body_mode;

use bevy::prelude::Component;

/// Per-body body-mode capability kit: which posture changes THIS body can
/// physically perform. The body-mode driver is capability-gated on this — a body
/// only crouches / morphs / climbs if it carries the matching flag, so the input
/// is a no-op for a body that lacks the capability (never a fallback to the home
/// avatar). Presence-gated: a body WITHOUT this component never body-modes at all.
///
/// This is the body-mode analogue of the movement kit on
/// [`crate::combat::CombatCapabilities`] (`can_blink` / `can_fly` / …): the
/// controller only *attempts* a posture change; the body decides whether it can.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyModeCapabilities {
    /// Can duck into a shorter Crouch stance while holding down + grounded.
    pub can_crouch: bool,
    /// Can curl into the smallest MorphBall stance (double-tap down).
    pub can_morph: bool,
Let me find the policy config that defines the render/actors boundary, and move on to sim_view analysis in parallel.

Read tracks.md
Bash Find policy config and seam docs
IN
echo "=== policy config files ===" && find . -path ./target -prune -o \( -name '*.toml' -o -name '*.json' \) -print 2>/dev/null | xargs grep -l "forbid\|denylist\|ambition_render\|ambition_actors" 2>/dev/null | grep -iv target | grep -i "polic\|boundar\|architect" | head && echo "=== search docs for render seam ===" && find docs -iname '*.md' 2>/dev/null | xargs grep -ln "sim_view\|read-model\|read model\|seam" 2>/dev/null | head

OUT
=== policy config files ===
./tests/ambition_workspace_policy/Cargo.toml
./tests/ambition_workspace_policy/policies/control_frame.toml
./tests/ambition_workspace_policy/policies/determinism.toml
./tests/ambition_workspace_policy/policies/game.toml
./tests/ambition_workspace_policy/policies/engine.toml
./tests/ambition_workspace_policy/fixtures/poison/manifest.toml
=== search docs for render seam ===
docs/archive/old-TODO-2026-07-10.md
docs/archive/TODO-character-catalog-and-hall.md
docs/archive/old-OVERNIGHT-TODO-2026-07-10.md
docs/archive/FEATURES.md
docs/vision/reference-material.md
docs/journals/content-authoring-pain-points.md
docs/brainstorms/technical_brainstorm.md
docs/brainstorms/after_intro_brainstorm.md
docs/tools/ldtk-tools.md
docs/recipes/adding-a-character.md
Now the workspace policy enforcement layer. Let me read the test harness and enumerate policies.

Bash Read sim_view lib.rs public API
IN
echo "=== sim_view lib.rs (public API) ===" && cat crates/ambition_sim_view/src/lib.rs

OUT
=== sim_view lib.rs (public API) ===
//! **[the observation boundary]** — the `SimView` read-model (E4).
//!
//! Everything here is a plain-data snapshot of sim state, rebuilt once per
//! sim tick by extraction systems that run LAST in the sim tail
//! (`SandboxSet::FeatureViewSync`) or as tail observers after
//! `CoreSimulation` (the camera resolve). Builders are pure functions of sim
//! state — no caching across ticks, no `Entity`/`Handle` borrows in the
//! rows — so every observer (render, RL observation, netcode confirmation,
//! the fighter brain, slower-light shaders) consumes the SAME facts.
//!
//! Render depends on THIS crate for sim facts; it never queries the sim
//! heart's live components (the boundary test in `ambition_render` pins
//! that).

use ambition_platformer_primitives::schedule::SimScheduleExt;
mod anim_index;
pub mod camera_snapshot;
mod facts;
mod pose_view;
mod view_index;

pub use anim_index::{
    rebuild_actor_anim_index, rebuild_boss_frame_index, ActorAnimFrame, ActorAnimIndex,
    ActorSpriteData, BossFrameIndex, BossFrameView, HazardLaneFact,
};
pub use facts::*;
pub use pose_view::{
    rebuild_body_pose_views, rebuild_shield_rings_view, BodyPoseView, ShieldRingFact,
    ShieldRingsView,
};
pub use view_index::{
    rebuild_actor_render_index, rebuild_boss_render_index, rebuild_feature_view_index,
    rebuild_nameplate_index, ActorRenderIndex, ActorRenderView, BossRenderIndex, BossRenderView,
    FeatureViewIndex, NameplateFact, NameplateIndex,
};

/// Rebuilds the observation read-models once per frame, sim-side:
/// [`FeatureViewIndex`] (geometry/state for every feature),
/// [`ActorRenderIndex`] / [`BossRenderIndex`] (materialized identity facts),
/// [`NameplateIndex`], [`BossFrameIndex`], the per-actor POSE snapshot
/// ([`ActorAnimIndex`]: overlay advance + anim pick), the player-bodied
/// [`BodyPoseView`] components, and [`ShieldRingsView`]. All let observers
/// read a snapshot instead of live-querying the sim's ECS.
pub struct FeatureViewSyncSchedulePlugin;

impl bevy::prelude::Plugin for FeatureViewSyncSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs;
        // Owned here (anti-god rule 5): the plugin that rebuilds the index
        // initializes it; consumers only read.
        app.init_resource::<ActorAnimIndex>();
        app.init_resource::<ShieldRingsView>();
        app.init_resource::<BossFrameIndex>();
        app.init_resource::<NameplateIndex>();
        app.add_systems(
            sim,
            (
                // The nameplate rows prefer the feature view's geometry, so
                // they rebuild strictly after it (same-frame read).
                (rebuild_feature_view_index, rebuild_nameplate_index).chain(),
                rebuild_actor_render_index,
                rebuild_boss_render_index,
                rebuild_boss_frame_index,
                // Overlay clocks advance right before their one reader
                // rebuilds the pose snapshot (§A9 ordering, preserved). The
                // overlay ADVANCE mutates sim components, so it stays defined
                // in the sim heart; this plugin only schedules it.
                (
                    ambition_actors::features::advance_actor_anim_overlays,
                    rebuild_actor_anim_index,
                )
                    .chain(),
                // Player-bodied pose components + the pooled shield-ring rows —
                // the per-body half of the pose read-model (E4 slices 1–4).
                rebuild_body_pose_views,
                rebuild_shield_rings_view,
            )
                .in_set(ambition_platformer_primitives::schedule::SandboxSet::FeatureViewSync),
        );
    }
}
Bash LOC by subdir in ambition_actors
IN
for d in crates/ambition_actors/src/*/; do loc=$(find "$d" -name '*.rs' | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}'); echo "$loc $(basename $d)"; done | sort -rn; find crates/ambition_actors/src -maxdepth 1 -name '*.rs' | xargs wc -l | tail -1

OUT
27266 features
6173 boss_encounter
4316 avatar
4316 abilities
2777 character_sprites
2121 affordances
1874 items
1776 projectile
1704 encounter
1645 dev
1612 world
1331 persistence
1284 audio
1196 time
1173 session
989 control
854 body_mode
812 menu
687 enemy_projectile
661 assets
584 schedule
543 gravity
515 dialog
458 music
357 character_roster
118 shrine
102 ability_cooldown
88 host
71 platformer_runtime
27 quest
10 effects
 1269 total
Read policy.rs
Now let me examine projectiles, portal split, persistence/load, and combat events/on_hit.

Let me verify a couple of dependency questions relevant to the merge analysis.

sprite_sheet clearly has gameplay-binding leaked in. Now let me analyze the audio/sfx/sfx_bank triple in parallel.

Let me examine the actor keystone, boss_encounter facade, starting_character, and character_sprites for content specificity.

Read lib.rs
Read actor.rs
Read kind.rs
Read mod.rs
Read lib.rs
Bash Audio triple: Cargo + tree + LOC
IN
cd /home/joncrall/code/ambition/crates && for c in ambition_audio ambition_sfx ambition_sfx_bank; do echo "======== $c ========"; echo "--- Cargo description + deps ---"; grep -E "^description|path = " $c/Cargo.toml; echo "--- files + LOC ---"; find $c -name '*.rs' -not -path '*/target/*' | xargs wc -l | sort -rn | head -15; echo ""; done

OUT
======== ambition_audio ========
--- Cargo description + deps ---
description = "Authored-audio stack: data schema, Kira playback library, adaptive music director."
ambition_sfx = { path = "../ambition_sfx" }
--- files + LOC ---
  5381 total
   720 ambition_audio/src/catalog.rs
   606 ambition_audio/src/library.rs
   577 ambition_audio/src/music/catalog.rs
   508 ambition_audio/src/selection.rs
   498 ambition_audio/src/music/director/adaptive.rs
   450 ambition_audio/src/bank_asset.rs
   348 ambition_audio/src/music/director/mod.rs
   343 ambition_audio/src/render.rs
   273 ambition_audio/src/spec.rs
   190 ambition_audio/src/music/channels.rs
   167 ambition_audio/src/music/director/simple.rs
   126 ambition_audio/src/web_unlock.rs
   124 ambition_audio/src/music/mod.rs
   111 ambition_audio/src/output.rs

======== ambition_sfx ========
--- Cargo description + deps ---
description = "SFX runtime contract for Ambition: SfxId, SfxClip, SfxProvider trait, and Bank/Filesystem/Silent/Layered providers."
ambition_sfx_bank = { path = "../ambition_sfx_bank" }
--- files + LOC ---
  788 total
  448 ambition_sfx/src/lib.rs
  239 ambition_sfx/src/ids.rs
  101 ambition_sfx/src/message.rs

======== ambition_sfx_bank ========
--- Cargo description + deps ---
description = "Reader + format spec for Ambition's binary SFX bank file (.sfxbank). Pure data; no audio or Bevy deps."
--- files + LOC ---
539 ambition_sfx_bank/src/lib.rs
Let me extract every declared policy from the TOML files and read the custom scanners.

Bash Check dependency directions and planning docs
IN
cd /home/joncrall/code/ambition
echo "=== does ldtk_map depend on world (adapter direction)? ==="
grep -nE 'ambition_world|ambition_engine_core' crates/ambition_ldtk_map/Cargo.toml
echo
echo "=== who depends on ambition_time? ==="
grep -rl 'ambition_time' crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's#/Cargo.toml##;s#.*/##' | sort
echo
echo "=== who depends on ambition_gameplay_trace? ==="
grep -rl 'ambition_gameplay_trace' crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's#/Cargo.toml##;s#.*/##' | sort
echo
echo "=== crate-remap / decomposition planning docs present? ==="
ls docs/planning/engine/ 2>/dev/null | head; echo "---"; ls crate-remap-idea.md decomposition.md 2>/dev/null; find docs -iname '*decomp*' 2>/dev/null | head

OUT
=== does ldtk_map depend on world (adapter direction)? ===
19:ambition_engine_core = { path = "../ambition_engine_core" }
24:ambition_world = { path = "../ambition_world", default-features = false }

=== who depends on ambition_time? ===
ambition
ambition_actors
ambition_combat
ambition_content
ambition_render
ambition_runtime
ambition_sim_view
ambition_time
ambition_world

=== who depends on ambition_gameplay_trace? ===
ambition_actors
ambition_gameplay_trace
ambition_projectiles

=== crate-remap / decomposition planning docs present? ===
architecture.md
boss-design.md
boss-system.md
canonical-sprite-generators.md
collision-and-ccd.md
combat-model.md
data-driven-sprites-and-characters.md
decomposition.md
encounter-orchestration.md
fable-final-audit-2026-07-07.md
---
crate-remap-idea.md
docs/planning/engine/decomposition.md
Let me read persistence, load, dev_tools, portal_presentation, and combat events/on_hit/hazards.

Bash Extract policy id/kind/rationale from toml
IN
cd tests/ambition_workspace_policy/policies; for f in repository.toml engine.toml game.toml control_frame.toml determinism.toml module_size.toml; do echo "########## $f ##########"; grep -nE '^\s*(id|kind|scope|rationale)\s*=' $f | head -200; echo; done

OUT
########## repository.toml ##########
11:id = "repository.core-crates-registered"
12:scope = "repository"
13:kind = "workspace-member"
16:rationale = "The tier anchors (foundation engine_core, headless runtime, content, app) and the policy package itself must stay registered workspace members, or `cargo test --workspace` silently stops covering them."
28:id = "repository.tier-manifests-exist"
29:scope = "repository"
30:kind = "required-path"
33:rationale = "The crate-tier layout (foundation crates under crates/, game crates under game/) is load-bearing; a missing tier anchor manifest means a crate move went wrong."

########## engine.toml ##########
12:id = "engine.engine-core-is-foundation-clean"
13:scope = "engine"
14:kind = "forbidden-source-reference"
17:rationale = "ambition_engine_core is the bottom foundation crate: geometry, body contract, control-frame vocabulary. It must never name the headless composition tier, the content crate, or the app shell — those are strictly above it."
27:id = "engine.host-names-no-content"
28:scope = "engine"
29:kind = "dependency-denylist"
32:rationale = "The windowed host is the ENGINE face's presentation companion, not a game: it must never depend on ambition_content. Locks the E5-step-5 exit invariant from the scaffold onward."
40:id = "engine.host-source-names-no-content"
41:scope = "engine"
42:kind = "forbidden-source-reference"
45:rationale = "The moment the host moves a system that reaches for a content type, this fails instead of the boundary rotting. Docs deliberately MENTION the forbidden crate, so comments are exempt."
55:id = "engine.render-never-names-live-sim-state"
56:scope = "engine"
57:kind = "forbidden-source-reference"
60:rationale = "Presentation reads ONLY the ambition_sim_view read-model — never the sim heart's live Body* clusters, actor/boss cluster refs, control seams, or item/ability sim components. Reintroducing a Query<&BodyKinematics> in render fails here. Whole-identifier match so GroundItemVisual does not trip GroundItem; vocabulary/data types (FeatureView, CharacterAnim, Health-as-data) are deliberately NOT forbidden."
81:id = "engine.render-no-actor-crate-dependency"
82:scope = "engine"
83:kind = "dependency-denylist"
86:rationale = "F1.5 complete: ambition_render must not depend on or feature-forward to ambition_actors. Render and the machinery meet only through lower view/vocabulary crates."
94:id = "engine.render-source-names-no-actors"
95:scope = "engine"
96:kind = "forbidden-source-reference"
99:rationale = "F1.5 complete: render source must not name ambition_actors in code, even where the manifest dep is already gone."
109:id = "engine.world-ir-dependency-allowlist"
110:scope = "engine"
111:kind = "dependency-allowlist"
114:rationale = "ambition_world owns the backend-agnostic world IR (room graph, placements, platform math, composited collision world). It may name only explicit world-IR dependencies; the exact ratchet forces a legacy entry out of the allow-list as each placement branch dissolves."
135:id = "engine.render-decoupled-member"
136:scope = "engine"
137:kind = "workspace-member"
140:rationale = "ambition_render is the sandbox's renderer and must stay a registered workspace member; the sim machinery and render meet only through lower vocabulary/read-model crates."
146:id = "engine.actor-manifest-no-render"
147:scope = "engine"
148:kind = "dependency-denylist"
151:rationale = "The sim machinery must not depend on its renderer (render depends on the sim, never the reverse)."
158:id = "engine.actor-source-no-render"
159:scope = "engine"
160:kind = "forbidden-source-reference"
163:rationale = "ambition_actors must not reference the render crate in code either."
171:id = "engine.menu-crate-manifest-no-actors"
172:scope = "engine"
173:kind = "dependency-denylist"
176:rationale = "ambition_menu is the reusable menu renderer; the game owns menu content, so it must not depend on the actor machinery."
183:id = "engine.menu-crate-source-no-actors"
184:scope = "engine"
185:kind = "forbidden-source-reference"
188:rationale = "ambition_menu must stay content-free — no ambition_actors references in code."
196:id = "engine.persistence-crate-member"
197:scope = "engine"
198:kind = "workspace-member"
201:rationale = "ambition_persistence owns stored shapes and must stay a registered workspace member."
207:id = "engine.persistence-crate-manifest-purity"
208:scope = "engine"
209:kind = "dependency-denylist"
212:rationale = "ambition_persistence owns stored shapes, not menu/UI/game machinery."
219:id = "engine.persistence-crate-source-purity"
220:scope = "engine"
221:kind = "forbidden-source-reference"
224:rationale = "ambition_persistence source must stay free of menu/UI/game machinery imports."
232:id = "engine.encounter-crate-member"
233:scope = "engine"
234:kind = "workspace-member"
237:rationale = "ambition_encounter owns reusable encounter state/vocabulary and must stay a registered workspace member."
243:id = "engine.encounter-crate-manifest-purity"
244:scope = "engine"
245:kind = "dependency-denylist"
248:rationale = "ambition_encounter is reusable encounter state/vocabulary; the LDtk loader, ECS spawning, banners, and save/quest adapters stay above it, so it must not reach into sim/content/render/host crates."
255:id = "engine.encounter-crate-source-purity"
256:scope = "engine"
257:kind = "forbidden-source-reference"
260:rationale = "ambition_encounter source must not reach into sim/content/render/host crates."
268:id = "engine.host-crate-member"
269:scope = "engine"
270:kind = "workspace-member"
273:rationale = "ambition_host, the windowed-host face, must stay a registered workspace member."
279:id = "engine.host-manifest-no-actors"
280:scope = "engine"
281:kind = "dependency-denylist"
284:rationale = "F1.10: the windowed host wires presentation/input/runtime seams, not the actor-systems crate."
291:id = "engine.host-source-no-actors"
292:scope = "engine"
293:kind = "forbidden-source-reference"
296:rationale = "F1.10: host source and smoke fixtures should reach actor-owned setup only through runtime facades, never name ambition_actors:: directly."
307:id = "engine.platformer-runtime-content-free"
308:scope = "engine"
309:kind = "forbidden-source-reference"
312:rationale = "the platformer_runtime facade in ambition_actors must remain reusable and content-free"
321:id = "engine.ambition_world-member"
322:scope = "engine"
323:kind = "workspace-member"
326:rationale = "ambition_world must stay a registered workspace member"
332:id = "engine.ambition_world-manifest-deny"
333:scope = "engine"
334:kind = "dependency-denylist"
337:rationale = "ambition_world is the backend-agnostic world IR"
344:id = "engine.ambition_world-source-purity"
345:scope = "engine"
346:kind = "forbidden-source-reference"
349:rationale = "ambition_world source stays free of forbidden refs"
356:id = "engine.ldtk-map-member"
357:scope = "engine"
358:kind = "workspace-member"
361:rationale = "ambition_ldtk_map registered"
367:id = "engine.ldtk-map-manifest-deny"
368:scope = "engine"
369:kind = "dependency-denylist"
372:rationale = "ambition_ldtk_map converts into world IR without depending on the sim heart"
379:id = "engine.ldtk-map-source-purity"
380:scope = "engine"
381:kind = "forbidden-source-reference"
384:rationale = "ambition_ldtk_map source must not reach upward into sim/app/render/content (test fixture path string exempt)"
394:id = "engine.ambition_items-member"
395:scope = "engine"
396:kind = "workspace-member"
399:rationale = "ambition_items must stay a registered workspace member"
405:id = "engine.ambition_items-absent-paths"
406:scope = "engine"
407:kind = "forbidden-path"
410:rationale = "ambition_items: removed facades stay gone"
416:id = "engine.ambition_items-manifest-deny"
417:scope = "engine"
418:kind = "dependency-denylist"
421:rationale = "ambition_items owns reusable catalog/shop data; UI state lives in ambition_inventory_ui"
428:id = "engine.ambition_items-source-purity"
429:scope = "engine"
430:kind = "forbidden-source-reference"
433:rationale = "ambition_items source stays free of forbidden refs"
442:id = "engine.ambition_inventory_ui-member"
443:scope = "engine"
444:kind = "workspace-member"
447:rationale = "ambition_inventory_ui must stay a registered workspace member"
453:id = "engine.ambition_inventory_ui-manifest-allow"
454:scope = "engine"
455:kind = "dependency-allowlist"
458:rationale = "ambition_inventory_ui is reusable menu-navigation state over ambition_ui_nav"
465:id = "engine.ambition_inventory_ui-source-purity"
466:scope = "engine"
467:kind = "forbidden-source-reference"
470:rationale = "ambition_inventory_ui source stays free of forbidden refs"
479:id = "engine.ambition_settings_menu-member"
480:scope = "engine"
481:kind = "workspace-member"
484:rationale = "ambition_settings_menu must stay a registered workspace member"
490:id = "engine.ambition_settings_menu-manifest-allow"
491:scope = "engine"
492:kind = "dependency-allowlist"
495:rationale = "ambition_settings_menu is the pure, renderer-agnostic settings IR"
502:id = "engine.ambition_settings_menu-source-purity"
503:scope = "engine"
504:kind = "forbidden-source-reference"
507:rationale = "ambition_settings_menu source stays free of forbidden refs"
516:id = "engine.ambition_dev_tools-member"
517:scope = "engine"
518:kind = "workspace-member"
521:rationale = "ambition_dev_tools must stay a registered workspace member"
527:id = "engine.ambition_dev_tools-manifest-allow"
528:scope = "engine"
529:kind = "dependency-allowlist"
532:rationale = "ambition_dev_tools is foundational dev-tool state; overlays/sim stay out"
539:id = "engine.ambition_dev_tools-source-purity"
540:scope = "engine"
541:kind = "forbidden-source-reference"
544:rationale = "ambition_dev_tools source stays free of forbidden refs"
553:id = "engine.ambition_dialog-member"
554:scope = "engine"
555:kind = "workspace-member"
558:rationale = "ambition_dialog must stay a registered workspace member"
564:id = "engine.ambition_dialog-manifest-allow"
565:scope = "engine"
566:kind = "dependency-allowlist"
569:rationale = "ambition_dialog is the reusable dialogue runtime; game bindings stay host-side"
576:id = "engine.ambition_dialog-source-purity"
577:scope = "engine"
578:kind = "forbidden-source-reference"
581:rationale = "ambition_dialog source stays free of forbidden refs"
590:id = "engine.ambition_interaction-manifest-deny"
591:scope = "engine"
592:kind = "dependency-denylist"
595:rationale = "ambition_interaction is a content-free data model over the actor/geometry foundations"
602:id = "engine.ambition_interaction-source-purity"
603:scope = "engine"
604:kind = "forbidden-source-reference"
607:rationale = "ambition_interaction source stays free of forbidden refs"
616:id = "engine.ambition_vfx-manifest-deny"
617:scope = "engine"
618:kind = "dependency-denylist"
621:rationale = "ambition_vfx is the reusable effect substrate; never depends on the game lib or cast crate"
628:id = "engine.ambition_vfx-source-purity"
629:scope = "engine"
630:kind = "forbidden-source-reference"
633:rationale = "ambition_vfx source stays free of forbidden refs"
642:id = "engine.ambition_projectiles-member"
643:scope = "engine"
644:kind = "workspace-member"
647:rationale = "ambition_projectiles must stay a registered workspace member"
653:id = "engine.ambition_projectiles-manifest-deny"
654:scope = "engine"
655:kind = "dependency-denylist"
658:rationale = "ambition_projectiles is the projectile MODEL \u2014 no sim-heart/combat/brain/host/content coupling"

########## game.toml ##########
10:id = "game.content-and-app-homes-exist"
11:scope = "game"
12:kind = "required-path"
15:rationale = "The named-content crate and the app-assembly shell are the two game-tier homes the composition root depends on; a missing src root means a move went wrong."
28:id = "game.no-legacy-runtime-in-app-src"
29:scope = "game"
30:kind = "forbidden-source-reference"
33:rationale = "The ECS migration deleted the SandboxRuntime/FeatureRuntime god-objects, the runtime.player shadow cache, the feature_runtime_phase helper, and the monolithic ae::Player aggregate. Canonical replacements live on dedicated Body* components/systems. Re-introducing any of these spellings means a shadow scratchpad crept back in. Test code and archived doc refs are exempt; a single reviewed occurrence opts out with ALLOW_LEGACY_RUNTIME."
58:id = "game.umbrella-and-demo-members"
59:scope = "game"
60:kind = "workspace-member"
63:rationale = "umbrella + demo homes are registered members"
69:id = "game.umbrella-not-app-shell"
70:scope = "game"
71:kind = "dependency-denylist"
74:rationale = "the ambition facade is an engine surface, not an app/content shell"
81:id = "game.umbrella-exposes-engine-surface"
82:scope = "game"
83:kind = "file-contains"
86:rationale = "ambition facade exposes the engine surface"
93:id = "game.demo-sanic-umbrella-only"
94:scope = "game"
95:kind = "dependency-allowlist"
98:rationale = "Sanic demo depends on the umbrella plus its own content only"
105:id = "game.demo-pocket-umbrella-only"
106:scope = "game"
107:kind = "dependency-allowlist"
110:rationale = "Pocket proves a fourth provider depends only on the umbrella"
117:id = "game.demo-mary-o-umbrella-only"
118:scope = "game"
119:kind = "dependency-allowlist"
122:rationale = "Mary-O demo depends on the umbrella plus its own content only"
129:id = "game.demos-reach-engine-through-facade"
130:scope = "game"
131:kind = "forbidden-source-reference"
134:rationale = "demos reach the engine through the ambition facade"
143:id = "game.app-umbrella-plus-local"
144:scope = "game"
145:kind = "dependency-allowlist"
148:rationale = "ambition_app uses the umbrella plus app-local content/extension crates and the reusable game providers it hosts"
162:id = "game.app-reaches-lower-through-facade"
163:scope = "game"
164:kind = "forbidden-source-reference"
167:rationale = "ambition_app code should reach reusable lower crates through the ambition facade"
177:id = "game.kaleidoscope-member"
178:scope = "game"
179:kind = "workspace-member"
182:rationale = "kaleidoscope is a registered member"
188:id = "game.kaleidoscope-menu-model-only"
189:scope = "game"
190:kind = "dependency-allowlist"
193:rationale = "kaleidoscope is an engine extension over the menu model"
200:id = "game.kaleidoscope-no-game-machinery"
201:scope = "game"
202:kind = "forbidden-source-reference"
205:rationale = "kaleidoscope names no game/app/content machinery"
212:id = "game.menu-crate-lunex-free"
213:scope = "game"
214:kind = "file-omits"
217:rationale = "ambition_menu is bevy_lunex-free; the cube renderer is the extension crate"
226:id = "game.named-content-paths"
227:scope = "game"
228:kind = "required-path"
231:rationale = "named Ambition content crate paths exist"
237:id = "game.content-plugin-composes"
238:scope = "game"
239:kind = "file-contains"
242:rationale = "AmbitionContentPlugin composes the content sub-plugins"
249:id = "game.app-installs-content-plugin"
250:scope = "game"
251:kind = "file-contains"
254:rationale = "app installs AmbitionContentPlugin"
261:id = "game.app-sim-resources-no-inline-content"
262:scope = "game"
263:kind = "file-omits"
266:rationale = "app/sim_resources.rs must not construct named content inline"
273:id = "game.app-plugins-no-inline-item-roster"
274:scope = "game"
275:kind = "file-omits"
278:rationale = "item roster installs through the content plugin"
287:id = "game.app-plugins-no-reown"
288:scope = "game"
289:kind = "file-omits"
292:rationale = "app/plugins.rs must not reintroduce subsystem-owned registrations"
301:id = "game.app-sim-systems-no-redefine"
302:scope = "game"
303:kind = "file-omits"
306:rationale = "app/sim_systems.rs must not re-define library-owned systems"
313:id = "game.engine-schedule-references-moved"
314:scope = "game"
315:kind = "file-contains"
318:rationale = "engine player schedule references the moved systems via library paths"
325:id = "game.app-keeps-host-reset-systems"
326:scope = "game"
327:kind = "file-contains"
330:rationale = "host/reset-bound systems remain defined in the app"
339:id = "game.abilities-under-layer"
340:scope = "game"
341:kind = "required-path"
344:rationale = "player abilities live under src/abilities/"
350:id = "game.abilities-not-at-crate-root"
351:scope = "game"
352:kind = "forbidden-path"
355:rationale = "abilities must not live at the crate root"
361:id = "game.app-composes-engine-plugins"
362:scope = "game"
363:kind = "file-contains"
366:rationale = "app composes the engine sim through the umbrella PlatformerEnginePlugins"
373:id = "game.engine-composes-abilities"
374:scope = "game"
375:kind = "file-contains"
378:rationale = "PlatformerEnginePlugins composes abilities through AmbitionAbilitiesPlugin"
387:id = "game.lib-menu-no-host-files"
388:scope = "game"
389:kind = "forbidden-path"
392:rationale = "lib menu should not regain app-host/backend menu files"
398:id = "game.lib-menu-keeps-map"
399:scope = "game"
400:kind = "required-path"
403:rationale = "lib menu keeps only map hydration/UI adapters"
409:id = "game.app-menu-host-stack"
410:scope = "game"
411:kind = "required-path"
414:rationale = "app menu host stack exists"
422:id = "game.lib-dev-no-overlays"
423:scope = "game"
424:kind = "forbidden-path"
427:rationale = "presentation-only overlays + dev-tool state/profiling moved out of the lib"
433:id = "game.lib-dev-keeps-trace"
434:scope = "game"
435:kind = "required-path"
438:rationale = "sim-coupled trace recorder stays sim-side"
444:id = "game.dev-tools-owns-state"
445:scope = "game"
446:kind = "required-path"
449:rationale = "ambition_dev_tools owns dev-tool state/profiler/persistence"
455:id = "game.app-dev-overlays"
456:scope = "game"
457:kind = "required-path"
460:rationale = "app dev overlay files exist"
466:id = "game.dev-consumers-use-dev-tools"
467:scope = "game"
468:kind = "forbidden-source-reference"
471:rationale = "external consumers name ambition_dev_tools directly; ambition_actors::dev keeps only trace"
480:id = "game.menu-map-state-home"
481:scope = "game"
482:kind = "required-path"
485:rationale = "renderer-agnostic MapMenuState lives in ambition_menu::map"
491:id = "game.actor-menu-map-model-gone"
492:scope = "game"
493:kind = "forbidden-path"
496:rationale = "actor-side map model facade deleted"
502:id = "game.map-state-consumers-menu-crate"
503:scope = "game"
504:kind = "forbidden-source-reference"
507:rationale = "consumers name ambition_menu::map::MapMenuState"
516:id = "game.actor-menu-ir-gone"
517:scope = "game"
518:kind = "forbidden-path"
521:rationale = "actor-side settings/menu IR facade deleted"
527:id = "game.app-menu-settings-lower-crates"
528:scope = "game"
529:kind = "forbidden-source-reference"
532:rationale = "app menu hosts import stored settings from ambition_persistence and menu IR from ambition_settings_menu"
541:id = "game.providers-are-host-ignorant"
542:scope = "game"
543:kind = "forbidden-source-reference"
546:rationale = "A provider expresses one game; the HOST decides home/initial routes, launcher, and process exit. A provider that names the Ambition launcher route, the shell-hosted marker, a host crate, or issues QuitToHome/ExitProcess routing itself has grown a host dependency — the exact coupling the host-relative return contract exists to prevent. (Providers' own *_LAUNCHER_ROUTE consts are conventions their standalone hosts choose to use; naming ANOTHER host's home is the defect.)"

########## control_frame.toml ##########
19:rationale = "ControlFrame allowlist: only the input layer may hold the global device frame; a body/sim holder is silently slot-0-only and breaks possession/co-op/netcode slot routing (N1). Every allowed holder is a justified input-layer bridge or a named multiplayer TODO."
26:scope = "engine"
29:scope = "engine"
32:scope = "engine"
35:scope = "engine"
38:scope = "engine"
41:scope = "engine"
44:scope = "engine"
47:scope = "engine"
50:scope = "engine"
53:scope = "engine"
56:scope = "engine"
59:scope = "engine"
62:scope = "engine"
65:scope = "engine"
68:scope = "engine"
71:scope = "engine"
74:scope = "game"

########## determinism.toml ##########
15:rationale = "N0.3 determinism lints: no ambient randomness, no wall-clock reads, no std-hash-order semantics, and Entity is never an ordering key — the standing enforcement that keeps level-2 same-build determinism true."
43:scope = "engine"
46:scope = "engine"
49:scope = "engine"
52:scope = "engine"
55:scope = "engine"
58:scope = "engine"
61:scope = "engine"
64:scope = "engine"
67:scope = "engine"
70:scope = "engine"
73:scope = "engine"
76:scope = "engine"
79:scope = "engine"
82:scope = "engine"
85:scope = "engine"
88:scope = "engine"
93:scope = "game"
96:scope = "game"
99:scope = "game"

########## module_size.toml ##########
13:id = "engine.module-size"
14:scope = "engine"
19:rationale = "Production modules over 5000 lines require deliberate review; split only when coherent boundaries exist."
Read lib.rs
Read lib.rs
Read lib.rs
Read lib.rs
Let me examine the SfxId content enum and the audio catalog to judge the three-crate split.

Let me nail down the boss_encounter↔features tangle, check starting_character for content leaks, and the goblin sprite reference.

Let me read the custom scanners and migration matrix, plus the remaining engine policy ids.

Bash Read SfxId enum content
IN
cd /home/joncrall/code/ambition/crates && echo "=== sfx/ids.rs: SfxId enum ===" && grep -n "enum SfxId" -A 50 ambition_sfx/src/ids.rs | head -60

OUT
=== sfx/ids.rs: SfxId enum ===
Let me read portal_presentation, dialog, combat events/hazards, and search for game-specific identifiers.

Read lib.rs
Bash Remaining engine ids + migration matrix
IN
cd tests/ambition_workspace_policy; echo "=== remaining engine.toml policy ids (from line 660) ==="; grep -nE '^\s*id\s*=' policies/engine.toml | sed -n '60,200p'; echo; echo "=== migration_matrix.toml (head 120) ==="; sed -n '1,120p' migration_matrix.toml

OUT
=== remaining engine.toml policy ids (from line 660) ===
771:id = "engine.ambition_portal-manifest-allow"
783:id = "engine.ambition_portal-source-purity"
795:id = "engine.portal-exposes-plugin"
807:id = "engine.portal-plugin-impl"
819:id = "engine.actors-no-portal-facade"
833:id = "engine.ambition_portal_presentation-member"
844:id = "engine.ambition_portal_presentation-extracted-paths"
855:id = "engine.ambition_portal_presentation-manifest-deny"
867:id = "engine.ambition_portal_presentation-manifest-allow"
879:id = "engine.ambition_portal_presentation-source-purity"
891:id = "engine.portal-presentation-plugin"
903:id = "engine.portal-mechanic-no-renderer-dep"
915:id = "engine.actors-no-portal-presentation-dep"
929:id = "engine.ambition_time-member"
940:id = "engine.ambition_time-extracted-paths"
951:id = "engine.ambition_time-manifest-deny"
963:id = "engine.ambition_time-source-purity"
975:id = "engine.time-exposes-plugin"
987:id = "engine.time-facades-removed"
998:id = "engine.time-world-time-bridge"
1010:id = "engine.time-sandbox-policy-exists"
1023:id = "engine.ambition_input-extracted-paths"
1034:id = "engine.ambition_input-absent-paths"
1045:id = "engine.ambition_input-manifest-deny"
1057:id = "engine.actors-no-input-shim"
1069:id = "engine.persistence-reexports-input-controls"
1081:id = "engine.actors-settings-surfaces-controls"
1095:id = "engine.ambition_touch_input-member"
1106:id = "engine.ambition_touch_input-manifest-deny"
1118:id = "engine.ambition_touch_input-manifest-allow"
1130:id = "engine.ambition_touch_input-source-purity"
1142:id = "engine.touch-input-render-edge"
1156:id = "engine.f2-schedule-labels-primitive-home"
1170:id = "engine.f2-asset-vocab-lower-crates"
1184:id = "engine.f2-character-anim-sprite-sheet"
1198:id = "engine.f2-encounter-vocab-encounter-crate"
1212:id = "engine.menu-backend-vocab-menu-crate"
1226:id = "engine.dialog-vocab-dialog-crate"
1240:id = "engine.developer-persistence-dev-tools"
1254:id = "engine.audio-facade-audio-crate"
1268:id = "engine.machinery-imports-no-content"
1283:id = "engine.combat-kit-content-free"
1297:id = "engine.presentation-no-archetype-enum"
1312:id = "engine.enemy-sim-reads-data"
1327:id = "engine.characters-content-free"
1341:id = "engine.sim-no-presentation-import"
1356:id = "engine.game-mode-schedule-vocab"
1372:id = "engine.platformer-primitives-extracted"
1383:id = "engine.platformer-primitives-no-actors"
1395:id = "engine.platformer-primitives-projectile-generic-spec"
1407:id = "engine.platformer-primitives-projectile-generic-body"
1419:id = "engine.platformer-primitives-projectile-generic-collision"
1431:id = "engine.platformer-primitives-projectile-generic-mod"
1443:id = "engine.actor-platformer-facade-shrunk"
1454:id = "engine.actor-platformer-facade-reexports"
1466:id = "engine.actor-orientation-facade"
1478:id = "engine.actor-physics-facade"
1492:id = "engine.asset-manager-member"
1503:id = "engine.asset-manager-no-audio"
1515:id = "engine.asset-manager-no-backend-providers"
1527:id = "engine.asset-manager-no-sfx-adapter"
1538:id = "engine.asset-manager-no-sfx-feature"
1552:id = "engine.projectile-schedule-facade"
1563:id = "engine.projectile-schedule-facade-owns"
1575:id = "engine.projectile-schedule-facade-omits"
1587:id = "engine.app-no-actor-projectile-steppers"
1599:id = "engine.content-no-actor-projectile-steppers"
1612:id = "engine.runtime-actor-projectile-centralized"
1627:id = "engine.runtime-member"
1638:id = "engine.runtime-required-deps"
1650:id = "engine.runtime-manifest-allow"
1662:id = "engine.runtime-manifest-deny"
1674:id = "engine.runtime-source-no-upper"
1689:id = "engine.control-frame-home"
1700:id = "engine.control-frame-exported"
1712:id = "engine.characters-no-input-dep"
1724:id = "engine.characters-no-input-source"
1736:id = "engine.input-reexports-control-frame"
1750:id = "engine.f2-actor-facades-gone"
1761:id = "engine.f2-actor-lib-no-facades"
1773:id = "engine.f2-possession-no-controlled-subject-reexport"
1785:id = "engine.f2-consumers-use-canonical-crates"
1799:id = "engine.actor-dialog-no-runtime-reexport"
1811:id = "engine.actor-yarn-no-runtime-reexport"
1825:id = "engine.actor-persistence-no-dev-tools-alias"
1839:id = "engine.touch-input-extracted"
1850:id = "engine.touch-input-not-in-app"
1861:id = "engine.app-wires-touch-plugin"
1873:id = "engine.app-no-in-app-mobile-input"
1887:id = "engine.non-portal-mechanics-runtime-raycast"
1901:id = "engine.portal-plugin-no-host-item-subsystem"
1913:id = "engine.portal-wiring-orders-against-set"
1925:id = "engine.portal-wiring-uses-item-set"
1939:id = "engine.portal-core-no-content-roster"
1955:id = "engine.portal-core-no-host-world-reset"
1971:id = "engine.music-director-content-agnostic"
1987:id = "engine.gravity-mechanic-extracted"
1998:id = "engine.gravity-visuals-in-render"
2009:id = "engine.gravity-owns-plugin"
2021:id = "engine.gravity-portal-independent"
2033:id = "engine.portal-no-gravity-symbols"
2049:id = "engine.portal-mechanic-crate-files"
2060:id = "engine.actor-portal-facade-gone"
2071:id = "engine.portal-plugin-labels-transit"
2085:id = "engine.actor-clusters-require-sweep"
2097:id = "engine.actor-clusters-no-optional-sweep"
2109:id = "engine.portal-transit-uses-sweep"
2121:id = "engine.portal-transit-no-anchor"
2133:id = "engine.portal-lib-no-anchor"
2147:id = "engine.clock-resets-owner-only"
2160:id = "engine.runtime-applies-clock-reset"
2174:id = "engine.player-fallbacks-no-raw-query"
2186:id = "engine.player-fallback-save-sync-documented"
2198:id = "engine.player-fallback-update-documented"
2211:id = "engine.load-shell-core-members"
2222:id = "engine.load-core-dependency-purity"
2234:id = "engine.game-shell-dependency-purity"
2246:id = "engine.load-presentation-dependency-direction"
2258:id = "engine.load-shell-source-content-free"
2276:id = "engine.audio-authority-is-app-local"
2295:id = "engine.movement-model-is-never-optional"
2319:id = "engine.crawler-flag-is-spawn-selection-only"
2341:id = "engine.pose-writes-are-authority-only"
2382:id = "engine.mechanics-consume-the-resolved-frame"
2419:id = "engine.character-authority-is-app-local"
2470:id = "engine.sfx-emission-captures-exact-audio-owner"
2492:id = "engine.shell-audio-authority-is-explicit"

=== migration_matrix.toml (head 120) ===
# Migration matrix for game/ambition_app/tests/architecture_boundaries.rs.
#
# Every #[test] that was in that file (frozen canonical list in
# fixtures/architecture_boundaries_source_tests.txt) maps to exactly one
# disposition. `migration_matrix_is_complete` (self-test) asserts the bijection,
# that every `declarative`/`custom` destination resolves to a real policy, and
# that the matrix cannot lie: a `legacy-pending` entry's fn must STILL be in the
# legacy file, and a migrated/removed/retained entry's fn must be GONE from it.
#
# disposition ∈ { declarative, custom, retained, removed, legacy-pending }
#   declarative     — moved to policies/*.toml; `policies` lists the IDs.
#   custom          — moved to a src/custom/*.rs scanner; `policies` names its ID.
#   retained        — deliberately kept as a crate-local behavioral test; see note.
#   removed         — obsolete; see note for justification.
#   legacy-pending  — not yet migrated; still lives in the legacy file.


[[entry]]
old_test = "architecture_boundaries_umbrella_crate_and_demo_homes_exist"
disposition = "declarative"
policies = ["game.umbrella-and-demo-members", "game.umbrella-not-app-shell", "game.umbrella-exposes-engine-surface", "game.demo-sanic-umbrella-only", "game.demo-mary-o-umbrella-only", "game.demos-reach-engine-through-facade"]

[[entry]]
old_test = "architecture_boundaries_app_uses_umbrella_manifest_surface"
disposition = "declarative"
policies = ["game.app-umbrella-plus-local", "game.app-reaches-lower-through-facade"]

[[entry]]
old_test = "architecture_boundaries_render_and_actor_crates_are_decoupled"
disposition = "declarative"
policies = ["engine.render-decoupled-member", "engine.actor-manifest-no-render", "engine.actor-source-no-render", "engine.render-no-actor-crate-dependency", "engine.render-source-names-no-actors"]

[[entry]]
old_test = "architecture_boundaries_platformer_runtime_stays_content_free"
disposition = "declarative"
policies = ["engine.platformer-runtime-content-free"]

[[entry]]
old_test = "architecture_boundaries_platformer_runtime_crate_is_extracted"
disposition = "declarative"
policies = ["engine.platformer-primitives-extracted", "engine.platformer-primitives-no-actors", "engine.platformer-primitives-projectile-generic-spec", "engine.platformer-primitives-projectile-generic-body", "engine.platformer-primitives-projectile-generic-collision", "engine.platformer-primitives-projectile-generic-mod", "engine.actor-platformer-facade-shrunk", "engine.actor-platformer-facade-reexports", "engine.actor-orientation-facade", "engine.actor-physics-facade"]

[[entry]]
old_test = "architecture_boundaries_sim_does_not_import_presentation"
disposition = "declarative"
policies = ["engine.sim-no-presentation-import"]

[[entry]]
old_test = "architecture_boundaries_menu_crate_stays_content_free"
disposition = "declarative"
policies = ["engine.menu-crate-manifest-no-actors", "engine.menu-crate-source-no-actors"]

[[entry]]
old_test = "architecture_boundaries_persistence_crate_owns_stored_shapes_only"
disposition = "declarative"
policies = ["engine.persistence-crate-member", "engine.persistence-crate-manifest-purity", "engine.persistence-crate-source-purity"]

[[entry]]
old_test = "architecture_boundaries_world_ir_and_ldtk_backend_are_split"
disposition = "declarative"
policies = ["engine.ambition_world-member", "engine.ambition_world-manifest-deny", "engine.ambition_world-source-purity", "engine.ldtk-map-member", "engine.ldtk-map-manifest-deny", "engine.ldtk-map-source-purity"]

[[entry]]
old_test = "architecture_boundaries_projectiles_crate_is_model_only"
disposition = "declarative"
policies = ["engine.ambition_projectiles-member", "engine.ambition_projectiles-manifest-deny", "engine.ambition_projectiles-source-purity", "engine.projectiles-actor-stepper-generic", "engine.projectiles-visual-kind-owns-expiry", "engine.projectiles-tests-travel-with-kit", "engine.projectiles-no-actor-facade-tests", "engine.projectiles-actor-mod-no-engine-tests"]

[[entry]]
old_test = "architecture_boundaries_encounter_crate_is_state_only"
disposition = "declarative"
policies = ["engine.encounter-crate-member", "engine.encounter-crate-manifest-purity", "engine.encounter-crate-source-purity"]

[[entry]]
old_test = "architecture_boundaries_items_crate_is_catalog_and_shop_only"
disposition = "declarative"
policies = ["engine.ambition_items-member", "engine.ambition_items-absent-paths", "engine.ambition_items-manifest-deny", "engine.ambition_items-source-purity"]

[[entry]]
old_test = "architecture_boundaries_inventory_ui_crate_is_menu_state_only"
disposition = "declarative"
policies = ["engine.ambition_inventory_ui-member", "engine.ambition_inventory_ui-manifest-allow", "engine.ambition_inventory_ui-source-purity"]

[[entry]]
old_test = "architecture_boundaries_asset_manager_is_backend_generic"
disposition = "declarative"
policies = ["engine.asset-manager-member", "engine.asset-manager-no-audio", "engine.asset-manager-no-backend-providers", "engine.asset-manager-no-sfx-adapter", "engine.asset-manager-no-sfx-feature"]

[[entry]]
old_test = "architecture_boundaries_projectile_schedule_wiring_lives_in_runtime"
disposition = "declarative"
policies = ["engine.projectile-schedule-facade", "engine.projectile-schedule-facade-owns", "engine.projectile-schedule-facade-omits", "engine.app-no-actor-projectile-steppers", "engine.content-no-actor-projectile-steppers", "engine.runtime-actor-projectile-centralized"]

[[entry]]
old_test = "architecture_boundaries_runtime_is_headless_composition_tier"
disposition = "declarative"
policies = ["engine.runtime-member", "engine.runtime-required-deps", "engine.runtime-manifest-allow", "engine.runtime-manifest-deny", "engine.runtime-source-no-upper"]

[[entry]]
old_test = "architecture_boundaries_host_does_not_depend_on_actors"
disposition = "declarative"
policies = ["engine.host-crate-member", "engine.host-manifest-no-actors", "engine.host-source-no-actors"]

[[entry]]
old_test = "architecture_boundaries_kaleidoscope_is_an_engine_extension"
disposition = "declarative"
policies = ["game.kaleidoscope-member", "game.kaleidoscope-menu-model-only", "game.kaleidoscope-no-game-machinery", "game.menu-crate-lunex-free"]

[[entry]]
old_test = "architecture_boundaries_settings_menu_ir_is_foundation_only"
disposition = "declarative"
policies = ["engine.ambition_settings_menu-member", "engine.ambition_settings_menu-manifest-allow", "engine.ambition_settings_menu-source-purity"]

[[entry]]
old_test = "architecture_boundaries_dev_tools_crate_is_foundation_only"
disposition = "declarative"
policies = ["engine.ambition_dev_tools-member", "engine.ambition_dev_tools-manifest-allow", "engine.ambition_dev_tools-source-purity"]

[[entry]]
old_test = "architecture_boundaries_dialog_crate_is_runtime_only"
disposition = "declarative"
Read lib.rs
Bash sfx_bank + audio headers and deps
IN
cd /home/joncrall/code/ambition/crates && echo "=== sfx_bank/lib.rs header ===" && head -40 ambition_sfx_bank/src/lib.rs && echo "" && echo "=== audio/catalog.rs header ===" && head -30 ambition_audio/src/catalog.rs && echo "" && echo "=== audio Cargo deps (full) ===" && grep -E "path = |^kira|^bevy" ambition_audio/Cargo.toml

OUT
=== sfx_bank/lib.rs header ===
//! `.sfxbank` binary file format reader.
//!
//! See `tools/ambition_sfx_pack/pack.py` for the canonical format spec
//! and the producer. This crate is intentionally pure-data: no audio
//! decoders, no Bevy, no async. Higher layers (`ambition_sfx`) wrap
//! these byte slices into playable clips.
//!
//! # Layout (little-endian)
//!
//! ```text
//! Header (40 bytes):
//!   magic           [u8; 8]   = b"AMBNDSFX"
//!   version         u32       = 1
//!   entry_count     u32
//!   entries_offset  u64
//!   payloads_offset u64
//!   names_offset    u64
//!
//! Entry table (entry_count * 64 bytes, sorted ascending by id_hash):
//!   id_hash         u64
//!   offset          u64
//!   length          u32
//!   codec           u8 (0=Wav, 1=Ogg, 2=Flac)
//!   channels        u8
//!   _pad0           u16
//!   sample_rate     u32
//!   duration_ms     u32
//!   default_gain_db f32
//!   peak_db         f32
//!   rms_db          f32
//!   flags           u32 (bit0=streamable_hint, bit1=looping)
//!   _reserved       [u8; 16]
//!
//! Payloads: concatenated, in entry order.
//!
//! Names section (debug; runtime may skip):
//!   per entry, in id_hash order: { len: u16, bytes: [u8; len] }
//! ```

use std::collections::HashMap;

=== audio/catalog.rs header ===
//! App-local authored-audio catalogs contributed by experience providers.
//!
//! Music and SFX remain separate authored concerns, while one provider id ties
//! them to the experience that owns their defaults. The registry is a Bevy
//! resource, so independent `App`s in one process may compose different games.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ambition_sfx::SfxId;
use bevy::prelude::{App, Resource};

use crate::spec::{MusicRegistry, MusicTrack, SfxRegistry};

/// One provider's immutable authored-audio definitions.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioCatalogFragment {
    provider_id: String,
    music: Option<MusicRegistry>,
    sfx: Option<SfxRegistry>,
}

impl AudioCatalogFragment {
    pub fn new(
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
    ) -> Result<Self, AudioCatalogError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {

=== audio Cargo deps (full) ===
kira = ["dep:bevy_kira_audio", "bevy/bevy_input_focus"]
ambition_sfx = { path = "../ambition_sfx" }
bevy = { version = "0.18.1", default-features = false }
bevy_kira_audio = { version = "0.25", default-features = false, features = ["wav", "ogg"], optional = true }
Read events.rs
Bash Analyze boss_encounter/features tangle and content leaks
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== what boss_encounter imports FROM features (crate::features::) ==="
grep -rhoE 'crate::features::[a-z_:]+[A-Za-z_]+' boss_encounter | sort | uniq -c | sort -rn | head -30
echo
echo "=== what features imports FROM boss_encounter ==="
grep -rhoE 'crate::boss_encounter::[a-zA-Z_:]+' features | sort | uniq -c | sort -rn | head -30
echo
echo "=== starting_character.rs head ==="; sed -n '1,30p' avatar/starting_character.rs
echo "=== character_sprites/assets.rs ~325-340 ==="; sed -n '325,340p' character_sprites/assets.rs

OUT
=== what boss_encounter imports FROM features (crate::features::) ===
      4 crate::features::ecs::boss_clusters::test_support
      4 crate::features::ecs::boss_clusters::BossEncounter
      3 crate::features::ecs::boss_clusters
      2 crate::features::boss_is_cleared
      1 crate::features::sync_boss_reward_chests_ecs
      1 crate::features::ecs::boss_clusters::BossConfig

=== what features imports FROM boss_encounter ===
     37 crate::boss_encounter::test_boss_catalog
     14 crate::boss_encounter::BossCatalog
      5 crate::boss_encounter::sprites::BossAnim
      4 crate::boss_encounter::sprites::BossAnimState
      3 crate::boss_encounter::BossEncounterPhase
      2 crate::boss_encounter::sprites::BossAnimFrame
      2 crate::boss_encounter::behavior::BossBehaviorProfile
      2 crate::boss_encounter::behavior::
      2 crate::boss_encounter::behavior
      2 crate::boss_encounter::CommandedMove
      2 crate::boss_encounter::BossEncounterPhase::Phase
      1 crate::boss_encounter::sprites::pick_boss_anim
      1 crate::boss_encounter::sprites::BossAnimator
      1 crate::boss_encounter::sprites::BossAnimFrame::new
      1 crate::boss_encounter::sprites::BossAnim::Rest
      1 crate::boss_encounter::notify_bosses_on_mount_death
      1 crate::boss_encounter::behavior::canonical_boss_id_from
      1 crate::boss_encounter::attack_geometry::volumes_for_profile
      1 crate::boss_encounter::attack_geometry
      1 crate::boss_encounter::PhaseTrigger
      1 crate::boss_encounter::FallingHazard
      1 crate::boss_encounter::EncounterDef
      1 crate::boss_encounter::BossRewardProfile::DropChest
      1 crate::boss_encounter::BossProfile
      1 crate::boss_encounter::BossEncounterRegistry
      1 crate::boss_encounter::BossEncounterPhase::Dormant
      1 crate::boss_encounter::ActorPhaseState

=== starting_character.rs head ===
//! Which character the local player STARTS as.
//!
//! The player entity is a *control box*: it carries `Brain::Player(slot)`, the
//! home-body integration loop, the player markers, and the full traversal
//! ability kit. WHICH character that box *wears* — its sprite, its combat
//! moveset, and its name — is chosen by the session-owned [`StartingCharacter`] component.
//! With no override the component is EMPTY and resolves (at spawn) to the
//! CONTENT-installed default character (C2) — the engine names no specific
//! character — so an untouched build spawns exactly as it did before.
//!
//! This is the runtime seam behind Jon's polish-list ask: *"swap my starting
//! character for PCA or a pirate ... just spawn the character and make its
//! brain the keyboard input."* Possession
//! ([`crate::abilities::traversal::possession`]) already proves
//! `Brain::Player` drives ANY body; this makes the *starting* body a choice
//! too without creating a character-specific movement route. The worn body
//! still enters the same frame-aware movement kernel as every other body.
//!
//! [`StartingCharacter`] is the session-owned startup selection. At spawn
//! ([`crate::session::setup`]) the chosen id is both overlaid onto the body
//! (moveset + name) AND recorded as the canonical [`WornCharacter`] identity
//! component ON the player entity. From then on the entity's component — not
//! this component — is the single source both gameplay and presentation derive
//! from: [`apply_worn_character_gameplay`] re-applies the kit on any change, and
//! the reusable `ambition_render` binder installs the sprite from the same
//! identity. Presentation reads the same session-owned identity rather than process state.

use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::system::{Commands, Query};
use bevy::prelude::{Changed, Component, Entity, Has, Name, Or, Res, With};
=== character_sprites/assets.rs ~325-340 ===
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.player = Some(asset);
            }
            "robot" => {
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.robot = Some(asset);
            }
            "goblin" => {
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
                out.goblin = Some(asset);
            }
            "sandbag" => {
                out.npcs.insert(cid.clone(), asset.clone());
                out.npcs.insert(entry.display_name.clone(), asset.clone());
SfxId isn't an enum. Let me check its definition and confirm whether it's a content-free ID or a hardcoded content list.

Let me read the custom scanners and find legacy-pending (not-yet-enforced) boundaries.

Let me search for game-specific identifiers and dependency relationships across the target crates.

Bash SfxId definition and any hardcoded named ids
IN
cd /home/joncrall/code/ambition/crates && echo "=== SfxId definition ===" && grep -n "struct SfxId\|SfxId(" ambition_sfx/src/ids.rs | head && echo "--- head of ids.rs ---" && head -60 ambition_sfx/src/ids.rs && echo "=== any hardcoded named sfx constants? ===" && grep -rhoE "SfxId::[a-z_]+|const [A-Z_]+: SfxId" ambition_sfx/src ambition_audio/src 2>/dev/null | sort -u | head -30

OUT
=== SfxId definition ===
--- head of ids.rs ---
//! Hand-maintained SFX ids for gameplay code that benefits from named constants.
//!
//! Adding to this list is purely an ergonomics call: the bank stores everything
//! regardless. Use `SfxId::from_static("foo.bar")` at the call site for one-off
//! or rare SFX. IDs match the catalog produced by `ambition_sfx_renderer` under
//! `tools/ambition_sfx_renderer/output/`. When in doubt, run:
//!
//! ```text
//! python3 tools/ambition_sfx_pack/pack.py --dump
//! ```
//!
//! and grep `crates/ambition_actors/assets/audio/sfx.bank.txt`.

use crate::SfxId;

// Player movement
pub const PLAYER_JUMP: SfxId = SfxId::from_static("player.jump");
pub const PLAYER_DOUBLE_JUMP: SfxId = SfxId::from_static("player.double_jump");
pub const PLAYER_DASH: SfxId = SfxId::from_static("player.dash");
pub const PLAYER_BLINK: SfxId = SfxId::from_static("player.blink");
pub const PLAYER_PRECISION_BLINK: SfxId = SfxId::from_static("player.precision_blink");
pub const PLAYER_POGO: SfxId = SfxId::from_static("player.pogo");
pub const PLAYER_LAND: SfxId = SfxId::from_static("player.land");
pub const PLAYER_FAST_FALL: SfxId = SfxId::from_static("player.fast_fall");
pub const PLAYER_WALL_JUMP: SfxId = SfxId::from_static("player.wall_jump");
pub const PLAYER_WALL_SLIDE: SfxId = SfxId::from_static("player.wall_slide");
pub const PLAYER_WALL_CLING: SfxId = SfxId::from_static("player.wall_cling");
pub const PLAYER_LEDGE_GRAB: SfxId = SfxId::from_static("player.ledge_grab");
pub const PLAYER_REBOUND: SfxId = SfxId::from_static("player.rebound");

// Player combat / vitals
pub const PLAYER_SLASH: SfxId = SfxId::from_static("player.slash");
pub const PLAYER_HIT: SfxId = SfxId::from_static("player.hit");
pub const PLAYER_DAMAGE: SfxId = SfxId::from_static("player.damage");
pub const PLAYER_HEAL: SfxId = SfxId::from_static("player.heal");
pub const PLAYER_DEATH: SfxId = SfxId::from_static("player.death");
pub const PLAYER_RESPAWN: SfxId = SfxId::from_static("player.respawn");
pub const PLAYER_RESET: SfxId = SfxId::from_static("player.reset");
pub const PLAYER_LOW_HEALTH_PULSE: SfxId = SfxId::from_static("player.low_health.pulse");
pub const PLAYER_STAMINA_EMPTY: SfxId = SfxId::from_static("player.stamina_empty");
pub const PLAYER_ABILITY_UNLOCK: SfxId = SfxId::from_static("player.ability_unlock");

// Player damage-type variants (when source is typed)
pub const PLAYER_HIT_FIRE: SfxId = SfxId::from_static("player.hit.fire");
pub const PLAYER_HIT_ICE: SfxId = SfxId::from_static("player.hit.ice");
pub const PLAYER_HIT_LIGHTNING: SfxId = SfxId::from_static("player.hit.lightning");
pub const PLAYER_HIT_POISON: SfxId = SfxId::from_static("player.hit.poison");

// Hazards (single-shot contacts)
pub const HAZARD_LAVA_SPLASH: SfxId = SfxId::from_static("hazard.lava.splash");
pub const HAZARD_ACID_SPLASH: SfxId = SfxId::from_static("hazard.acid.splash");
pub const HAZARD_SPIKE_HIT: SfxId = SfxId::from_static("hazard.spike.hit");
pub const HAZARD_ELECTRIC_ARC: SfxId = SfxId::from_static("hazard.electric.arc");
pub const HAZARD_SAW_HIT: SfxId = SfxId::from_static("hazard.saw.hit");
// Looped hazard ambients (start/stop on volume entry/exit): wiring lives in TODO
// until the loop-lifecycle subsystem lands.
pub const HAZARD_WIND_GUST_LOOP: SfxId = SfxId::from_static("hazard.wind.gust_loop");
pub const HAZARD_POISON_CLOUD_LOOP: SfxId = SfxId::from_static("hazard.poison.cloud_loop");
pub const HAZARD_ELECTRIC_LOOP: SfxId = SfxId::from_static("hazard.electric.loop");
pub const HAZARD_SAW_LOOP: SfxId = SfxId::from_static("hazard.saw.loop");
=== any hardcoded named sfx constants? ===
SfxId::from_hash
SfxId::from_static
SfxId::new
const DIALOGUE_BLIP_ALICE: SfxId
const DIALOGUE_BLIP_ARCHITECT: SfxId
const DIALOGUE_BLIP_BOB: SfxId
const DIALOGUE_BLIP_CREATOR: SfxId
const DIALOGUE_BLIP_DARK_LORD: SfxId
const DIALOGUE_BLIP_GATE_JANITOR: SfxId
const DIALOGUE_BLIP_GENERIC: SfxId
const DIALOGUE_BLIP_GOBLIN_CHIEFTAIN: SfxId
const DIALOGUE_BLIP_HAND_SAINT: SfxId
const DIALOGUE_BLIP_KERNEL_GUIDE: SfxId
const DIALOGUE_BLIP_MANIFEST_CLERK: SfxId
const DIALOGUE_BLIP_MERCHANT_PROTOTYPE: SfxId
const DIALOGUE_BLIP_MILITARY_GENERAL: SfxId
const DIALOGUE_BLIP_NEWS_BOARD: SfxId
const DIALOGUE_BLIP_NINJA: SfxId
const DIALOGUE_BLIP_OILER: SfxId
const DIALOGUE_BLIP_PIRATE: SfxId
const DIALOGUE_BLIP_PULSE_VOYAGER: SfxId
const DIALOGUE_BLIP_ROBOT: SfxId
const DIALOGUE_BLIP_SHOUT_GENERIC: SfxId
const DIALOGUE_BLIP_TECH_BRO: SfxId
const DIALOGUE_BLIP_VAULT_KEEPER: SfxId
const DIALOGUE_BLIP_WEIRD_HERMIT: SfxId
const DIALOGUE_BLIP_WHISPER_GENERIC: SfxId
const DIALOGUE_CHOICE_APPEAR: SfxId
const DIALOGUE_CHOICE_SELECT: SfxId
const DIALOGUE_LINE_ADVANCE: SfxId
Bash Find not-yet-enforced boundaries in migration matrix
IN
cd tests/ambition_workspace_policy; echo "=== legacy-pending / retained / removed dispositions ==="; grep -B2 -A4 'disposition = "legacy-pending"\|disposition = "retained"\|disposition = "removed"' migration_matrix.toml | head -120

OUT
=== legacy-pending / retained / removed dispositions ===
Read content_ownership.rs
Let me do final checks: the named-slot sprite struct, dialog bindings, music content, and count of workspace deps.

Update Todos

Map crate landscape: LOC, deps, growth since last ledger (done inline)

Read existing planning docs, rulings, smells (done inline)

Collect 6 exploration-agent reports (actors, foundation, content-data, presentation, app/facade, midlayer)

Synthesize findings into ranked split/reorg/unification opportunities

Write recon+plan doc into docs/planning and commit

Summarize verdicts for Jon
Bash Find reverse dependencies of target crates
IN
cd /home/joncrall/code/ambition
echo "=== who depends on ambition_interaction ==="
grep -rln "ambition_interaction" crates/*/Cargo.toml | sed 's|/Cargo.toml||;s|crates/||'
echo
echo "=== who depends on ambition_cutscene ==="
grep -rln "ambition_cutscene" crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||'
echo
echo "=== who depends on ambition_combat ==="
grep -rln "ambition_combat" crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||'
echo
echo "=== who depends on ambition_projectiles ==="
grep -rln "ambition_projectiles" crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||'
echo
echo "=== who depends on ambition_encounter ==="
grep -rln "ambition_encounter" crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||'
echo
echo "=== who depends on ambition_dialog / cutscene consumers ==="
grep -rln "ambition_dialog" crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||'

OUT
=== who depends on ambition_interaction ===
ambition
ambition_combat
ambition_encounter
ambition_actors
ambition_interaction
ambition_render
ambition_sim_view
ambition_sprite_sheet

=== who depends on ambition_cutscene ===
crates/ambition_cutscene
crates/ambition
crates/ambition_actors
game/ambition_content
crates/ambition_render
crates/ambition_touch_input

=== who depends on ambition_combat ===
crates/ambition_combat
crates/ambition
crates/ambition_actors
crates/ambition_sim_view
crates/ambition_sprite_sheet
crates/ambition_runtime
game/ambition_content
crates/ambition_render

=== who depends on ambition_projectiles ===
crates/ambition
crates/ambition_actors
crates/ambition_projectiles
crates/ambition_runtime
crates/ambition_sim_view
crates/ambition_render
game/ambition_content

=== who depends on ambition_encounter ===
crates/ambition
crates/ambition_actors
crates/ambition_encounter
crates/ambition_sim_view
crates/ambition_runtime
game/ambition_content

=== who depends on ambition_dialog / cutscene consumers ===
crates/ambition
crates/ambition_actors
crates/ambition_dialog
crates/ambition_runtime
crates/ambition_render
game/ambition_content
Bash Check named sprite fields, dialog bindings, music, dep count
IN
cd /home/joncrall/code/ambition/crates/ambition_actors
echo "=== count workspace path deps in Cargo.toml ==="
grep -cE 'path = "\.\./ambition' Cargo.toml
echo "--- list them ---"
grep -oE 'ambition_[a-z_]+ = \{ path' Cargo.toml | sed 's/ = { path//' | sort -u | tr '\n' ' '
echo; echo
echo "=== CharacterSpriteAssets named fields (content leak) ==="
grep -nE 'pub (robot|goblin|sandbag|player|parrot|pirate|slug|shark)' src/character_sprites/assets.rs | head
echo "=== dialog/yarn_bindings.rs head ==="; sed -n '1,20p' src/dialog/yarn_bindings.rs
echo "=== music/ files + head ==="; find src/music -name '*.rs' | xargs wc -l | sort -rn; sed -n '1,15p' src/music/mod.rs 2>/dev/null

OUT
=== count workspace path deps in Cargo.toml ===
28
--- list them ---
ambition_asset_manager ambition_audio ambition_characters ambition_combat ambition_cutscene ambition_dev_tools ambition_dialog ambition_encounter ambition_engine_core ambition_entity_catalog ambition_gameplay_trace ambition_input ambition_interaction ambition_items ambition_ldtk_map ambition_menu ambition_persistence ambition_platformer_primitives ambition_portal ambition_projectiles ambition_settings_menu ambition_sfx ambition_sim_view ambition_sprite_sheet ambition_time ambition_ui_nav ambition_vfx ambition_world 

=== CharacterSpriteAssets named fields (content leak) ===
=== dialog/yarn_bindings.rs head ===
//! Yarn command + function + markup registrations — the "vocabulary"
//! that authored `.yarn` content can invoke at runtime.
//!
//! The bindings split into three concerns:
//!
//! **Commands** (`<<set_flag X>>` syntax). Bevy systems with
//! `In<T>` parameters. Registered on the runner's `commands_mut()`
//! via `world.register_system(...)`. Each one writes to a typed
//! game-state channel (`GameplayEffect::SetFlag`, `SfxMessage::Play`,
//! …). Authored dialogue uses them to *drive* gameplay.
//!
//! **Functions** (`<<if boss_cleared("X")>>` syntax). Pure functions
//! registered on the runner's `library_mut()`. Functions can't be
//! Bevy systems — they're called synchronously from the runtime
//! interpreter — so they read save state through a shared
//! [`YarnStateMirror`] refreshed each frame by
//! [`refresh_yarn_state_mirror`]. Authored dialogue uses them to
//! *read* gameplay.
//!
//! **Markup cues** (`Speaker: [shout]LINE[/shout]` inline). The
=== music/ files + head ===
  458 total
  211 src/music/tests.rs
  206 src/music/intent.rs
   41 src/music/mod.rs
//! Sandbox music adapters over the `ambition_audio` music core.
//!
//! Game-side music glue: [`intent`] (encounter / room / radio state ->
//! [`MusicIntent`]), authored goblin cue data, and settings ->
//! [`ambition_audio::MusicMix`] sync.

#![cfg(feature = "audio")]

use bevy::prelude::*;

use ambition_persistence::settings::UserSettings;

mod intent;

#[cfg(test)]
Read module_size.rs
Bash module size policy + app policies detail
IN
cd tests/ambition_workspace_policy; echo "=== module_size.toml ==="; cat policies/module_size.toml; echo; echo "=== game.toml app-specific policy details (lines 261-340) ==="; sed -n '260,340p' policies/game.toml

OUT
=== module_size.toml ===
# D-B's module-size gate (docs/planning/engine/decomposition.md).
#
# The standard: a production module over 5k lines deserves deliberate review.
# This is a generous architectural tripwire, not pressure to split coherent code
# merely to satisfy a number. The scanner is src/custom/module_size.rs.
#
# Scope: every PRODUCTION `.rs` under crates/*/src and game/*/src — engine AND
# content/demo code. Standalone test files (`tests.rs`, `/tests/`) are excluded by
# path (workspace::is_test_path); inline `#[cfg(test)]` counts toward its file.
#
# An intentional exception may use a reviewed `[[waiver]]`; stale waivers fail.

id = "engine.module-size"
scope = "engine"
# Cross-cutting: the gate scans every production module under crates/*/src +
# game/*/src, so no single crate owns it — the `workspace` owner sentinel.
owners = ["workspace"]
source_doc = "docs/planning/engine/decomposition.md"
rationale = "Production modules over 5000 lines require deliberate review; split only when coherent boundaries exist."
limit = 5000
roots = ["crates", "game"]

=== game.toml app-specific policy details (lines 261-340) ===
[[policy]]
id = "game.app-sim-resources-no-inline-content"
scope = "game"
kind = "file-omits"
owners = ["ambition_content"]
watch_paths = ["game/ambition_app/src/app/sim_resources.rs"]
rationale = "app/sim_resources.rs must not construct named content inline"
source_doc = "docs/architecture/architecture-boundaries.md"
severity = "error"
forbid = ["QuestRegistry::default()", "BossEncounterRegistry::default()", "default_cutscene_library()", "default_room_cutscene_bindings()", "install_boss_banter", "install_pirate_banter", "crate::intro::IntroPlugin"]
file = "game/ambition_app/src/app/sim_resources.rs"

[[policy]]
id = "game.app-plugins-no-inline-item-roster"
scope = "game"
kind = "file-omits"
owners = ["ambition_content"]
watch_paths = ["game/ambition_app/src/app/plugins.rs"]
rationale = "item roster installs through the content plugin"
source_doc = "docs/architecture/architecture-boundaries.md"
severity = "error"
forbid = ["ambition_items::OwnedItems::starter()"]
file = "game/ambition_app/src/app/plugins.rs"


# was: architecture_boundaries_app_plugins_does_not_reown_moved_subsystems
[[policy]]
id = "game.app-plugins-no-reown"
scope = "game"
kind = "file-omits"
owners = ["ambition_app"]
watch_paths = ["game/ambition_app/src/app/plugins.rs"]
rationale = "app/plugins.rs must not reintroduce subsystem-owned registrations"
source_doc = "docs/architecture/architecture-boundaries.md"
severity = "error"
forbid = ["fn register_portal_systems", "fn register_item_pickup_systems", "ambition_portal::portal_fire_system", "ambition_portal::portal_projectile_step", "ambition_portal::portal_transit", "ambition_actors::item_pickup::pickup_held_item_system", "ambition_actors::item_pickup::throw_held_item_system", "ambition_actors::item_pickup::ground_item_physics"]
file = "game/ambition_app/src/app/plugins.rs"


# was: architecture_boundaries_input_timer_systems_moved_to_actors
[[policy]]
id = "game.app-sim-systems-no-redefine"
scope = "game"
kind = "file-omits"
owners = ["ambition_app"]
watch_paths = ["game/ambition_app/src/app/sim_systems.rs"]
rationale = "app/sim_systems.rs must not re-define library-owned systems"
source_doc = "docs/architecture/architecture-boundaries.md"
severity = "error"
forbid = ["fn sync_live_player_dev_edits_system", "fn apply_suspended_time_scale_system", "fn input_timer_system", "fn interaction_input_system", "fn cleanup_timers_system"]
file = "game/ambition_app/src/app/sim_systems.rs"

[[policy]]
id = "game.engine-schedule-references-moved"
scope = "game"
kind = "file-contains"
owners = ["ambition_app"]
watch_paths = ["crates/ambition_runtime/src/player_schedule.rs"]
rationale = "engine player schedule references the moved systems via library paths"
source_doc = "docs/architecture/architecture-boundaries.md"
severity = "error"
contains = ["ambition_dev_tools::sync_live_player_dev_edits_system", "ambition_actors::time::time_control::apply_suspended_time_scale_system", "ambition_actors::control::input_timer_system", "ambition_actors::control::interaction_input_system", "ambition_actors::control::cleanup_timers_system"]
file = "crates/ambition_runtime/src/player_schedule.rs"

[[policy]]
id = "game.app-keeps-host-reset-systems"
scope = "game"
kind = "file-contains"
owners = ["ambition_app"]
watch_paths = ["game/ambition_app/src/app/sim_systems.rs"]
rationale = "host/reset-bound systems remain defined in the app"
source_doc = "docs/architecture/architecture-boundaries.md"
severity = "error"
contains = ["fn apply_player_reset_input_system", "fn apply_room_replay_request_system"]
file = "game/ambition_app/src/app/sim_systems.rs"


# was: architecture_boundaries_abilities_live_under_abilities_layer
[[policy]]
id = "game.abilities-under-layer"
scope = "game"
While the agents work, let me measure where the last five days of growth actually landed, per crate.

Bash Inspect named sprite slot struct
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== find named sprite slot struct fields ==="
grep -nE '^\s*pub \w+:' character_sprites/assets.rs | head -30
echo "=== the match arms mapping content ids to slots ==="
grep -nE '=> \{|"\w+" =>' character_sprites/assets.rs | head -30

OUT
=== find named sprite slot struct fields ===
143:    pub collision: ae::Vec2,
144:    pub render_size: ae::Vec2,
=== the match arms mapping content ids to slots ===
251:        crate::persistence::settings::TextureResolutionScale::Potato => {
254:        crate::persistence::settings::TextureResolutionScale::Quarter => {
257:        crate::persistence::settings::TextureResolutionScale::Half => {
260:        crate::persistence::settings::TextureResolutionScale::Full => {
314:            "player" => {
328:            "robot" => {
333:            "goblin" => {
338:            "sandbag" => {
343:            _ => {
Bash Net LOC change per crate since 2026-07-10 from git history
IN
git log --since=2026-07-10 --numstat --format= -- 'crates/*/src/*' 'game/*/src/*' | awk -F'\t' '{split($3,p,"/"); add[p[2]]+=$1; del[p[2]]+=$2} END {for (c in add) printf "%6d +%d/-%d %s\n", add[c]-del[c], add[c], del[c], c}' | sort -rn | head -25

OUT
  5099 +5733/-634 ambition_game_shell
  4871 +8768/-3897 ambition_engine_core
  4680 +20769/-16089 ambition_actors
  2528 +3464/-936 ambition_audio
  1640 +1663/-23 ambition_load_presentation
  1628 +2836/-1208 ambition_demo_sanic
  1586 +2818/-1232 ambition_app
  1278 +2247/-969 ambition_render
  1274 +1586/-312 ambition_demo_smb1
  1226 +6294/-5068 ambition_characters
  1163 +2124/-961 ambition_platformer_primitives
  1158 +1161/-3 ambition_load
   987 +4969/-3982 ambition_runtime
   854 +1209/-355 ambition
   698 +2878/-2180 ambition_content
   616 +765/-149 ambition_encounter
   454 +455/-1 ambition_demo_mary_o
   335 +454/-119 ambition_demo_sanic_app
   322 +1795/-1473 ambition_combat
   266 +291/-25 ambition_demo_pocket
   231 +269/-38 ambition_demo_smb1_app
    82 +111/-29 ambition_sfx
    66 +329/-263 ambition_asset_manager
    42 +95/-53 ambition_world
    37 +47/-10 ambition_host
Bash Grep game-specific proper names in target crates
IN
cd /home/joncrall/code/ambition
echo "=== game-specific proper names in TARGET crates (combat, projectiles, portal(_pres), encounter, interaction, cutscene, dialog, dev_tools, persistence, load) ==="
grep -rinE "mockingbird|clockwork_warden|gradient_sentinel|behemoth|spaghetti_monster|trex|mode_collapse|exploding_gradient|overflow_boss|gnu_ton|hadouken|mary_o|sanic|pocket|kaleidoscope|smirking|rope_cut|cut.rope" \
  crates/ambition_combat/src crates/ambition_projectiles/src crates/ambition_portal/src crates/ambition_portal_presentation/src crates/ambition_encounter/src crates/ambition_interaction/src crates/ambition_cutscene/src crates/ambition_dialog/src crates/ambition_dev_tools/src crates/ambition_persistence/src crates/ambition_load/src crates/ambition_load_presentation/src \
  --include=*.rs 2>/dev/null | grep -v "/tests" | grep -viE "test|assert" | head -50

OUT
=== game-specific proper names in TARGET crates (combat, projectiles, portal(_pres), encounter, interaction, cutscene, dialog, dev_tools, persistence, load) ===
crates/ambition_combat/src/hazards.rs:97:            // (dash, Sanic run) must not tunnel through a thin spike between
crates/ambition_combat/src/lib.rs:64:/// floaty drop, not a brick. Tuned by feel against the mockingbird
crates/ambition_combat/src/events.rs:322:    /// Player projectile (Fireball / Hadouken). No knockback today;
crates/ambition_combat/src/components/actors.rs:674:/// bespoke per-boss content (e.g. the Smirking Behemoth victory NPC in
crates/ambition_combat/src/components/actors.rs:675:/// `crate::ambition_content::bosses::cut_rope`) tags the entity with this
crates/ambition_projectiles/src/kind.rs:3://! This is *named game content* — the Fireball / Hadouken tier and the numbers
crates/ambition_projectiles/src/kind.rs:27:///   - `Hadouken`: a 2-step grace quarter-circle
crates/ambition_projectiles/src/kind.rs:30:///     projectile than `HadoukenSuper`.
crates/ambition_projectiles/src/kind.rs:31:///   - `HadoukenSuper`: the traditional 3-step quarter-circle
crates/ambition_projectiles/src/kind.rs:39:    /// Grace-input Hadouken. Travels horizontally, expires on first
crates/ambition_projectiles/src/kind.rs:41:    Hadouken,
crates/ambition_projectiles/src/kind.rs:42:    /// Full-input Hadouken. Same trajectory shape as `Hadouken` but
crates/ambition_projectiles/src/kind.rs:44:    HadoukenSuper,
crates/ambition_projectiles/src/kind.rs:52:            Self::Hadouken => 3.0,
crates/ambition_projectiles/src/kind.rs:53:            Self::HadoukenSuper => 5.0,
crates/ambition_projectiles/src/kind.rs:62:            Self::Hadouken => 3,
crates/ambition_projectiles/src/kind.rs:63:            Self::HadoukenSuper => 5,
crates/ambition_projectiles/src/kind.rs:73:            Self::Hadouken => 0.55,
crates/ambition_projectiles/src/kind.rs:74:            Self::HadoukenSuper => 0.85,
crates/ambition_projectiles/src/kind.rs:82:            Self::Hadouken => 520.0,
crates/ambition_projectiles/src/kind.rs:83:            Self::HadoukenSuper => 640.0,
crates/ambition_projectiles/src/kind.rs:92:            Self::Hadouken => 1.60,
crates/ambition_projectiles/src/kind.rs:93:            Self::HadoukenSuper => 1.80,
crates/ambition_projectiles/src/kind.rs:104:            Self::Hadouken => Vec2::new(16.0, 12.0),
crates/ambition_projectiles/src/kind.rs:105:            Self::HadoukenSuper => Vec2::new(22.0, 16.0),
crates/ambition_projectiles/src/kind.rs:109:    /// Per-second downward acceleration. Fireballs arc; Hadoukens fly straight.
crates/ambition_projectiles/src/kind.rs:113:            Self::Hadouken | Self::HadoukenSuper => 0.0,
crates/ambition_projectiles/src/kind.rs:118:    /// bounce twice (Mario-like / arcade-style); Hadoukens don't bounce.
crates/ambition_projectiles/src/kind.rs:122:            Self::Hadouken | Self::HadoukenSuper => 0,
crates/ambition_projectiles/src/kind.rs:129:            Self::Hadouken => "hadouken",
crates/ambition_projectiles/src/kind.rs:130:            Self::HadoukenSuper => "hadouken_super",
crates/ambition_projectiles/src/kind.rs:250:        let h = ProjectileKind::Hadouken.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
crates/ambition_projectiles/src/kind.rs:251:        let charged = ProjectileKind::Hadouken.charged_spec(h, 2);
crates/ambition_projectiles/src/kind.rs:258:    fn fireball_bounces_twice_hadouken_never() {
crates/ambition_projectiles/src/state.rs:24:    /// having consumed the press for a Hadouken / HadoukenSuper.
crates/ambition_projectiles/src/state.rs:32:    /// - press WITH a recent motion gesture → fires Hadouken/Super
crates/ambition_projectiles/src/state.rs:41:    pub hadouken: bool,
crates/ambition_projectiles/src/state.rs:42:    pub hadouken_super: bool,
crates/ambition_projectiles/src/state.rs:49:            hadouken: true,
crates/ambition_projectiles/src/state.rs:50:            hadouken_super: true,
crates/ambition_projectiles/src/diagnostics.rs:8:/// goal is concrete feedback when "the Hadouken won't come out": the
crates/ambition_projectiles/src/diagnostics.rs:43:        Some(crate::ProjectileKind::HadoukenSuper) => "HadoukenSuper",
crates/ambition_projectiles/src/diagnostics.rs:44:        Some(crate::ProjectileKind::Hadouken) => "Hadouken (grace)",
crates/ambition_projectiles/src/visual_kind.rs:36:    /// Player Hadouken — cool blue energy ball.
crates/ambition_projectiles/src/visual_kind.rs:37:    Hadouken,
crates/ambition_projectiles/src/visual_kind.rs:38:    /// Player super Hadouken — stronger blue tint.
crates/ambition_projectiles/src/visual_kind.rs:39:    HadoukenSuper,
crates/ambition_projectiles/src/visual_kind.rs:130:            // fireball; cool blue Hadouken; stronger blue super.
crates/ambition_projectiles/src/visual_kind.rs:143:            Self::Hadouken => ProjectileArt {
crates/ambition_projectiles/src/visual_kind.rs:153:                label: "hadouken",
Key finding: ambition_sfx/ids.rs hardcodes named Ambition characters (DIALOGUE_BLIP_ALICE, DARK_LORD, GOBLIN_CHIEFTAIN, HAND_SAINT, etc.). Now let me examine entity_catalog, items, and ldtk_map.

Bash Locate named sprite slot struct definition
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== struct with player/robot/goblin/sandbag fields ==="
grep -rnE 'pub (player|robot|goblin|sandbag):' character_sprites/ | head
echo "=== the enclosing struct def ==="
grep -rn 'struct.*{' character_sprites/mod.rs character_sprites/assets.rs | head
echo "--- context around 300-315 assets.rs ---"; sed -n '300,318p' character_sprites/assets.rs

OUT
=== struct with player/robot/goblin/sandbag fields ===
=== the enclosing struct def ===
character_sprites/assets.rs:142:pub struct SpriteBodyCollision {
--- context around 300-315 assets.rs ---
            asset_catalog,
            asset_server,
            layouts,
            &asset_id,
            &sheet_spec,
            variant,
            Some(cid),
            quality,
        ) else {
            skipped_no_path.push(cid.as_str());
            continue;
        };
        loaded += 1;
        match cid.as_str() {
            "player" => {
                // Store under the typed field for the runtime's
                // fast-path consumers (`runtime/setup.rs`,
                // `enemy_asset`). ALSO key the npcs HashMap by the
                // display name so a hall pedestal with
Bash entity_catalog, items, ldtk_map: Cargo/tree/LOC
IN
cd /home/joncrall/code/ambition/crates && for c in ambition_entity_catalog ambition_items ambition_ldtk_map; do echo "======== $c ========"; grep -E "^description" $c/Cargo.toml; echo "--- deps ---"; grep -E "path = " $c/Cargo.toml; echo "--- files + LOC ---"; find $c -name '*.rs' -not -path '*/target/*' | xargs wc -l | sort -rn | head -18; echo "--- non-rs data files ---"; find $c -type f -not -name '*.rs' -not -path '*/target/*' | grep -vE "MODULES.md|README" | head; echo ""; done

OUT
======== ambition_entity_catalog ========
description = "Reusable, content-free entity-contract + moveset vocabulary: EntityDef contract bundles and the Smash-model MoveSpec timeline (windows / hit volumes / events on the owner's proper time), with headless RON parsing + structural validators. Gameplay truth lives here; visual storage lives in ambition_sprite_sheet."
--- deps ---
--- files + LOC ---
 2035 total
  980 ambition_entity_catalog/src/lib.rs
  563 ambition_entity_catalog/src/tests.rs
  492 ambition_entity_catalog/src/placements.rs
--- non-rs data files ---
ambition_entity_catalog/Cargo.toml

======== ambition_items ========
description = "Reusable item catalog and shop primitives for Ambition-style platformers."
--- deps ---
ambition_characters = { path = "../ambition_characters" }
ambition_persistence = { path = "../ambition_persistence" }
--- files + LOC ---
  979 total
  625 ambition_items/src/lib.rs
  201 ambition_items/src/tests.rs
  153 ambition_items/src/shop.rs
--- non-rs data files ---
ambition_items/Cargo.toml

======== ambition_ldtk_map ========
description = "LDtk backend adapter for Ambition authored world IR."
--- deps ---
ambition_asset_manager = { path = "../ambition_asset_manager", features = ["bevy"] }
ambition_engine_core = { path = "../ambition_engine_core" }
ambition_entity_catalog = { path = "../ambition_entity_catalog" }
ambition_platformer_primitives = { path = "../ambition_platformer_primitives" }
ambition_world = { path = "../ambition_world", default-features = false }
--- files + LOC ---
  5378 total
   908 ambition_ldtk_map/src/conversion/mod.rs
   763 ambition_ldtk_map/src/conversion/entity_converters.rs
   445 ambition_ldtk_map/src/surfaces.rs
   372 ambition_ldtk_map/src/intgrid.rs
   330 ambition_ldtk_map/src/lib.rs
   304 ambition_ldtk_map/src/project.rs
   234 ambition_ldtk_map/src/loading.rs
   232 ambition_ldtk_map/src/conversion/entity_converters/surface_ramp_winding_oracle.rs
   230 ambition_ldtk_map/src/bevy_runtime/asset.rs
   218 ambition_ldtk_map/src/fields.rs
   203 ambition_ldtk_map/src/bevy_runtime/indices.rs
   198 ambition_ldtk_map/src/manifest.rs
   195 ambition_ldtk_map/src/bevy_runtime/systems.rs
   164 ambition_ldtk_map/src/bevy_runtime/plugin.rs
   149 ambition_ldtk_map/src/fields/tests.rs
   146 ambition_ldtk_map/src/bevy_runtime/components.rs
   142 ambition_ldtk_map/src/hot_reload.rs
--- non-rs data files ---
ambition_ldtk_map/Cargo.toml
Let me get the app's LOC grouped by subsystem and read the app entry module plus the RL/menu classification.

Bash Find out struct fields
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
grep -rnE 'goblin:|robot:|sandbag:' character_sprites/*.rs
echo "--- struct defs in character_sprites ---"
grep -rnE 'pub struct \w+' character_sprites/mod.rs | head

OUT
--- struct defs in character_sprites ---
Bash App LOC by subsystem with test split
IN
cd game/ambition_app/src; echo "=== LOC by top-level subsystem ==="; for d in app menu dev rl_sim host headless bin; do total=$(find $d -name "*.rs" 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}'); testtotal=$(find $d -name "*tests*.rs" -o -name "tests.rs" 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}'); echo "$d: $total LOC (of which ~$testtotal in test files)"; done; echo; echo "root files:"; ls *.rs | xargs wc -l

OUT
=== LOC by top-level subsystem ===
app: 5598 LOC (of which ~0 in test files)
menu: 10082 LOC (of which ~4367 in test files)
dev: 2774 LOC (of which ~0 in test files)
rl_sim: 1694 LOC (of which ~461 in test files)
host: 352 LOC (of which ~0 in test files)
headless: 203 LOC (of which ~203 in test files)
bin: 1427 LOC (of which ~0 in test files)

root files:
 164 headless.rs
  55 lib.rs
 219 total
Read mod.rs
Let me examine what sim_view reads from actors and what it publishes, plus check the policy engine.toml for the render/actors denylist.

Bash entity_catalog headers
IN
cd /home/joncrall/code/ambition/crates && echo "=== entity_catalog lib.rs header ===" && head -45 ambition_entity_catalog/src/lib.rs && echo "" && echo "=== entity_catalog placements.rs header ===" && head -30 ambition_entity_catalog/src/placements.rs

OUT
=== entity_catalog lib.rs header ===
//! Entity-contract + moveset vocabulary — the gameplay-truth schema.
//!
//! This crate is the typed spine of the `EntityCatalog` target in
//! `docs/planning/engine/data-driven-sprites-and-characters.md`: entities as
//! **contract bundles** (not categories), and abilities as **Smash-model move
//! timelines** that every actor plays through the same system.
//!
//! Two rules carry the design:
//!
//! - **One clock per move: the owner's proper time.** Every duration in a
//!   [`MoveSpec`] is seconds of the *owning actor's* clock — its entity dt
//!   (sim dt × whatever dilation that actor experiences: bullet-time, a time
//!   bubble, a relativistic zone). The bound clip's playback is slaved to the
//!   move's normalized phase, so a dilated actor's picture and hit windows
//!   slow together and can never desync. Dilation is a property of the
//!   actor's clock, never of this data — the schema stays
//!   frame-of-reference-free.
//! - **Entity-local logical space.** Move volumes are authored in the
//!   entity's local coordinates (+x = facing, y = up, origin = body center),
//!   never atlas pixels. Quality tiers rescale render textures; they cannot
//!   touch this data.
//!
//! The engine owns the *primitives* here (window, volume, event, gate,
//! cancel edge); content composes them into moves. A move is data — giving
//! the goblin the player's slash is a re-binding, not a Rust change.
//!
//! Authored as RON (this is Rust/hand-authored data; only Python-authored
//! interchange uses JSON). Headless by construction: no Bevy, no assets —
//! a simulation can parse, validate, and play a move without loading a PNG.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

pub mod placements;

// ---------------------------------------------------------------------------
// Ability vocabulary: the ONE effect reference + its opaque params.
// ---------------------------------------------------------------------------

/// Opaque, structured parameters for a technique or prefab. Wraps a parsed
/// `ron::Value`; the consuming effect hydrates its OWN typed struct via
/// [`ParamValue::hydrate`], so this crate stays ignorant of every
/// content-owned param shape (fable review AJ1, option A). The authored RON is
/// byte-identical to a `Reflect`-typed form, so if a visual move editor ever

=== entity_catalog placements.rs header ===
//! The authored-placement schema vocabulary — architecture.md §4b.
//!
//! These are the closed, serde-able authoring enums that authored maps
//! declare over content: brains (who a spawned actor becomes), the
//! damage relationship/category, and the hazard/prop respawn policy.
//! They are pure data (no `Vec2`, no runtime state, no Bevy) so they live
//! in the Tier-0 catalog — below every crate that interprets them. The
//! sim/content LOWERS these records into behavior at room-load; the arrow
//! is always sim/content → catalog, never the reverse.

use serde::{Deserialize, Serialize};

/// Damage/team relationship used by hitboxes and hurtboxes — the `can_damage`
/// matrix that decides whether one side's hit may affect another.
///
/// Deliberately distinct from `ActorFaction` (`ambition_characters`), which
/// is a `#[derive(Component)]` actor-side tag (`is_player_side`/`is_hostile_side`,
/// with `Npc`/`Boss` variants). This one is the *damage* relationship; that one
/// is the *ECS actor* tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageTeam {
    Player,
    Enemy,
    Neutral,
    Environment,
}

impl DamageTeam {
    /// True when damage from `self` is allowed to affect `target` by default.
    pub fn can_damage(self, target: Self) -> bool {
Let me examine combat's on_hit/hitbox for projectile+actor overlap, and check persistence save_data for named content.

Bash items header + string literals
IN
cd /home/joncrall/code/ambition/crates && echo "=== items lib.rs header ===" && head -50 ambition_items/src/lib.rs && echo "" && echo "=== items: named content? ===" && grep -rhoiE '"[a-z_]+"' ambition_items/src/lib.rs ambition_items/src/shop.rs | sort -u | head -40

OUT
=== items lib.rs header ===
//! Canonical finite item catalog — the game's complete set of pickup items.
//!
//! Jon's design call (2026-06-03): the inventory menu is modeled on the
//! Ocarina-of-Time "Select Item" subscreen, which is a **6 × 4 = 24-slot grid**
//! (`submodules/ambition_menu/DESIGN-OOT-DEMO.md`: "The Items page uses a
//! 6 × 4 item grid based on OoT's inventory slot order"). That slot count is not
//! just a UI detail — **24 is the finite number of distinct pickup items in this
//! game.** Every collectible/equippable/usable item the player can ever hold maps
//! to exactly one of these 24 slots, in a fixed grid order.
//!
//! This module is the source of truth for that set. It is deliberately
//! presentation-independent: the unified tabbed menu renders
//! it, but pickups, dialogue (`<<give_item>>` /
//! `inventory_has`), and the equip path all read/write [`OwnedItems`] here. The
//! menu can be cut without touching this catalog.
//!
//! Some slots map to systems that already exist (portal gun, axe, javelin,
//! gun-sword, fireball, bubble shield, health/mana cells). Others are
//! reserved placeholders for
//! planned items (puppy-slug gun, grapple, morph ball, bombs, the Alice/Bob
//! cartography key items) — they still occupy a real, stable slot so the grid
//! shows "every item you could ever have," OoT-style, with un-acquired entries
//! dimmed.

pub mod shop;

use bevy::prelude::Resource;

/// Number of item slots — the OoT item subscreen's 6 × 4 grid, and therefore the
/// total number of distinct pickup items in the game.
pub const ITEM_GRID_COLS: usize = 6;
pub const ITEM_GRID_ROWS: usize = 4;
pub const ITEM_COUNT: usize = ITEM_GRID_COLS * ITEM_GRID_ROWS; // 24

/// Broad behavior class for an item. Drives how the menu's confirm action treats
/// the slot and how the slot reads ("Equip" vs "Use" vs key item).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ItemCategory {
    /// An equippable weapon/tool that grants an `ActionSet` via a `HeldItem`
    /// (portal gun, axe, gun-sword, …). Confirm = equip/unequip.
    Weapon,
    /// A movement/utility ability (blink, fly, morph ball, …). Owned = available;
    /// confirm currently just inspects (real toggle wiring is a follow-up).
    Ability,
    /// A stackable consumable (health/mana cell, bomb, …). Confirm = use one.
    Consumable,
    /// A unique quest/key item (map fragment, sealed note, gate key, …). Owned
    /// flag only; not directly usable from the grid.
    KeyItem,
    /// A reserved slot kept to hold the grid's shape — a future item lands here.

=== items: named content? ===
"Axe"
"Blink"
"Bomb"
"Equip"
"Fireball"
"Flight"
"Javelin"
"PortalGun"
"Use"
"axe"
"blink"
"bomb"
"bubbleshield"
"datachip"
"debuglens"
"fieldsurvey"
"fireball"
"fly"
"gatekey"
"goldpouch"
"grapple"
"gun_sword"
"gunsword"
"healthcell"
"healthpotion"
"javelin"
"manacell"
"mapfragment"
"mark_recall"
"markrecall"
"morphball"
"portal_gun"
"portalgun"
"puppy_slug_gun"
"puppysluggun"
"reservedslot"
"sealednote"
"sparebattery"
Bash sim_view actor read sites
IN
echo "=== sim_view: what it reads from ambition_actors ===" && grep -rn "ambition_actors" crates/ambition_sim_view/src | head -40

OUT
=== sim_view: what it reads from ambition_actors ===
crates/ambition_sim_view/src/lib.rs:70:                    ambition_actors::features::advance_actor_anim_overlays,
crates/ambition_sim_view/src/camera_snapshot.rs:13:use ambition_actors::rooms::{
crates/ambition_sim_view/src/camera_snapshot.rs:545:    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>,
crates/ambition_sim_view/src/camera_snapshot.rs:562:            &ambition_actors::avatar::PlayerBlinkCameraState,
crates/ambition_sim_view/src/anim_index.rs:7:use ambition_actors::features::{
crates/ambition_sim_view/src/anim_index.rs:20:/// [`ambition_actors::actor::AncillaryMovementBundle`] (the same bundle the player nests)
crates/ambition_sim_view/src/anim_index.rs:35:    pub ground: &'static ambition_actors::actor::BodyGroundState,
crates/ambition_sim_view/src/anim_index.rs:40:    pub flight: &'static ambition_actors::actor::BodyFlightState,
crates/ambition_sim_view/src/anim_index.rs:41:    pub body_mode: &'static ambition_actors::actor::BodyModeState,
crates/ambition_sim_view/src/anim_index.rs:42:    pub env_contact: &'static ambition_actors::actor::BodyEnvironmentContact,
crates/ambition_sim_view/src/anim_index.rs:43:    pub abilities: &'static ambition_actors::actor::BodyAbilities,
crates/ambition_sim_view/src/anim_index.rs:44:    pub shield: &'static ambition_actors::actor::BodyShieldState,
crates/ambition_sim_view/src/anim_index.rs:49:    pub anim: Option<&'static ambition_actors::actor::BodyAnimFacts>,
crates/ambition_sim_view/src/anim_index.rs:117:/// uses ([`ambition_actors::character_sprites::pick_actor_anim`] → `body_view_from_body`).
crates/ambition_sim_view/src/anim_index.rs:127:        let anim = ambition_actors::character_sprites::pick_actor_anim(
crates/ambition_sim_view/src/anim_index.rs:137:            ambition_actors::character_sprites::ActorAnimState {
crates/ambition_sim_view/src/anim_index.rs:191:    pub anim: ambition_actors::boss_encounter::sprites::BossAnimState,
crates/ambition_sim_view/src/anim_index.rs:199:    pub cursor_anim: ambition_actors::boss_encounter::sprites::BossAnim,
crates/ambition_sim_view/src/anim_index.rs:252:        ambition_actors::features::BossClusterRef,
crates/ambition_sim_view/src/anim_index.rs:262:        Option<&ambition_actors::boss_encounter::sprites::BossAnimFrame>,
crates/ambition_sim_view/src/anim_index.rs:265:    use ambition_actors::boss_encounter::sprites::BossAnim;
crates/ambition_sim_view/src/anim_index.rs:286:            ambition_actors::features::volumes_for_profile(
crates/ambition_sim_view/src/view_index.rs:10:use ambition_actors::features::HazardFeature;
crates/ambition_sim_view/src/view_index.rs:11:use ambition_actors::features::{
crates/ambition_sim_view/src/view_index.rs:121:            Option<&ambition_actors::platformer_runtime::orientation::ActorRoll>,
crates/ambition_sim_view/src/view_index.rs:131:        Without<ambition_actors::features::BossConfig>,
crates/ambition_sim_view/src/view_index.rs:136:        ambition_actors::features::BossClusterRef,
crates/ambition_sim_view/src/view_index.rs:149:        Option<&ambition_actors::platformer_runtime::orientation::ActorRoll>,
crates/ambition_sim_view/src/view_index.rs:579:    bosses: Query<(&FeatureId, ambition_actors::features::BossClusterRef)>,
crates/ambition_sim_view/src/facts.rs:14:use ambition_actors::actor::{BodyKinematics, BodyMana, PlayerEntity, PrimaryPlayer};
crates/ambition_sim_view/src/facts.rs:81:        &ambition_actors::features::HeldItem,
crates/ambition_sim_view/src/facts.rs:112:    grounds: Query<&ambition_actors::items::pickup::GroundItem>,
crates/ambition_sim_view/src/facts.rs:141:    items: Query<&ambition_actors::items::world_item::WorldItem>,
crates/ambition_sim_view/src/facts.rs:143:    use ambition_actors::items::world_item::WorldItemPayload;
crates/ambition_sim_view/src/facts.rs:172:        &ambition_actors::items::pickup::HeldProjectile,
crates/ambition_sim_view/src/facts.rs:190:    marks: Query<&ambition_actors::abilities::traversal::mark_recall::PlayerMark>,
crates/ambition_sim_view/src/facts.rs:208:    switches: Query<&ambition_actors::gravity::GravityFlipSwitch>,
crates/ambition_sim_view/src/facts.rs:229:    shrines: Query<&ambition_actors::shrine::HealShrine>,
crates/ambition_sim_view/src/facts.rs:243:    mut activation: ResMut<ambition_actors::shrine::ShrineActivationPulse>,
crates/ambition_sim_view/src/facts.rs:267:        &ambition_actors::features::ActorDisposition,
Bash Trace out struct type
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
grep -rn 'out.goblin\|\.goblin\b\|goblin:' character_sprites/ | head
echo "--- what type is out ---"
grep -rnB2 'out.player = Some' character_sprites/assets.rs | head
echo "--- search whole crate for a struct with goblin field ---"
grep -rnE '(goblin|robot|sandbag)\s*:\s*Option' --include='*.rs' . | head

OUT
character_sprites/assets.rs:336:                out.goblin = Some(asset);
--- what type is out ---
character_sprites/assets.rs-324-                out.npcs.insert(cid.clone(), asset.clone());
character_sprites/assets.rs-325-                out.npcs.insert(entry.display_name.clone(), asset.clone());
character_sprites/assets.rs:326:                out.player = Some(asset);
--- search whole crate for a struct with goblin field ---
Read on_hit.rs
Bash List sim_view published types
IN
echo "=== sim_view published read-model types (pub struct/enum) ===" && grep -rhn "pub struct\|pub enum" crates/ambition_sim_view/src | grep -iv "test" | head -60

OUT
=== sim_view published read-model types (pub struct/enum) ===
27:pub struct ActorSpriteData {
56:pub struct ActorAnimFrame {
77:pub struct ActorAnimIndex {
169:pub struct HazardLaneFact {
184:pub struct BossFrameIndex {
190:pub struct BossFrameView {
28:pub struct BodyPoseView {
206:pub struct ShieldRingFact {
216:pub struct ShieldRingsView(pub Vec<ShieldRingFact>);
44:pub struct FeatureViewSyncSchedulePlugin;
29:pub struct FeatureViewIndex {
378:pub struct ActorRenderView {
390:pub struct ActorRenderIndex {
502:pub struct BossRenderView {
508:pub struct BossRenderIndex {
598:pub struct NameplateFact {
609:pub struct NameplateIndex {
34:pub struct CameraSnapshot2d {
67:pub struct SceneCaptureRequest {
113:pub struct CameraFocus2d {
138:pub struct CameraBlinkInput {
146:pub enum CameraSnapshotResolveMode {
155:pub struct CameraSnapshotResolveInput<'a> {
503:pub struct CameraViewport {
521:pub struct CameraExtraClamp(pub Option<ae::Vec2>);
528:pub struct ResolvedCameraSnapshot {
647:pub struct CameraObservationPlugin;
27:pub struct PlayerHudFacts {
64:pub struct HeldItemView(pub Option<HeldItemFact>);
67:pub struct HeldItemFact {
101:pub struct GroundItemsView(pub Vec<GroundItemFact>);
104:pub struct GroundItemFact {
125:pub struct WorldItemsView(pub Vec<WorldItemFact>);
128:pub struct WorldItemFact {
157:pub struct HeldShotsView(pub Vec<HeldShotFact>);
160:pub struct HeldShotFact {
186:pub struct MarkBeaconsView(pub Vec<ae::Vec2>);
198:pub struct GravitySwitchesView(pub Vec<GravitySwitchFact>);
201:pub struct GravitySwitchFact {
219:pub struct ShrinesView(pub Vec<ShrineFact>);
222:pub struct ShrineFact {
254:pub struct WieldedGunSwordsView(pub Vec<GunSwordFact>);
257:pub struct GunSwordFact {
306:pub struct ProjectileView {
359:pub struct DynamicFeatureFact {
376:pub struct DynamicFeatureViews(pub Vec<DynamicFeatureFact>);
503:pub struct BlinkPreviewFact {
595:pub struct SimViewPlugin;
Bash Grep engine policy for boundaries
IN
echo "=== engine.toml policy (render/actors/sim_view boundary) ===" && grep -n "render\|actors\|sim_view\|forbid\|denylist\|roots\|name =" tests/ambition_workspace_policy/policies/engine.toml | head -60

OUT
=== engine.toml policy (render/actors/sim_view boundary) ===
14:kind = "forbidden-source-reference"
20:roots = ["crates/ambition_engine_core/src"]
21:forbid = ["ambition_content", "ambition_app", "ambition_runtime"]
29:kind = "dependency-denylist"
42:kind = "forbidden-source-reference"
45:rationale = "The moment the host moves a system that reaches for a content type, this fails instead of the boundary rotting. Docs deliberately MENTION the forbidden crate, so comments are exempt."
48:roots = ["crates/ambition_host/src"]
49:forbid = ["ambition_content"]
51:# ── migrated from crates/ambition_render/tests/observation_boundary.rs (Task 4) ──
53:# was: render_never_names_live_sim_state (E4 observation boundary)
55:id = "engine.render-never-names-live-sim-state"
57:kind = "forbidden-source-reference"
58:owners = ["ambition_render"]
59:watch_paths = ["crates/ambition_render/src"]
60:rationale = "Presentation reads ONLY the ambition_sim_view read-model — never the sim heart's live Body* clusters, actor/boss cluster refs, control seams, or item/ability sim components. Reintroducing a Query<&BodyKinematics> in render fails here. Whole-identifier match so GroundItemVisual does not trip GroundItem; vocabulary/data types (FeatureView, CharacterAnim, Health-as-data) are deliberately NOT forbidden."
64:roots = ["crates/ambition_render/src"]
65:forbid = [
79:# was: render_has_no_actor_crate_dependency_after_f15 (manifest half)
81:id = "engine.render-no-actor-crate-dependency"
83:kind = "dependency-denylist"
84:owners = ["ambition_render"]
85:watch_paths = ["crates/ambition_render/Cargo.toml"]
86:rationale = "F1.5 complete: ambition_render must not depend on or feature-forward to ambition_actors. Render and the machinery meet only through lower view/vocabulary crates."
89:manifest = "crates/ambition_render/Cargo.toml"
90:deny = ["ambition_actors"]
92:# was: render_has_no_actor_crate_dependency_after_f15 (source half)
94:id = "engine.render-source-names-no-actors"
96:kind = "forbidden-source-reference"
97:owners = ["ambition_render"]
98:watch_paths = ["crates/ambition_render/src"]
99:rationale = "F1.5 complete: render source must not name ambition_actors in code, even where the manifest dep is already gone."
102:roots = ["crates/ambition_render/src"]
103:forbid = ["ambition_actors"]
127:# Repetitive crate-purity rules only (workspace-member / dependency-denylist /
128:# forbidden-source-reference). Bespoke semantic assertions stay in the legacy
131:# was: architecture_boundaries_render_and_actor_crates_are_decoupled
132:# (the render→actor half is covered by engine.render-no-actor-crate-dependency +
133:#  engine.render-source-names-no-actors from Task 4; these cover the rest)
135:id = "engine.render-decoupled-member"
138:owners = ["ambition_render"]
140:rationale = "ambition_render is the sandbox's renderer and must stay a registered workspace member; the sim machinery and render meet only through lower vocabulary/read-model crates."
143:members = ["ambition_render"]
146:id = "engine.actor-manifest-no-render"
148:kind = "dependency-denylist"
149:owners = ["ambition_actors"]
150:watch_paths = ["crates/ambition_actors/Cargo.toml"]
151:rationale = "The sim machinery must not depend on its renderer (render depends on the sim, never the reverse)."
154:manifest = "crates/ambition_actors/Cargo.toml"
155:deny = ["ambition_render"]
158:id = "engine.actor-source-no-render"
160:kind = "forbidden-source-reference"
161:owners = ["ambition_actors"]
162:watch_paths = ["crates/ambition_actors/src"]
163:rationale = "ambition_actors must not reference the render crate in code either."
166:roots = ["crates/ambition_actors/src"]
167:forbid = ["ambition_render"]
171:id = "engine.menu-crate-manifest-no-actors"
173:kind = "dependency-denylist"
176:rationale = "ambition_menu is the reusable menu renderer; the game owns menu content, so it must not depend on the actor machinery."
180:deny = ["ambition_actors"]
Bash Inspect combat on_hit and module headers
IN
cd /home/joncrall/code/ambition
echo "=== combat events.rs: HitSource / projectile references ==="
grep -n "Projectile\|HitSource\|projectile" crates/ambition_combat/src/events.rs | head -30
echo
echo "=== combat: hitbox/on_hit/slots/targeting one-line headers ==="
for f in hitbox/mod.rs on_hit.rs slots.rs targeting.rs hazards.rs breakables.rs falling_chest.rs authored_volumes.rs path_motion.rs held_items.rs banner.rs variation.rs; do
  echo "--- $f ---"; head -3 crates/ambition_combat/src/$f | sed 's/^/  /'
done

OUT
=== combat events.rs: HitSource / projectile references ===
142:/// projectile, boss swing); leave `None` for impulse-free hits
143:/// (player slash, player projectile into a feature, pogo).
304:/// `HitSource` partitions naturally into two directions:
305:/// - **Attacker-side** (`PlayerSlash`, `PlayerProjectile`,
310:///   `EnemyProjectile`, `BossBody`, `BossAttack`) — consumed by the
317:pub enum HitSource {
322:    /// Player projectile (Fireball / Hadouken). No knockback today;
323:    /// the projectile's own velocity carries the visual feedback.
324:    PlayerProjectile,
342:    /// Hit by an enemy-fired projectile (pirate volley, etc).
345:    EnemyProjectile,
355:impl HitSource {
363:            HitSource::PlayerSlash { .. }
364:                | HitSource::PlayerProjectile
365:                | HitSource::PogoBounce
366:                | HitSource::EnemyChargeCrash
376:    /// hits (player slash, player projectile, hazard areas in
403:/// - `PlayerSlash` / `PlayerProjectile`: broadcast match — every
422:    pub source: HitSource,
424:    /// sources (slash, pogo, player projectile) stamp the player whose
432:    /// `EnemyProjectile` (string-`ProjectileOwnerId`-owned, not entity-
443:    /// field on `HitSource::PlayerSlash`; pogo / player-projectile
447:    /// target source. Empty for ordinary one-frame projectiles /

=== combat: hitbox/on_hit/slots/targeting one-line headers ===
--- hitbox/mod.rs ---
  //! Hitbox-entity lifecycle: spawn → overlap-check → despawn.
  //!
  //! Per the actor/brain follow-up plan
--- on_hit.rs ---
  //! On-hit techniques — the conditional-hit primitive of the ability model.
  //!
  //! A [`HitVolume`] (ambition_entity_catalog::HitVolume) may carry an
--- slots.rs ---
  //! Anti-clump attack-slot arbitration.
  //!
  //! Default sandbox enemies all resolve "Chase" to "walk toward the
--- targeting.rs ---
  //! Per-frame `ActorTarget` selection for non-player actors.
  //!
  //! Runs at the top of the actor simulation chain so each enemy /
--- hazards.rs ---
  //! Hazard tick: patrol motion, contact damage, and the impact SFX/VFX
  //! published to the presentation/audio buses.
  
--- breakables.rs ---
  //! Per-frame tick for breakable feature entities: respawn countdown
  //! and the stand-to-break collapse trigger.
  
--- falling_chest.rs ---
  //! Falling-chest physics for ECS reward chests.
  //!
  //! Reward chests spawned mid-air by `sync_boss_reward_chests_ecs`
--- authored_volumes.rs ---
  //! App-local authored attack-volume resolution.
  //!
  //! Combat owns the query shape but not sprite metadata. The composition root
--- path_motion.rs ---
  //! `PathMotion`: waypoint-following used by moving hazards/platforms.
  //!
  //! Walks a position along a `ambition_engine_core::KinematicPath` by `speed * dt`
--- held_items.rs ---
  //! ECS-owned held item capability for actors.
  //!
  //! The item component is the durable answer to "what is this actor holding?".
--- banner.rs ---
  //! Gameplay banner ticking and deferred-request application.
  
  use super::*;
--- variation.rs ---
  //! Stable per-actor variation helpers for ECS feature actors.
  //!
  //! Enemy brain configs should be deterministic for a given authored actor id,
Bash Peek at the ambition umbrella crate
IN
ls crates/ambition/src/ && head -60 crates/ambition/src/lib.rs

OUT
lib.rs
prelude.rs
provider.rs
session_world.rs
//! Public facade for Ambition-derived platformer games.
//!
//! This crate is the E9 umbrella surface: a downstream game should depend on
//! `ambition` plus its own content crate instead of copying the app shell's wall
//! of lower `ambition_*` dependencies. It deliberately re-exports the engine,
//! host, renderer, model, and vocabulary crates without depending on any named
//! game content or the `ambition_app` shell.

pub mod prelude;
pub mod provider;
pub mod session_world;

pub use ambition_actors as actors;
pub use ambition_asset_manager as asset_manager;
pub use ambition_audio as audio;
pub use ambition_characters as characters;
pub use ambition_combat as combat;
pub use ambition_cutscene as cutscene;
pub use ambition_dev_tools as dev_tools;
pub use ambition_dialog as dialog;
pub use ambition_encounter as encounter;
pub use ambition_engine_core as engine_core;
pub use ambition_entity_catalog as entity_catalog;
pub use ambition_game_shell as game_shell;
pub use ambition_host as host;
pub use ambition_input as input;
pub use ambition_interaction as interaction;
pub use ambition_inventory_ui as inventory_ui;
pub use ambition_items as items;
pub use ambition_ldtk_map as ldtk_map;
pub use ambition_load as load;
pub use ambition_load_presentation as load_presentation;
pub use ambition_menu as menu;
pub use ambition_persistence as persistence;
pub use ambition_platformer_primitives as platformer;
pub use ambition_portal as portal;
pub use ambition_portal_presentation as portal_presentation;
pub use ambition_projectiles as projectiles;
pub use ambition_render as render;
pub use ambition_runtime as runtime;
pub use ambition_settings_menu as settings_menu;
pub use ambition_sfx as sfx;
pub use ambition_sfx_bank as sfx_bank;
pub use ambition_sim_view as sim_view;
pub use ambition_sprite_sheet as sprite_sheet;
pub use ambition_time as time;
pub use ambition_touch_input as touch_input;
pub use ambition_ui_nav as ui_nav;
pub use ambition_vfx as vfx;
pub use ambition_world as world;
// Re-exported so a game can name bevy TYPES through `ambition::bevy::…`. NOTE:
// this does NOT let a crate `#[derive(Component)]`/`#[derive(Resource)]` through
// the umbrella alone — bevy's derive macros resolve `::bevy_ecs` via the
// CONSUMER's own Cargo.toml (`BevyManifest`), which a re-export does not satisfy.
// A content crate that defines its own components/resources must ALSO list `bevy`
// in its manifest (one line, version pinned by the workspace). See
// docs/planning/demos/README.md.
pub use bevy;

/// Engine assembly helpers most games need first.
Let me classify the menu and RL subsystems (generic engine machinery vs Ambition content) for the app-decomposition question.

The policy file is a goldmine. Let me read the exact forbidden-identifier list for render and the menu-crate rules.

Bash Read render + menu policy detail
IN
sed -n '51,200p' tests/ambition_workspace_policy/policies/engine.toml

OUT
# ── migrated from crates/ambition_render/tests/observation_boundary.rs (Task 4) ──

# was: render_never_names_live_sim_state (E4 observation boundary)
[[policy]]
id = "engine.render-never-names-live-sim-state"
scope = "engine"
kind = "forbidden-source-reference"
owners = ["ambition_render"]
watch_paths = ["crates/ambition_render/src"]
rationale = "Presentation reads ONLY the ambition_sim_view read-model — never the sim heart's live Body* clusters, actor/boss cluster refs, control seams, or item/ability sim components. Reintroducing a Query<&BodyKinematics> in render fails here. Whole-identifier match so GroundItemVisual does not trip GroundItem; vocabulary/data types (FeatureView, CharacterAnim, Health-as-data) are deliberately NOT forbidden."
source_doc = "docs/planning/engine/decomposition.md#E4"
severity = "error"
whole_ident = true
roots = ["crates/ambition_render/src"]
forbid = [
    "BodyKinematics", "BodyGroundState", "BodyWallState", "BodyBlinkState",
    "BodyFlightState", "BodyDashState", "BodyLedgeState", "BodyModeState",
    "BodyEnvironmentContact", "BodyAbilities", "BodyDodgeState", "BodyShieldState",
    "BodyCombat", "BodyHealth", "BodyMana", "BodyWallet", "BodyMelee",
    "BodyAnimFacts", "BodyBaseSize",
    "ControlledSubject", "ActorControl", "SlotControls", "PlayerBlinkCameraState",
    "ActorSpriteData", "BossClusterRef", "BossAttackState", "BossPhase",
    "ActorDisposition", "ActorIdentity", "ActorStatus", "ActorConfig", "ActorRoll",
    "MeleeSwing", "CenteredAabb", "FeatureSimEntity", "FeatureName",
    "HeldItem", "GroundItem", "HeldProjectile", "PlayerMark", "HealShrine",
    "GravityFlipSwitch", "LiveProjectile", "PlayerProjectileState",
]

# was: render_has_no_actor_crate_dependency_after_f15 (manifest half)
[[policy]]
id = "engine.render-no-actor-crate-dependency"
scope = "engine"
kind = "dependency-denylist"
owners = ["ambition_render"]
watch_paths = ["crates/ambition_render/Cargo.toml"]
rationale = "F1.5 complete: ambition_render must not depend on or feature-forward to ambition_actors. Render and the machinery meet only through lower view/vocabulary crates."
source_doc = "docs/planning/engine/decomposition.md#F1.5"
severity = "error"
manifest = "crates/ambition_render/Cargo.toml"
deny = ["ambition_actors"]

# was: render_has_no_actor_crate_dependency_after_f15 (source half)
[[policy]]
id = "engine.render-source-names-no-actors"
scope = "engine"
kind = "forbidden-source-reference"
owners = ["ambition_render"]
watch_paths = ["crates/ambition_render/src"]
rationale = "F1.5 complete: render source must not name ambition_actors in code, even where the manifest dep is already gone."
source_doc = "docs/planning/engine/decomposition.md#F1.5"
severity = "error"
roots = ["crates/ambition_render/src"]
forbid = ["ambition_actors"]

# ── migrated from crates/ambition_world/src/lib.rs dependency ratchet (Task 4) ──

# was: ambition_world_dependency_allowlist_ratchets_world_ir_purity
[[policy]]
id = "engine.world-ir-dependency-allowlist"
scope = "engine"
kind = "dependency-allowlist"
owners = ["ambition_world"]
watch_paths = ["crates/ambition_world/Cargo.toml"]
rationale = "ambition_world owns the backend-agnostic world IR (room graph, placements, platform math, composited collision world). It may name only explicit world-IR dependencies; the exact ratchet forces a legacy entry out of the allow-list as each placement branch dissolves."
source_doc = "docs/planning/engine/decomposition.md#W3"
severity = "error"
exact = true
manifest = "crates/ambition_world/Cargo.toml"
allow = [
    "ambition_engine_core",
    "ambition_entity_catalog",
    "ambition_platformer_primitives",
    "ambition_time",
]

# ── architecture_boundaries.rs decomposition, batch 1 (Task 6) ─────────────────
# Repetitive crate-purity rules only (workspace-member / dependency-denylist /
# forbidden-source-reference). Bespoke semantic assertions stay in the legacy
# file until Task 9. Stable IDs derive from the old test names.

# was: architecture_boundaries_render_and_actor_crates_are_decoupled
# (the render→actor half is covered by engine.render-no-actor-crate-dependency +
#  engine.render-source-names-no-actors from Task 4; these cover the rest)
[[policy]]
id = "engine.render-decoupled-member"
scope = "engine"
kind = "workspace-member"
owners = ["ambition_render"]
watch_paths = ["Cargo.toml"]
rationale = "ambition_render is the sandbox's renderer and must stay a registered workspace member; the sim machinery and render meet only through lower vocabulary/read-model crates."
source_doc = "docs/planning/engine/decomposition.md#F1.5"
severity = "error"
members = ["ambition_render"]

[[policy]]
id = "engine.actor-manifest-no-render"
scope = "engine"
kind = "dependency-denylist"
owners = ["ambition_actors"]
watch_paths = ["crates/ambition_actors/Cargo.toml"]
rationale = "The sim machinery must not depend on its renderer (render depends on the sim, never the reverse)."
source_doc = "docs/planning/engine/decomposition.md#F1.5"
severity = "error"
manifest = "crates/ambition_actors/Cargo.toml"
deny = ["ambition_render"]

[[policy]]
id = "engine.actor-source-no-render"
scope = "engine"
kind = "forbidden-source-reference"
owners = ["ambition_actors"]
watch_paths = ["crates/ambition_actors/src"]
rationale = "ambition_actors must not reference the render crate in code either."
source_doc = "docs/planning/engine/decomposition.md#F1.5"
severity = "error"
roots = ["crates/ambition_actors/src"]
forbid = ["ambition_render"]

# was: architecture_boundaries_menu_crate_stays_content_free
[[policy]]
id = "engine.menu-crate-manifest-no-actors"
scope = "engine"
kind = "dependency-denylist"
owners = ["ambition_menu"]
watch_paths = ["crates/ambition_menu/Cargo.toml"]
rationale = "ambition_menu is the reusable menu renderer; the game owns menu content, so it must not depend on the actor machinery."
source_doc = "docs/planning/engine/decomposition.md"
severity = "error"
manifest = "crates/ambition_menu/Cargo.toml"
deny = ["ambition_actors"]

[[policy]]
id = "engine.menu-crate-source-no-actors"
scope = "engine"
kind = "forbidden-source-reference"
owners = ["ambition_menu"]
watch_paths = ["crates/ambition_menu/src"]
rationale = "ambition_menu must stay content-free — no ambition_actors references in code."
source_doc = "docs/planning/engine/decomposition.md"
severity = "error"
roots = ["crates/ambition_menu/src"]
forbid = ["ambition_actors"]

# was: architecture_boundaries_persistence_crate_owns_stored_shapes_only
[[policy]]
id = "engine.persistence-crate-member"
scope = "engine"
kind = "workspace-member"
owners = ["ambition_persistence"]
watch_paths = ["Cargo.toml"]
Let me check persistence save_data and quest for named content, and confirm the projectiles/combat victim-side split.

Bash Menu genericity check
IN
cd game/ambition_app/src; echo "=== menu/mod.rs ==="; cat menu/mod.rs; echo; echo "=== does the app menu name Ambition-specific content? (quests/bosses/specific rooms) ==="; grep -rlnE "cut_rope|goblin|gnu_ton|gradient_sentinel|AMBITION_EXPERIENCE|smirking" menu/ | head; echo "--- menu/model.rs head ---"; sed -n '1,30p' menu/model.rs

OUT
=== menu/mod.rs ===
//! Game-side menu host stack: backend-agnostic page model, dispatcher, item
//! effects, and the flat-grid / 3D-cube presentation hosts.
//!
//! `ambition::actors::menu` keeps the lib-coupled settings IR, Map tab, and
//! backend selector; this crate owns app-level wiring and effects.

pub mod dispatch;
pub mod effects;
#[cfg(feature = "bevy_ui_menu")]
pub mod grid_backend;
pub mod kaleidoscope_app;
pub mod model;
pub(crate) mod quality_confirm;
#[cfg(test)]
mod test_support;

#[cfg(all(test, feature = "bevy_ui_menu", feature = "kaleidoscope_menu"))]
mod parity_tests;

=== does the app menu name Ambition-specific content? (quests/bosses/specific rooms) ===
--- menu/model.rs head ---
//! The data seam between Ambition's live 24-item inventory and the reusable
//! `ambition::menu` 3D-cube OoT pause menu (#31).
//!
//! The game owns the item state (`ambition::actors::items`); this module builds the cube's
//! page MODELS from it via the lib's host-data seam (`ItemsOnlyPageSpec`, which is
//! deliberately renderer-agnostic — it can feed the Lunex cube, a Bevy-UI grid
//! fallback, or a test renderer). The cube RENDERER itself is the shared lib.
//!
//! This gives the "wire us up to use it" part: our `Item::ALL` (already 24 in OoT
//! grid order) → the cube's items page, with owned/equipped/selected reflected and
//! a host-defined [`MenuPageAction`] emitted back to the game.
//!
//! ## Items-page layout (matches `ambition_mock_demo`)
//!
//! The proven demo (`crates/ambition_mock_demo/src/app/models.rs`) does NOT render
//! each item's full description inside its grid cell (that overlapping mush is the
//! bug this file fixes). Instead it shows short item NAMES in a compact grid and
//! renders the *focused* item's wrapped description once, in a dedicated detail
//! panel beside the grid. We replicate that structure here:
//!
//! * the grid sits in the left/centre (panel rect [`GRID_RECT`]), each cell shows a
//!   short, wrapped item name and a one-word action hint (Equip/Use/...),
//! * the [`DETAIL_PANEL_RECT`] on the right shows the [`MenuFocus`]ed item's name +
//!   wrapped description (filled by `lunex_kaleidoscope_app::kaleidoscope_sync_detail_panel`),
//! * the L/R page-turn buttons live in the *side margins* ([`EDGE_LEFT_RECT`] /
//!   [`EDGE_RIGHT_RECT`]) OUTSIDE the grid, exactly like the demo.

use ambition::menu::{
    InventoryItemNode, ItemsOnlyPageSpec, MenuColor, MenuControlKind, MenuPageModel, MenuRect,
    MenuTextAlign,
Read mod.rs
ambition_items is explicitly game content ("the game's complete set of pickup items", hardcoded 24-item roster) living in crates/. Now let me check the "catalog" concept overlap and inspect ldtk_map for leakage.

Bash Check persistence for named content
IN
cd /home/joncrall/code/ambition
echo "=== persistence save_data.rs: top + named content check ==="
head -40 crates/ambition_persistence/src/save_data.rs
echo
echo "=== named content in persistence ==="
grep -rinE "mockingbird|clockwork|behemoth|spaghetti|trex|mode_collapse|gnu_ton|mary|sanic|boss_intro|specific" crates/ambition_persistence/src --include=*.rs | grep -viE "test|assert" | head -20
echo
echo "=== quest/mod.rs top ==="
head -30 crates/ambition_persistence/src/quest/mod.rs

OUT
=== persistence save_data.rs: top + named content check ===
//! Pure save-game data shapes (`SandboxSaveData`, `PersistedEncounter`,
//! `PersistedSwitch`, ability/quest flags) — the vocabulary the save format
//! is built from.
//!
//! These types are pure data + `serde`: no Bevy, file paths, autosave timing,
//! or LDtk. The Bevy-side disk shim that loads/saves them lives in the sibling
//! `crate::save` module. Keeping the shapes I/O-free gives reusable
//! mechanics (encounter defeat, switch latch, ability flags) one canonical form
//! shared across sandbox and any future story / editor tooling.

use serde::{Deserialize, Serialize};

/// One persisted encounter (e.g. goblin encounter) entry. Only the terminal /
/// in-progress states matter for save reconstruction; `Inactive`
/// reconstructs to "fresh attempt available" without needing an entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEncounter {
    pub id: String,
    pub state: PersistedEncounterState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedEncounterState {
    /// Default for an encounter that has never been triggered, or one
    /// that was reset via a switch. Not usually written to disk —
    /// missing entries reconstruct to this value.
    #[default]
    Untouched,
    /// Cleared all waves. Surviving terminal state.
    Cleared,
    /// Player died. Resets back to `Untouched` on switch reset; written
    /// so a save mid-attempt restores meaningfully.
    Failed,
}

impl PersistedEncounter {
    pub fn new(id: impl Into<String>, state: PersistedEncounterState) -> Self {
        Self {
            id: id.into(),
            state,

=== named content in persistence ===
crates/ambition_persistence/src/quest/mod.rs:110:/// A complete quest: title, summary shown in the menu, and ordered
crates/ambition_persistence/src/quest/mod.rs:117:    pub summary: String,
crates/ambition_persistence/src/quest/mod.rs:125:        summary: impl Into<String>,
crates/ambition_persistence/src/quest/mod.rs:131:            summary: summary.into(),
crates/ambition_persistence/src/quest/mod.rs:219:    pub fn hud_summary(&self) -> String {
crates/ambition_persistence/src/quest/mod.rs:293:            "summary",
crates/ambition_persistence/src/settings/gameplay.rs:46:    /// path. goblin_encounter waves are authored with specific positions /
crates/ambition_persistence/src/quest/registry.rs:54:            .map(|q| q.hud_summary())
crates/ambition_persistence/src/quest/registry.rs:58:    pub fn active_quest_summary(&self) -> Option<String> {
crates/ambition_persistence/src/quest/registry.rs:62:            .map(|q| q.hud_summary())
crates/ambition_persistence/src/quest/registry.rs:154:    fn active_quest_summary_finds_one_active() {
crates/ambition_persistence/src/quest/registry.rs:159:        let summary = registry.active_quest_summary();

=== quest/mod.rs top ===
//! Quest data types and progression rules.
//!
//! A quest is a fixed-order list of named steps. Progression is driven
//! by *advance events* (the sandbox feeds them in from gameplay):
//! talking to an NPC, defeating a boss, picking up an item, entering a
//! room. Each step declares the events that satisfy it; when the
//! current step's predicate is met, the quest advances.
//!
//! This module is intentionally Bevy-free so the same data can be
//! serialized into the save file (`PersistedQuest`) and used from
//! tests / headless / RL drivers.
//!
//! Failure paths (timed quests, mutually-exclusive choices, side
//! quests that gate later content) are not modeled yet — the quest
//! state machine only encodes the simplest "do these in order"
//! pattern. When real quests demand more, extend `QuestStepCondition`
//! with disjunction / failure variants.

use serde::{Deserialize, Serialize};

use crate::save_data::PersistedQuestState;

pub mod registry;
pub use registry::{
    apply_quest_advance_events, push_room_entered_quest_event_for_room, QuestRegistry,
};

/// A single advance event the sandbox emits during gameplay.
///
/// String ids are used (not integer keys) so authoring can stay
Bash Catalog concept overlap across three crates
IN
cd /home/joncrall/code/ambition/crates && echo "=== 'Catalog' struct/type defs across the three ===" && grep -rn "struct .*Catalog\|enum .*Catalog\|type .*Catalog" ambition_entity_catalog/src ambition_characters/src ambition_items/src | head -30 && echo "" && echo "=== items ItemId / ItemSlot roster enum ===" && grep -n "enum ItemId\|enum ItemSlot\|pub const.*ItemId\|fn all_items\|ITEM_SLOTS\|CATALOG" ambition_items/src/lib.rs | head

OUT
=== 'Catalog' struct/type defs across the three ===
ambition_entity_catalog/src/lib.rs:737:pub struct EntityCatalogDoc {
ambition_entity_catalog/src/lib.rs:749:pub enum CatalogError {
ambition_characters/src/actor/character_catalog/mod.rs:47:pub struct CharacterCatalog(CharacterCatalogData);
ambition_characters/src/actor/character_catalog/mod.rs:200:pub struct CharacterCatalogPlugin {
ambition_characters/src/actor/character_catalog/registry.rs:18:pub struct CharacterCatalogFragment {
ambition_characters/src/actor/character_catalog/registry.rs:102:pub struct CharacterCatalogRegistry {
ambition_characters/src/actor/character_catalog/registry.rs:208:pub struct CharacterCatalogDefaults(pub BTreeMap<String, String>);
ambition_characters/src/actor/character_catalog/registry.rs:218:pub struct CharacterCatalogOwners(pub BTreeMap<String, String>);
ambition_characters/src/actor/character_catalog/registry.rs:227:pub struct AssembledCharacterCatalog {
ambition_characters/src/actor/character_catalog/registry.rs:234:pub enum CharacterCatalogAssemblyError {
ambition_characters/src/actor/character_catalog/entry.rs:343:pub struct CharacterCatalogEntry {
ambition_characters/src/actor/character_catalog/entry.rs:608:pub struct CharacterCatalogData {
ambition_items/src/lib.rs:129:pub struct ItemCatalog {

=== items ItemId / ItemSlot roster enum ===
121:/// Content-installed item CATALOG override (C1), mirroring
150:static ITEM_CATALOG_OVERRIDE: std::sync::OnceLock<ItemCatalog> = std::sync::OnceLock::new();
155:    let _ = ITEM_CATALOG_OVERRIDE.set(catalog);
162:    ITEM_CATALOG_OVERRIDE
Bash ldtk entity_converters header + named content
IN
cd /home/joncrall/code/ambition/crates && echo "=== ldtk entity_converters.rs header ===" && head -40 ambition_ldtk_map/src/conversion/entity_converters.rs && echo "" && echo "=== ldtk: named Ambition content? ===" && grep -rhoiE "goblin|mockingbird|sentinel|dark.?lord|alice|\bbob\b|architect|vault.?keeper|sandbag|mary.?o|sanic" ambition_ldtk_map/src | sort | uniq -c | sort -rn | head -20

OUT
=== ldtk entity_converters.rs header ===
//! The engine's standard LDtk entity converters — the built-in vocabulary
//! registered in `standard_converters()`. One `convert_*` per LDtk entity
//! type, all with the uniform [`LdtkEntityConverter`] signature so the
//! registry treats engine and content converters identically. Helpers +
//! `RoomEmission` stay in the parent and are reached via `super::*`
//! (descendant visibility).

use super::*;
use ambition_entity_catalog::placements::{HazardSpec, PlacementSchema};

/// `PlayerStart` — the area's spawn point (box center).
pub(super) fn convert_player_start(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    Ok(RoomEmission::spawn(ctx.min + ctx.size * 0.5))
}

/// Surface-shaped identifiers (`Solid`, `OneWayPlatform`, `BlinkWall`,
/// `HazardBlock`, `PogoOrb`, `ReboundPad`, `BreakablePlatform`,
/// `BreakablePogoOrb`) all share one typed parse → compile pipeline, so
/// collision/contact systems consume a single runtime IR.
pub(super) fn convert_surface(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    let spec = parse_surface_spec(entity, min, size, name)?;
    let compiled = compile_surface(&spec)?;
    Ok(RoomEmission::from_compiled(compiled))
}

/// `StitchedBoundary` / `EncounterTrigger` / `LockWall` are read by their own
/// consumers off the raw `LdtkProject` and never join the emission stream.
pub(super) fn convert_consumed_elsewhere(_ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    Ok(RoomEmission::ignored())
}

pub(super) fn convert_loading_zone(ctx: &LdtkEntityCtx<'_>) -> Result<RoomEmission, String> {
    let (entity, name, min, size) = ctx.parts();
    Ok(RoomEmission::zone(LoadingZone {
        id: field_string(entity, "id").unwrap_or_else(|| entity.iid.clone()),
        name,
        activation: match field_string(entity, "activation")
            .unwrap_or_else(|| "Door".to_string())
            .as_str()

=== ldtk: named Ambition content? ===
     10 sanic
      2 alice
      2 Goblin
Let me compute reverse-dependencies to identify leaf candidates (which modules nobody imports).

The seam is explicitly enforced (Health-as-data and FeatureView are allowed vocabulary; live Body* clusters are forbidden). Now let me verify render's truly-unused deps and pivot to touch_input, portal_presentation, and the UI cluster.

Initial recon is done and the six deep-dive agents are still running. A quick interim picture of what I've established so far:

The lay of the land. The workspace is 49 members. Heavyweights: ambition_actors 68.7k LOC, ambition_characters 23.4k, ambition_engine_core 23.3k, ambition_app 22.3k, ambition_content 14.3k. The planning stack (architecture.md's tiered end-state, decomposition.md's measured ledger, the 2026-07-10 ruling that no further actors split is owed) is far more developed than "last full audit" suggested — but it was last re-baselined 2026-07-09/10, and the code has moved a lot since.

Fresh growth data (git, since 2026-07-10): game_shell +5.1k (essentially a new crate from the multi-game host work), engine_core +4.9k, actors +4.7k net with 20.8k/16.1k added/deleted churn (the melee/moveset unifications), audio +2.5k, load/load_presentation +2.8k combined. The decomposition ledger's destination-crate table is now stale — several "small" crates aren't small anymore.

Suspects already visible from the dependency graph alone:

ambition_touch_input depends on ambition_actors and ambition_render — architecture.md places it in Tier 2 (device→intent) with arrows only toward kernels. That's a documented-plan violation at HEAD.
ambition_render has 19 workspace deps; the doc permits Tier 0–2 vocabularies but the breadth deserves a per-dep audit (an agent is on it).
ambition_sprite_sheet (Tier 0, "sprite-geometry authority") depends on combat, interaction, persistence, world — upward-looking for a Tier-0 vocabulary crate.
ambition_sim_view depends on ambition_actors — legal per the tiers, but it makes the observation boundary rebuild whenever the sim heart changes.
Once the agents report, I'll synthesize a ranked opportunity list (respecting the binding rulings, flagging where new evidence justifies revisiting one) and write it into docs/planning/ as the recon deliverable.

Bash Compute reverse dependency counts per module
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== REVERSE DEPS: how many OTHER top-level modules reference each (importers) ==="
for target in features boss_encounter avatar abilities character_sprites affordances items projectile encounter dev world persistence audio time session control body_mode menu enemy_projectile assets schedule gravity dialog music shrine actor combat rooms ldtk_world; do
  importers=""
  for d in features boss_encounter avatar abilities character_sprites affordances items projectile encounter dev world persistence audio time session control body_mode menu enemy_projectile assets schedule gravity dialog music shrine host quest; do
    [ "$d" = "$target" ] && continue
    n=$(grep -rhoE "crate::${target}\b" "$d" "$d.rs" 2>/dev/null | wc -l)
    [ "$n" -gt 0 ] && importers="$importers $d($n)"
  done
  cnt=$(echo $importers | wc -w)
  echo "[$cnt] $target <=$importers"
done

OUT
=== REVERSE DEPS: how many OTHER top-level modules reference each (importers) ===
[17] features <= boss_encounter(53) avatar(18) abilities(60) affordances(8) items(30) projectile(16) encounter(24) dev(2) world(3) time(2) session(13) control(1) body_mode(6) enemy_projectile(25) assets(1) gravity(9) dialog(3)
[9] boss_encounter <= features(96) items(1) projectile(2) encounter(1) world(1) audio(1) session(4) enemy_projectile(1) assets(8)
[9] avatar <= features(19) character_sprites(2) projectile(6) dev(7) time(2) session(10) control(6) enemy_projectile(5) schedule(1)
[6] abilities <= features(3) avatar(1) items(17) session(5) control(1) enemy_projectile(1)
[3] character_sprites <= features(11) boss_encounter(2) assets(2)
[1] affordances <= avatar(1)
[4] items <= features(11) abilities(16) session(11) dialog(12)
[5] projectile <= features(16) avatar(2) abilities(4) items(2) enemy_projectile(16)
[5] encounter <= features(16) boss_encounter(4) audio(1) session(1) music(2)
[0] dev <=
[5] world <= features(23) avatar(1) encounter(1) dev(5) session(5)
[5] persistence <= character_sprites(7) audio(2) assets(7) schedule(3) host(2)
[1] audio <= music(1)
[5] time <= features(20) avatar(1) world(2) session(4) control(3)
[4] session <= avatar(1) audio(1) assets(7) gravity(1)
[10] control <= features(11) avatar(5) abilities(2) affordances(1) items(1) projectile(2) dev(1) world(3) body_mode(3) schedule(1)
[1] body_mode <= avatar(1)
[0] menu <=
[2] enemy_projectile <= features(11) abilities(12)
[3] assets <= character_sprites(4) audio(3) session(5)
[9] schedule <= features(14) avatar(1) abilities(1) items(1) encounter(2) dev(2) audio(7) session(2) gravity(4)
[0] gravity <=
[0] dialog <=
[1] music <= audio(25)
[2] shrine <= features(1) items(1)
[19] actor <= features(70) avatar(42) abilities(29) character_sprites(30) affordances(25) items(8) projectile(12) encounter(2) dev(2) world(4) persistence(4) audio(10) session(9) control(14) body_mode(14) enemy_projectile(7) gravity(12) dialog(6) shrine(5)
[11] combat <= features(165) avatar(14) abilities(1) character_sprites(9) projectile(1) dev(1) world(1) time(4) control(4) body_mode(1) enemy_projectile(4)
[10] rooms <= features(88) encounter(1) dev(2) world(2) audio(1) session(6) menu(2) assets(1) music(1) quest(1)
[7] ldtk_world <= features(2) encounter(8) persistence(1) audio(1) session(1) menu(1) assets(3)
Let me verify portal core cleanliness, dev_tools consumers, and combat's actor-AI coupling.

Bash Verify unused render deps
IN
cd /home/joncrall/code/ambition/crates/ambition_render && echo "=== any 'use ambition_input' or ambition_interaction in code? ===" && grep -rn "use ambition_input\|ambition_input::\|use ambition_interaction\|ambition_interaction::" src && echo "--- (empty above = unused) ---" && echo "=== vfx/sfx/time/cutscene/asset_manager/projectiles quick verify ===" && grep -rn "use ambition_vfx\|use ambition_sfx\|use ambition_cutscene\|use ambition_asset_manager\|use ambition_projectiles" src | head

OUT
=== any 'use ambition_input' or ambition_interaction in code? ===
src/cutscene/mod.rs:176:                        // `ambition_input::presets::ControlPreset::input_map`
--- (empty above = unused) ---
=== vfx/sfx/time/cutscene/asset_manager/projectiles quick verify ===
src/ui_fonts.rs:15:use ambition_asset_manager::AssetId;
src/ui_fonts.rs:17:use ambition_asset_manager::sandbox_assets::{ids, SandboxAssetCatalog};
src/fx.rs:15:use ambition_sfx::{SfxMessage, SfxWriter};
src/fx.rs:26:pub use ambition_vfx::vfx::{
src/rendering/slash_visuals.rs:25:use ambition_vfx::vfx::{SlashKind, VfxMessage};
src/rendering/projectile_visuals.rs:24:use ambition_projectiles::{
src/cutscene/mod.rs:13:use ambition_cutscene::{ActiveCutscene, CutsceneAdvanceRequest};
ldtk_map is largely a generic adapter. Let me pinpoint the sanic/alice/goblin leaks in ldtk and check whether items' default catalog is hardcoded content or an override seam.

Bash Read touch_input lib and dep usage map
IN
echo "=== touch_input lib.rs ===" && cat /home/joncrall/code/ambition/crates/ambition_touch_input/src/lib.rs && echo "=== touch_input: dep usage by file ===" && cd /home/joncrall/code/ambition/crates/ambition_touch_input && for c in actors render cutscene ui_nav persistence input engine_core platformer_primitives; do echo "--- ambition_$c ---"; grep -rln "ambition_$c" src; done

OUT
=== touch_input lib.rs ===
//! Mobile / touch presentation-input adapter for the Android demo path.
//!
//! Goal: a sideloadable Pixel-class APK where the sandbox is playable
//! with on-screen joysticks + controller-like touch buttons. This crate owns
//! both the rendered touch HUD and the fold from touch joystick / virtual button
//! state into the same `ControlFrame` resource consumed by the simulator. The
//! Leafwing keyboard/gamepad pipeline remains the canonical desktop input
//! surface.
//!
//! Extracted from `ambition_app::host::mobile_input` (app-thinness, ADR 0019):
//! reusable touch presentation/input infrastructure any platformer host would
//! want, so it lives beside the input/render seams rather than inside the app
//! binary. The direct `ambition_render` edge is intentional: the crate draws its
//! own overlay quads and text. The module has no app-only coupling — it
//! reads/writes only the `ambition_input` / `ambition_actors` /
//! `ambition_render` / `ambition_ui_nav` / `ambition_cutscene` library seams.
//!
//! Two layers:
//!
//! 1. **Pure helper (always built)** — [`fold_touch_into_control_frame`]
//!    takes a [`TouchInputState`] plus a deadzone and returns a
//!    `ControlFrame`. Pure data, unit-tested, no Bevy /
//!    `virtual_joystick` dep. RL agents, tests, and the Bevy systems
//!    all share this. See [`mod@state`].
//! 2. **Bevy plugin (gated behind `mobile_touch`)** — wires
//!    `virtual_joystick` Move + Aim sticks plus a small button UI to
//!    the helper, then writes `ControlFrame`. Lives in
//!    [`mod@bevy_plugin`].
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`state`] — pure types ([`TouchInputState`], [`TouchButton`],
//!   [`apply_deadzone`], [`fold_touch_into_control_frame`]); always
//!   built.
//! - [`exclusion`] — ECS marker + pure hit-test helpers for touch UI
//!   regions that should not become menu drag-scroll gestures;
//!   `mobile_touch`-gated.
//! - [`layout`] — touch HUD positions + visible-circle hit testing;
//!   `mobile_touch`-gated.
//! - [`menu_bridge`] — touch/mouse/joystick → `ControlFrame` /
//!   `MenuControlFrame` merge; `mobile_touch`-gated.
//! - [`bevy_plugin`] — system registration, spawning, visuals,
//!   resource/component definitions; `mobile_touch`-gated.
//!
//! Tests live in `tests.rs`.

// The pure touch STATE + fold. Its consumers (`bevy_plugin`, `menu_bridge`) are
// `mobile_touch`-gated, but the module compiles unconditionally so its unit tests
// run in every build. Without the feature, most of it is legitimately unreachable.
#[cfg_attr(not(feature = "mobile_touch"), allow(dead_code))]
mod state;

#[cfg(feature = "mobile_touch")]
pub mod exclusion;

#[cfg(feature = "mobile_touch")]
pub mod layout;

#[cfg(feature = "mobile_touch")]
pub mod menu_bridge;

#[cfg(test)]
mod tests;

// `TouchButton` is referenced by `bevy_plugin::super::TouchButton`; keep
// it re-exported so the plugin can construct buttons without a deeper
// import path. `apply_deadzone`/`fold_touch_into_control_frame`/
// `TouchInputState` are exercised only by the tests submodule, which
// reaches them via `super::state::*` and does not need a re-export.
pub use state::TouchButton;

/// Bevy plugin wiring `virtual_joystick` to the `ControlFrame` seam.
/// Gated behind the `mobile_touch` feature so desktop / gamepad /
/// headless / RL builds don't pull in `virtual_joystick` and don't
/// register the touch systems.
#[cfg(feature = "mobile_touch")]
pub mod bevy_plugin;

/// The touch-controls Bevy plugin — the single entry point the host adds. Re-exported
/// at the crate root so the host wires `ambition_touch_input::TouchControlsPlugin`
/// without reaching into the submodule.
#[cfg(feature = "mobile_touch")]
pub use bevy_plugin::TouchControlsPlugin;
=== touch_input: dep usage by file ===
--- ambition_actors ---
src/lib.rs
src/bevy_plugin.rs
--- ambition_render ---
src/lib.rs
src/bevy_plugin.rs
--- ambition_cutscene ---
src/lib.rs
src/menu_bridge.rs
--- ambition_ui_nav ---
src/lib.rs
src/bevy_plugin.rs
--- ambition_persistence ---
src/menu_bridge.rs
src/tests.rs
src/bevy_plugin.rs
--- ambition_input ---
src/lib.rs
src/state.rs
src/menu_bridge.rs
src/tests.rs
src/bevy_plugin.rs
--- ambition_engine_core ---
src/bevy_plugin.rs
--- ambition_platformer_primitives ---
src/tests.rs
src/menu_bridge.rs
src/bevy_plugin.rs
Bash Locate named content in ldtk_map
IN
cd /home/joncrall/code/ambition/crates && echo "=== ldtk sanic/alice/goblin sites ===" && grep -rniE "sanic|alice|goblin" ambition_ldtk_map/src | head -20

OUT
=== ldtk sanic/alice/goblin sites ===
ambition_ldtk_map/src/fields/tests.rs:102:        parse_pickup_kind("flag:seen_alice"),
ambition_ldtk_map/src/fields/tests.rs:104:            flag: "seen_alice".into()
ambition_ldtk_map/src/fields/tests.rs:133:        parse_enemy_brain("Goblin"),
ambition_ldtk_map/src/fields/tests.rs:134:        CharacterBrain::Custom(s) if s == "Goblin"
ambition_ldtk_map/src/conversion/mod.rs:723:    // GeoId determinism contract, the W2 sanic IR proof, and the F7 fixes
ambition_ldtk_map/src/conversion/mod.rs:858:    /// THE W2 IR PROOF, restored: the sanic area (richest IR surface — the
ambition_ldtk_map/src/conversion/mod.rs:862:    fn the_sanic_area_round_trips_as_a_ron_room() {
ambition_ldtk_map/src/conversion/mod.rs:865:        let sanic = room_set
ambition_ldtk_map/src/conversion/mod.rs:868:            .find(|room| room.id == "sanic_sandbox")
ambition_ldtk_map/src/conversion/mod.rs:869:            .expect("the sanic area exists in the sandbox world");
ambition_ldtk_map/src/conversion/mod.rs:871:            !sanic.world.chains.is_empty(),
ambition_ldtk_map/src/conversion/mod.rs:872:            "fixture: the sanic area exercises the chains channel"
ambition_ldtk_map/src/conversion/mod.rs:875:            spec: sanic.clone(),
ambition_ldtk_map/src/conversion/mod.rs:887:        assert_eq!(twin_set.active_spec().id, "sanic_sandbox");
Bash items default catalog content
IN
cd /home/joncrall/code/ambition/crates && echo "=== items: default catalog def (hardcoded content?) ===" && sed -n '120,175p' ambition_items/src/lib.rs && echo "=== grep default_catalog / fn default ===" && grep -n "fn default_catalog\|impl Default for ItemCatalog\|fn builtin\|24 items\|ItemDef {" ambition_items/src/lib.rs | head

OUT
=== items: default catalog def (hardcoded content?) ===

/// Content-installed item CATALOG override (C1), mirroring
/// the boss sheet/profile registry pattern in the actor sim.
/// A content game authors its item table in `items.ron` (a `Vec<ItemMeta>` in grid
/// order) and installs it via [`install_item_catalog`]; an installed row REPLACES
/// the built-in default for that grid slot. Absent rows (and no install) fall back
/// to the built-in [`ITEM_META`] — the E58/C6 "empty default = built-in" pattern,
/// so no core edit is needed to re-author item flavor.
#[derive(Clone, Debug, Default)]
pub struct ItemCatalog {
    rows: Vec<ItemMeta>,
}

impl ItemCatalog {
    /// Parse an item-catalog RON document (`[ItemMeta]` in grid order) — the
    /// content layer's install entry point.
    pub fn from_ron(ron: &str) -> Self {
        let rows = ron::from_str(ron).unwrap_or_else(|err| {
            panic!("items.ron failed to deserialize as Vec<ItemMeta>: {err}")
        });
        Self { rows }
    }

    fn row(&self, index: usize) -> Option<&ItemMeta> {
        self.rows.get(index)
    }
}

/// Content-installed item-catalog override. Set once at plugin-build time;
/// ADDITIVE per grid slot (the engine ships its own 24-item default table).
static ITEM_CATALOG_OVERRIDE: std::sync::OnceLock<ItemCatalog> = std::sync::OnceLock::new();

/// Install the authored item catalog — `ambition_content` calls this at
/// plugin-build time alongside the other roster installs.
pub fn install_item_catalog(catalog: ItemCatalog) {
    let _ = ITEM_CATALOG_OVERRIDE.set(catalog);
}

/// Resolve an item's metadata: the content-authored override row for `index` if one
/// was installed, else the built-in default. Both live behind a process-global, so
/// the borrow is effectively `'static`.
fn item_meta(index: usize) -> &'static ItemMeta {
    ITEM_CATALOG_OVERRIDE
        .get()
        .and_then(|c| c.row(index))
        .unwrap_or_else(|| &ITEM_META[index])
}

/// One row per [`Item`], in discriminant order — the engine's built-in default
/// item table. See [`ItemMeta`].
static ITEM_META: std::sync::LazyLock<[ItemMeta; ITEM_COUNT]> = std::sync::LazyLock::new(|| {
    use ItemCategory::*;
    [
        ItemMeta {
            display_name: "Portal Gun".into(),
            description: "Fire a linked blue/orange portal pair. Carries momentum.".into(),
=== grep default_catalog / fn default ===
353:    /// All 24 items in grid order. The compile-time length check below pins the
Let me see exactly what touch_input pulls from actors and render, then start the UI cluster analysis.

Bash Check portal render leakage, dev_tools prod usage, combat targeting coupling
IN
cd /home/joncrall/code/ambition
echo "=== who depends on ambition_dev_tools ==="
grep -rln "ambition_dev_tools" crates/*/Cargo.toml game/*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||'
echo
echo "=== dev_tools referenced in non-dev/non-app prod scheduling? (search host/runtime) ==="
grep -rln "ambition_dev_tools\|DeveloperTools\|SandboxDevState" crates/ambition_host/src crates/ambition_runtime/src crates/ambition_game_shell/src 2>/dev/null
echo
echo "=== portal core: any Sprite/Mesh/Material/Image (render) leakage? ==="
grep -rinE "\bSprite\b|\bMesh\b|Material|Handle<Image>|ColorMaterial|render" crates/ambition_portal/src --include=*.rs | grep -v "//" | grep -viE "test|rendered|renderer|// " | head -15
echo
echo "=== combat targeting.rs: actor coupling (imports) ==="
grep -n "^use\|ambition_" crates/ambition_combat/src/targeting.rs | head -20

OUT
=== who depends on ambition_dev_tools ===
crates/ambition
crates/ambition_actors
crates/ambition_dev_tools
crates/ambition_runtime
crates/ambition_sim_view
crates/ambition_render

=== dev_tools referenced in non-dev/non-app prod scheduling? (search host/runtime) ===
crates/ambition_host/src/portal.rs
crates/ambition_runtime/src/progression_schedule.rs
crates/ambition_runtime/src/sim_core_resources.rs
crates/ambition_runtime/src/player_schedule.rs
crates/ambition_runtime/src/lib.rs

=== portal core: any Sprite/Mesh/Material/Image (render) leakage? ===
crates/ambition_portal/src/placement.rs:94:    let into_render = Vec2::new(-n_in.x, n_in.y);
crates/ambition_portal/src/placement.rs:95:    let out_render = Vec2::new(n_out.x, -n_out.y);
crates/ambition_portal/src/placement.rs:96:    let dot = into_render.dot(out_render);
crates/ambition_portal/src/placement.rs:97:    let cross = into_render.x * out_render.y - into_render.y * out_render.x;

=== combat targeting.rs: actor coupling (imports) ===
11:use ambition_engine_core as ae;
12:use bevy::prelude::*;
14:use super::components::{
17:use super::FeatureSimEntity;
18:use ambition_characters::actor::BodyHealth;
19:use ambition_engine_core::BodyKinematics;
20:use ambition_platformer_primitives::markers::PlayerEntity;
100:/// another module, so ownership travels with the types into `ambition_combat`.
119:/// [`ambition_characters::brain::Brain::Player`]) fights as [`ActorFaction::Player`]
127:    brain: Option<&ambition_characters::brain::Brain>,
129:    if brain.is_some_and(ambition_characters::brain::Brain::is_player) {
198:            Option<&ambition_characters::brain::Brain>,
209:            Option<&ambition_characters::brain::Brain>,
247:        let player_controlled = brain.is_some_and(ambition_characters::brain::Brain::is_player);
Bash touch_input actor/render/ui_nav usage detail
IN
cd /home/joncrall/code/ambition/crates/ambition_touch_input && echo "=== actors usage ===" && grep -n "ambition_actors" src/bevy_plugin.rs && echo "=== render usage ===" && grep -n "ambition_render" src/bevy_plugin.rs && echo "=== ui_nav usage ===" && grep -n "ambition_ui_nav" src/bevy_plugin.rs && echo "=== cutscene usage ===" && grep -n "ambition_cutscene" src/menu_bridge.rs

OUT
=== actors usage ===
158:                        .after(ambition_actors::schedule::populate_menu_control_frame_from_actions)
168:                        .before(ambition_actors::schedule::apply_menu_frame_to_cutscene_request)
176:                        .before(ambition_actors::schedule::MenuNavConsume),
190:                        .after(ambition_actors::schedule::populate_control_frame_from_actions)
210:                        .before(ambition_actors::control::populate_slot_controls)
216:                        .before(ambition_actors::schedule::MenuNavConsume),
233:                        .after(ambition_actors::affordances::AffordancesSystemSet::Compute),
235:                        .after(ambition_actors::affordances::AffordancesSystemSet::Compute),
237:                        .after(ambition_actors::affordances::AffordancesSystemSet::Compute),
390:    gravity: Option<Res<ambition_actors::physics::GravityField>>,
395:    let gdir = ambition_actors::physics::gravity_dir_or_default(gravity.as_deref());
785:/// [`ambition_actors::affordances::PlayerAffordances`] table. Held as
819:    affordances: Res<ambition_actors::affordances::PlayerAffordances>,
822:    use ambition_actors::affordances::{InteractVariant, VariantLabel};
935:    active: Res<ambition_actors::affordances::ActiveInputMethod>,
947:        let next = ambition_actors::affordances::glyph_for(sa, &preset, active.0);
969:        With<ambition_actors::actor::PrimaryPlayer>,
=== render usage ===
30:use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
146:                    .after(ambition_render::ui_fonts::load_ui_fonts),
=== ui_nav usage ===
31:use ambition_ui_nav::DragScrollState;
=== cutscene usage ===
52:    cutscene: Res<ambition_cutscene::ActiveCutscene>,
Bash UI cluster interdependencies
IN
cd /home/joncrall/code/ambition/crates && for c in ambition_menu ambition_settings_menu ambition_ui_nav ambition_inventory_ui ambition_game_shell ambition_dialog; do echo "========== $c =========="; grep -A40 '^\[dependencies\]' $c/Cargo.toml | grep -E 'ambition_|^\[' | grep -v '^\[dependencies\]' | sed '/^\[/q' | grep 'ambition_'; done && echo "========== game/ambition_menu_kaleidoscope ==========" && grep -A40 '^\[dependencies\]' /home/joncrall/code/ambition/game/ambition_menu_kaleidoscope/Cargo.toml | grep 'ambition_'

OUT
========== ambition_menu ==========
# The bevy_lunex cube renderer moved to the `ambition_menu_kaleidoscope`
========== ambition_settings_menu ==========
ambition_persistence = { path = "../ambition_persistence" }
ambition_input = { path = "../ambition_input" }
========== ambition_ui_nav ==========
ambition_input = { path = "../ambition_input" }
========== ambition_inventory_ui ==========
ambition_ui_nav = { path = "../ambition_ui_nav" }
========== ambition_game_shell ==========
ambition_menu = { path = "../ambition_menu", default-features = false, optional = true }
ambition_audio = { path = "../ambition_audio", default-features = false }
ambition_load = { path = "../ambition_load" }
ambition_platformer_primitives = { path = "../ambition_platformer_primitives" }
ambition_sfx = { path = "../ambition_sfx" }
# un-configures `bevy::input::gamepad` (same pairing as ambition_input).
# unify them in — `cargo test -p ambition_game_shell` alone did not build.
========== ambition_dialog ==========
ambition_engine_core = { path = "../ambition_engine_core" }
ambition_ui_nav = { path = "../ambition_ui_nav" }
ambition_input = { path = "../ambition_input" }
ambition_sfx = { path = "../ambition_sfx" }
ambition_persistence = { path = "../ambition_persistence" }
# ambition_actors's base bevy dep so feature unification stays headless.
# winit, which needs a display backend to compile at all (mirrors ambition_audio).
========== game/ambition_menu_kaleidoscope ==========
ambition_menu = { path = "../../crates/ambition_menu" }
Let me read the full dependency sections for the UI cluster crates properly.

Items has a content-override seam but still hardcodes the Item enum + count. ldtk_map's content hits are all in tests. Let me confirm the Item enum and the ldtk test fixture source.

Bash Item enum variants (hardcoded roster)
IN
cd /home/joncrall/code/ambition/crates && echo "=== Item enum variants ===" && grep -n "enum Item" -A 40 ambition_items/src/lib.rs | grep -E "^\s+[A-Z][a-zA-Z]+," | head -30

OUT
=== Item enum variants ===
Bash Check runtime dev_tools usage and interaction contents
IN
cd /home/joncrall/code/ambition
echo "=== what runtime imports from dev_tools ==="
grep -rn "dev_tools\|DeveloperTools\|SandboxDevState\|Editable\|sync_live_player_dev" crates/ambition_runtime/src/progression_schedule.rs crates/ambition_runtime/src/player_schedule.rs crates/ambition_runtime/src/sim_core_resources.rs crates/ambition_runtime/src/lib.rs 2>/dev/null | head -20
echo
echo "=== interaction PickupKind / what interaction owns (enum decls) ==="
grep -n "pub enum\|pub struct\|pub fn" crates/ambition_interaction/src/lib.rs | head -40

OUT
=== what runtime imports from dev_tools ===
crates/ambition_runtime/src/player_schedule.rs:14://!   `.after(sync_live_player_dev_edits_system).before(input_timer_system)`
crates/ambition_runtime/src/player_schedule.rs:95:                ambition_dev_tools::sync_live_player_dev_edits_system,
crates/ambition_runtime/src/player_schedule.rs:103:        // Ordered after part A's tail (`sync_live_player_dev_edits_system`).
crates/ambition_runtime/src/player_schedule.rs:158:                .after(ambition_dev_tools::sync_live_player_dev_edits_system),
crates/ambition_runtime/src/sim_core_resources.rs:65:            .init_resource::<ambition_dev_tools::profiling::StartupProfiler>()
crates/ambition_runtime/src/sim_core_resources.rs:80:            .init_resource::<ambition_dev_tools::SandboxDevState>()
crates/ambition_runtime/src/sim_core_resources.rs:140:            .init_resource::<ambition_dev_tools::dev_tools::DeveloperTools>()
crates/ambition_runtime/src/sim_core_resources.rs:141:            .init_resource::<ambition_dev_tools::dev_tools::EditablePlayerStats>()
crates/ambition_runtime/src/sim_core_resources.rs:142:            .init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>()
crates/ambition_runtime/src/sim_core_resources.rs:143:            .init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
crates/ambition_runtime/src/progression_schedule.rs:81:                ambition_dev_tools::dev_tools::sync_player_stats_with_inspector,
crates/ambition_runtime/src/lib.rs:92:    pub use ambition_dev_tools::SandboxDevState;
crates/ambition_runtime/src/lib.rs:105:    pub use ambition_dev_tools::dev_tools::{EditableAbilitySet, EditableMovementTuning};

=== interaction PickupKind / what interaction owns (enum decls) ===
13:pub struct Interactable {
23:    pub fn new(
42:pub enum InteractionKind {
78:pub struct Pickup {
86:    pub fn new(id: impl Into<String>, kind: PickupKind) -> Self {
98:pub enum PickupKind {
108:pub struct Chest {
116:    pub fn new(id: impl Into<String>, reward: Option<PickupKind>) -> Self {
127:pub enum ChestState {
139:pub enum BreakableTrigger {
152:    pub fn allows_hit(self) -> bool {
156:    pub fn allows_stand(self) -> bool {
167:pub enum BreakableCollision {
180:    pub fn blocks_movement(self) -> bool {
185:    pub fn is_solid(self) -> bool {
192:pub struct Breakable {
208:    pub fn new(id: impl Into<String>, max_hp: i32) -> Self {
221:    pub fn apply_damage(&mut self, amount: i32) -> bool {
233:pub enum BreakableState {
Bash Full UI cluster dep sections and descriptions
IN
cd /home/joncrall/code/ambition/crates && for c in ambition_menu ambition_settings_menu ambition_ui_nav ambition_inventory_ui ambition_game_shell ambition_dialog; do echo "========== $c =========="; awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&/ambition_/{print}' $c/Cargo.toml; echo "-- description --"; grep -m1 '^description' $c/Cargo.toml; done && echo "========== menu_kaleidoscope ==========" && awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&/ambition_/{print}' /home/joncrall/code/ambition/game/ambition_menu_kaleidoscope/Cargo.toml && grep -m1 '^description' /home/joncrall/code/ambition/game/ambition_menu_kaleidoscope/Cargo.toml

OUT
========== ambition_menu ==========
# The bevy_lunex cube renderer moved to the `ambition_menu_kaleidoscope`
-- description --
description = "Reusable Bevy inventory/menu UI data model and interaction primitives."
========== ambition_settings_menu ==========
ambition_persistence = { path = "../ambition_persistence" }
ambition_input = { path = "../ambition_input" }
-- description --
description = "The renderer-agnostic settings + system menu IR (E1e): SettingsMenuModel / SettingsOption / apply_settings_option built from ambition_persistence::settings::UserSettings, and the System-menu layer (Radio / Video / Audio / Controls / Gameplay / Language / Reset / Quit / Developer) on top of it. Pure logic — no bevy, no renderer — so both the bevy-UI grid and the lunex cube render the same model. Depends only on the persistence settings vocabulary. Carved out of ambition_actors (dissolves the settings-IR god-dep)."
========== ambition_ui_nav ==========
ambition_input = { path = "../ambition_input" }
-- description --
description = "Reusable, content-free UI/menu navigation primitives: windowed list math (visible window + discrete scroll-to-row), pointer row-activation resolution, and drag-scroll accumulation. Keeps the keyboard/gamepad/touch menu paths from drifting apart. Pure over the input foundation; extracted from ambition_actors."
========== ambition_inventory_ui ==========
ambition_ui_nav = { path = "../ambition_ui_nav" }
-- description --
description = "Reusable inventory menu-navigation state for Ambition-style platformers."
========== ambition_game_shell ==========
ambition_menu = { path = "../ambition_menu", default-features = false, optional = true }
ambition_audio = { path = "../ambition_audio", default-features = false }
ambition_load = { path = "../ambition_load" }
ambition_platformer_primitives = { path = "../ambition_platformer_primitives" }
ambition_sfx = { path = "../ambition_sfx" }
# un-configures `bevy::input::gamepad` (same pairing as ambition_input).
# unify them in — `cargo test -p ambition_game_shell` alone did not build.
-- description --
description = "Renderer-independent routing and scoped lifecycle for top-level game experiences, plus neutral sequences and a minimal launcher."
========== ambition_dialog ==========
ambition_engine_core = { path = "../ambition_engine_core" }
ambition_ui_nav = { path = "../ambition_ui_nav" }
ambition_input = { path = "../ambition_input" }
ambition_sfx = { path = "../ambition_sfx" }
ambition_persistence = { path = "../ambition_persistence" }
# ambition_actors's base bevy dep so feature unification stays headless.
# winit, which needs a display backend to compile at all (mirrors ambition_audio).
-- description --
description = "Reusable dialogue runtime: the poll-based DialogState view model (typewriter reveal + option selection), the typewriter-SFX selection rules, the input/reveal Bevy systems, and the bevy_yarnspinner↔DialogState bridge plus the generic Yarn binding-installer seam. Content-free — a host's game-specific Yarn commands/functions register through the installer seam; the host maps DialogState.active onto its own session mode. Extracted from ambition_actors (E1c)."
========== menu_kaleidoscope ==========
ambition_menu = { path = "../../crates/ambition_menu" }
description = "The bevy_lunex 3D OoT-style cube renderer for the ambition_menu page model — the FIRST engine extension crate (E1e). Optional for any game: a host installs KaleidoscopeMenuPlugin to render the same backend-agnostic MenuPageModel as a rotating cube instead of the flat bevy_ui grid. Engine-only deps (ambition_menu + bevy + bevy_lunex); names no game content."
Bash UI cluster module LOC maps
IN
cd /home/joncrall/code/ambition/crates && for c in ambition_menu ambition_settings_menu ambition_ui_nav ambition_inventory_ui ambition_dialog; do echo "========== $c src (LOC) =========="; find $c/src -name '*.rs' | xargs wc -l | sort -n | tail -12; done

OUT
========== ambition_menu src (LOC) ==========
    8 ambition_menu/src/render/mod.rs
   87 ambition_menu/src/map.rs
   93 ambition_menu/src/backend.rs
  246 ambition_menu/src/render/bevy_ui/spawn.rs
  383 ambition_menu/src/render/bevy_ui/tests.rs
  551 ambition_menu/src/render/bevy_ui/mod.rs
 1074 ambition_menu/src/lib.rs
 2442 total
========== ambition_settings_menu src (LOC) ==========
   17 ambition_settings_menu/src/lib.rs
   77 ambition_settings_menu/src/settings/tests.rs
  253 ambition_settings_menu/src/settings/apply.rs
  315 ambition_settings_menu/src/settings/mod.rs
  361 ambition_settings_menu/src/system/tests.rs
  555 ambition_settings_menu/src/settings/build.rs
  665 ambition_settings_menu/src/system/mod.rs
 2243 total
========== ambition_ui_nav src (LOC) ==========
   17 ambition_ui_nav/src/lib.rs
   85 ambition_ui_nav/src/drag.rs
  224 ambition_ui_nav/src/pointer.rs
  402 ambition_ui_nav/src/list.rs
  728 total
========== ambition_inventory_ui src (LOC) ==========
  10 ambition_inventory_ui/src/lib.rs
 139 ambition_inventory_ui/src/model.rs
 149 total
========== ambition_dialog src (LOC) ==========
   35 ambition_dialog/src/content.rs
   59 ambition_dialog/src/lib.rs
  121 ambition_dialog/src/bindings.rs
  187 ambition_dialog/src/context.rs
  191 ambition_dialog/src/tests.rs
  280 ambition_dialog/src/systems.rs
  295 ambition_dialog/src/speech_sfx.rs
  345 ambition_dialog/src/bridge.rs
  511 ambition_dialog/src/runtime.rs
 2024 total
Bash ldtk test fixture provenance
IN
cd /home/joncrall/code/ambition/crates && echo "=== ldtk sanic test fixture source ===" && sed -n '855,890p' ambition_ldtk_map/src/conversion/mod.rs && echo "=== where does the test load the project from? ===" && grep -rn "include_str\|read_to_string\|\.ldtk\|fixture\|assets/" ambition_ldtk_map/src/conversion/mod.rs | head

OUT
=== ldtk sanic test fixture source ===
        );
    }

    /// THE W2 IR PROOF, restored: the sanic area (richest IR surface — the
    /// chains channel) round-trips serialize∘parse as a string fixed point
    /// and re-enters a RoomSet with no LDtk in the second path.
    #[test]
    fn the_sanic_area_round_trips_as_a_ron_room() {
        let project = LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
        let room_set = project.to_room_set().expect("sandbox composes");
        let sanic = room_set
            .rooms
            .iter()
            .find(|room| room.id == "sanic_sandbox")
            .expect("the sanic area exists in the sandbox world");
        assert!(
            !sanic.world.chains.is_empty(),
            "fixture: the sanic area exercises the chains channel"
        );
        let doc = ambition_world::ron_room::RonRoomDoc {
            spec: sanic.clone(),
            links: Vec::new(),
        };
        let baked = ambition_world::ron_room::room_doc_to_ron(&doc).expect("bakes");
        let reloaded = ambition_world::ron_room::room_doc_from_ron(&baked).expect("parses");
        let rebaked = ambition_world::ron_room::room_doc_to_ron(&reloaded).expect("re-bakes");
        assert_eq!(baked, rebaked, "serialize∘parse is a fixed point");
        let twin_set = ambition_world::rooms::RoomSet::from_parts(
            reloaded.spec.id.clone(),
            vec![reloaded.spec],
            reloaded.links,
        );
        assert_eq!(twin_set.active_spec().id, "sanic_sandbox");
    }

    #[test]
=== where does the test load the project from? ===
ambition_ldtk_map/src/conversion/mod.rs:59:        // project without that area (synthetic fixtures, partial checkouts)
ambition_ldtk_map/src/conversion/mod.rs:872:            "fixture: the sanic area exercises the chains channel"
Now let me examine portal_presentation contents, game_shell structure, and hunt for Ambition-specific hardcoded content across the presentation/UI stack.

Bash Find item enum + ldtk dev loader source path
IN
cd /home/joncrall/code/ambition/crates && echo "=== find the item roster enum ===" && grep -nE "pub enum [A-Z]" ambition_items/src/lib.rs && echo "--- the enum with 24 variants (context around ITEM_COUNT usage) ---" && grep -n "repr\|discriminant\|Item::" ambition_items/src/lib.rs | head -5 && echo "=== load_default_for_dev source ===" && grep -rn "fn load_default_for_dev" -A 15 ambition_ldtk_map/src/*.rs ambition_ldtk_map/src/**/*.rs 2>/dev/null | head -25

OUT
=== find the item roster enum ===
38:pub enum ItemCategory {
67:pub enum Item {
--- the enum with 24 variants (context around ITEM_COUNT usage) ---
63:/// slot index = `row * 6 + col`). The discriminant order **is** the grid order;
66:#[repr(usize)]
103:/// match the [`Item`] discriminants (pinned by `item_meta_table_is_index_aligned`).
168:/// One row per [`Item`], in discriminant order — the engine's built-in default
356:        Item::PortalGun,
=== load_default_for_dev source ===
ambition_ldtk_map/src/loading.rs:127:    pub fn load_default_for_dev() -> Result<Self, String> {
ambition_ldtk_map/src/loading.rs-128-        let config = SandboxAssetConfig {
ambition_ldtk_map/src/loading.rs-129-            sprite_folder: "sprites".to_string(),
ambition_ldtk_map/src/loading.rs-130-            asset_profile: AssetProfile::DesktopDevLoose,
ambition_ldtk_map/src/loading.rs-131-        };
ambition_ldtk_map/src/loading.rs-132-        let inputs = SandboxCatalogInputs {
ambition_ldtk_map/src/loading.rs-133-            worlds: world_manifest()
ambition_ldtk_map/src/loading.rs-134-                .worlds
ambition_ldtk_map/src/loading.rs-135-                .iter()
ambition_ldtk_map/src/loading.rs-136-                .map(|source| WorldCatalogRow {
ambition_ldtk_map/src/loading.rs-137-                    id: source.id.clone(),
ambition_ldtk_map/src/loading.rs-138-                    asset_path: source.asset_path.clone(),
ambition_ldtk_map/src/loading.rs-139-                    required: source.required,
ambition_ldtk_map/src/loading.rs-140-                    loose_path: source.loose_path.clone(),
ambition_ldtk_map/src/loading.rs-141-                    embedded_bevy_path: source.embedded_bevy_path,
ambition_ldtk_map/src/loading.rs-142-                })
Bash Compute total and test LOC per target crate
IN
cd /home/joncrall/code/ambition
for c in ambition_combat ambition_projectiles ambition_portal ambition_portal_presentation ambition_encounter ambition_interaction ambition_cutscene ambition_dialog ambition_dev_tools ambition_persistence ambition_load ambition_load_presentation; do
  tot=$(find crates/$c/src -name '*.rs' | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')
  testtot=$(find crates/$c/src -name '*.rs' \( -path '*test*' -o -name 'tests.rs' \) | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}')
  printf "%-32s total=%6s  tests=%6s\n" "$c" "$tot" "${testtot:-0}"
done

OUT
ambition_combat                  total=  9960  tests=  3237
ambition_projectiles             total=  2244  tests=   594
ambition_portal                  total=  5246  tests=  1524
ambition_portal_presentation     total=  6505  tests=  1573
ambition_encounter               total=  1698  tests=     0
ambition_interaction             total=   298  tests=     0
ambition_cutscene                total=   399  tests=     0
ambition_dialog                  total=  2024  tests=   191
ambition_dev_tools               total=  1665  tests=   109
ambition_persistence             total=  3688  tests=   330
ambition_load                    total=  1158  tests=   241
ambition_load_presentation       total=  1640  tests=   495
Bash portal_presentation lib and deps
IN
echo "=== portal_presentation lib.rs ===" && cat /home/joncrall/code/ambition/crates/ambition_portal_presentation/src/lib.rs && echo "=== portal_presentation deps ===" && awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&/ambition_/{print}' /home/joncrall/code/ambition/crates/ambition_portal_presentation/Cargo.toml

OUT
=== portal_presentation lib.rs ===
//! Default renderer for the headless [`ambition_portal`] mechanic.
//!
//! Provides placed-portal visuals, mid-transit body pieces, disorientation
//! indicators, through-portal view windows, and a sequestered compatibility
//! module for Ambition's portal-gun sprites. Hosts sync the
//! crate-owned seams ([`PortalWorldFrame`], [`PortalSceneBody`],
//! [`PortalGunArt`], [`PortalAimHint`]) and may replace any visual by disabling
//! that [`PortalPresentationPlugin`] flag and registering an alternative system.
//!
//! Depends only on `bevy`, `ambition_engine_core`,
//! `ambition_platformer_primitives`, and `ambition_portal`; it never names a host
//! crate.

use bevy::prelude::*;

use ambition_engine_core as ae;

mod camera_continuity;
mod clip_material;
mod effects;
mod gun_visuals;
mod plugin;
#[cfg(feature = "effect_view_cones")]
mod view_cones;
mod visuals;

pub use camera_continuity::{
    camera_roll_for_portal_transit, PortalCameraContinuityCamera, PortalCameraContinuityConfig,
    PortalCameraContinuityFocus, PortalCameraContinuityHostView, PortalCameraContinuitySelection,
    PortalCameraContinuityState, PortalCameraTransitMode,
};
pub use clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial,
    SpriteFrameBasis, CLIP_PLANE_OFF,
};
pub use effects::{PortalEffectSelection, PortalVisualEffect};
pub use gun_visuals::{sync_portal_mode_indicator, PortalModeIndicator};
pub use plugin::{PortalPresentationPlugin, PortalPresentationSet};
#[cfg(feature = "effect_view_cones")]
pub use view_cones::{
    debug_portal_view_zones, flush_portal_view_cone_debug_dump,
    handle_portal_view_cone_dump_hotkey, selected_portal_view_cone_debug_rows,
    sync_portal_view_cones, PortalApertureLosQuality, PortalCaptureCameraMode,
    PortalCaptureQualityBudget, PortalConeMesh, PortalDebugOverlay, PortalViewConeConfig,
    PortalViewConeDebugDumpRequest, PortalViewConeDebugRow, PortalViewConeMode,
    PortalViewConeSourceClipPolicy, PortalViewConeVisibilityMode, PortalViewRig, PortalViewer,
    PORTAL_WINDOW_RENDER_LAYER,
};
pub use visuals::{
    sync_portal_body_pieces, sync_portal_disorientation_indicator, sync_portal_visuals,
    PortalBodyPiece, PortalDisorientIndicator, PortalVisual,
};

/// Host-observation systems that publish data into this crate's presentation
/// seams run in this set. Renderers can order presentation after it without
/// depending on a concrete host crate.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalObservationSet;

/// Portal composite z band — the ONE place the seam's front-to-back order is
/// declared. A through-portal window shows a captured composite of the FAR
/// side, so it draws OVER the exit body copy, which then reads as the single
/// seamless source of the far side instead of a second sprite laid on top. It
/// stays BELOW actors (`WORLD_Z_PLAYER` = 20) so a near-side actor standing
/// in front of the aperture still correctly occludes the window, and BELOW
/// the rim/label overlay band ([`PORTAL_RIM_OVERLAY_Z`]) so a portal's
/// identifying frame always draws whole.
///
/// The transiting body itself draws as texture-clipped PIECES (see
/// [`sync_portal_body_pieces`]), on the WORLD layer — captures photograph
/// them, so through a DISJOINT pair's window you see your own copy emerging
/// (the wormhole view). The `here` slice draws in the actor band; the
/// emerged `through` slice — like the fallback unclipped exit copy — sits at
/// [`PORTAL_EXIT_COPY_Z`], just BELOW the window, so wherever a wormhole
/// pane covers the exit region its captured copy is the single image shown.
/// At a DOORWAY pair (opposed faces across a thin slab, see
/// `PortalViewConeConfig::doorway_pair_max_gap`) the pane is clipped to the
/// slab and the slices are clipped to be OUTSIDE it, so both slices draw
/// direct and crisp and the chart swap at the centroid snap trades like for
/// like — the doorway is a hole, not a wormhole, and never photographs a
/// region that is also directly on screen. The held gun decomposes the same
/// way (`gun_visuals`).
///
/// Within the band, a pair's two overlapping panes (thin-wall doorway) sort
/// by PAIRWISE FRONT-SIDE DOMINANCE with hysteresis, not by radial distance
/// (see `view_cones::mesh::pane_z`) — radial distance is near-tied everywhere
/// around a thin-wall seam and alternated the opaque panes frame-to-frame. A
/// fully unambiguous overlapping composite would still need per-window
/// stenciling (review report, Q9), but the dominant pane is now stable.
pub const PORTAL_WINDOW_Z: f32 = 9.5;
/// The exit-side body slice z (just below [`PORTAL_WINDOW_Z`]).
pub const PORTAL_EXIT_COPY_Z: f32 = 9.4;
/// Portal rim/core/label overlay z — ABOVE the window band: the identifying
/// frame is an OVERLAY on the seam, so a portal always draws whole instead of
/// a pane of takeover glass hiding the partner's rim / its own back half (the
/// c136/c137 "portal only half appearing"). The glass stays the single source
/// of the far-side IMAGE (exit copy and captures sit below it unchanged);
/// only the thin frame sits on top. Still below actors, so a body in front of
/// the surface occludes the frame naturally, and the emerging `through` slice
/// at [`PORTAL_EXIT_COPY_Z`] passes BEHIND the thin rim bar — the ring reads
/// as being in front of the body it emits, as it should.
pub const PORTAL_RIM_OVERLAY_Z: f32 = 10.0;

/// The host-world half of the render transform: the world's size, copied from
/// the host each frame. Engine coordinates are top-left-origin y-down; Bevy's
/// 2D camera is centered y-up; [`Self::to_render`] is the one adapter between
/// them (delegating to `ambition_engine_core::config::world_size_to_bevy` so
/// the math is defined exactly once).
///
/// Host seam: keep `size` synced (e.g. from Ambition's `RoomGeometry`). A zero
/// size just centers everything on the camera origin for a frame — wrong but
/// harmless until the first sync runs.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PortalWorldFrame {
    /// World size in engine units (the world's bottom-right corner).
    pub size: Vec2,
}

impl PortalWorldFrame {
    /// Engine world position → Bevy render translation at layer `z`.
    pub fn to_render(&self, p: Vec2, z: f32) -> Vec3 {
        ae::config::world_size_to_bevy(self.size, p, z)
    }
}

/// Host seam: marks the visual entity whose sprite the mid-transit body-piece
/// decomposition draws (in Ambition, the player's sprite entity). The entity
/// must also carry the runtime `BodyKinematics` plus `Sprite` + `Visibility`;
/// `PortalTransit` / `ActorRoll` are read when present.
#[derive(Component)]
pub struct PortalSceneBody;

/// Host seam: the loaded portal-gun art (blue / orange mode sprites). The
/// crate defines the resource; the HOST loads it — asset paths are content.
/// Absent resource → the held gun doesn't draw and the ground pickup falls
/// back to a marker quad.
#[derive(Resource)]
pub struct PortalGunArt {
    pub blue: Handle<Image>,
    pub orange: Handle<Image>,
}

/// Host seam: content-agnostic aim hint for the held-gun presentation — the
/// resolved world-space direction the barrel should point (the same aim the
/// host's input adapter resolves for `FirePortalGun`). The host writes it each
/// frame; [`sync_portal_mode_indicator`] reads it, so portal presentation
/// never imports a host input type. Zero / unset aim falls back to facing.
///
/// Initialised by [`PortalPresentationPlugin`] (it is render-only state, so it
/// lives here rather than in the headless mechanic's plugin).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PortalAimHint {
    /// Resolved aim direction (need not be normalized; zero falls back to facing).
    pub aim: Vec2,
}
=== portal_presentation deps ===
# `ambition_portal` mechanic refuses to own): the 2D render pipeline for
ambition_engine_core = { path = "../ambition_engine_core" }
# roll reads. Same stack position as `ambition_portal` (ADR 0019).
ambition_platformer_primitives = { path = "../ambition_platformer_primitives" }
# host crate (ambition_actors / ambition_content).
ambition_portal = { path = "../ambition_portal" }
Bash game_shell structure
IN
echo "=== game_shell src tree LOC ===" && find /home/joncrall/code/ambition/crates/ambition_game_shell/src -name '*.rs' | xargs wc -l | sort -n | tail -20 && echo "=== game_shell lib.rs head ===" && head -40 /home/joncrall/code/ambition/crates/ambition_game_shell/src/lib.rs

OUT
=== game_shell src tree LOC ===
    52 /home/joncrall/code/ambition/crates/ambition_game_shell/src/id.rs
    76 /home/joncrall/code/ambition/crates/ambition_game_shell/src/launcher.rs
    82 /home/joncrall/code/ambition/crates/ambition_game_shell/src/lib.rs
   102 /home/joncrall/code/ambition/crates/ambition_game_shell/src/input.rs
   107 /home/joncrall/code/ambition/crates/ambition_game_shell/src/frontend.rs
   292 /home/joncrall/code/ambition/crates/ambition_game_shell/src/preparation.rs
   315 /home/joncrall/code/ambition/crates/ambition_game_shell/src/sequence.rs
   396 /home/joncrall/code/ambition/crates/ambition_game_shell/src/plugin.rs
   409 /home/joncrall/code/ambition/crates/ambition_game_shell/src/session/tests.rs
   426 /home/joncrall/code/ambition/crates/ambition_game_shell/src/pause_menu.rs
   462 /home/joncrall/code/ambition/crates/ambition_game_shell/src/experience.rs
   560 /home/joncrall/code/ambition/crates/ambition_game_shell/src/session.rs
   563 /home/joncrall/code/ambition/crates/ambition_game_shell/src/router.rs
   611 /home/joncrall/code/ambition/crates/ambition_game_shell/src/tests.rs
   646 /home/joncrall/code/ambition/crates/ambition_game_shell/src/basic_presentation.rs
  5099 total
=== game_shell lib.rs head ===
//! Top-level game-shell routing without game-specific route names or rendering.
//!
//! A host configures separate initial and home routes. Registered experiences
//! receive scoped activation identities and report semantic completion, failure,
//! navigation, or `QuitToHome`; they never hard-code the menu that launched
//! them. The neutral sequence and launcher modules are reusable shell
//! experiences, not a universal gameplay state machine.

mod experience;
mod frontend;
mod id;
mod input;
mod launcher;
mod plugin;
mod preparation;
mod router;
mod sequence;
mod session;

#[cfg(feature = "basic_presentation")]
mod basic_presentation;

#[cfg(feature = "basic_presentation")]
mod pause_menu;

pub use experience::{
    ExperienceAvailability, ExperienceRegistration, ShellExperienceAppExt, ShellExperienceRegistry,
};
pub use frontend::*;
pub use id::*;
pub use input::*;
pub use launcher::*;
pub use plugin::{AmbitionGameShellPlugin, ShellLauncherPlugin, ShellSequencePlugin};
pub use preparation::*;
pub use router::*;
pub use sequence::*;
pub use session::*;

#[cfg(feature = "basic_presentation")]
pub use basic_presentation::{BasicSequenceRoot, BasicShellPresentationPlugin, BasicShellUiRoot};
Read lib.rs (lines 63-102)
Bash Item enum variants + ldtk world_manifest
IN
cd /home/joncrall/code/ambition/crates && echo "=== world_manifest source (does ldtk hardcode game world paths?) ===" && grep -rn "fn world_manifest" -A 30 ambition_ldtk_map/src/manifest.rs | head -40

OUT
=== world_manifest source (does ldtk hardcode game world paths?) ===
ambition_ldtk_map/src/manifest.rs:91:pub fn world_manifest() -> &'static WorldManifest {
ambition_ldtk_map/src/manifest.rs-92-    #[cfg(test)]
ambition_ldtk_map/src/manifest.rs-93-    {
ambition_ldtk_map/src/manifest.rs-94-        // Test fixture = the game's REAL worlds, read cross-crate (the
ambition_ldtk_map/src/manifest.rs-95-        // explicit cross-crate fixture pattern) so this crate's conversion /
ambition_ldtk_map/src/manifest.rs-96-        // ron-room contract tests exercise real data without shipping any.
ambition_ldtk_map/src/manifest.rs-97-        // Restored by the fable final audit (F7): the W3 carve dropped it,
ambition_ldtk_map/src/manifest.rs-98-        // which is what orphaned the ruled contract tests.
ambition_ldtk_map/src/manifest.rs-99-        WORLD_MANIFEST.get_or_init(test_fixture_manifest)
ambition_ldtk_map/src/manifest.rs-100-    }
ambition_ldtk_map/src/manifest.rs-101-    #[cfg(not(test))]
ambition_ldtk_map/src/manifest.rs-102-    {
ambition_ldtk_map/src/manifest.rs-103-        WORLD_MANIFEST.get().unwrap_or_else(|| {
ambition_ldtk_map/src/manifest.rs-104-            panic!(
ambition_ldtk_map/src/manifest.rs-105-                "world manifest not installed — the game's content must call \
ambition_ldtk_map/src/manifest.rs-106-                 install_world_manifest() before any world load \
ambition_ldtk_map/src/manifest.rs-107-                 (AmbitionContentPlugin / the app's sim-entry choke points do)"
ambition_ldtk_map/src/manifest.rs-108-            )
ambition_ldtk_map/src/manifest.rs-109-        })
ambition_ldtk_map/src/manifest.rs-110-    }
ambition_ldtk_map/src/manifest.rs-111-}
ambition_ldtk_map/src/manifest.rs-112-
ambition_ldtk_map/src/manifest.rs-113-/// The cross-crate test fixture: the game's real worlds under
ambition_ldtk_map/src/manifest.rs-114-/// `game/ambition_content/assets/worlds`, entry room = the hub.
ambition_ldtk_map/src/manifest.rs-115-#[cfg(test)]
ambition_ldtk_map/src/manifest.rs-116-fn test_fixture_manifest() -> WorldManifest {
ambition_ldtk_map/src/manifest.rs-117-    let worlds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
ambition_ldtk_map/src/manifest.rs-118-        .join("../../game/ambition_content/assets/worlds");
ambition_ldtk_map/src/manifest.rs-119-    let source = |id: &str, file: &str, required: bool| WorldSource {
ambition_ldtk_map/src/manifest.rs-120-        id: AssetId::new(id),
ambition_ldtk_map/src/manifest.rs-121-        asset_path: format!("game://worlds/{file}"),
Now let me hunt for Ambition-game-specific hardcoded content across the presentation/UI stack.

Bash Hunt Ambition-specific content in presentation/UI
IN
echo "=== named characters in content ===" && ls game/ambition_content/ 2>/dev/null && find game/ambition_content -name '*.rs' | head && echo "=== grep for likely character/story names across presentation+UI crates ===" && grep -rniE "mary|pirate|sanic|pocket|kaleidoscope|\bboss[a-z_]*name|hero|protagonist" crates/ambition_render/src crates/ambition_portal_presentation/src crates/ambition_menu/src crates/ambition_dialog/src crates/ambition_game_shell/src crates/ambition_touch_input/src crates/ambition_settings_menu/src crates/ambition_inventory_ui/src crates/ambition_ui_nav/src 2>/dev/null | grep -vi "primary\|binary\|summary\|boundary" | head -40

OUT
=== named characters in content ===
Cargo.toml
MODULES.md
assets
src
tests
game/ambition_content/src/falling_sand.rs
game/ambition_content/src/provider.rs
game/ambition_content/src/character_catalog.rs
game/ambition_content/src/content_validation.rs
game/ambition_content/src/quest.rs
game/ambition_content/src/duel_arena.rs
game/ambition_content/src/worlds.rs
game/ambition_content/src/enemy_roster.rs
game/ambition_content/src/lib.rs
game/ambition_content/src/audio_registries.rs
=== grep for likely character/story names across presentation+UI crates ===
crates/ambition_render/src/platformer_presentation.rs:9://! exit 3 (`game/ambition_demo_sanic_app`) proved that assembles and steps the
crates/ambition_render/src/platformer_presentation.rs:31://! portal-window render, or its kaleidoscope cube. Those are the GAME's, and
crates/ambition_render/src/rendering/item_visuals.rs:137:        // Same `lasersword_with_guns` proportions the pirates hold (177×46).
crates/ambition_render/src/rendering/item_visuals.rs:199:/// own pickups' images (e.g. Mary-O's milk carton), keeping asset knowledge out of
crates/ambition_render/src/rendering/item_visuals.rs:330:    // direction it fires — just like the pirates' wielded gun-sword. Melee /
crates/ambition_render/src/rendering/health.rs:50:    let Some((health, boss_name)) = boss_render.iter().find_map(|(id, ident)| {
crates/ambition_render/src/rendering/health.rs:71:    let boss_name = boss_name.as_str();
crates/ambition_render/src/rendering/health.rs:115:                Name::new(format!("Boss Health Panel: {boss_name}")),
crates/ambition_render/src/rendering/health.rs:119:                    Text::new(boss_name.to_string()),
crates/ambition_render/src/rendering/health.rs:135:                        Name::new(format!("Boss Health Track: {boss_name}")),
crates/ambition_render/src/rendering/health.rs:145:                            Name::new(format!("Boss Health Fill: {boss_name}")),
crates/ambition_render/src/rendering/mod.rs:44:mod pirate_weapon;
crates/ambition_render/src/rendering/mod.rs:228:/// and finishes with the pirate rider composite. Carved out of
crates/ambition_render/src/rendering/mod.rs:302:                // Gun-sword visual on the rider — composite pirate-
crates/ambition_render/src/rendering/mod.rs:311:                pirate_weapon::sync_pirate_weapon_visuals,
crates/ambition_render/src/rendering/actors/boss.rs:59:        let boss_name = boss_ident.name.as_str();
crates/ambition_render/src/rendering/actors/boss.rs:61:        let _ = boss_name;
crates/ambition_render/src/rendering/world.rs:417:/// Sanic-specific drawing; any game that authors a chain gets a matching visual.
crates/ambition_render/src/rendering/world.rs:963:/// presentation plugin drives, not just Mary-O's bricks.
crates/ambition_render/src/rendering/pirate_weapon.rs:3://! Mounted riders and dismounted pirates both keep the component while
crates/ambition_render/src/rendering/pirate_weapon.rs:32:pub struct PirateWeaponVisual;
crates/ambition_render/src/rendering/pirate_weapon.rs:39:/// Texture handle for the pirate rider weapon overlay. Kept alive in
crates/ambition_render/src/rendering/pirate_weapon.rs:42:pub(crate) struct PirateWeaponVisualArt {
crates/ambition_render/src/rendering/pirate_weapon.rs:46:impl PirateWeaponVisualArt {
crates/ambition_render/src/rendering/pirate_weapon.rs:74:/// pirate's gun-sword reads bigger than a raider's automatically.
crates/ambition_render/src/rendering/pirate_weapon.rs:79:/// automatically for left-facing pirates.
crates/ambition_render/src/rendering/pirate_weapon.rs:85:/// PirateRaider (78-tall) and a PirateHeavy (110-tall) without
crates/ambition_render/src/rendering/pirate_weapon.rs:88:pub fn sync_pirate_weapon_visuals(
crates/ambition_render/src/rendering/pirate_weapon.rs:96:    existing: Query<Entity, With<PirateWeaponVisual>>,
crates/ambition_render/src/rendering/pirate_weapon.rs:97:    mut art: Local<Option<PirateWeaponVisualArt>>,
crates/ambition_render/src/rendering/pirate_weapon.rs:107:    let art = art.get_or_insert_with(|| PirateWeaponVisualArt::load(&asset_server));
crates/ambition_render/src/rendering/pirate_weapon.rs:135:            // the pirate's hand rather than disappearing behind their
crates/ambition_render/src/rendering/pirate_weapon.rs:164:                PirateWeaponVisual,
crates/ambition_render/src/rendering/pirate_weapon.rs:165:                Name::new("Pirate gun-sword"),
crates/ambition_render/src/rendering/actors/worn_binder_tests.rs:135:    let e = spawn_worn(&mut app, "sanic");
crates/ambition_render/src/rendering/actors/worn_binder_tests.rs:139:        "sanic",
crates/ambition_menu/src/backend.rs:26:pub const KALEIDOSCOPE_MENU_BACKEND_ENABLED: bool = cfg!(feature = "kaleidoscope_menu");
crates/ambition_menu/src/backend.rs:36:    LunexKaleidoscope,
crates/ambition_menu/src/backend.rs:41:        if KALEIDOSCOPE_MENU_BACKEND_ENABLED {
crates/ambition_menu/src/backend.rs:42:            Self::LunexKaleidoscope
ldtk_map is a clean adapter (content only in #[cfg(test)] cross-crate fixtures; production uses install_world_manifest()). Let me do a final named-content sweep across audio and entity_catalog, plus confirm the full audio music-track content.

Found significant game-specific content (pirate_weapon, Mary-O, Sanic). Let me look closer at the most content-specific render modules and check the menu/dialog crates for hardcoded content.

Bash audio + entity_catalog named-content sweep
IN
cd /home/joncrall/code/ambition/crates && echo "=== audio: named content (music tracks / boss themes / character themes)? ===" && grep -rhoE 'from_static\("[^"]+"\)|"[a-z_]+\.[a-z_.]+"|MusicTrack|boss_theme|const [A-Z_]+: (SfxId|MusicId)' ambition_audio/src | grep -iE "boss|theme|hub|sanic|goblin|mockingbird|sentinel|architect|alice|track" | sort -u | head -30 && echo "" && echo "=== entity_catalog: any named content? ===" && grep -rniE "goblin|sanic|mockingbird|sentinel|alice|\bbob\b|architect|portal_gun|fireball" ambition_entity_catalog/src | grep -v "test" | head

OUT
=== audio: named content (music tracks / boss themes / character themes)? ===
"boss.shatter"
"sanic.ring"
MusicTrack
boss_theme

=== entity_catalog: any named content? ===
ambition_entity_catalog/src/placements.rs:1://! The authored-placement schema vocabulary — architecture.md §4b.
ambition_entity_catalog/src/placements.rs:387:/// The CLOSED authored-placement schema (architecture.md §4b.3): everything an
ambition_entity_catalog/src/placements.rs:455:    /// (trash grunts: skitters, lurkers, raiders, goblins).
ambition_entity_catalog/src/lib.rs:25://! the goblin the player's slash is a re-binding, not a Rust change.
Let me quickly confirm the banter/npc content and music goblin cue.

Bash Check banter and music content
IN
cd /home/joncrall/code/ambition/crates/ambition_actors/src
echo "=== banter.rs ==="; sed -n '1,63p' features/banter.rs
echo "=== music/intent.rs goblin refs ==="; grep -n 'goblin\|Goblin' music/intent.rs | head

OUT
=== banter.rs ===
//! Combat-banter registry (generic half).
//!
//! The registry TYPE + pick/set mechanics live machinery-side so the
//! combat hit path (`crate::features::ecs::damage`) can read it; the
//! authored line sets (pirate barks, intro raiders, boss banter) are
//! content and populate it via plugin startup systems in the
//! `ambition_content` crate.

use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Resource, Default, Debug, Clone)]
pub struct CombatBanterRegistry {
    /// Lines an enemy yells when hit. Indexed by enemy display
    /// name. The line picked rotates with strike count to avoid repetition.
    pub on_hit: HashMap<String, Vec<&'static str>>,
    /// Lines an actor mutters periodically during a fight even when
    /// not being hit. Used by the boss idle-bark ticker so the giant
    /// has personality between strikes.
    pub idle: HashMap<String, Vec<&'static str>>,
}

impl CombatBanterRegistry {
    /// Pick a hit-bark line for the named enemy based on a rotation
    /// counter (typically derived from the enemy's hit count). Returns
    /// `None` if the enemy has no registered lines — the combat
    /// system silently skips the bubble in that case.
    pub fn pick_hit_bark(&self, enemy_name: &str, rotation: u32) -> Option<&'static str> {
        pick_line(&self.on_hit, enemy_name, rotation)
    }

    /// Bulk-register a set of hit-bark lines for one enemy name.
    /// Overwrites any existing entry for that name.
    pub fn set_hit_barks(&mut self, enemy_name: impl Into<String>, lines: Vec<&'static str>) {
        self.on_hit.insert(enemy_name.into(), lines);
    }

    /// Pick an idle-bark line by name + rotation counter. Same shape
    /// as `pick_hit_bark` so the caller can use a simple per-actor
    /// tick counter (e.g. number of idle barks fired so far).
    pub fn pick_idle_bark(&self, name: &str, rotation: u32) -> Option<&'static str> {
        pick_line(&self.idle, name, rotation)
    }

    /// Bulk-register idle barks for one actor. Overwrites any
    /// existing entry.
    pub fn set_idle_barks(&mut self, name: impl Into<String>, lines: Vec<&'static str>) {
        self.idle.insert(name.into(), lines);
    }
}

fn pick_line(
    table: &HashMap<String, Vec<&'static str>>,
    name: &str,
    rotation: u32,
) -> Option<&'static str> {
    let lines = table.get(name)?;
    if lines.is_empty() {
        return None;
    }
    Some(lines[(rotation as usize) % lines.len()])
}
=== music/intent.rs goblin refs ===
Bash Full named sfx constant roster
IN
cd /home/joncrall/code/ambition/crates && echo "=== full DIALOGUE_BLIP + boss/enemy sfx named constants (content roster in ambition_sfx) ===" && grep -hoE "const [A-Z_]+: SfxId = SfxId::from_static\(\"[^\"]+\"\)" ambition_sfx/src/ids.rs | grep -iE "boss|goblin|mockingbird|sentinel|blip|enemy|npc" | head -50

OUT
=== full DIALOGUE_BLIP + boss/enemy sfx named constants (content roster in ambition_sfx) ===
const DIALOGUE_BLIP_GENERIC: SfxId = SfxId::from_static("dialogue.blip.generic")
const DIALOGUE_BLIP_SHOUT_GENERIC: SfxId = SfxId::from_static("dialogue.blip.shout.generic")
const DIALOGUE_BLIP_ALICE: SfxId = SfxId::from_static("dialogue.blip.alice")
const DIALOGUE_BLIP_ARCHITECT: SfxId = SfxId::from_static("dialogue.blip.architect")
const DIALOGUE_BLIP_BOB: SfxId = SfxId::from_static("dialogue.blip.bob")
const DIALOGUE_BLIP_CREATOR: SfxId = SfxId::from_static("dialogue.blip.creator")
const DIALOGUE_BLIP_DARK_LORD: SfxId = SfxId::from_static("dialogue.blip.dark_lord")
const DIALOGUE_BLIP_GATE_JANITOR: SfxId = SfxId::from_static("dialogue.blip.gate_janitor")
const DIALOGUE_BLIP_HAND_SAINT: SfxId = SfxId::from_static("dialogue.blip.hand_saint")
const DIALOGUE_BLIP_KERNEL_GUIDE: SfxId = SfxId::from_static("dialogue.blip.kernel_guide")
const DIALOGUE_BLIP_MANIFEST_CLERK: SfxId = SfxId::from_static("dialogue.blip.manifest_clerk")
const DIALOGUE_BLIP_NEWS_BOARD: SfxId = SfxId::from_static("dialogue.blip.news_board")
const DIALOGUE_BLIP_NINJA: SfxId = SfxId::from_static("dialogue.blip.ninja")
const DIALOGUE_BLIP_OILER: SfxId = SfxId::from_static("dialogue.blip.oiler")
const DIALOGUE_BLIP_PIRATE: SfxId = SfxId::from_static("dialogue.blip.pirate")
const DIALOGUE_BLIP_PULSE_VOYAGER: SfxId = SfxId::from_static("dialogue.blip.pulse_voyager")
const DIALOGUE_BLIP_ROBOT: SfxId = SfxId::from_static("dialogue.blip.robot")
const DIALOGUE_BLIP_TECH_BRO: SfxId = SfxId::from_static("dialogue.blip.tech_bro")
const DIALOGUE_BLIP_VAULT_KEEPER: SfxId = SfxId::from_static("dialogue.blip.vault_keeper")
const DIALOGUE_BLIP_WEIRD_HERMIT: SfxId = SfxId::from_static("dialogue.blip.weird_hermit")
Bash Inspect content-specific render modules
IN
echo "=== pirate_weapon.rs head ===" && head -40 crates/ambition_render/src/rendering/pirate_weapon.rs && echo "=== content-suspicious render modules purpose (first doc line each) ===" && for f in bubble_shield morph_ball deep_dream shrine_visuals mark_beacon slash_visuals gate_portal_visuals gravity_visuals pirate_weapon; do echo "--- $f ---"; head -3 crates/ambition_render/src/rendering/$f.rs | grep '//'; done

OUT
=== pirate_weapon.rs head ===
//! Gun-sword (`lasersword_with_guns`) visual layered on top of any
//! actor entity carrying the item read-model facts.
//! Mounted riders and dismounted pirates both keep the component while
//! they still have the weapon, so this visual is item-driven rather
//! than mount-state-driven.
//!
//! Each frame we:
//! 1. Find every alive actor holding the `gun_sword` item.
//! 2. Compute the rider's hand world position from `rider.pos` +
//!    facing-aware hand offset (`HAND_OFFSET_NORM` scaled by the
//!    rider's body height).
//! 3. Compute the aim direction from the hand to the primary player
//!    body (`atan2(dy, dx)`).
//! 4. Spawn a sprite for the gun-sword's idle frame, positioned at
//!    the hand and rotated so the blade points along the aim
//!    direction.
//!
//! Despawn-and-respawn each tick — no per-entity lifecycle plumbing, the
//! visual set always reflects the live rider set.

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::WieldedGunSwordsView;

#[derive(Component)]
pub struct PirateWeaponVisual;

/// Filename of the wielded gun-sword spritesheet. Lives under
/// the actor asset sprite folder (installed by
/// `python3 -m ambition_sprite2d_renderer install lasersword_with_guns`).
const WEAPON_SHEET_PATH: &str = "sprites/lasersword_with_guns_spritesheet.png";

/// Texture handle for the pirate rider weapon overlay. Kept alive in
/// system-local state so the per-frame overlay rebuild does not also call
=== content-suspicious render modules purpose (first doc line each) ===
--- bubble_shield ---
// ---------------------------------------------------------------------------
// Bubble shield sprite (procedural)
// ---------------------------------------------------------------------------
--- morph_ball ---
// ---------------------------------------------------------------------------
// Morph ball sprite (procedural)
// ---------------------------------------------------------------------------
--- deep_dream ---
//! Per-sprite deep-dream shader experiment for the Puppy Slug enemy.
//!
//! The regular character sprite remains visible and authoritative: the normal
--- shrine_visuals ---
//! Shrine visuals: the obelisk
//! sprite sync + activation-pulse animation. Reads the sim shrine state
//! (HealShrine plus the lower ShrineActivationPulse resource) from the sim/read-model seam.
--- mark_beacon ---
//! The recall-mark beacon visual (was in abilities/traversal/mark_recall) — a
//! glowing beacon sprite at the player's dropped mark. Render-only; reads the
//! sim-side mark read-model.
--- slash_visuals ---
//! Player melee slash effect — the `robot_slash` spritesheet hooked up as a
//! one-shot VFX.
//!
--- gate_portal_visuals ---
//! Gate-portal presentation: sprite visibility / animation row / ring spin
//! driven by the sim's `GatePortalRegistry` phase (E4 slices 10+20 — these
//! systems used to live INSIDE the sim crate and matched render entities by
--- gravity_visuals ---
//! Gravity-zone / gravity-switch visuals (visible build only — registered by
//! the presentation rendering plugin). Extracted from `ambition_portal::presentation`
//! (Stage 6 follow-up): these visualize a *gravity mechanic*, not a portal, and
--- pirate_weapon ---
//! Gun-sword (`lasersword_with_guns`) visual layered on top of any
//! actor entity carrying the item read-model facts.
//! Mounted riders and dismounted pirates both keep the component while
Bash menu data model and content check
IN
echo "=== menu lib.rs head (data model?) ===" && head -50 crates/ambition_menu/src/lib.rs && echo "=== does menu name Ambition content? ===" && grep -rniE "ambition|mary|sanic|pirate|quit to|new game|continue" crates/ambition_menu/src | grep -vi "crate\|//" | head

OUT
=== menu lib.rs head (data model?) ===
//! Engine-side unified menu: the renderer-agnostic content model plus two
//! interchangeable presentations of it.
//!
//! This crate is split into host-owned DATA and renderer-owned PRESENTATION.
//! Hosts build generic [`MenuPageModel`] / [`ItemsOnlyPageSpec`] values from
//! their own resources, then translate the [`MenuActionActivated`] /
//! [`MenuClosedRequested`] messages this crate emits back into gameplay events;
//! it never names `OwnedItems`, health, or player components. This crate ships
//! the flat tabbed [`render::bevy_ui`] renderer; the bevy_lunex 3D OoT-style
//! cube renderer is the optional `ambition_menu_kaleidoscope` extension crate
//! (E1e) — both consume the same page model, which is what validates the seam.
//!
//! [`AmbitionInventoryUiPlugin`] installs only the renderer-agnostic
//! resources/messages, so a host can keep it even with no renderer enabled.

use bevy::prelude::{App, Component, Message, Plugin, Resource};

pub mod backend;
pub mod map;
pub mod render;

/// A normalized page-space rectangle.
///
/// Coordinates are percentages in the page's local 2D layout space. `(0, 0)` is
/// the top-left corner and `(100, 100)` is the bottom-right corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MenuRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn inset(self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            w: self.w - amount * 2.0,
            h: self.h - amount * 2.0,
        }
    }
}

/// Renderer-independent color token.
#[derive(Clone, Copy, Debug, PartialEq)]
=== does menu name Ambition content? ===
Let me get the audio named-content roster (music tracks / sfx constants in ambition_audio) to complete section 6.

Bash Named content in ambition_audio
IN
cd /home/joncrall/code/ambition/crates && echo "=== audio: named SfxId/track constants ===" && grep -rhoE 'from_static\("[^"]+"\)' ambition_audio/src | sort -u | head -40 && echo "=== audio: boss_theme / named music usages ===" && grep -rniE "boss_theme|sanic|hub|goblin|mockingbird|sentinel|\.ring" ambition_audio/src | grep -v "//" | head -15

OUT
=== audio: named SfxId/track constants ===
from_static("late.bank.cue")
from_static("late.bank.id")
from_static("shared")
=== audio: boss_theme / named music usages ===
ambition_audio/src/selection.rs:470:            "sanic",
ambition_audio/src/selection.rs:482:        selection.select_gameplay(1, "sanic", Some(music("fast")), None, BTreeSet::new());
ambition_audio/src/selection.rs:483:        selection.select_gameplay(2, "sanic", Some(music("fast")), None, BTreeSet::new());
ambition_audio/src/selection.rs:495:        selection.refresh_provider_sfx_ids("sanic", BTreeSet::from([late]));
ambition_audio/src/bank_asset.rs:429:            "sanic",
ambition_audio/src/catalog.rs:559:                    "sanic",
ambition_audio/src/catalog.rs:613:        a.register("sanic", BTreeMap::from([(id("sanic.ring"), 3)]))
ambition_audio/src/catalog.rs:617:        b.register("sanic", BTreeMap::from([(id("sanic.ring"), 3)]))
ambition_audio/src/catalog.rs:626:        assert!(!a.ids_for("sanic").contains(&id("boss.shatter")));
ambition_audio/src/catalog.rs:638:            .register("sanic", BTreeMap::from([(id("shared.thud"), 42)]))
ambition_audio/src/music/director/simple.rs:126:            "a Sanic session may only play Sanic-authored tracks"
ambition_audio/src/music/director/simple.rs:136:            "ambition_boss_theme",
ambition_audio/src/music/director/simple.rs:140:        let candidates = ids(&["ambition_boss_theme", "ambition_room_calm"]);
ambition_audio/src/music/director/mod.rs:218:        let mut sanic = MusicAuthority::governed(vec!["you_are_too_slow".to_string()]);
ambition_audio/src/music/director/mod.rs:219:        sanic.authorize_cues(Vec::<String>::new());
Let me compute the module-level LOC grouping for ambition_characters to finalize its module map.

Let me check the remaining crates for hardcoded content and enumerate render's hardcoded asset paths (a content-coupling signal).

Bash render hardcoded asset paths + named content
IN
echo "=== hardcoded asset/sprite paths in render (content coupling) ===" && grep -rniE "\"(sprites|fonts|assets)/[^\"]+\"|_spritesheet|\.png\"" crates/ambition_render/src | head -30 && echo "=== named enemies/chars in render doc+code ===" && grep -rniE "puppy|slug|robot_slash|milk carton|obelisk" crates/ambition_render/src | head

OUT
=== hardcoded asset/sprite paths in render (content coupling) ===
crates/ambition_render/src/rendering/item_visuals.rs:21:// (Read from `lasersword_spritesheet.yaml`, row `idle`, frame 0.)
crates/ambition_render/src/rendering/item_visuals.rs:22:const LASERSWORD_SHEET_PATH: &str = "sprites/lasersword_spritesheet.png";
crates/ambition_render/src/rendering/item_visuals.rs:120:        .map(|&id| (id, assets.load(format!("sprites/props/gauntlet_{id}.png"))))
crates/ambition_render/src/rendering/item_visuals.rs:123:        axe: assets.load("sprites/props/axe.png"),
crates/ambition_render/src/rendering/item_visuals.rs:124:        javelin: assets.load("sprites/props/javelin.png"),
crates/ambition_render/src/rendering/item_visuals.rs:125:        gunsword: assets.load("sprites/props/gunsword.png"),
crates/ambition_render/src/rendering/item_visuals.rs:389:            fireball: asset_server.load(format!("sprites/props/gauntlet_{FIREBALL_ID}.png")),
crates/ambition_render/src/rendering/mark_beacon.rs:49:        let mut sprite = Sprite::from_image(asset_server.load("sprites/props/mark_beacon.png"));
crates/ambition_render/src/rendering/pirate_weapon.rs:37:const WEAPON_SHEET_PATH: &str = "sprites/lasersword_with_guns_spritesheet.png";
crates/ambition_render/src/rendering/pirate_weapon.rs:58:/// frame. Numbers read from `lasersword_with_guns_spritesheet.yaml`,
crates/ambition_render/src/rendering/shrine_visuals.rs:68:/// `sprites/shrine_spritesheet.png` sheet (with a flat `sprites/props/shrine.png`
crates/ambition_render/src/rendering/shrine_visuals.rs:260:            ShrineVisualSource::Flat(asset_server.load("sprites/props/shrine.png"))
crates/ambition_render/src/rendering/shrine_visuals.rs:263:        ShrineVisualSource::Flat(asset_server.load("sprites/props/shrine.png"))
crates/ambition_render/src/rendering/shrine_visuals.rs:275:    let image = asset_server.load("sprites/shrine_spritesheet.png");
crates/ambition_render/src/rendering/projectile_visuals.rs:224:    let mut sprite = Sprite::from_image(asset_server.load(format!("sprites/{}", record.image)));
crates/ambition_render/src/rendering/slash_visuals.rs:109:        image: asset_server.load(format!("sprites/{SLASH_SHEET}_spritesheet.png")),
=== named enemies/chars in render doc+code ===
crates/ambition_render/src/rendering/deep_dream.rs:1://! Per-sprite deep-dream shader experiment for the Puppy Slug enemy.
crates/ambition_render/src/rendering/deep_dream.rs:31:const SHADER_ASSET_PATH: &str = "shaders/puppy_slug_deep_dream.wgsl";
crates/ambition_render/src/rendering/deep_dream.rs:41:/// Install the material plugin that backs the puppy-slug deep-dream overlay.
crates/ambition_render/src/rendering/deep_dream.rs:42:pub fn add_puppy_slug_deep_dream_material_plugin(app: &mut App) {
crates/ambition_render/src/rendering/deep_dream.rs:43:    app.add_plugins(Material2dPlugin::<PuppySlugDeepDreamMaterial>::default());
crates/ambition_render/src/rendering/deep_dream.rs:46:/// Custom material used by the one-off puppy-slug shader.
crates/ambition_render/src/rendering/deep_dream.rs:58:pub struct PuppySlugDeepDreamMaterial {
crates/ambition_render/src/rendering/deep_dream.rs:70:impl Material2d for PuppySlugDeepDreamMaterial {
crates/ambition_render/src/rendering/deep_dream.rs:82:pub struct PuppySlugDeepDreamSource {
crates/ambition_render/src/rendering/deep_dream.rs:89:pub struct PuppySlugDeepDreamOverlay {
Bash Module-level LOC grouping for ambition_characters
IN
cd /home/joncrall/code/ambition/crates/ambition_characters && echo "=== LOC by top-level module (incl tests) ===" && for m in actor brain boss_encounter equipment perception; do total=$(find src/$m* -name '*.rs' 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}'); [ -f src/$m.rs ] && total=$(wc -l < src/$m.rs); echo "$m: check"; done; echo "--- precise per top module ---"; echo "actor:      $(find src/actor -name '*.rs' | xargs wc -l | tail -1 | awk '{print $1}')"; echo "brain:      $(find src/brain -name '*.rs' | xargs wc -l | tail -1 | awk '{print $1}')"; echo "boss_encounter: $(find src/boss_encounter* -name '*.rs' 2>/dev/null | xargs wc -l | tail -1 | awk '{print $1}') (+ boss_encounter.rs $(wc -l < src/boss_encounter.rs))"; echo "equipment:  $(wc -l < src/equipment.rs)"; echo "perception: $(find src -path '*perception*' -name '*.rs' | xargs wc -l | tail -1 | awk '{print $1}')"; echo ""; echo "=== brain submodule breakdown ==="; for s in action_set boss_pattern fighter smash state_machine; do echo "brain/$s: $(find src/brain/$s* -name '*.rs' 2>/dev/null | xargs wc -l | tail -1 | awk '{print $1}')"; done; echo "brain/ (mod+player+snapshot): $(wc -l src/brain/mod.rs src/brain/player.rs src/brain/player/tests.rs src/brain/snapshot.rs src/brain/tests.rs 2>/dev/null | tail -1 | awk '{print $1}')"

OUT
=== LOC by top-level module (incl tests) ===
actor: check
brain: check
boss_encounter: check
equipment: check
perception: check
--- precise per top module ---
actor:      3401
brain:      17381
boss_encounter: 654 (+ boss_encounter.rs 422)
equipment:  655
perception: 1233

=== brain submodule breakdown ===
brain/action_set: 1541
brain/boss_pattern: 5188
brain/fighter: 1950
brain/smash: 4621
brain/state_machine: 2185
brain/ (mod+player+snapshot): 1896
Bash settings_menu/dialog/game_shell content check
IN
echo "=== settings_menu system options (hardcoded?) ===" && grep -rniE "Quit|Radio|\"[A-Z][a-z]+ Menu\"|ambition|mary|sanic" crates/ambition_settings_menu/src | grep -vi "//" | head -20 && echo "=== dialog: any hardcoded story text? ===" && grep -rniE "\"[A-Z][a-z]+ [a-z]+ [a-z]+" crates/ambition_dialog/src | grep -viE "//|test|new game|panic|expect|assert|debug|format|println|warn|error|Failed|missing|invalid|unknown|node|yarn" | head && echo "=== game_shell content-specific route names? ===" && grep -rniE "mary|sanic|pirate|pocket|ambition_content|\"ambition\"" crates/ambition_game_shell/src | grep -vi "//" | head

OUT
=== settings_menu system options (hardcoded?) ===
crates/ambition_settings_menu/src/settings/tests.rs:5:use ambition_persistence::settings::UserSettings;
crates/ambition_settings_menu/src/settings/mod.rs:28:use ambition_persistence::settings::audio::AudioSettings;
crates/ambition_settings_menu/src/settings/mod.rs:276:        ambition_persistence::settings::video::ScreenShaderSettings::percent(value)
crates/ambition_settings_menu/src/settings/apply.rs:5:use ambition_persistence::settings::video::CameraZoomPreset;
crates/ambition_settings_menu/src/settings/apply.rs:6:use ambition_persistence::settings::UserSettings;
crates/ambition_settings_menu/src/settings/apply.rs:11:        label: "Close Menu".to_string(),
crates/ambition_settings_menu/src/settings/apply.rs:26:    use ambition_persistence::host::windowing::DisplayModeKind;
crates/ambition_settings_menu/src/settings/apply.rs:27:    use ambition_persistence::settings::controls::{
crates/ambition_settings_menu/src/settings/apply.rs:30:    use ambition_persistence::settings::gameplay::Difficulty;
crates/ambition_settings_menu/src/settings/apply.rs:31:    use ambition_persistence::settings::video::{
crates/ambition_settings_menu/src/settings/apply.rs:59:                ambition_persistence::host::windowing::prev_display_mode(cur)
crates/ambition_settings_menu/src/settings/apply.rs:61:                ambition_persistence::host::windowing::next_display_mode(cur)
crates/ambition_settings_menu/src/settings/apply.rs:187:            let len = ambition_input::KeyboardPreset::presets().len();
crates/ambition_settings_menu/src/settings/apply.rs:236:            s * ambition_persistence::settings::gameplay::GameplaySettings::DAMAGE_STEP,
crates/ambition_settings_menu/src/settings/build.rs:5:use ambition_persistence::settings::audio::AudioSettings;
crates/ambition_settings_menu/src/settings/build.rs:6:use ambition_persistence::settings::video::CameraZoomPreset;
crates/ambition_settings_menu/src/settings/build.rs:7:use ambition_persistence::settings::UserSettings;
crates/ambition_settings_menu/src/settings/build.rs:10:    use ambition_persistence::host::windowing::DisplayModeKind;
crates/ambition_settings_menu/src/settings/build.rs:11:    use ambition_persistence::settings::controls::{
crates/ambition_settings_menu/src/settings/build.rs:14:    use ambition_persistence::settings::gameplay::Difficulty;
=== dialog: any hardcoded story text? ===
crates/ambition_dialog/src/lib.rs:39:    reason = "DialogChoice surfaces in the UI's choice-row layout"
crates/ambition_dialog/src/speech_sfx.rs:271:            talk_blip_id_for_speaker("Shadow Oni Leader", "", DialogSpeechStyle::Normal),
crates/ambition_dialog/src/bridge.rs:95:        "spawned DialogueRunner entity {entity:?}",
=== game_shell content-specific route names? ===
crates/ambition_game_shell/src/launcher.rs:57:            title: "Ambition".to_owned(),
crates/ambition_game_shell/src/experience.rs:305:            reg("sanic", "Sanic", "sanic_gameplay"),
crates/ambition_game_shell/src/experience.rs:306:            ShellRouteSpec::new("sanic_gameplay", "sanic"),
crates/ambition_game_shell/src/experience.rs:309:            reg("sanic", "Sanic", "sanic_gameplay"),
crates/ambition_game_shell/src/experience.rs:310:            ShellRouteSpec::new("sanic_gameplay", "sanic"),
crates/ambition_game_shell/src/experience.rs:320:    #[should_panic(expected = "duplicate shell experience id 'sanic'")]
crates/ambition_game_shell/src/experience.rs:324:            reg("sanic", "Sanic", "sanic_gameplay"),
crates/ambition_game_shell/src/experience.rs:325:            ShellRouteSpec::new("sanic_gameplay", "sanic"),
crates/ambition_game_shell/src/experience.rs:329:            reg("sanic", "Impostor", "impostor_route"),
crates/ambition_game_shell/src/experience.rs:330:            ShellRouteSpec::new("impostor_route", "sanic"),
I have gathered comprehensive data. Here is the full recon report.

Crate-Decomposition Recon: Foundation Layer
Oracle used throughout: "could another platformer be built by ADDING a content crate without editing core?" LOC figures below are per-file source lines; each crate's headline LOC includes co-located #[cfg(test)] files (engine_core is ~13.8k source + ~9.4k test).

1. ambition_engine_core — module map (source LOC)
Zero workspace deps; deps are bevy_ecs (derive-only), bevy_math, parry2d, serde. lib.rs doc explicitly frames this as "the pure, content-free movement/physics MODEL." It is broadly a coherent kernel, not a grab-bag, but it has grown into ~6 distinguishable sub-clusters and carries a few content-flavored constants.

Cluster A — Geometry & collision primitives (pure math, no ECS state):

geometry.rs (518) — Bevy Aabb2d-backed collision helpers: center/half, strict platformer overlap (edge-touch = miss), Parry swept-box.
cast.rs (395) — the swept-primitive library ("THE SWEEP LAW"); the single home for sweep_hit / body_sweep / raycast_solids. Note: ray_aabb was explicitly moved down from platformer_primitives.
collision_semantics.rs (391) — gravity-relative support/surface classification kernel shared by BOTH engine_core movement AND platformer_primitives::kinematic (dedup of two near-identical copies).
combat_volume.rs (328) — hit/hurt shape: AABB / OBB / convex, Parry-backed.
volume_shape.rs (176) — authored local-space (+x=forward) hit/hurt shape; place_at mirrors to facing.
geo_id.rs (216) — durable geometry identity (GeoId/GeoFaceRef) substrate for deltas/portals/saves.
Cluster B — Authored world / room IR:

world.rs (1177) — Block/BlockKind/World/SurfaceChain/water/climbable/BlinkWallTier. The big one.
kinematic_path.rs (42) — declarative waypoint paths for moving platforms/hazards.
config.rs (87) — engine↔Bevy coordinate transforms + z-layer/grid/window-size constants.
Cluster C — Reference frame / relativity (spatial):

reference_frame.rs (619) — AccelerationFrame, the 3-frame model (input/local-body/world) + transforms; MotionFrame, LocalAxes, WorldVec2.
frame.rs (132) — portal aperture frame geometry (origin+normal+pair-map); deliberately in core so cast can be portal-aware without depending on the portal gameplay crate.
Cluster D — Body-state ECS aggregate:

body_clusters.rs (719) — the ~20 Component clusters (BodyKinematics, BodyGroundState, BodyDashState, …) + the BodyClustersMut struct-of-&mut view. The authoritative body every actor carries.
abilities.rs (651) — capability flags (AbilitySet/AbilityGrant): "may this verb be used at all."
player_state.rs (491) — LocomotionState / BodyMode / ResourceMeter vocabulary (Bevy-free).
Cluster E — Movement kernel (movement/ subtree, the bulk):

movement/mod.rs (416) — step_motion, the ONE movement entry; frame law.
movement/surface_momentum/mod.rs (1362) — circle-proxy surface-follower solver (arc-length riding).
movement/integration.rs (630) — the normal-gravity spine (integrate_normal_spine).
movement/collision.rs (570) — controlled-body sweep + one-way/hazard/rebound helpers.
movement/adhesive_crawler.rs (518) — surface-glued crawler policy ("PuppySlug crawl").
movement/tuning.rs (506) — MovementTuning / AxisSweptParams + hardcoded DEFAULT_* constants (GRAVITY=2250, JUMP_SPEED=630, DASH_SPEED=760, BLINK_DISTANCE=190…).
movement/model.rs (405) — MotionModel identity/params/state + switch_motion_model.
movement/abilities.rs (256) — composable apply_<verb> limbs.
movement/kernel.rs (243) — frame-aware kernel facade.
movement/blink.rs (237) — blink teleport destination logic.
movement/authority.rs (228) — the non-kernel motion authorities: transit_body (discrete) / carry_body (constraint).
movement/simulation.rs (151), control.rs (149), facts.rs (108), input.rs (107, InputState), ops.rs (106), events.rs (55), player.rs (14).
ledge_grab/mod.rs (371) + ledge_grab/runtime.rs (658) — ledge probe + engine-owned state machine.
Cluster F — Input / netcode vocabulary:

control_frame.rs (292) — device-agnostic ControlFrame + ControlFrameLatch.
input_stream.rs (348) — per-tick input artifact for replay/RL/desync-forensics/wire (netcode N0.2).
Verdict on coherence: A genuine kernel. The natural split lines are clean: A (geometry+cast+collision_semantics+combat/volume shapes) could be an ambition_collision/geometry crate; C (reference_frame+frame) is a self-contained relativity unit; F (control_frame+input_stream) is a netcode-vocabulary unit; E is the movement kernel proper. The cluster components (D) and world IR (B) are the two "state schema" pieces the kernel operates on.

NOT-foundation / content-flavored flags inside core:

movement/tuning.rs — Ambition's specific tuning numbers (GRAVITY=2250, JUMP_SPEED=630, etc.) are baked as pub const DEFAULT_*. They are overridable via MovementTuning (content hydrates from RON), so the oracle survives — but the defaults are Ambition's, a mild content smell in a "content-free" crate.
config.rs — WINDOW_W=1600, WINDOW_H=900, z-layer constants: app/presentation values, not simulation. Minor.
abilities.rs — helper presets ("classic platformer protagonist" kit, "endgame sandbox defaults") bake a specific ability kit; framed as reusable presets so tolerable.
world.rs::BlinkWallTier / BlockKind::BlinkWall — a wall keyed to the blink ability's tier. Game-mechanic vocabulary, but generic enough (tiered pass-through gate).
Game proper nouns appear ONLY in comments/tests, never in the type vocabulary — good hygiene. Examples: world.rs:1146 "Sanic-momentum", cast.rs:66 "a Sanic run", geometry.rs:492 & collision_semantics.rs:154 "mockingbird", adhesive_crawler.rs:3 "PuppySlug crawl". These are documentation, not identifiers.
2. Overlap: engine_core vs platformer_primitives
The boundary is pure logic (engine_core) vs ECS wiring/runtime seams (platformer_primitives), and it is largely correct — platformer_primitives depends DOWN on engine_core and never up. Concrete seams:

Collision: platformer_primitives::kinematic.rs (the generic enemy/NPC sweep) consumes engine_core::collision_semantics and calls engine_core::cast::body_sweep directly (kinematic.rs:34, :279). The near-duplicate helper copies were already collapsed into collision_semantics. Good — no live duplication, this is the intended "two sweeps, one semantic kernel" arrangement. The historical ray_aabb duplication was resolved by moving it into cast (per collision-and-ccd §3.4b).
Reference frame: the PURE frame math is in engine_core::reference_frame + frame; the ECS resolver half is platformer_primitives::frame_env.rs (259) + gravity.rs::GravityField::motion_frame() which just constructs engine_core::MotionFrame::from_direction. This is a deliberate split (pure vs per-body resource), not a duplicate concept.
Portal math: engine_core::frame owns aperture GEOMETRY; platformer_primitives::math.rs + transit.rs own the reflect/rotate-between-normals VECTOR math and dispatch the game-wide map convention. Two files, one concept, cleanly layered (core = geometry, primitives = the convention flag).
Things on the arguably-wrong side / worth scrutiny in platformer_primitives:

shrine.rs (8) — ShrineActivationPulse, a save/heal-shrine presentation pulse. This is a specific game feature, not a reusable primitive. Content leak. (And note ambition_world::rooms::specs.rs also has ShrineSpec → the "shrine" concept is scattered across two foundation crates; see §3.)
camera_ease.rs (191) + camera_layers.rs (35) — Bevy camera presentation state/markers. Not "physics primitives"; they were parked here during an F1.5 move so render/host could share them without depending on ambition_actors. Legitimate home problem, but they make the crate name ("primitives") a partial misnomer.
world_item_art.rs (108) — provider→renderer art-contribution seam; doc example hardcodes super_mary_o_milk_carton.png. Generic mechanism, content-flavored doc.
schedule.rs (394) — GameMode { Cutscene, Dialogue, … } and per-boss schedule slots. Runtime schedule vocabulary naming content phases; generic but leans app-specific.
Net: the crate is really "reusable runtime seams + a few presentation/feature bits," broader than its "primitives" name. The engine_core↔primitives boundary itself is sound; no concept is duplicated across the two, and no dependency edge points the wrong way.

3. The smaller crates — coherence, naming, merge/leak
ambition_time (326, single file): Coherent and well-named. ClockObserver / ClockDomain / WorldTime / ClockState / TimePlugin — content-free by construction (ClockObserver is a bare u8 newtype specifically to avoid coupling to a game's player type). 9 dependents. Too small to be a crate? No — keep it. It is a legitimate narrow-waist leaf: nearly everything reads dt through it, and folding it into engine_core would drag a bevy (app/plugin) dependency into the deliberately-thin core. Standalone is correct.

ambition_world (3,480): Coherent "backend-agnostic authored world IR" — room graph (petgraph), placements, metadata, moving-platform math, and the composited CollisionWorld. Well-named. Does it belong with ldtk_map? No, keep separate: ambition_ldtk_map depends DOWN on ambition_world as an adapter (ldtk→IR). Merging would invert the "IR is backend-neutral, LDtk is one backend" design and pull the LDtk parser into everyone who wants room data. Content leak: rooms/specs.rs::ShrineSpec resolves to platformer_primitives::HealShrine; rooms/metadata.rs carries biome/music_track (authored-content fields, but genuinely data, acceptable); rooms/camera.rs doc references Mary-O. The shrine coupling is the one real cross-crate content concept (see §5 note).

ambition_input (2,340): Coherent device→ControlFrame adapter; correct downward dep on engine_core. Naming smells: the leafwing action enum is SandboxAction and lib.rs says "for the sandbox" — content-flavored naming for what should be a generic action set. Content leak: motion_input.rs bakes fighting-game move names — "upgrade a plain Fireball press into Hadouken / HadoukenSuper." Specific moves in a foundation input crate; a different platformer would have to edit this to add gestures.

ambition_gameplay_trace (1,482): Coherent, well-named "flight recorder." Content-free vocabulary — boss/enemy/slug appear only in tests/docs, and factions are stringly-typed buckets (player|boss|enemy|npc|body). actor_trace.rs is notably the "relativity-respecting" no-privileged-observer sibling of the player-centric buffer. Depends on engine_core + input (for ControlFrame per frame). Fine as-is. Only 3 dependents (actors/projectiles).

ambition_asset_manager (4,013): The one crate that is genuinely three things stapled together:

Core catalog/resolver (id/kind/location/manifest/policy/preload/profile/resolver/bevy_integration, ~1,729 LOC) — clean, generic, reusable. Good.
sandbox_assets/ (1,129 LOC, ~28%) — pure Ambition CONTENT wiring: hardcoded world.sandbox_ldtk, world.intro_ldtk, world.cut_rope_ldtk, world.hall_ldtk, boss sheets (gradient sentinel + mockingbird), dialog fonts, music. This is the single biggest content-leak block in the whole foundation layer and is the textbook thing the oracle wants split OUT into a content crate.
asset_publish/ (889 LOC) — a build/tooling concern (publish/install generated sprites, hygiene, manifest walk). Belongs in a dev-tools/build crate, not the runtime resolver. Recommendation: asset_manager should shed both subtrees — sandbox_assets → a game content crate, asset_publish → tooling — leaving a ~1.7k clean generic catalog.
4. Game-specific identifier grep (files/lines)
All hits are in comments, doc-strings, or #[cfg(test)] fixtures — none are in production type/field/const names, except the input-gesture and asset-catalog cases noted. Key locations:

engine_core (docs/tests only — clean):

world.rs:1146 "dash / Sanic-momentum bursts"; cast.rs:66 "a blink, a dash, a Sanic run"
geometry.rs:492, collision_semantics.rs:154, collision_semantics/tests.rs:106 "mockingbird" (OOB regression provenance)
movement/adhesive_crawler.rs:3 "the PuppySlug crawl"
abilities.rs:3,148,180 "endgame sandbox"; :433,620 "classic platformer protagonist" kit
body_clusters.rs:5,161,185,193 "player, enemy, NPC, boss" (generic vocabulary, fine)
platformer_primitives:

lifecycle/session.rs:3-4 "a launched Sanic run, a launched Mary-O run"
sim_id.rs:29,65,125-142 boss/minion (test fixtures + doc)
schedule.rs:172-173 "the cut-rope boss," :257-259,325 Cutscene/Dialogue modes
projectile/collision.rs:14 "the player-robot boss"; kinematic/tests.rs:134 mockingbird regression
shrine.rs — ShrineActivationPulse (production type; feature-specific)
ambition_world:

rooms/camera.rs:61-64 Mary-O level-scroll doc; rooms/metadata.rs:137 in_mode("sanic") doc
rooms/graph.rs:130 "blink / dash / Sanic"; rooms/specs.rs:111-120 ShrineSpec → HealShrine
debug_label.rs:35 Boss enum variant (generic label bucket)
ambition_input:

motion_input.rs:2-3 Hadouken / HadoukenSuper / Fireball (production gesture vocabulary — real leak)
actions.rs / lib.rs — SandboxAction, "for the sandbox" naming
ambition_gameplay_trace: actor_trace.rs:2,11,31,36,398-469 boss/enemy/slug/mockingbird — all doc + test fixtures. Clean.

ambition_asset_manager (production content):

sandbox_assets/ids.rs:11-14 SANDBOX_LDTK/INTRO_LDTK/CUT_ROPE_LDTK/HALL_LDTK; :63-65 boss_sprite
sandbox_assets/builders/visuals.rs:82-130 "player/robot/goblin/sandbag", SPRITE_ROBOT_URL, "mockingbird boss"
sandbox_assets/mod.rs:5,55 boss sheets; preload.rs:30-33 biome/boss-room/cutscene groups
resolver/tests.rs sprite.cutscene.boss_intro (test)
ambition_time: zero hits. Fully generic.

5. Relativity / reference-frame / motion-authority theme
The theme splits cleanly into a spatial axis and a temporal axis, and both are well-isolated rather than scattered:

Spatial reference frames (gravity-relative):

Pure math home: engine_core::reference_frame.rs (619) owns AccelerationFrame + the three-frame model (input / local-body / world) + all transforms, plus MotionFrame, LocalAxes, WorldVec2, ScreenAxes. frame.rs (132) owns the portal-aperture frame as a sibling.
ECS resolver half: platformer_primitives::frame_env.rs (259) resolves "one frame per body per tick, consumed everywhere," fed by gravity.rs::GravityField::motion_frame().
The concept is pervasive but typed, not duplicated: MotionFrame/AccelerationFrame are referenced across ~30 engine_core files (reference_frame.rs 43, kernel/tests 23, wall_collision 16, input.rs 11, authority.rs 11, integration.rs 10…). That breadth is by design — ADR 0024's frame law requires every directional quantity to carry its frame in its type — so it reads as one coherent well-enforced invariant, not scatter.
Motion authorities: movement/authority.rs (228) is the single explicit home for the non-kernel authorities — transit_body (discrete: blink/portal/respawn arrivals) and carry_body (external kinematic constraint: moving-platform carry, attractor pull). Cleanly isolated in one file; the kernel (step_motion) is the only other writer of authoritative pose.

Temporal "relativity" (proper time / cognitive time): lives entirely in ambition_time — ClockDomain::{SimClock, PlayerClock(observer), WallClock}, WorldTime::{sim_dt, wall_dt, player_dt (observer cognitive time), entity_dt (per-entity proper time)}, per ADR 0010/0011. platformer_primitives::time.rs::SimDt is the inversion seam mirroring the host's scaled dt into the content-free runtime. gameplay_trace::actor_trace.rs explicitly bills itself the "relativity-respecting" no-privileged-observer recorder.

Assessment: The relativity/reference-frame machinery is one of the cleanest parts of the foundation. Spatial frames = a 2-file pure unit in core (reference_frame + frame) mirrored by one ECS resolver in primitives; temporal frames = the whole of ambition_time; motion authorities = one file. Nothing is duplicated and the split lines are natural. If anything, reference_frame.rs + frame.rs + the frame_env resolver would make a tidy standalone ambition_reference_frame crate.

Split-planning takeaways (against the oracle)
Cleanest extractable units from engine_core: (a) geometry+cast+collision_semantics+combat/volume shapes; (b) reference_frame+frame (+frame_env from primitives); (c) control_frame+input_stream (netcode vocabulary). The movement kernel + body clusters + world IR are the irreducible core.
Biggest content leaks to evict (oracle-critical): asset_manager::sandbox_assets (1,129 LOC of hardcoded LDtk/boss/font/music ids), input::motion_input Hadouken/Fireball gestures + SandboxAction naming, and the shrine concept split across platformer_primitives::shrine + world::ShrineSpec. These are the places a second platformer would be forced to edit core rather than add a crate.
Tooling misfit: asset_manager::asset_publish (889 LOC) is build tooling in a runtime crate.
Keep-as-is: ambition_time (correct narrow-waist leaf), ambition_world vs ldtk_map (correct adapter direction), ambition_gameplay_trace (clean, content-free).
Let me confirm a few final details: the game_shell "Ambition" title context, settings menu labels, and verify ui_nav/inventory_ui purity.

Foundation-layer report is in — headline findings: engine_core is a genuinely coherent kernel (not a grab-bag) with three cleanly extractable sub-units if wanted; the real oracle violations in the foundation are asset_manager::sandbox_assets (~1.1k LOC of hardcoded Ambition asset wiring inside a supposedly generic crate), Hadouken/Fireball gesture vocabulary baked into ambition_input, and the shrine concept scattered across platformer_primitives + world. Relativity machinery (reference frames, motion authorities, time domains) is among the cleanest parts of the codebase. Five agents still running; I'll synthesize once they report.

Bash Confirm game_shell/settings content specificity
IN
echo "=== launcher.rs title context (50-65) ===" && sed -n '48,66p' crates/ambition_game_shell/src/launcher.rs && echo "=== is experience.rs sanic in tests? (300-312) ===" && sed -n '298,312p' crates/ambition_game_shell/src/experience.rs && echo "=== settings system menu labels ===" && grep -rnE "label:|\"(Radio|Video|Audio|Controls|Gameplay|Language|Reset|Quit|Developer|System)" crates/ambition_settings_menu/src/system/mod.rs | head -20

OUT
=== launcher.rs title context (50-65) ===
    /// `None` hides it (kiosk builds / hosts that own quit elsewhere).
    /// Selecting it emits the semantic `ShellCommand::ExitProcess`; acting on
    /// the resulting `ShellEvent::ExitRequested` remains the HOST's job.
    pub exit_label: Option<String>,
}

impl Default for ShellLauncherPresentation {
    fn default() -> Self {
        Self {
            title: "Ambition".to_owned(),
            empty_message: "No experiences registered".to_owned(),
            footer: "Arrow keys select · Enter launches".to_owned(),
            exit_label: Some("Exit".to_owned()),
        }
    }
}

#[derive(Resource, Default, Clone, Debug, Eq, PartialEq)]
pub struct ShellLauncherState {
=== is experience.rs sanic in tests? (300-312) ===
        ExperienceRegistration::new(id, name, route)
    }

    #[test]
    fn identical_re_registration_is_idempotent() {
        let mut app = App::new();
        app.register_experience(
            reg("sanic", "Sanic", "sanic_gameplay"),
            ShellRouteSpec::new("sanic_gameplay", "sanic"),
        );
        app.register_experience(
            reg("sanic", "Sanic", "sanic_gameplay"),
            ShellRouteSpec::new("sanic_gameplay", "sanic"),
        );
        assert_eq!(
=== settings system menu labels ===
crates/ambition_settings_menu/src/system/mod.rs:277:            Self::Radio => "Radio",
crates/ambition_settings_menu/src/system/mod.rs:278:            Self::Video => "Video",
crates/ambition_settings_menu/src/system/mod.rs:279:            Self::Audio => "Audio",
crates/ambition_settings_menu/src/system/mod.rs:280:            Self::Controls => "Controls",
crates/ambition_settings_menu/src/system/mod.rs:281:            Self::Gameplay => "Gameplay",
crates/ambition_settings_menu/src/system/mod.rs:282:            Self::Language => "Language",
crates/ambition_settings_menu/src/system/mod.rs:283:            Self::ResetAllSettings => "Reset All Settings",
crates/ambition_settings_menu/src/system/mod.rs:284:            Self::QuitToHome => "Quit to Title",
crates/ambition_settings_menu/src/system/mod.rs:285:            Self::Quit => "Quit to Desktop",
crates/ambition_settings_menu/src/system/mod.rs:286:            Self::Developer => "Developer",
crates/ambition_settings_menu/src/system/mod.rs:287:            Self::ResetSandbox => "Reset Sandbox",
crates/ambition_settings_menu/src/system/mod.rs:302:            Self::Developer => "Developer inspectors, debug visuals, and feel profiles.",
crates/ambition_settings_menu/src/system/mod.rs:359:    pub label: String,
crates/ambition_settings_menu/src/system/mod.rs:368:    pub label: String,
crates/ambition_settings_menu/src/system/mod.rs:377:    pub label: String,
crates/ambition_settings_menu/src/system/mod.rs:378:    pub value_label: String,
crates/ambition_settings_menu/src/system/mod.rs:386:    pub label: String,
crates/ambition_settings_menu/src/system/mod.rs:424:    pub fn cycle(id: DevToggleId, value_label: impl Into<String>) -> (DevToggleId, bool, String) {
crates/ambition_settings_menu/src/system/mod.rs:535:                label: id.label().to_string(),
crates/ambition_settings_menu/src/system/mod.rs:549:                label: name.clone(),
Bash ui_nav and inventory_ui contents
IN
echo "=== ui_nav lib.rs ===" && cat crates/ambition_ui_nav/src/lib.rs && echo "=== inventory_ui lib.rs + model head ===" && cat crates/ambition_inventory_ui/src/lib.rs && head -30 crates/ambition_inventory_ui/src/model.rs

OUT
=== ui_nav lib.rs ===
//! Shared UI/menu navigation helpers.
//!
//! Pause menus, dialog choices, inventory/map pages, and mobile touch gestures
//! should all consume the same small set of semantic primitives: windowed list
//! math, discrete scroll-to-row navigation, pointer row activation, and drag
//! accumulation. Keeping those pieces here prevents the keyboard/gamepad/touch
//! paths from drifting apart as the desktop-first UI gets mobile affordances.

mod drag;
mod list;
mod pointer;

pub use drag::DragScrollState;
#[cfg(feature = "input")]
pub use list::apply_vertical_scroll;
pub use list::visible_window_start;
pub use pointer::{resolve_selectable_row_interaction, MenuFocusOwner, MenuFocusState};
=== inventory_ui lib.rs + model head ===
//! Inventory menu-navigation state.
//!
//! The item store itself is the `OwnedItems` catalog in `ambition_items`;
//! this crate owns only the `InventoryUiState` resource (selection / tab /
//! scroll / focus) driven by the unified menu. Keeping this state here lets the
//! reusable item catalog stay below menu-navigation and presentation tiers.

mod model;

pub use self::model::{InventoryTab, InventoryUiState};
//! Inventory-screen UI state model: [`InventoryUiState`] (visible/selected/tab/
//! scroll/focus) + the [`InventoryTab`] enum (Items / Map / Quests).
//!
//! This is menu-NAVIGATION state only — it holds no items. The actual item
//! store is the `OwnedItems` catalog in `ambition_items`; this struct just
//! tracks where the cursor is in the unified menu's inventory view.

// The legacy adventure-menu UI that consumed the per-entity inventory component
// markers was deleted in Phase D2; the unified menu reads the data model
// (`InventoryUiState`) + the `OwnedItems` catalog (`ambition_items`) directly. The
// dead markers and the legacy 3-kind `ItemKind`/`PlayerInventory` bag were
// removed once `OwnedItems` became the single item store. What remains is the
// live menu-navigation state.
#![allow(dead_code)]

use bevy::prelude::*;

use ambition_ui_nav::MenuFocusState;

/// Top-level adventure-menu tab.
///
/// Keep this intentionally small: this is not an editor/debug surface, it is
/// the phone-friendly player-facing overlay that mirrors the Zelda-style
/// left/right page mental model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum InventoryTab {
    #[default]
    Items,
    Map,
    Quests,
I have gathered comprehensive data. Let me do two final targeted checks to solidify the render→world-IR seam observation and confirm the touch_input render coupling is fonts-only.

Bash Confirm render world-IR direct reads and leafwing usage
IN
echo "=== render/world.rs: reads world IR directly (no read-model) ===" && grep -n "RoomSet\|RoomSpec\|PlacementSchema\|Authored\|Res<\|SessionWorldRef" crates/ambition_render/src/rendering/world.rs | head -20 && echo "=== confirm render leafwing usage ===" && grep -rn "leafwing" crates/ambition_render/src | head

OUT
=== render/world.rs: reads world IR directly (no read-model) ===
31:/// reads the active room from [`RoomSet`] and rebuilds its static visuals +
38:    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<
39:        ambition_world::rooms::RoomSet,
41:    physics_settings: Res<ambition_platformer_primitives::physics::PhysicsSandboxSettings>,
42:    assets: Option<Res<GameAssets>>,
43:    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
44:    active_session: Option<Res<ActiveSessionScope>>,
76:    spec: &ambition_world::rooms::RoomSpec,
102:    // Per-family authored visuals. Each family carries an Authored<T>
108:        if let ambition_entity_catalog::placements::PlacementSchema::Hazard(hazard) = &record.schema
110:            let authored = ambition_world::rooms::Authored {
121:        if let ambition_entity_catalog::placements::PlacementSchema::Pickup(pickup) = &record.schema
138:        if let ambition_entity_catalog::placements::PlacementSchema::Chest(chest) = &record.schema {
139:            let authored = ambition_world::rooms::Authored {
150:        if let ambition_entity_catalog::placements::PlacementSchema::Breakable(breakable) =
201:        if let ambition_entity_catalog::placements::PlacementSchema::Interactable(spec_i) =
204:            let authored = ambition_world::rooms::Authored {
520:    assets: Option<Res<GameAssets>>,
564:    // (no-asset mode, missing file). Authored entity-derived blocks
757:    authored: &ambition_world::rooms::Authored<ambition_world::rooms::HazardVolumeSpec>,
=== confirm render leafwing usage ===
Bash touch_input render coupling and menu_bridge purpose
IN
echo "=== touch_input bevy_plugin render imports (full) ===" && grep -n "ambition_render" crates/ambition_touch_input/src/*.rs && echo "=== menu_bridge purpose (head) ===" && head -20 crates/ambition_touch_input/src/menu_bridge.rs

OUT
=== touch_input bevy_plugin render imports (full) ===
crates/ambition_touch_input/src/lib.rs:13://! binary. The direct `ambition_render` edge is intentional: the crate draws its
crates/ambition_touch_input/src/lib.rs:16://! `ambition_render` / `ambition_ui_nav` / `ambition_cutscene` library seams.
crates/ambition_touch_input/src/bevy_plugin.rs:30:use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
crates/ambition_touch_input/src/bevy_plugin.rs:146:                    .after(ambition_render::ui_fonts::load_ui_fonts),
=== menu_bridge purpose (head) ===
//! Bridge touch / mouse / joystick input into both the gameplay
//! `ControlFrame` and the menu-side `MenuControlFrame`.
//!
//! Two systems live here:
//!
//! - [`fold_to_control_frame`] — gameplay merge (axis exclusive,
//!   buttons OR-merge, activity-gated against keyboard).
//! - [`fold_to_menu_control_frame`] — menu/dialog merge (touch
//!   buttons + analog stick + drag-scroll).
//!
//! The systems read [`super::bevy_plugin::MobileTouchState`] (a
//! Bevy `Resource` wrapping the pure [`super::state::TouchInputState`])
//! plus [`super::bevy_plugin::TouchControlsVisible`] /
//! [`super::bevy_plugin::MenuTouchGestureState`], and write
//! [`ambition_input::ControlFrame`] / [`ambition_input::MenuControlFrame`].
//! They are scheduled by [`super::bevy_plugin::TouchControlsPlugin`].

use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
I have thorough data now. Here is the full recon report.

Crate-Decomposition Recon: Ambition Gameplay Midlayer
Workspace has ~48 crates. All 10 targets sit above the foundation layer (ambition_engine_core, ambition_platformer_primitives, ambition_characters, ambition_entity_catalog, ambition_vfx, ambition_sfx, ambition_time) and below the sim heart (ambition_actors, 68.7k LOC) and the host/runtime/app layer. Dependency edges confirm the intended arrows: combat/projectiles/encounter/etc. are consumed BY ambition_actors, never the reverse. LOC (source incl. tests):

crate	total	test LOC	note
ambition_combat	9,960	3,237	
ambition_portal_presentation	6,505	1,573	bigger than portal core
ambition_portal	5,246	1,524	
ambition_persistence	3,688	330	
ambition_projectiles	2,244	594	
ambition_dialog	2,024	191	
ambition_encounter	1,698	inline	tests are #[cfg(test)] in-file
ambition_dev_tools	1,665	109	
ambition_load_presentation	1,640	495	
ambition_load	1,158	241	
ambition_cutscene	399	inline	
ambition_interaction	298	inline	
1. Module maps + LOC (combat, portal, persistence) + smaller-crate identities
ambition_combat (9,960) — crates/ambition_combat/src/

moveset/          931 mod + 702 prefabs + 1729 tests   <- the unified move runtime
  mod.rs          MovePlayback, advance_move_playback, trigger_moveset_moves, dispatch_move_events
  prefabs.rs      MoveSpec builders (attack_move_from_melee, fire_move_from_ranged, actor moveset assembly)
components/       actors.rs 685 + features.rs 203 + mod.rs 268   <- ECS combat components
lib.rs            643    Damage, DamageVolume, AttackIntent/Spec/View (legacy directional-attack helpers)
hitbox/           mod.rs 389 + tests 579                <- Hitbox entity lifecycle, apply_hitbox_damage
events.rs         450    HitEvent, HitSource, HitTarget, FeatureVisualKind, GameplayBanner
slots.rs          404    anti-clump attack-slot arbitration (enemy AI)
targeting.rs      348 (+536 tests)  ActorTarget selection for non-player actors (enemy AI)
on_hit.rs         256 (+206 tests)  HitboxOnHit / pogo_bounce technique primitive
util.rs           254
path_motion.rs    234    waypoint follower for moving hazards/platforms
breakables.rs     217    breakable feature tick
hazards.rs        201 (+187 tests) + hazard_runtime.rs 97   contact-damage hazards
authored_volumes.rs 175  App-local authored attack-polygon resolution seam
falling_chest.rs  151    reward-chest fall physics
components/features.rs 203
variation.rs 67, held_items.rs 26, banner.rs 22
Combat deps: ambition_characters, engine_core, entity_catalog, interaction, platformer_primitives, sfx, time, vfx. Notably does NOT depend on ambition_actors or ambition_projectiles.

ambition_portal (5,246) — crates/ambition_portal/src/

placement.rs      728 (+533 tests)  portal fit/facing/transit-roll geometry
transit.rs        548    portal_transit, carves, PortalBody/PortalPolicy, teleport
view.rs           516 (+523 tests)  through-portal view-map + view cone MATH (not rendering)
color.rs          299    PortalChannel/GunColor
pieces.rs         262 (+222 tests)  pure portal-piece geometry (the Core invariant)
types.rs          212    PlacedPortal, cooldowns, constants
plugin.rs         180    PortalPlugin / PortalSimulationPlugin
gun_projectile.rs 159 + gun.rs 59 + gun_pickup.rs 44 + gun_lifecycle.rs 40   <- gun_* compat modules
link.rs           138    portal-pair linking
eviction.rs       120    straddler eviction
messages.rs 100, lifecycle.rs 96, tuning.rs 93, schedule.rs 38, lib.rs 90
Deps: only bevy, serde, engine_core, platformer_primitives. Fully host-free/deterministic.

ambition_persistence (3,688) — crates/ambition_persistence/src/

settings/video/mod.rs   717 + quality.rs 412 + tests 216   <- bulk is video settings
save_data.rs            524    SandboxSaveData, PersistedEncounter/Switch/Quest, ability/quest flags (pure serde)
quest/mod.rs            401 + registry.rs 171               quest data types + progression rules
settings/gameplay.rs    344
save.rs                 191    Bevy disk shim (load/save/autosave systems)
settings/audio.rs 176, persistence.rs 132 (+114 tests), platform_paths.rs 78, mod.rs 71
host/windowing.rs       103
lib.rs                  35     PersistenceSchedulePlugin
Deps: engine_core, input, bevy, ron, serde.

Smaller-crate one-paragraph identities
ambition_projectiles (2,244) — the reusable, content-free projectile MODEL/vocabulary. Owns the player-shot charge/gesture state machine (PlayerProjectileState), per-projectile ECS components, named shot kinds + art descriptors, spawn gating/pooling, and pure portal-aperture transit. It re-exports the actual projectile PHYSICS primitive from ambition_platformer_primitives::projectile — it is a combat-content crate, not a physics crate (see §3).
ambition_portal (5,246) — the reusable content-free portal MECHANIC: topology, placement/fit, transit math, view-cone/view-map geometry, carves, eviction, lifecycle, events. The Ambition portal-GUN workflow is sequestered in named gun_* modules and is explicitly not the conceptual core; static/scripted/moving portals work without a gun.
ambition_encounter (1,698) — generic wave/lockdown encounter vocabulary + headless phase machine + the ONE unified encounter-timeline authority (EncounterScript). Owns authored data (EncounterSpec/waves/mobs/lock-wall), the EncounterPhase reducer, participants, objective, registry/switch-activation, music request, reward math, and the neutral timeline (beats/triggers/effects).
ambition_interaction (298) — tiny pure-data building-block crate: Interactable/InteractionKind (Door/Npc/Chest…), Pickup/PickupKind, Chest/ChestState, Breakable/BreakableState/BreakableTrigger/BreakableCollision. No systems, no Bevy. Just serde components.
ambition_cutscene (399) — single-file pure-data cutscene scripting: CutsceneBeat (Wait/Dialogue/CameraPan/Fade/SetFlag/Banner) + CutsceneScript. Bevy-imported but logic-free/headless-testable; presentation lives in the host.
ambition_dialog (2,024) — reusable content-free dialogue runtime: poll-based DialogState UI read model, typewriter reveal + input translators, and a bevy_yarnspinner bridge (YarnBridgePlugin) with a YarnContentBindings installer seam. Game-specific Yarn commands/functions stay host-side.
ambition_dev_tools (1,665) — reusable developer-tooling STATE + live-edit logic carved out of actors: DeveloperTools toggles, reflected editable player-tuning/ability/stats resources + their engine conversions, SandboxDevState, profiler marks, developer.ron persistence, and the sync_live_player_dev_edits_system. The egui overlay UI stays in ambition_app.
ambition_load (1,158) — headless, game-neutral loading COORDINATION: records what a game waits on, keeps activation barriers honest, and makes cancellation/supersession/streaming/prefetch/commit-authorization composable via LoadCoordinator + LoadCommand/LoadEvent. Renders nothing, owns no destination. Deps: only bevy.
ambition_load_presentation (1,640) — replaceable, shell-integrated presentation for unresolved load barriers: delayed reveal, honest semantic progress, optional activity lifecycles, ready-hold/Continue, scoped cleanup. Consumes ambition_load facts + ambition_game_shell routes; contains no game-specific activity branches (has a basic_presentation feature for a no-art default).
2. combat: is the moveset system the ONE combat vocabulary?
Largely yes — the moveset (MoveSpec + combat::moveset) is now the single dispatch vocabulary for melee, ranged, AND special. Confirmed at crates/ambition_combat/src/moveset/mod.rs:

trigger_moveset_moves (line ~586) resolves EVERY offensive verb through the same path — special_pressed → move_for_verb("special"), melee/pogo_pressed → move_for_directional_verb(ATTACK_VERB, dir, grounded), fire → move_for_verb(RANGED_VERB) — all producing a MovePlayback.
advance_move_playback is the ONE system turning the authored timeline into simulation (windows→hitbox entities via the existing apply_hitbox_damage path; events→MoveEventMessage).
dispatch_move_events (line ~718) is content-free and bridges back to the pre-existing seams rather than forking: Effect(key) writes the SAME ActorActionMessage::Special(SpecialActionSpec::Special(key)) the brain special path emits; Ranged writes the SAME ActorActionMessage::Ranged that the mature enemy-projectile consumer already handles. So specials and shots reuse existing plumbing with zero parallel hit code.
The old flat BodyMelee driver is gone; comments state "the ONLY melee path." project_moveset_melee_to_body_melee keeps BodyMelee alive purely as a read-model for the anim/HUD/telegraph consumers.
lib.rs still carries the OLD directional-attack helpers (AttackIntent, AttackSpec, resolve_attack_intent_from_view, attack_spec_from_view, attack_hitbox_from_view, ~lines 159-482). These are legacy-shaped but are now consumed by moveset::prefabs to synthesize MoveSpecs — a lowering step, not a live parallel path. Their own inline tests still exercise them directly. prefabs.rs also imports AttackIntent/AttackSpec, confirming they feed the moveset builder. This is the most obvious "loose thread" for the planning session: two attack-authoring shapes coexist (the AttackSpec struct and MoveSpec), with AttackSpec demoted to a prefab input.
Parallel paths still outside combat: the BOSS attack code is NOT on the moveset yet. ambition_actors/src/boss_encounter/ carries its own attack_geometry/ (590 LOC mod + 964 LOC sprite-metadata-derivation tests + frame/aabb) and ambition_characters/src/brain/boss_pattern/ (1,213 mod + 714 tick + validator + seeds + control_flow, ~4k LOC) — a separate boss-pattern schedule/DSL. The moveset code comments explicitly anticipate this: "the exact seam the boss's Special(key) profiles reuse once the boss folds onto the moveset." So melee+ranged+special are unified for player/enemy actors; boss patterns remain a second combat vocabulary pending fold-in.

Overlap with ambition_actors: combat owns several enemy-AI concerns that are conceptually actor-sim: targeting.rs (per-frame ActorTarget selection, imports ambition_characters::brain::Brain + actor::BodyHealth), slots.rs (anti-clump attack-slot arbitration), and hazards.rs/breakables.rs/falling_chest.rs (feature ticks). These run ON actor bodies through the shared ambition_characters vocabulary (combat does not depend on actors), but they blur the "combat = damage" seam. The victim-side hit ROUTING and the player charge/anim input stepper deliberately live UP in ambition_actors (stated in projectiles' lib doc) because they're woven with un-carved boss/world state.

Overlap with ambition_projectiles: minimal and one-directional. Combat's events.rs defines the attribution enum HitSource::{PlayerProjectile, EnemyProjectile} and comments reference "Fireball/Hadouken," but combat does not depend on the projectiles crate — projectiles depends on combat conceptually via the shared HitSource/damage vocabulary. The two meet only at the HitEvent channel.

3. projectiles: why separate from combat?
It exists as a combat-content/vocabulary crate, not a physics crate. Its lib doc (crates/ambition_projectiles/src/lib.rs) is explicit: the generic projectile PHYSICS primitive (spec/body/per-frame tick/world collision — ProjectileBody, ProjectileSpec, resolve_world_collision, WorldHitPolicy) lives DOWN in ambition_platformer_primitives::projectile and is merely re-exported at the crate root. This crate owns the two faction faces of one idea (player Fireball/Hadouken shot + enemy volley) sharing that one primitive, plus the named ProjectileKind content (kind.rs openly labels itself "named game content — the Fireball/Hadouken tier and the numbers behind it"), the charge/gesture state machine, ECS components, art descriptors, spawn pooling, and pure portal transit.

Why separate from combat rather than a combat::projectiles module: (a) it needs ambition_portal, ambition_world, ambition_input, and ambition_gameplay_trace as deps that combat does not want; (b) it is content (named kinds + tuning tables) whereas combat aims to be the content-free kit; (c) the motion-gesture buffer is reused beyond projectiles (lives in ambition_input).

Duplication risk: the actual firing/victim-routing is split — ambition_actors/src/projectile/systems.rs (832 LOC) holds the woven player-charge/anim input stepper and hit routing that the crate deliberately left in the sim heart. And ranged fire is ALSO expressible via the moveset (fire_move_from_ranged in combat + the MoveEventKind::Ranged bridge). So a shot can originate from two authoring surfaces (a moveset "ranged" move vs. the direct frame.fire path), both funneling into the same ActorActionMessage::Ranged consumer — convergent, not duplicated, but two front doors. No genuine second projectile-physics implementation was found.

4. portal vs portal_presentation split
Clean, and inverted from what the LOC labels suggest — presentation (6,505) is LARGER than core (5,246), dominated by the view-cone rendering: portal_presentation/src/view_cones.rs 1,145 + view_cones/debug.rs 1,078 + view_cones/geometry.rs 897 (+945 geometry tests) + mesh.rs. Those are feature-gated behind effect_view_cones.

portal core is clean of presentation. Deps are only bevy, serde, engine_core, platformer_primitives — no render/material/image types. The one grep hit for "render" is local variable names (into_render/out_render) in coordinate-frame math in placement.rs. view.rs is view-MAP/cone MATH (rotations, UVs), explicitly pub mod view "because renderers build capture cameras + cone UVs from it" — the geometry is core, the meshes/materials are presentation.
portal_presentation owns PortalClipMaterial, PortalVisual/PortalBodyPiece, camera continuity, gun visuals, and the view-cone meshes. It names no host crate (deps: bevy, engine_core, platformer_primitives, portal) and exposes host seams (PortalWorldFrame, PortalSceneBody, PortalGunArt, PortalAimHint) for the host to sync. The gun sprites are sequestered in a gun_visuals compat module, mirroring core's gun_* sequestration.
Verdict: a textbook clean core/presentation seam. Both halves keep the portal-gun as a clearly-labeled, removable compatibility layer, matching the north-star.

5. encounter: boss vs wave schema unification status
The status doc's premise is half-resolved. The TIMELINE has been unified; the PHASE-STATE machines have NOT.

What ambition_encounter (the crate) owns as the generic vocabulary:

EncounterSpec / EncounterWaveSpec / EncounterMobSpec / LockWallSpec — authored wave data.
EncounterPhase (Inactive → Starting{remaining} → Active{wave_index, remaining_mobs} → Cleared/Failed) + EncounterState reducer — the WAVE state machine.
EncounterParticipants/EncounterRole/Ownership, EncounterObjective, EncounterRegistry/SwitchActivation, EncounterMusicRequest, reward math.
The unified timeline — timeline.rs declares itself "the ONE timeline authority": EncounterScript/EncounterBeat/EncounterTrigger(Gate/MemberDied/AllMembersDead/Timer)/EncounterEffect(ForceKill/Banner/SetMusic/CommandMoveTo/DropHazard) + the generic EncounterScript::advance.
What ambition_actors/src/boss_encounter uses (a DIFFERENT phase schema):

Phase state comes from ambition_characters::boss_encounter: BossEncounterSpec, BossEncounterPhase (Dormant → Intro → Phase1 → Transition → Phase2 → Stagger → Enrage → Death), ActorPhaseState, PhaseTrigger/PhaseTriggerCondition(HpBelow/TimeInPhase/External). HP-fraction/threshold driven, entity-local.
It ADDS its own encounter-entity model: EncounterDef, EncounterProgress, MemberProgress, ReleaseOnDeath, CommandedMove, FallingHazard, plus catalog.rs (893), behavior.rs (667), systems.rs (587), attack_geometry/ (~2k incl. tests), sprites/.
Confirmation of the current unification boundary: boss_encounter/mod.rs re-exports the timeline FROM the generic crate (pub use ambition_encounter::{EncounterBeat, EncounterEffect, EncounterGate, EncounterScript, EncounterTrigger}), and encounter_script.rs consumes ambition_encounter::EncounterScript::advance and only supplies the actor-touching EXECUTION (the cut-rope fight: Gate("rope_cut") → CommandMoveTo + DropHazard → ForceKill). So the boss crate no longer duplicates the timeline — that was already merged. What it still duplicates conceptually is the coarse phase/lifecycle machine: BossEncounterPhase+ActorPhaseState vs EncounterPhase+EncounterState are two separate state machines with separate music/lock/HUD handling, plus the boss-only PhaseTrigger condition set overlapping the timeline's EncounterTrigger (both have Timer/External-gate/all-dead flavors). That phase-machine unification is the piece still outstanding.

6. interaction (298) and cutscene (399): too small, or clean seams?
Both are clean seams, not premature splits, but for different reasons.

interaction (298) is a pure-data leaf that many crates share: depended on by ambition_combat, ambition_encounter, ambition_actors, ambition_render, ambition_sim_view, ambition_sprite_sheet (6 consumers). It defines the PickupKind/Breakable*/Chest*/Interactable vocabulary that combat's DamageVolume/breakables and encounter's rewards both build on (e.g. EncounterSpec.reward: ambition_interaction::PickupKind). Small LOC but high fan-in and zero Bevy/system weight — exactly the kind of foundational vocabulary crate that SHOULD be tiny. Keep.
cutscene (399) is a single-file pure-data script type (CutsceneBeat/CutsceneScript), Bevy-imported but logic-free. Consumers: ambition_actors (runtime playback in ambition_actors/src/cutscene.rs, 197 LOC), ambition_render, ambition_touch_input, game/ambition_content. Its presentation deliberately lives in the host. It's a thin seam; the only question for planning is whether it should merge with ambition_dialog (they overlap on the Dialogue{speaker,text} beat) or ambition_encounter's timeline — currently cutscene is a THIRD scripting vocabulary alongside EncounterScript and Yarn dialogue.
7. persistence + load + load_presentation: coherent or scattered?
Two SEPARATE and coherent stories that don't overlap — the naming ("persistence" vs "load") is the only source of confusion.

ambition_persistence = SAVE/SETTINGS data + disk I/O. Owns the pure serde shapes (SandboxSaveData, PersistedEncounter/Switch/Quest, ability/quest flags in save_data.rs), the quest data+progression rules (quest/), all settings (settings/ — video/audio/gameplay, the bulk of the crate), and the thin Bevy disk shim (save.rs + settings/persistence.rs) with PersistenceSchedulePlugin (startup-load + autosave, omitted for headless/RL). Save shapes are I/O-free and reused across sandbox/tests/RL. This is the "save game" story and it is coherent and self-contained.
ambition_load = ASSET/scene LOADING coordination (activation barriers, streaming, prefetch, commit authorization). Completely unrelated to save-games; deps are only bevy. Headless/neutral. The "loading screen backend" story.
ambition_load_presentation = the loading-screen FRONTEND (delayed reveal, progress, activities, Continue). Consumes ambition_load + ambition_game_shell.
Verdict: NOT scattered. Save/settings is one coherent crate; asset-load coordination+presentation is a clean core/presentation pair. The only risk is nominal — a newcomer could conflate "persistence" (saves) with "load" (asset streaming); they are orthogonal. Note ambition_dialog and ambition_dev_tools BOTH depend on ambition_persistence (dialog for save-flag <<if boss_cleared>> bindings; dev_tools for developer.ron), which is the expected fan-in for a save-data leaf.

8. dev_tools: contents + does it leak into prod?
Contents (crates/ambition_dev_tools/src/): dev_tools/editable.rs (526 — reflected EditablePlayerStats/EditableMovementTuning/EditableAbilitySet + engine conversions + inspector-sync clusters), dev_tools/developer_tools.rs (316 — DeveloperTools toggle resource), dev_tools/profiles.rs (242 — movement/debug profile enums), persistence.rs (162 — developer.ron), profiling.rs (161 — startup profiler marks), lib.rs (131 — SandboxDevState + sync_live_player_dev_edits_system).

Does it leak into prod paths? Partially, by design, but state-only. The egui overlay UI is correctly kept out (stated to live in ambition_app). However the STATE + live-edit SYSTEMS are wired into the core runtime schedule, not gated behind a dev feature:

ambition_runtime/src/sim_core_resources.rs init_resources SandboxDevState, DeveloperTools, EditablePlayerStats, EditableMovementTuning, EditableAbilitySet, and StartupProfiler.
ambition_runtime/src/player_schedule.rs schedules sync_live_player_dev_edits_system as an ordering anchor in the player pipeline; progression_schedule.rs schedules sync_player_stats_with_inspector.
ambition_runtime/src/lib.rs re-exports SandboxDevState/EditableAbilitySet/EditableMovementTuning from its prelude.
So a shipped/headless build still carries the dev-tools resources and runs the live-edit sync each frame (the sync is written to be cheap and no-op when the inspector resource is unchanged, with an equality guard). Consumers: ambition_actors, ambition_runtime, ambition_sim_view, ambition_render. For the planning oracle this is the one crate whose "dev" identity bleeds into the always-on runtime — a candidate for a dev feature gate.

9. Ambition-game-specific identifiers found in these crates
The target crates are mostly clean of hard-coded named content; game names appear almost entirely in DOC COMMENTS and one named-content enum. Concrete findings:

Actual code (baked-in named content):

ambition_projectiles/src/kind.rs + visual_kind.rs + state.rs + diagnostics.rs: ProjectileKind::{Fireball, Hadouken, HadoukenSuper} and PlayerProjectileState.hadouken/hadouken_super — the crate self-identifies this as "named game content." String ids "hadouken", "hadouken_super". This is the single genuine game-specific vocabulary embedded in a target crate.
Doc-comment/example references only (no code coupling):

ambition_combat/src/lib.rs:64 — "tuned by feel against the mockingbird arena" (comment).
ambition_combat/src/components/actors.rs:674 — "the Smirking Behemoth victory NPC in crate::ambition_content::bosses::cut_rope" (comment pointing OUT to content).
ambition_combat/src/hazards.rs:97 — "Sanic run" (comment, refers to the ambition_demo_sanic demo).
ambition_combat/src/events.rs, on_hit.rs — "Fireball / Hadouken", "pirate volley" in comments.
ambition_encounter/src/state.rs — "goblin encounter" in a doc comment; ambition_persistence — "goblin_encounter", "goblin encounter" in comments only; save schema itself is string-id generic.
ambition_encounter/src/timeline.rs + boss_encounter/encounter_script.rs — the "cut-rope boss fight" and gate "rope_cut" are used as the WORKED EXAMPLE in docs; the effect vocabulary is generic (the actual cut-rope content lives in game/ambition_content).
Named bosses (for context — these live in ambition_actors/src/boss_encounter, NOT in the 10 target crates, and are DATA-resolved, no lib constructors): mockingbird, clockwork_warden (legacy alias gradient_sentinel), smirking_behemoth_boss, flying_spaghetti_monster_boss, gnu_ton_rider, trex_boss, mode_collapse_boss, exploding_gradient_boss, overflow_boss. These appear in profile.rs only inside #[cfg(test)] assertions; the shipping code resolves every boss from boss_encounters/<id>.ron + boss_profiles.ron with no hard-coded roster (ADR 0017). Demo protagonists mary_o, sanic, pocket live in game/ crates, not the targets.

Bottom line for the oracle: among the 10 targets, only ambition_projectiles hard-codes named content (the Fireball/Hadouken enum), and it flags itself as such — a candidate to push down into ambition_content once the ControlFrame→actor-intent extraction lands (noted in its own doc). All other named-content references are doc comments or test fixtures; the runtime vocabulary is generic and data-resolved, consistent with the "add a content crate without editing core" north-star. The two remaining cross-core seams flagged for the planning session are (a) boss attack patterns/phase-state not yet folded onto the moveset+encounter vocabularies (§2, §5), and (b) dev-tools state living in the always-on runtime (§8).

Midlayer report is in. Highlights: the moveset really is the single combat vocabulary now for player/enemy melee+ranged+special — boss patterns remain the one unfolded second vocabulary. ProjectileKind::{Fireball, Hadouken} is confirmed baked-in named content inside crates/ (self-flagged in its own docs). Also notable: the repo now has three scripting vocabularies (CutsceneScript, EncounterScript, Yarn dialogue), and dev-tools state/systems run unconditionally in the production runtime schedule. Four agents still working.

I have a comprehensive picture. Here is the full recon report.

ambition_actors — Crate-Decomposition Recon
Crate total: 68,709 LOC across 31 top-level modules. 28 workspace path-deps. Cargo self-description already claims "content-free simulation systems … and compatibility facades" — but the reality is a hub crate that owns the entire actor/boss/ability simulation and still bakes in named content in a handful of places.

The crate has already undergone heavy extraction (combat→ambition_combat, projectiles→ambition_projectiles, portal→ambition_portal, render→ambition_render, time→ambition_time, most content→game/ambition_content). Many top-level modules are now thin facades re-exporting an external crate plus a few sim-woven steppers that "can't leave yet." The residual mass is concentrated in one place.

1. Module map (LOC + what each actually does)
Read from mod.rs/key files, not names.

Module	LOC	What it actually does
features/	27,266	Misnamed — NOT feature toggles. The enemy/NPC/boss ECS actor simulation + shared room props (pickups, chests, breakables, switches, hazards, mounts). This is the crate's real payload. features/ecs/ alone is 21,295.
boss_encounter/	6,173	Boss-fight coordinator: BossCatalog/registry (App-local, provider-contributed), behavior/profile/spec data schemas, attack-geometry hitbox math, encounter-entity + scripted beats, boss sprite anim. Bidirectionally tangled with features/ecs/boss_clusters.
avatar/	4,316	The home avatar (player control box): identity bundle, respawn/blink camera safety, starting-character selection, emitted trail (1,049 LOC), home-body integration tick, moving-platform advance.
abilities/	4,316	"Ambition's player ability/weapon kit": traversal (blink/dive/grapple/possession/mark_recall), ranged (beam/meteor/shockwave/vortex/volley/bomb/sentry), thrown (gravity_grenade, puppy_slug_gun). Mostly pure fns invoked from combat/item/projectile code.
character_sprites/	2,777	Character-catalog-aware sprite loading, animation state, body-collision/hitbox joins. Has a hardcoded content-id→named-slot map (see §5).
affordances/	2,121	"What would each button do right now?" — per-frame verb table for HUD labeling (input × body × world → verb). Near-leaf (only avatar imports it).
items/	1,874	Facade over ambition_items + actor-side pickup/throw/world-item steppers that mutate bodies/gravity/portals/abilities.
projectile/	1,776	Thin facade over ambition_projectiles + the player-faction sim steppers (step_projectiles, charge_projectile_input) that weave world/actor/boss.
encounter/	1,704	Facade over ambition_encounter (generic enemy-wave/arena-lockdown) + LDtk/ECS/save/banner adapters. Distinct from boss_encounter.
dev/	1,645	Dev tooling (facade over ambition_dev_tools) + flight-recorder trace sampling. 0 reverse-deps (leaf).
world/	1,612	Room graph/spawning, code-first room builder, LDtk hot-reload loader, Avian2D physics adapter, moving platforms, ECS world overlay. Note rooms/ldtk_world re-exported at crate root — heavily used (features→rooms 88).
persistence/	1,331	Save-data shapes + settings facade over ambition_persistence/ambition_settings_menu.
audio/	1,284	Kira runtime, web-unlock, SFX-bank loader (facade over ambition_audio). Near-leaf (only music imports).
time/	1,196	Game-specific time-control POLICY (Regime/requester table/feel smoother) on top of ambition_time vocabulary.
session/	1,173	Session lifecycle: startup setup, reset/respawn, RON data manifests. The assembly-side glue ambition_app calls.
control/	989	The local control seam: device frame → slot → body carrying that slot's player brain. LocalPlayer, PlayerInputFrame, slot gestures.
body_mode/	854	Crouch/climb/morph-ball/stand-up posture driver (capability-gated). Near-leaf (only avatar).
menu/	812	Menu content model + settings IR + Map tab (over ambition_menu). 0 reverse-deps (leaf).
enemy_projectile/	687	Thin facade over ambition_projectiles::enemy + one effect-apply stepper.
assets/	661	Asset-catalog wiring, character/sprite asset registration.
schedule/	584	Schedule-label vocabulary (SandboxSet re-exported from primitives) + device→ControlFrame populate systems. NOT the app assembly (that's in ambition_app).
gravity/	543	Gravity/anti-grav field application over bodies. 0 reverse-deps (leaf).
dialog/	515	Game-specific Yarn bindings + GameMode sync over ambition_dialog. 0 reverse-deps (leaf).
music/	458	Music intent (encounter/room/radio → MusicIntent) over ambition_audio. Near-leaf (only audio, bidirectional).
character_roster/	357	Character-catalog loading (test include from ambition_content).
time.rs/actor.rs/cutscene.rs/shrine/ability_cooldown/platformer_runtime/host/quest/effects/config/physics	small	Root-level glue + facades. actor.rs (172) is the keystone actor-vocabulary re-export (see §2). platformer_runtime/physics/effects/quest are near-empty facades.
Root lib.rs (305) owns ActorDiedMessage, DeathCause, SandboxSimState, SafePositionContext, and the remember_safe_player_position safety classifier.

features/ internal breakdown (the 27k elephant)
features/ecs/ 21,295 — the ECS backbone. Largest files: actors/update.rs (1,519), spawn_actors.rs (1,470), mount/ (1,990), damage/+damage_apply (3,067 total), actor_clusters.rs (815), perception.rs (501), bosses/ (1,791), boss_clusters.rs (429), brain_builders.rs (436), aggression, interact, pickups, encounter_rewards, save_sync, spawn_static.rs (573).
features/enemies/ 2,504 — archetype-data-driven enemy integration (grounded+aerial) onto the shared movement spine; CharacterRoster.
features/bosses/ 847 — boss special-spec resolver + tuning.
features/ files: actor_movement_tests.rs (854), mod.rs (509, the schedule root — 4 schedule plugins), movement_fx.rs (400), npcs.rs (279), bosses.rs (226), ledge_grab.rs, banter.rs, swim/.
2. Major subsystems & internal coupling
Forward use crate::X reference counts (parenthesized) map the internal graph:

The hubs (high fan-in, hard to move):

features — fan-in from 17 modules (abilities 60, boss_encounter 53, items 30, enemy_projectile 25, encounter 24, avatar 19, projectile 16, session 13, gravity 9…). It also has high fan-out (combat 165, boss_encounter 96, rooms 88, actor 70). This is the classic god-hub: everything depends on it and it depends on everything.
actor (actor.rs) — fan-in from 19 modules (features 70, avatar 42, character_sprites 30, abilities 29, affordances 25, control 14, body_mode 14, gravity 12, projectile 12, audio 10, session 9). But it is a pure re-export facade of ambition_engine_core / ambition_characters / ambition_platformer_primitives / ambition_combat body types. Cheap to satisfy — it's a vocabulary keystone, not a real code tangle.
combat (= ambition_combat, aliased at crate root) — fan-in 11 (features 165!). Already an external crate; the direction is clean.
control (fan-in 10), rooms/world (10), schedule (9), boss_encounter (9), avatar (9), time (5), persistence (5).
The leaves (0 reverse-deps — clean to extract): dev, menu, gravity, dialog.

Near-leaves (1–2 importers): affordances←avatar, body_mode←avatar, audio↔music (bidirectional, small), shrine, enemy_projectile←{features,abilities}, assets, character_sprites.

The key tangles:

features ↔ boss_encounter (bidirectional). features→boss_encounter: BossCatalog (14), sprites::BossAnim* (12), BossEncounterPhase, behavior::BossBehaviorProfile, attack_geometry, plus 37 test_boss_catalog (test). boss_encounter→features: ecs::boss_clusters::{BossEncounter, BossConfig, boss_is_cleared} (~15, incl. test_support). Root cause: boss HP/phase cluster ECS components live in features/ecs/boss_clusters.rs, but the boss catalog/behavior/sprite data lives in boss_encounter/. Neither can leave without the other.
abilities → features (60). Abilities reach deep into the actor sim (spawn_runtime_minion, ActorFaction, HeldItem, can_damage matrix). Blocks abilities from becoming a leaf crate.
items ↔ abilities ↔ features. items→features(30)+abilities(17); abilities→items(16). Three-way weave through pickup/throw/held-item.
avatar ↔ features ↔ control. avatar→{actor 42, features 18, combat 14, control 5}; features→avatar 19; control→{actor 14, avatar 6}. The home-player is welded into the actor sim (avatar's advance_moving_platforms, apply_player_heal_requests, integrate_sim_bodies treat home body and actors uniformly — deliberate "player is an actor" design).
enemy_projectile ↔ projectile (16 each way) and both → features — projectile steppers query bosses/breakables/actors.
3. External workspace deps per subsystem (which subsystem drags in the heaviest deps)
Subsystem	External ambition_* crates pulled	Weight
features	characters, dev_tools, engine_core, entity_catalog, interaction, persistence, platformer_primitives, sfx, sprite_sheet, time, vfx, world (+ combat/encounter/projectiles via re-export)	Heaviest — 12+ direct, transitively touches nearly all 28
boss_encounter	characters, encounter, engine_core, persistence, platformer_primitives, sprite_sheet, time	7
items	characters, combat, engine_core, input, items, persistence, platformer_primitives, portal	8 (drags portal)
world	characters, dev_tools, engine_core, ldtk_map, platformer_primitives, sfx, time, vfx, world	9 (drags ldtk/avian)
encounter	asset_manager, encounter, engine_core, entity_catalog, interaction, persistence, platformer_primitives, world	8
session	audio, dev_tools, engine_core, persistence, platformer_primitives, world	6
character_sprites	asset_manager, characters, engine_core, persistence, sprite_sheet	5
audio	asset_manager, audio, engine_core, platformer_primitives, sfx	5 (drags kira)
projectile	characters, engine_core, input, projectiles, sfx, vfx	6
avatar	characters, engine_core, input, platformer_primitives, world	5
abilities	characters, engine_core, platformer_primitives	3 (clean externals; its weight is the internal features(60) tie)
dialog	dialog, engine_core, persistence	3
menu	input, menu, platformer_primitives	3
persistence	dev_tools, persistence, settings_menu	3
dev	engine_core, gameplay_trace, input, platformer_primitives	4
body_mode / gravity / shrine / character_roster / platformer_runtime	characters/engine_core/platformer_primitives only	1–3 (lightest)
Takeaway: features is the sole subsystem that touches entity_catalog, interaction, sfx, sprite_sheet, vfx, and dev_tools simultaneously. world and items are the ones that drag heavy optional backends (ldtk/avian, portal). The genuinely light subsystems (body_mode, gravity, abilities-externals, shrine) are the ones whose extraction wouldn't move the dep graph much on their own.

4. Candidate split seams
Ranked by cleanliness of the resulting dependency direction.

A. Leaf extractions — cleanest, do first (0 reverse-deps)
Candidate	LOC	Depends on	Depended on by	Tangle blocking split
ambition_dev_runtime (dev/)	~1,645	avatar, world, features, schedule, trace	nothing	Reaches into features/avatar internals for the debug subject + trace sampling. Needs a read-only observation surface (partly exists via ambition_sim_view).
ambition_menu_content (menu/)	~812	ambition_menu, rooms, ldtk_world	nothing	Only touches rooms/ldtk_world for the Map tab — narrow. Nearly free.
ambition_gravity (gravity/)	~543	physics, actor, features(9), schedule	nothing	features(9) = queries actor bodies; would need the body cluster query from a shared crate (already in engine_core).
fold dialog/ into ambition_dialog	~515	ambition_dialog, items, actor	nothing	Yarn bindings reference items::shop + save data — game-specific vocabulary; better as content, not a reusable crate.
B. Audio/music cluster
Candidate	LOC	Notes
ambition_audio_runtime (audio/ + music/)	~1,742	audio↔music bidirectional but self-contained; depends on ambition_audio. Only external tie-in is session/encounter/rooms for music intent. Clean-ish behind the audio feature.
C. The boss carve (medium — one tangle to cut)
Candidate	LOC	Depends on	Depended on by	Tangle blocking split
ambition_boss (boss_encounter/ + features/ecs/boss_clusters.rs + features/ecs/bosses/ + features/bosses/)	~9,600	characters, encounter, engine_core, platformer_primitives, sprite_sheet, combat	features (via boss_is_cleared, damage routing)	The single blocker is that boss cluster ECS components (boss_clusters.rs, 429) live in features/ecs while catalog/behavior/sprites live in boss_encounter. Move the cluster components down with the boss code and the bidirectional edge collapses to one-way (boss depends on the actor spine, not vice-versa). Damage routing (features/ecs/damage/boss_hit.rs) would need a HitEvent seam (combat already provides it).
D. The actor-sim core (the real prize, hardest)
Candidate	LOC	Notes
ambition_actor_sim (features/ecs + features/enemies + features/mod schedule)	~24,000	This is the north-star "core" — the content-free enemy/NPC/boss/prop simulation. For "add a content crate without editing core" to be true, this must be the crate content plugs into. Blockers: (1) it imports avatar/control for the home body (the "player is an actor" weld — needs the home body to be spawned by content and integrated through a generic body query, which is partly done via integrate_sim_bodies); (2) it imports boss_encounter (resolve via C); (3) it imports rooms/world overlay (needs ambition_world runtime to own the overlay — noted as pending "W3" in comments); (4) abilities imports it (60) rather than the reverse, so abilities must sit above it.
E. Player cluster
Candidate	LOC	Notes
ambition_player (avatar/ + control/ + affordances/ + body_mode/)	~8,280	Cohesive (control→avatar→affordances all cluster). Depends on actor vocab + features sim. Blocker: bidirectional with features (features→avatar 19 for home-body integration). Would sit above ambition_actor_sim, consuming it.
F. Already-thin facades (collapse rather than extract)
projectile/ (1,776) + enemy_projectile/ (687) + items/ glue + platformer_runtime/physics/effects are mostly re-exports of already-extracted crates plus sim steppers. These should collapse into ambition_actor_sim (D) rather than become crates — they're the "steppers that can't leave until the actor domain carves" per their own module docs.

Recommended order: B (audio) + A leaves → C (boss carve, cuts the worst tangle) → D (actor_sim core) → E (player) → F (fold facades into D).

5. Ambition-game-specific content that should migrate to game/ambition_content
The crate is mostly disciplined (most content already left; named RONs are pulled via include_str! only in #[cfg(test)] fixtures). Concrete residual leaks:

File	LOC-region	What leaks
boss_encounter/catalog.rs	~640–670 (default_boss_sprite_filenames, test_boss_sprite_filenames)	Non-test hardcoded named-boss sprite paths: gnu_ton, giant_gnu, gnu_ton_rider, flying_spaghetti_monster_boss, trex_boss. Should be provider-contributed data, not baked in the machinery crate.
boss_encounter/catalog.rs	676–755	include_str! of 9 named boss RONs (clockwork_warden, mockingbird, gnu_ton_rider, smirking_behemoth, flying_spaghetti_monster, trex, mode_collapse, exploding_gradient, overflow) + boss_profiles.ron/boss_sheets.ron, plus special_anim_keys map (overfit_volley, eye_beam, saddle_point, gradient_cascade…). All #[cfg(test)] but reach across the tree into ../../../../game/ambition_content/. Move the fixtures into ambition_content's own test suite.
character_sprites/assets.rs	314–343	Non-test match cid.as_str() on content character IDs "player", "robot", "goblin", "sandbag" mapping to named struct slots. Hardcodes the Ambition roster into loading machinery.
abilities/thrown/puppy_slug_gun.rs	whole file (189)	The "puppy-slug gun" is explicitly Jon's designed item content — summons "puppy_slug" archetype allies. Ambition-specific ability content.
abilities/ (whole tree, 4,316)	—	Self-described "Ambition's player ability/weapon kit" (meteor, vortex, sentry, gravity_grenade, shockwave, volley…). Straddles machinery/content; the named set is Ambition's. Candidate for ambition_content (or an ambition_abilities content crate) once the features(60) tie is cut.
features/enemies/mod.rs	297–335, 936	#[cfg(test)] COMBAT_BRAIN_KEYS/ALL_BRAIN_KEYS const arrays (puppy_slug, pirate_raider, burning_flying_shark, gradient_seeker, pirate_shark_rider, cellular_automaton_fighter, exploding_mite…) + test_roster() include_str! of character_archetypes.ron. Test-only, but names the entire Ambition roster.
character_roster.rs	12	include_str!("…/game/ambition_content/assets/data/character_catalog.ron") (test).
encounter/loading.rs	151	#[cfg(test)] "goblin_encounter" fixture.
features/ecs/effect_bus.rs	94–96	#[cfg(test)] "goblin_encounter" in a test.
boss_encounter/{systems,behavior,profile}.rs	scattered	"clockwork_warden"/"gradient_sentinel" appear only in #[cfg(test)] — acceptable, but the test fixtures should live with the content.
music/mod.rs doc + music/intent.rs	—	Doc mentions "authored goblin cue data"; the actual cue lines are content (registered from ambition_content). Verify no baked lines remain.
Net: the only production-code content leaks are (a) boss_encounter/catalog.rs default sprite-filename tables, and (b) character_sprites/assets.rs named content-id match arms. Everything else is test-fixture coupling to the game/ambition_content/ path. features/banter.rs is correctly generic (registry type here, lines in content).

6. Overlap / duplication with sibling crates
Investigated ambition_combat, ambition_encounter, ambition_projectiles, ambition_platformer_primitives. Verdict: no true logic duplication — the crate is a thin ECS-woven consumer of each. The overlaps are all "facade + sim-stepper" seams, deliberate and documented:

ambition_combat — Aliased as crate::combat at the crate root (pub use ambition_combat as combat). features/ecs/mod.rs re-exports banner, breakables, falling_chest, hazards, held_items, hitbox, targeting, variation straight from it. The generic combat mechanics (HitEvent vocabulary, hitbox, targeting matrix, hazard runtime) are already in ambition_combat; features/ecs/damage/ (2,312 LOC) is the actor-side application of those events (routes hits to actor/boss/breakable clusters, drops loot, respawns). Not duplication — but damage/ is large and could be re-examined for logic that belongs down in combat.
ambition_encounter — crate::encounter is an explicit facade ("Authored data, registry resources, event vocabulary, reward math, and the headless state machine live in ambition_encounter"). Local keeps only LDtk/ECS/save/banner adapters. boss_encounter reuses ambition_encounter's timeline (EncounterBeat/EncounterScript) rather than defining its own. Clean.
ambition_projectiles — projectile/mod.rs and enemy_projectile/mod.rs are pub use ambition_projectiles::* facades; the model (shot vocabulary, components, spawn pool, portal transit) is fully in the crate. Only the world/actor/boss-woven steppers remain here (they consume the model — legal direction). Zero duplication.
ambition_platformer_primitives — schedule (SandboxSet/CombatSet), markers (ControlledSubject/PlayerEntity), lifecycle (FeatureSimEntity/SessionScopedEntity), frame_env, camera_ease all come from it. crate::platformer_runtime and crate::actor are re-export facades over it. No duplication — this is the foundation the crate is built on.
One thing to flag: crate::encounter vs crate::boss_encounter are two separate encounter systems in this same crate (generic enemy-wave lockdown vs. scripted single-boss phases). That's intentional per the docs, but if a future ambition_boss crate (seam C) forms, the boss-side should consume ambition_encounter's timeline directly and the crate::encounter facade's boss-adjacent bits (rebuild_encounter_switch_index, lock walls) should be examined for whether they belong to the generic encounter crate.

One-paragraph answer to the north-star
Today: no — adding a platformer means editing features (the 27k god-hub every subsystem both feeds and consumes) and boss_encounter (bidirectionally welded to features/ecs/boss_clusters), plus touching the two production content leaks (boss_encounter/catalog.rs sprite tables, character_sprites/assets.rs id match). The path to "yes" is: cut the boss↔features tangle by moving boss cluster components down with the boss code (seam C), lift abilities/items/avatar above a carved ambition_actor_sim (seams D/E), and relocate the two hardcoded content tables to ambition_content. The leaf extractions (dev, menu, gravity, dialog, audio) are free wins that shrink the crate ~5k LOC without touching the hard tangles.

I have gathered comprehensive data across all eight target crates. Here is the full report.

Crate-decomposition recon: data/content-adjacent cluster
Orienting fact that frames everything below: the workspace is already split into crates/ (intended engine) and game/ (content — holds ambition_content plus playable demos ambition_demo_mary_o [Mario], ambition_demo_sanic [Sonic], ambition_demo_pocket, ambition_menu_kaleidoscope, and the ambition_app wiring binary). So the north-star oracle is already partly enacted: the demos ARE "another platformer built by adding a content crate." The question for each crates/ member is whether it holds its content-side of that line or has leaked game data across it.

1. ambition_characters (23,351 LOC)
Module map (LOC includes tests)
Top module	LOC	Concern
brain/	17,381	The universal brain (player/NPC/enemy/boss AI dispatch). The whole crate's mass.
actor/	3,401	ActorControl/ActorControlFrame contract, AI intent, pose/faction/body/worn vocabulary, and the character-catalog schema+registry+validator (actor/character_catalog/*).
perception/	1,233	WorldView/WorldMemory — the "world-out" perception port.
equipment.rs	655	A3 equipment→params fold (numeric modifiers, grants, on-hit armor).
boss_encounter*	654	Boss phase progression state machine + spec schema.
lib.rs	27	Re-exports.
brain/ breaks down as: boss_pattern/ 5,188 · smash/ 4,621 · state_machine/ 2,185 · fighter/ 1,950 · action_set/ 1,541 · mod+player+snapshot 1,896.

Generated vs hand-written vs data
Zero generated code. Zero embedded data. No build.rs, no .ron/.json files in the crate (only Cargo.toml, MODULES.md, brain/README.md).
All 23,351 LOC is hand-written. Test code is heavy: the */tests.rs files alone total 8,014 LOC (~34%), plus 53 additional inline #[cfg(test)] modules — realistically ~40% of the crate is tests. So roughly ~14k LOC of hand-written machinery logic.
Is it misplaced game content?
No. This is machinery, correctly placed. It self-describes as "the content-free vocabulary that makes an entity controllable" and states named content lives in ambition_content (crates/ambition_characters/src/lib.rs:19-21). A grep for the game's named cast (bosses/heroes) in src/ returns nothing but test placeholders ("Robot", "Sniper", "Alpha/Beta"). The Smash brain module (brain/smash/mod.rs, "SSBB Subspace-Emissary feel") is a generic brawler-AI template, not Super-Smash content. The character catalog is schema only: CharacterCatalog parses a RON string the host passes in (CharacterCatalogPlugin { catalog_ron: &'static str }), and the actual roster tests were explicitly moved out to ambition_actors::character_roster (comment at character_catalog/mod.rs:229-233).

Generic machinery mixed in with data
Machinery present (all reusable): the equipment system (equipment.rs — pure resolved_param fold), the ability/action sets (brain/action_set/), the moveset params (delegated to ambition_entity_catalog::EffectRef/ParamValue), the boss-pattern move DSL + validator (brain/boss_pattern/), the character-catalog schema/validator/resolver.

The only borderline data-shaped items, both defensibly kept as machinery:

brain/boss_pattern/seeds.rs — a SeedArchetype enum of 9 attack archetypes "extracted from the shipped roster." The doc explicitly says "the catalog itself is content (game/ambition_content/assets/data/boss_seeds.ron)"; this file is only the vocabulary + measured envelope bands.
brain/fighter/scenarios.rs — 8 fixture WorldView situations for the fighter-brain test rig (ledge trap, juggle escape, etc.). A generic test fixture, not shipped content.
Proposed split line
The crate is already on the correct (machinery) side of the engine/content line — no split needed to satisfy the oracle. If a decomposition is wanted for size rather than content-purity, the natural internal seams are: (a) lift brain/smash/ (4,621) and brain/boss_pattern/ (5,188) into sibling ambition_brain_* crates over the actor control contract, and (b) equipment.rs is nearly free-standing (deps only on action_set) and could become its own tiny crate. But none of these are content violations. The dependency ambition_entity_catalog is deliberately a downward leaf edge (the crate's own comment explains it).

2. ambition_sprite_sheet (5,989 LOC) — NOT content-free despite its Cargo description
Cargo.toml calls it "Reusable, content-free sprite-sheet metadata vocabulary." That is true of about half the crate; the other half is Ambition-specific gameplay→sprite binding. The crate is two things fused:

(A) The clean, content-free metadata pipeline — lib.rs (SheetRecord/AnimationMetrics/PixelRect/FrameRect), frames.rs, character/sheets/*, character/anim.rs, pack.rs/sprite_packs.rs (the ultrapack catalog). This is the schema + SheetRegistry resource. Legitimately reusable.

(B) Ambition gameplay-binding that leaked in:

src/game_assets/entity_sprite.rs — an EntitySprite enum that is a hardcoded Ambition sprite catalog: ChestClosed, PickupHealth, PickupCurrency, PickupAbility, BossCore, SandbagDummy, PogoOrb, ReboundPad, SwitchArmed, SoftBlinkWall/HardBlinkWall, DoorZone, ProjectileEnergy ("Fireball + Hadouken share the same sprite"), LockWallTile, etc. — with baked relative PNG paths.
src/boss.rs (1,269 LOC) — a BossAnim enum (FloorSlam/SideSweep/SpikeHalo/DashEcho) and boss-sheet spec, with doc comments naming specific Ambition bosses ("gradient sentinel ships 7 rows; the mockingbird ships 6").
Why it depends on combat / interaction / persistence / world (actual use sites)
The three "gameplay" deps come entirely from one file, src/game_assets/resolvers.rs, which reverse-maps gameplay entity-specs → EntitySprite:

ambition_combat::events::FeatureVisualKind (resolvers.rs:8)
ambition_world::rooms::{LoadingZoneActivation, HazardVolumeSpec, PickupSpec, ChestSpec, BreakableSpec, InteractableSpec, InteractionKindSpec, PickupKindSpec} (resolvers.rs:9,51-145) + RoomMetadata in game_assets/mod.rs:39
ambition_interaction::{Chest, Interactable, InteractionKind, BreakableState} (resolvers.rs:71,100,104,153-158)
ambition_entity_catalog::placements::{CharacterBrain, BossBrain} (resolvers.rs:113-136, incl. a character_brain_is_sandbag special-case)
The ambition_persistence dep is different in character: it is settings::{VisualQualityBudget, TextureResolutionScale} used in boss.rs:17,927 and game_assets/{mod,entity_sprite}.rs to pick a resolution tier. That one is a defensible asset-pipeline concern (quality scaling), unlike the combat/world/interaction/entity_catalog deps, which are pure gameplay→art binding that belongs above this crate.

Verdict: it is a sprite-metadata pipeline with a gameplay-presentation resolver bolted on. The clean split is: keep (A) as the reusable ambition_sprite_sheet; move game_assets/resolvers.rs + the EntitySprite roster + boss.rs's named-boss knowledge out to the content/presentation layer (ambition_actors or ambition_content). That single move drops the ambition_combat, ambition_interaction, ambition_world, and ambition_entity_catalog edges.

The build.rs
build.rs embeds sprite sheets into the binary so Android/wasm builds (where the dev-machine asset path doesn't exist) don't get an empty index and panic on first load_spec("goblin"). It:

Reaches cross-crate into ../ambition_actors/assets/sprites{,_0_5x,_0_25x,_potato}/ (note: not its own assets), scans root + one subdir level for *_spritesheet.ron, and emits OUT_DIR/baked_sheet_rons.rs = pub static BAKED_SHEET_RONS: &[(&str, &str)] via include_str! (keyed root or root.<tier> for quality variants).
bake_pack_catalogs() embeds each assets/sprite_packs/<tier>/ultrapack.json as BAKED_PACK_CATALOGS: &[(&str,&str)].
So the crate physically bakes the game's actual sprite data (goblin, robot, bosses) from a sibling crate's asset tree at compile time — another way content is entangled here.

3. audio / sfx / sfx_bank triple
Crate	LOC	Role	Deps
ambition_sfx_bank	539	Pure binary .sfxbank format reader (header/entry-table/payloads/names, little-endian). "Pure data; no audio or Bevy deps." Leaf.	none
ambition_sfx	788	SFX runtime contract: SfxId (hashed-string id), SfxClip, SfxProvider trait + Bank/Filesystem/Silent/Layered providers, plus ids.rs (named SfxId constants) and message.rs.	ambition_sfx_bank
ambition_audio	5,381	Authored-audio stack: catalog/registry (catalog.rs, spec.rs), Kira playback (library.rs, render.rs, output.rs, bank_asset.rs, feature-gated kira), and the adaptive music director (music/director/{adaptive,simple}.rs, music/{catalog,channels}.rs), plus selection.rs, web_unlock.rs.	ambition_sfx
Are three crates justified?
Two of the three splits are well-justified; the sfx_bank↔sfx split is the one worth questioning.

ambition_audio separate: yes, clearly. It carries the heavy bevy_kira_audio (optional kira feature) + the whole music director. Keeping it separate lets headless/RL builds compile ambition_sfx (for SfxId event enqueuing) without any audio backend — ambition_actors' Cargo.toml explicitly relies on this ("ambition_sfx stays in the dep graph regardless of the audio feature… no audio backend deps").
ambition_sfx_bank separate: defensible. Its whole reason to exist is to be Bevy-free and audio-free so the Python packer's consumers and tooling can read banks (tools/ambition_sfx_pack/pack.py). Merging it into ambition_sfx would pull the SfxId/SfxProvider/Bevy-adjacent surface onto pure format-reading code. It's small (539) and the boundary is real, so the split is reasonable, though a merge into ambition_sfx behind a feature would also be tenable — this is the one merge candidate.
Recommendation: keep all three; if pressure to reduce crate count exists, fold sfx_bank into sfx (feature-gated) — but not audio.

4. entity_catalog vs characters vs items — the "catalog" concepts
Three distinct "Catalog" types, and they are layered, not duplicated:

ambition_entity_catalog::EntityCatalogDoc (lib.rs:737) — the Tier-0, headless, content-free entity-contract + moveset schema (EntityDef bundles, MoveSpec timelines with windows/volumes/events on the owner's proper time, EffectRef/ParamValue, and placements.rs's closed authoring enums DamageTeam/CharacterBrain/BossBrain/HazardSpec). No Bevy, no deps at all. This is the shared vocabulary the other two build on.
ambition_characters::CharacterCatalog (character_catalog/mod.rs:47) — the actor-roster schema: brain presets, action-set presets, barks, momentum/axis tuning per character-id. Schema + validator + registry only; roster data passed in as RON by the host. Depends on ambition_entity_catalog (downward).
ambition_items::ItemCatalog (lib.rs:129) — see §below; this one is a content-override wrapper over a hardcoded default.
Relationship: entity_catalog = the primitives; characters = who-an-actor-becomes over those primitives; items = a pickup roster. No real duplication of machinery — the three "Catalog" names denote three different tiers of the same data-driven pipeline. The one genuine overlap worth flagging is that entity_catalog::placements::CharacterBrain/BossBrain and characters' brain presets both name "who a spawned actor becomes"; placements.rs:34 documents that its DamageTeam is deliberately distinct from characters::ActorFaction, so this is a managed, intentional separation rather than accidental dup.

5. ambition_ldtk_map (5,378 LOC) — clean adapter
Description: "LDtk backend adapter for Ambition authored world IR." It holds the line well.

The core is a generic converter registry: conversion/entity_converters.rs has one convert_* per generic platformer LDtk entity type (PlayerStart, Solid, OneWayPlatform, BlinkWall, HazardBlock, PogoOrb, ReboundPad, BreakablePlatform, LoadingZone, EncounterTrigger, LockWall) → RoomEmission. Plus intgrid.rs, surfaces.rs, fields.rs, project.rs, bevy_runtime/*, hot_reload.rs. None of these encode named Ambition content.
No hardcoded world paths in production code. manifest.rs:91-110 requires the game to call install_world_manifest(); production world_manifest() panics if content hasn't installed one ("the game's content must call install_world_manifest()"). This is exactly the additive-content seam the oracle wants.
Gameplay does not leak in structurally — it depends on entity_catalog (placement schema), world (room IR), engine_core, platformer_primitives, asset_manager. All are contract/IR layers, not gameplay systems.
Ambition-specific content is test-only: every named-content hit is behind #[cfg(test)]:

manifest.rs:113-121 — a test_fixture_manifest() that reaches cross-crate into ../../game/ambition_content/assets/worlds (the "explicit cross-crate fixture pattern," deliberate).
conversion/mod.rs:858-889 — test the_sanic_area_round_trips_as_a_ron_room loads sanic_sandbox via LdtkProject::load_default_for_dev().
fields/tests.rs:102,133 — flag:seen_alice, parse_enemy_brain("Goblin").
Verdict: a clean format adapter. The only cleanup is that its tests bind to the real content world; the production surface is content-neutral.

6. Ambition-game-specific identifiers found in crates/ (engine/content violations)
Ordered by severity.

Hard violations (named content in production/non-test code)
crates/ambition_items/ — the worst offender; the entire crate is game content.

lib.rs:1 self-describes as "Canonical finite item catalog — the game's complete set of pickup items… 24 is the finite number of distinct pickup items in this game."
lib.rs:67-96 — a closed #[repr(usize)] enum Item with 24 hardcoded named variants: PortalGun, Axe, Javelin, GunSword, PuppySlugGun, Fireball, Blink, Fly, Grapple, MorphBall, MarkRecall, BubbleShield, HealthCell, ManaCell, SpareBattery, DataChip, Bomb, GoldPouch, MapFragment, SealedNote, FieldSurvey, GateKey, DebugLens, ReservedSlot.
lib.rs:168+ — built-in ITEM_META table with display strings ("Portal Gun", …). There is a content-override seam (install_item_catalog/ITEM_CATALOG_OVERRIDE, lib.rs:121-165) but it only re-skins per-slot metadata; the item set, count (24), and discriminant order are baked into the type system. Directly fails the oracle: you cannot give another platformer a different item roster without editing this enum Item in a crates/ engine crate.
crates/ambition_sfx/src/ids.rs — named Ambition cast baked into an engine crate.
The DIALOGUE_BLIP_* constants name the game's specific speaker roster: ALICE, BOB, ARCHITECT, CREATOR, DARK_LORD, GATE_JANITOR, HAND_SAINT, KERNEL_GUIDE, MANIFEST_CLERK, MERCHANT_PROTOTYPE, MILITARY_GENERAL, NEWS_BOARD, NINJA, OILER, PIRATE, PULSE_VOYAGER, ROBOT, TECH_BRO, VAULT_KEEPER, WEIRD_HERMIT, GOBLIN_CHIEFTAIN (plus generics). These are SfxId::from_static("dialogue.blip.<name>") convenience constants — ergonomic, not load-bearing (the bank stores everything regardless), but they are Ambition characters living in an engine crate. Movement/combat/hazard constants above them (PLAYER_JUMP, HAZARD_LAVA_SPLASH, …) are generic-platformer vocabulary and fine.

crates/ambition_sprite_sheet/ — named content in production code (detailed in §2).

game_assets/entity_sprite.rs EntitySprite enum: Ambition's fixed sprite roster (BossCore, SandbagDummy, PogoOrb, ReboundPad, ProjectileEnergy = "Fireball + Hadouken", LockWallTile, SoftBlinkWall/HardBlinkWall, NpcTerminal, …) with baked PNG paths.
boss.rs doc/comments name specific bosses: gradient sentinel, mockingbird; game_assets/mod.rs:5 names character sheets "robot/goblin/boss."
game_assets/resolvers.rs special-cases character_brain_is_sandbag.
build.rs bakes the game's real goblin/boss sheets from ../ambition_actors/assets/sprites*/.
Soft violations (named content confined to #[cfg(test)])
crates/ambition_ldtk_map/ — tests only: sanic_sandbox (conversion/mod.rs:862-887), seen_alice / Goblin brain (fields/tests.rs:102,133), and a cross-crate fixture into game/ambition_content/assets/worlds (manifest.rs:116-121). Production code is clean.
crates/ambition_audio/ — tests only: provider id "sanic", sanic.ring, boss.shatter, ambition_boss_theme, you_are_too_slow, "a Sanic session may only play Sanic-authored tracks" (selection.rs:470-495, catalog.rs:559-638, bank_asset.rs:429, music/director/{simple,mod}.rs). Production audio machinery is content-free.
Clean (no game-content identifiers found)
crates/ambition_characters/ — machinery only; named cast explicitly lives in ambition_content (only test-placeholder names like "Robot"/"Sniper" appear). The boss_pattern/smash archetype names (Sweep, Slam, BodyNova, FloorSlam) are generic move-geometry vocabulary, not named bosses.
crates/ambition_entity_catalog/ — content-free schema; "goblin" appears only in doc comments illustrating re-binding ("giving the goblin the player's slash is a re-binding, not a Rust change") and placements.rs:455 ("skitters, lurkers, raiders, goblins") as enemy-class prose, not identifiers.
crates/ambition_sfx_bank/ — pure format reader, no content.
One-line summary per crate
ambition_characters — machinery, correctly placed, zero content/zero generated code, ~40% tests; big but clean.
ambition_sprite_sheet — half clean metadata pipeline, half leaked gameplay→sprite binding; the game_assets/resolvers.rs file alone creates the combat/world/interaction/entity_catalog deps and should move to content.
ambition_sfx_bank / ambition_sfx / ambition_audio — well-layered (pure format → contract → Kira/music); keep audio separate; sfx_bank is the only merge candidate. But ambition_sfx/ids.rs smuggles the named dialogue cast into engine space.
ambition_entity_catalog — clean Tier-0 schema; the shared spine the other catalogs build on.
ambition_items — not an engine crate at all; a hardcoded 24-item game roster (enum Item) that belongs in game/ambition_content.
ambition_ldtk_map — clean adapter with a proper install_world_manifest() seam; content only in tests.
I have gathered comprehensive data. Here is the full recon report.

Crate-Decomposition Recon: Ambition App/Content/Facade/Orchestration Layer
Orientation: the dependency spine
bevy → crates/ambition_engine_core (foundation) → ~40 crates/ambition_* engine crates → crates/ambition (facade/umbrella) → game/ambition_content + game/ambition_demo_* (content/providers) → game/ambition_app (multi-game host). The workspace Cargo.toml registers 48 members. run_game.sh defaults to target_pkg="ambition_app" / target_bin="ambition_game_bin" (the host title screen); mode aliases sanic/mary-o retarget it to ambition_demo_*_app standalone shells (windowed by default via --features visible).

1. crates/ambition facade — clean prelude that ALSO carries a provider engine (943 LOC)
Four files:

crates/ambition/src/lib.rs (87) — pure re-export. pub use ambition_actors as actors; … 38 ambition_* crate aliases, plus pub use bevy;. Also three curated helper modules that only re-export: engine (add_headless_foundation, init_engine_states, PlatformerEnginePlugins, SandboxSetsPlugin, SimCoreResourcesPlugin), windowed_host (HostCameraPlugin, PlatformerHostPlugins, HostInputBindingsPlugin), renderer, presentation (PlatformerPresentationPlugin).
crates/ambition/src/prelude.rs (14) — pure curated re-export set.
crates/ambition/src/session_world.rs (14) — pure re-export of SessionRoot/SessionWorldRef/SessionWorldMut + PlatformerSession*.
crates/ambition/src/provider.rs (828 — NOT a re-export; this is real logic).
provider.rs is the authoring/orchestration engine every provider (demo AND Ambition itself) is built on. It owns:

AuthoredCatalogFragments (builder: with_music/with_procedural_sfx/with_adaptive_cues/with_packed_sfx) + validate().
PlatformerExperienceAuthoring — the registration seam: installs resources, adds PlatformerProviderRuntimePlugin, calls register_gameplay_experience, wires the standard_platformer_preparation_plan, loading spec.
PlatformerAuthoredCatalogRegistry, PlatformerStreamingReadiness resources + update_streamable_packed_sfx system.
PlatformerPreparation<'w> SystemParam — the ~250-line preparation state machine that drives all 10 PREPARE_*_WORK_ID steps (catalogs → world → sprites → music → sfx → adaptive → defaults → packed-sfx → session), emitting LoadCommands and LoadFailures.
PreparedPlatformerSessions<M> + cleanup_prepared_platformer_sessions::<M> + PlatformerSessionBuilder (spawns the simulation world via actors::session::setup::simulation_world).
Assessment: the facade is 3 clean re-export files + 1 substantial logic file. It carries the shared provider-preparation/session-build machinery. This is arguably mis-homed: provider.rs reaches into characters, audio, load, game_shell, sprite_sheet, dev_tools, actors::session — it is orchestration, not vocabulary. It is the load-bearing reason the facade is not a pure prelude. The game.umbrella-not-app-shell and game.umbrella-exposes-engine-surface policies enforce that the facade stays engine-surface and names no app/content, but do NOT stop it from owning provider logic.

2. game/ambition_app at 22,349 LOC — module map + classification
LOC by subsystem
Subsystem	LOC	of which test
menu/	10,082	~4,367
app/	5,598	0
dev/	2,774	0
rl_sim/	1,694	~461
bin/	1,427	0
host/	352	0
headless.rs+tests	367	203
lib.rs	55	0
Roughly ~5,200 LOC (23%) is test code; the app is nowhere near "thin wiring." Manifest deps (game/ambition_app/Cargo.toml): the three demo providers (ambition_demo_sanic/_pocket/_mary_o), ambition facade, ambition_content, ambition_menu_kaleidoscope, bevy.

Biggest individual offenders (file paths)
LOC	File	Kind
2008	game/ambition_app/src/menu/kaleidoscope_app/lunex_kaleidoscope_app_tests.rs	test
1826	game/ambition_app/src/menu/kaleidoscope_app.rs	engine machinery
1201	game/ambition_app/src/menu/grid_backend.rs	engine machinery
1199	game/ambition_app/src/menu/model.rs	engine machinery
1071	game/ambition_app/src/menu/grid_backend/tests.rs	test
997	game/ambition_app/src/dev/portal_inspector.rs	engine/dev machinery
987	game/ambition_app/src/app/cli.rs	wiring
929	game/ambition_app/src/dev/debug_overlay/gizmos.rs	engine/dev machinery
872	game/ambition_app/src/app/plugins.rs	wiring
655	game/ambition_app/src/menu/model/tests.rs	test
633	game/ambition_app/src/menu/parity_tests.rs	test
587	game/ambition_app/src/rl_sim/runtime.rs	engine machinery
504	game/ambition_app/src/bin/capture_scene.rs	tooling binary
494	game/ambition_app/src/app/world_flow/room_flow.rs	engine machinery (app-local by design)
490	game/ambition_app/src/app/scene_setup.rs	wiring
442	game/ambition_app/src/app/setup_systems.rs	wiring
Classification
(a) Pure wiring / plugin assembly — legitimately app-local (~5,600 LOC, app/ + host/):

app/plugins.rs (872) — SandboxSimulationPlugin/SandboxLdtkPlugin/SandboxPresentationPlugin; composes ambition_content::AmbitionContentPlugin + runtime::PlatformerEnginePlugins + host::PlatformerHostPlugins, then pins ~20 app-local systems into engine ordering slots. This is the real "assembly" file and reads like it.
app/cli.rs (987) — arg parsing, build_visible_app, run_visible, run_shared_host_headless, run_shared_host_acceptance_cycle.
app/shell_host.rs (316) — compose_ambition_shell_host (the multi-game title-screen host), startup vanity, install_ambition_shell_visuals, process exit, Quit-to-Home.
app/scene_setup.rs (490), setup_systems.rs (442), resources.rs (296), sim_resources.rs (63), sim_systems.rs (198), phases.rs (64), feedback.rs (112), dev_runtime.rs (354), host/ (platform/android/desktop/framepace, 352).
(b) Engine machinery that should move DOWN into crates/ (~13,000 LOC gross, ~8,600 non-test):

menu/ (10,082, ~5,700 non-test) — the single biggest decomposition target. menu/mod.rs self-describes as "Game-side menu host stack: backend-agnostic page model, dispatcher, item effects, and the flat-grid / 3D-cube presentation hosts." It is renderer-agnostic (model.rs builds MenuPageModel/ItemsOnlyPageSpec from live item state; feeds Lunex cube, Bevy-UI grid, or a test renderer). grep for Ambition-specific content (cut_rope/goblin/gnu_ton/boss names/AMBITION_EXPERIENCE) inside menu/ returned nothing — it is generic pause/inventory-menu wiring, not Ambition content. Files: kaleidoscope_app.rs (1826), grid_backend.rs (1201), model.rs (1199), dispatch.rs (254), effects.rs (270), kaleidoscope_app/{dev_toggles,pointer,cache,scroll,scrim}.rs, quality_confirm.rs.
rl_sim/ (1,694) — the RL/programmatic-sim environment. SandboxSim step API + AgentAction/AgentObservation/reward/random_policy. Explicitly built as "a small public API so external drivers — RL agents, fuzz harnesses, scripted-replay tools, Python bindings via PyO3 — can build on top of one shared seam." This is a gym/harness engine, not app content.
dev/ (2,774) — debug/inspection machinery. portal_inspector.rs (997), debug_overlay/{gizmos.rs 929, prims.rs 251}, debug_overlay.rs (226), fps_overlay.rs (308). Generic developer overlays.
app/world_flow/room_flow.rs (494) + world_flow.rs (82) — the room-transition APPLY composer (load_room + render spawns). E5 deliberately left this app-local as a "slot," but it is generic engine composition any second game needs.
app/hud.rs (357) and app/player_clone.rs (262) (brain-driven player clone on key K) — generic gameplay/presentation machinery.
(c) Ambition content that should move into ambition_content: Largely already extracted — the game.app-sim-resources-no-inline-content, game.app-plugins-no-inline-item-roster, game.app-plugins-no-reown, game.app-sim-systems-no-redefine policies actively forbid named content in the app (see §6). Residual Ambition-specific bits: the "Powered by Ambition" startup vanity card (shell_host.rs compose_ambition_startup_sequence), and content references reached through the plugin (ambition_content::AmbitionContentPlugin, ambition_content::bosses::tick_boss_idle_barks, ambition_content::falling_sand::FallingSandRoomPlugin, ambition_content::items::AmbitionItemRosterPlugin) — these are calls, not inlined content, so they are correctly homed.

(d) Test infrastructure (~5,200 LOC): menu/kaleidoscope_app/lunex_kaleidoscope_app_tests.rs (2008), menu/grid_backend/tests.rs (1071), menu/model/tests.rs (655), menu/parity_tests.rs (633), menu/test_support.rs (89), rl_sim/tests.rs (223) + reward/tests.rs + action/action_tests.rs, headless/tests.rs (203). Binaries in bin/ (1,427): capture_scene.rs (504), trace_replay.rs (294), headless.rs (231), rl_smoke.rs (225), rl_random_walker.rs (150), ambition_game_bin.rs (23).

3. game/ambition_content at 14,311 LOC — mostly clean content, but registers presentation/orchestration it should not
Module map (top files)
portal/tests.rs (1551), falling_sand.rs (1354), bosses/specials/gradient_sentinel.rs (934), content_validation.rs (621), portal/plugin.rs (532), bosses/gnu_ton/tests.rs (482), bosses/cut_rope/arena.rs (404), quest.rs (356), intro/ (route_state/plugin/cutscene/sprites ~1400 total), bosses/ (mod, banter, yarn, gnu_ton, cut_rope, specials/* — the boss cast), portal/ (ability/inventory/fire/shot/transit/input/host adapters), duel_arena.rs (148), character_catalog.rs (121), music.rs (186), worlds.rs (122), dialogue/, provider.rs (194), plugin.rs (111).

This is cleanly content: named cast (bosses, enemy roster), quests, dialogue/cutscenes, the intro story, level/world manifests, music cues, item catalog data, the cross-content validator. lib.rs states the strict one-way rule: "content → machinery, never the reverse." Registration flows through one seam, AmbitionContentPlugin (plugin.rs).

Why it depends on ambition_render and ambition_runtime
ambition_render (4 real usages) — a genuine boundary smell but narrow:

intro/plugin.rs:30 use ambition_render::quality::ResolvedVisualQuality — reads a render resource.
bosses/cut_rope/mod.rs:31 use ambition_render::rendering::PropVisual — content naming a render component.
portal/tests.rs:908,932 ambition_render::rendering::PlayerVisual — test-only.
So content does reach two presentation types (PropVisual, PlayerVisual in production via cut_rope). These are the exact "content registers presentation it shouldn't" cases — small, but real; a second game's content crate would inherit the same render coupling.

ambition_runtime — mostly legitimate, one architectural reason:

provider.rs:16 PlatformerSessionWorld — needed to author sessions (unavoidable given provider pattern).
bosses/specials/mod.rs + bosses/specials/snapshot.rs — SnapshotRegistry / snapshot_pod! / snapshot_unit_enum!. The Cargo.toml comment (N3.1) justifies it: "this crate's boss specials hold SIM STATE, so they register their own snapshot codecs … each sim crate registers its components' serialization." This is defensible — netcode determinism requires content-owned sim state to register its own codec — but it means content depends on the runtime composition tier for snapshotting, not just vocabulary.
Much of what looks like runtime is actually ambition_actors::platformer_runtime::* (the reusable facade in the actors crate — gravity, collision, transit, body), not the ambition_runtime crate.
Net: content is ~95% clean content. The leaks are (a) two render presentation types (PropVisual/PlayerVisual) and ResolvedVisualQuality, and (b) the snapshot-codec registration coupling to ambition_runtime. The render dep is the one worth flagging for the decomposition session.

4. Demo crates — what a game implements, and how much is boilerplate
What a demo game must implement (the provider contract)
A provider is a Bevy Plugin. Every one (ambition_demo_sanic/src/provider.rs, ambition_demo_mary_o/src/provider.rs, ambition_demo_pocket/src/lib.rs, and ambition_content/src/provider.rs) implements the identical shape:

Consts: *_EXPERIENCE, *_GAMEPLAY_ROUTE, *_CHARACTER_ID (+ optional *_LAUNCHER_ROUTE).
A *SessionWorld struct + *_session_world() builder producing RoomGeometry / RoomSet / ActiveRoomMetadata / StartingCharacter.
A zero-size *ProviderMarker + type Prepared*Sessions = PreparedPlatformerSessions<Marker>.
impl Plugin::build: install content (character/audio catalog fragments), call PlatformerExperienceAuthoring::new(...).register(app), init_resource::<Prepared*Sessions>(), add *_prepare_session (in AmbitionLoadSet::Contributors) + cleanup_prepared_platformer_sessions::<Marker>, add *_activate_session (in GameplaySessionSet::Providers).
*_prepare_session — filters ShellEvent::PreparationRequested by experience id, builds PlatformerSessionWorld::new(...), calls preparation.prepare(...).
*_activate_session — filters GameplaySessionEvent::Activated, takes the prepared world, calls builder.build(activation, scope, live_world, CHARACTER_ID).
That is the entire hook surface: two systems + one plugin + a room + a catalog fragment. No trait to implement — it is a data + plugin convention on top of ambition::provider::*. The traits/exts a demo touches: GameplaySessionAppExt (register_gameplay_experience, via .register()), AudioCatalogAppExt, CharacterCatalogAppExt, WorldItemArtAppExt (Mary-O only).

Duplication (the boilerplate is heavy on the _app side, light on the provider side)
Provider crates: sanic/provider.rs (132) and mary_o/provider.rs (188) and pocket (the whole prepare/activate pair) are near-verbatim — the prepare_session/activate_session bodies differ only in the experience-id string and character const. ~100 lines of prepare/activate logic is copy-pasted per game.
Standalone _app crates: THIS is the real duplication. game/ambition_demo_sanic_app/src/lib.rs (429) and game/ambition_demo_mary_o_app/src/lib.rs (330) are structurally identical: build_demo_app / build_demo_app_with_home / compose_*_shell / build_windowed_demo_app(RenderMode) / install_*_asset_resources / load_*_game_assets / install_*_audio / setup_*_audio_library / the RenderMode enum / desktop_asset_root. The windowed-app assembly (DefaultPlugins + WgpuSettings headless recipe, init_engine_states, engine+host groups, presentation plugin, GameAssets load, Kira audio library) is duplicated line-for-line, differing only in title string, provider name, and sprite-sheet stems. _app/main.rs (122/120) are also parallel. ~330–430 LOC of windowed-host scaffolding copied per demo — the strongest candidate for a shared PlatformerDemoHost helper crate.
Pocket has no _app crate (host-only fixture, 266 LOC total).
Is "add a game = add a crate" real?
Mostly yes for the crate; NO for the multi-game host. A provider crate depends only on ambition (+ its own content) — enforced by game.demo-*-umbrella-only allowlists and standalone_demos_do_not_depend_on_ambition_app. A standalone _app runs with "Zero engine edits, zero ambition_app" (the lib.rs doctrine). Pocket's test proves a 4th provider composes in an alternate host with no core edits.

But to appear on the Ambition title-screen host, you still edit core/app in two places:

game/ambition_app/Cargo.toml — add the ambition_demo_<x> path dep.
game/ambition_app/src/app/shell_host.rs:90-95 — add the plugin to the app.add_plugins((AmbitionExperiencePlugin, SanicExperiencePlugin, MaryOExperiencePlugin, PocketExperiencePlugin)) tuple.
The launcher entries derive from registrations (no per-game match arms), so it is only those two lines — but they are hand-edited in the app crate. There is no dynamic/registry-driven provider discovery. So the north-star oracle ("build another platformer by ADDING a content crate without editing core") is TRUE for a standalone game and FALSE for adding a game to the shared host.

5. ambition_runtime vs ambition_host — clear boundary, minimal duplication
ambition_runtime (7,702 LOC) — the headless, content-free SIM composition tier.

lib.rs (339): PlatformerEnginePlugins PluginGroup (the ~25 content-free sim plugins: sets, sim-core resources, snapshot registry, world-prep, brain, abilities, trail, gravity, item pickup, features, LDtk spine, encounter/cutscene, effects, sim-view, reset, trace, affordances, camera observation, combat, player schedule, room-transition, progression, mode-scope, portal), SandboxSetsPlugin, add_headless_foundation, init_engine_states, SIM_TICK_HZ, the two-clock (frame-stepped vs fixed_tick) machinery. Plus facade modules demo_fixture, host_input, host_seams.
The bulk is snapshot/ (~6,000 LOC: codecs 1402, mod 1192, registry 709, restore 492, motion_codec 368, tests 1912) — the N3.1 netcode serialization tier. Plus the schedule plugins (combat_schedule, player_schedule, progression_schedule, portal_schedule, room_schedule, projectile_schedule, mode_scope, sim_core_resources, input_stream, session_world).
Explicit self-declaration: "This is the demo gate: a demo app depends on ambition_runtime, never on ambition_app… Presentation, audio, windowing, dev tools, and CONTENT are never in this group."
ambition_host (997 LOC) — the windowed presentation companion.

lib.rs (254): PlatformerHostPlugins group = HostCameraPlugin (camera follow/shake, viewport publish, CameraViewState) + HostInputBindingsPlugin (feature input: leafwing input map, device→ControlFrame/MenuControlFrame bridge, frame→tick latch for fixed-tick, headless-safe InputPlugin/CursorMoved registration).
portal.rs (743, feature portal_render): portal camera-continuity + observation glue.
Declares its own boundary: "The host MAY dep ambition_render/ambition_input/ambition_runtime; it must NEVER dep ambition_actors or ambition_content" — enforced by engine.host-* policies.
Boundary clarity: excellent and explicitly documented. Runtime = "what headless/RL also runs"; host = "what only a window needs." The split is enforced by engine.runtime-manifest-{allow,deny}, engine.runtime-source-no-upper, engine.host-manifest-no-actors, engine.host-source-no-actors, engine.host-names-no-content.

Duplication with ambition_app wiring: low and intentional. app/plugins.rs::add_simulation_plugins calls runtime::PlatformerEnginePlugins; add_presentation_plugins calls host::PlatformerHostPlugins. The app composes ON TOP of both rather than reimplementing them. The residual app-local sim systems (register_app_local_sim_systems) are the deliberate E5 "slots" (reset/replay consumers, home-reset policy, room-transition APPLY, player clone) — pinned into engine ordering, not duplicated. The genuine duplication is elsewhere: the _app demo crates re-implement windowed-host asset/audio composition (see §4) that neither runtime nor host provides.

6. tests/ambition_workspace_policy — enforced boundaries (7,418 LOC)
Harness (tests/policy.rs): three aggregating scope runners — repository_policies, engine_policies, game_policies — plus per-scanner poison self-tests. Declarative policies live in policies/*.toml (parsed by src/rules/* — dependency, source_reference, file_content, paths, workspace_member); custom scanners live in src/custom/*.

Declarative policy kinds
workspace-member, required-path, forbidden-path, dependency-allowlist, dependency-denylist, forbidden-source-reference, file-contains, file-omits.

Repository scope (repository.toml)
repository.core-crates-registered — tier anchors (engine_core, runtime, content, app) + the policy package stay registered members.
repository.tier-manifests-exist — the crates/ vs game/ tier layout manifests exist.
Engine scope (engine.toml, ~140 policies) — representative boundaries
engine.engine-core-is-foundation-clean — foundation names no runtime/content/app.
engine.host-names-no-content + engine.host-source-names-no-content — host never deps/names ambition_content.
engine.host-manifest-no-actors + engine.host-source-no-actors — host reaches actors only via runtime facades.
engine.render-never-names-live-sim-state — render reads only ambition_sim_view read-model, never live Body*/actor/boss clusters.
engine.render-no-actor-crate-dependency + engine.render-source-names-no-actors — render ⊥ actors (F1.5).
engine.actor-manifest-no-render + engine.actor-source-no-render — sim ⊥ its renderer.
engine.platformer-runtime-content-free — the platformer_runtime facade stays content-free.
engine.runtime-{member,required-deps,manifest-allow,manifest-deny,source-no-upper} — runtime is the headless composition tier, names nothing above it.
engine.sim-no-presentation-import — sim imports no presentation.
engine.machinery-imports-no-content, engine.combat-kit-content-free, engine.characters-content-free, engine.enemy-sim-reads-data, engine.presentation-no-archetype-enum — machinery/combat/characters stay content-free.
Per-crate purity triplets (member + manifest allow/deny + source purity) for: ambition_world, ambition_ldtk_map, ambition_items, ambition_inventory_ui, ambition_settings_menu, ambition_dev_tools, ambition_dialog, ambition_interaction, ambition_vfx, ambition_projectiles, ambition_portal, ambition_portal_presentation, ambition_time, ambition_input, ambition_touch_input, ambition_encounter, ambition_persistence, ambition_menu, ambition_asset_manager.
engine.menu-crate-{manifest,source}-no-actors — reusable menu renderer stays content-free.
engine.asset-manager-no-{audio,backend-providers,sfx-adapter,sfx-feature} — asset manager is backend-generic.
engine.load-shell-core-members, engine.load-core-dependency-purity, engine.game-shell-dependency-purity, engine.load-presentation-dependency-direction, engine.load-shell-source-content-free — the load/shell tier is content-free.
engine.audio-authority-is-app-local, engine.character-authority-is-app-local, engine.sfx-emission-captures-exact-audio-owner, engine.shell-audio-authority-is-explicit — provider-relative audio/character authority.
Movement/determinism narrow guards: engine.movement-model-is-never-optional, engine.crawler-flag-is-spawn-selection-only, engine.pose-writes-are-authority-only, engine.mechanics-consume-the-resolved-frame, plus portal/gravity extraction and clock-reset ownership policies.
Game scope (game.toml, ~45 policies)
game.content-and-app-homes-exist, game.named-content-paths — content + app roots exist.
game.umbrella-and-demo-members, game.umbrella-not-app-shell, game.umbrella-exposes-engine-surface — facade is engine surface, not app/content.
game.demo-{sanic,pocket,mary-o}-umbrella-only — each demo deps only the umbrella (+ own content).
game.demos-reach-engine-through-facade, game.app-reaches-lower-through-facade — demos/app reach lower crates via the facade.
game.app-umbrella-plus-local — app deps allowlist (umbrella + content/extension + the three providers).
game.app-installs-content-plugin, game.content-plugin-composes — app installs AmbitionContentPlugin; the plugin composes its sub-plugins.
App-thinness guards (the key ones for §2): game.app-sim-resources-no-inline-content (no QuestRegistry::default()/BossEncounterRegistry::default()/IntroPlugin/banter installs inline), game.app-plugins-no-inline-item-roster (no OwnedItems::starter()), game.app-plugins-no-reown (no re-registering portal/item-pickup systems), game.app-sim-systems-no-redefine (no redefining input_timer_system etc.), game.engine-schedule-references-moved + game.app-keeps-host-reset-systems.
game.no-legacy-runtime-in-app-src — no SandboxRuntime/FeatureRuntime/runtime.player god-objects.
game.providers-are-host-ignorant — a provider must NOT name the launcher route, the shell-hosted marker, a host crate, or issue QuitToHome/ExitProcess (the host-relative return contract).
game.kaleidoscope-{member,menu-model-only,no-game-machinery}, game.menu-crate-lunex-free — kaleidoscope is an engine extension over the menu model.
Structural: game.abilities-under-layer/-not-at-crate-root, game.lib-menu-no-host-files/-keeps-map, game.lib-dev-no-overlays/-keeps-trace, game.dev-tools-owns-state, game.map-state-consumers-menu-crate, etc.
Custom scanners (src/custom/)
module_size.rs — 5000-line production-module gate (engine.module-size, data in module_size.toml), waiver-reason quality enforced.
determinism.rs — N0.3: no ambient randomness / wall-clock reads / std-hash-order / Entity-as-ordering-key (engine + game scope).
control_frame.rs — ControlFrame holder allowlist (only the input layer holds the global device frame; slot-0 routing / N1).
lifecycle.rs — raw-spawn gate (session-scope spawn discipline).
content_ownership.rs — engine.enemy-config-archetype-free: durable ActorConfig/ActorMut structs carry generic kit data, never the roster enum CharacterArchetype (what lets the roster leave machinery for content).
session_world.rs — canonical SessionRoot/SessionWorld access gate.
migration_matrix.rs — asserts every retired architecture_boundaries.rs test maps to a live policy (bijection honesty check).
Plus in-test guards: app_layer_does_not_bind_the_selected_character_sprite (S3), standalone_demos_do_not_depend_on_ambition_app (S4), movement-kernel/pose-write/frame-reconstruction poison guards.
Boundaries NOT yet enforced that these policies imply SHOULD be
Migration matrix status: grep for legacy-pending/retained/removed in migration_matrix.toml returned nothing — all legacy architecture_boundaries.rs tests are fully migrated. No pending-migration debt is tracked there.
The 22k ambition_app is not size-gated meaningfully. engine.module-size limit is 5000 and only fires per-FILE; no policy caps a CRATE's total, and no policy forbids the menu/ (10k), rl_sim/ (1.7k), or dev/ (2.7k) engine-machinery subsystems from living in the app. The app-thinness policies (app-*-no-reown, -no-redefine, -no-inline-content) are all narrow file-omits denylists targeting SPECIFIC identifiers in SPECIFIC files — they stop named-content regressions but do NOT enforce that the app is thin overall. A policy that forbids new engine-generic subsystems (menu host, RL env, dev overlays) in game/ambition_app/src is the gap.
game.providers-are-host-ignorant is enforced, but there is no positive "the host discovers providers dynamically" policy — the two-line manual edit in shell_host.rs to register a provider is unguarded, so the "add a game to the host without editing the host" property is neither achieved nor enforced.
Content→render coupling is only partially locked. engine.render-never-names-live-sim-state guards render, but there is no symmetric "content-names-no-render-presentation-types" policy — hence bosses/cut_rope/mod.rs naming ambition_render::rendering::PropVisual and intro/plugin.rs naming ResolvedVisualQuality pass unflagged (see §3).
crates/ambition (facade) has no policy preventing it from owning logic — provider.rs (828 LOC of orchestration) is allowed. Only "exposes engine surface / not app-shell" is checked.
7. Engine-worthy machinery locked inside ambition_app / ambition_runtime
Things a second real game needs but currently can only get by depending on ambition_app (which the demo-gate policy forbids):

In game/ambition_app (the big ones):

The unified pause/inventory menu host stack — menu/ (~5,700 non-test LOC). Backend-agnostic MenuPageModel/ItemsOnlyPageSpec builder (model.rs), dispatcher, item effects, and BOTH presentation hosts (kaleidoscope_app.rs Lunex 3D-cube + grid_backend.rs Bevy-UI grid). A second game gets the ambition_menu renderer and ambition_menu_kaleidoscope extension via the facade, but the HOST WIRING that turns live game state into pages, dispatches actions, and drives the two backends lives only in the app. This is the highest-value extraction candidate (proposed: crates/ambition_menu_host).
The RL / programmatic-sim environment — rl_sim/ (1,694 LOC). SandboxSim step API, AgentAction/AgentObservation/reward/random_policy. Explicitly designed as a reusable driver seam (RL, fuzz, replay, PyO3) yet trapped in the app. Any second game wanting a headless gym harness must depend on ambition_app.
Developer overlays / inspection — dev/ (2,774 LOC). debug_overlay + gizmos + prims, fps_overlay, portal_inspector. Generic runtime-debug tooling.
The room-transition APPLY composer — app/world_flow/room_flow.rs (494). E5 left it as an app "slot," but load_room + render-spawn composition is generic engine glue every game repeats.
The windowed-host assembly recipe — currently NOT in the app at all but COPY-PASTED across game/ambition_demo_*_app/src/lib.rs (~330–430 LOC each): build_windowed_demo_app(RenderMode) (DefaultPlugins + no-backend WgpuSettings CI recipe), install_*_asset_resources, load_*_game_assets, install_*_audio/setup_*_audio_library. This "PlatformerDemoHost / windowed-shell" scaffolding is engine-worthy and should be a shared crate rather than duplicated per demo (and re-derived a third time inside ambition_app's visible composition).
The multi-game host composition itself — app/shell_host.rs (316). compose_ambition_shell_host (title-screen launcher hosting a provider tuple), install_ambition_shell_visuals (provider-agnostic per-session room presentation), startup vanity, Quit-to-Home. A second title-screen host for a different game set would copy this file wholesale.
HUD (app/hud.rs 357), player-clone (app/player_clone.rs 262), CLI/composition harness (app/cli.rs 987), scene/setup assembly (scene_setup.rs, setup_systems.rs).
In crates/ambition_runtime: this crate is already the clean, reusable content-free tier — nothing engine-worthy is trapped ABOVE its consumers; it is correctly positioned as "the demo gate." The one nuance: snapshot/ (~6,000 LOC of netcode codecs) lives here and is depended on by CONTENT (ambition_content registers boss-special codecs against ambition_runtime::snapshot::SnapshotRegistry), which is arguably a vocabulary tier (the SnapshotContribution trait) that could sit lower so content doesn't reach the composition tier for it — but this is a fine-grained call, not trapped machinery.

Bottom line for the planning session: ambition_runtime/ambition_host are exemplary — clean, documented, enforced boundaries. The decomposition debt is concentrated in ambition_app, which is ~60% engine-generic machinery (menu host, RL env, dev overlays, windowed-host recipe, room-transition composer) rather than thin wiring. Extracting menu/, rl_sim/, dev/, and a shared windowed-demo-host crate would move ~10,000+ LOC of reusable machinery below the demo gate, and adding a policy that forbids new engine-generic subsystems in game/ambition_app/src (plus a provider-discovery mechanism replacing the manual shell_host.rs tuple edit) would make the north-star oracle true and enforced.

The ambition_actors anatomy is in — and it's the most decision-relevant report yet. Key tension: the agent found real seams (a ~9.6k boss carve blocked only by boss_clusters.rs living in features/ecs; ~5k of free leaf extractions; two production content leaks — hardcoded boss sprite tables and a named character-id match), but the 2026-07-10 ruling says no further actors split is owed. My synthesis will separate what's compatible with that ruling (content eviction, intra-crate de-tangling, facade collapse) from what would require Jon to revisit it (the boss/actor-sim/player carves). Three agents left.

The content-data cluster report lands some of the strongest findings yet: ambition_items is a closed 24-variant enum Item game roster living in crates/ (a hard oracle violation by construction), ambition_sfx/ids.rs bakes Ambition's named dialogue cast into engine space, and ambition_sprite_sheet is half clean metadata pipeline, half leaked gameplay→art binding — one file (game_assets/resolvers.rs) creates its four suspicious deps. Notably this contradicts the decomposition playbook's exit criterion #2 ("named-content grep over engine crates hits zero") — the enforcement grep evidently has a narrower net than the reality. ambition_characters meanwhile is confirmed clean machinery (~40% tests). Two agents remaining.

The app/facade layer report is in and it's the richest so far. The standout: ambition_app is ~60% engine-generic machinery, not thin wiring — a ~5.7k non-test backend-agnostic menu host stack with zero Ambition content in it (new evidence against the old E1e "menu stays app-side" ruling), the 1.7k RL/gym environment (rl_sim/), 2.8k of dev overlays, and a windowed-host recipe copy-pasted ~400 LOC per demo _app crate. Also: the ambition facade smuggles 828 LOC of real provider-orchestration logic in what's supposed to be a pure re-export crate, and adding a game to the shared title-screen host still requires hand-editing the app. One agent left (presentation stack), then I'll synthesize.

I have all the data I need. Here is the full reconnaissance report.

Crate-Decomposition Recon: Presentation / UI Stack
Method note: LOC from wc -l on src/**/*.rs; dep classification from actual use/path sites in source; seam rules confirmed against the machine-enforced policy in /home/joncrall/code/ambition/tests/ambition_workspace_policy/policies/engine.toml.

0. The seam is real and machine-enforced (context for everything below)
engine.toml pins the render/sim seam with two policy families:

engine.render-never-names-live-sim-state (whole-identifier forbid list): render source may NOT name live sim clusters — BodyKinematics, BodyGroundState, …, BodyCombat, BodyMana, …, BossClusterRef, ActorDisposition, HeldItem, GroundItem, PlayerMark, HealShrine, GravityFlipSwitch, LiveProjectile, etc. The rationale text explicitly says "vocabulary/data types (FeatureView, CharacterAnim, Health-as-data) are deliberately NOT forbidden."
engine.render-no-actor-crate-dependency + engine.render-source-names-no-actors: render must not depend on or name ambition_actors at all (manifest + source). Verified clean: the only ambition_actors strings in render src are two doc-comment [WorldItem] intra-doc links in item_visuals.rs.
So the seam question is not "does render touch actors" (it doesn't) — it's "which sibling vocabulary/IR crates does render still name, and could those be folded into the read-model."

1. ambition_render (10,895 LOC, 20 workspace deps: 19 required + 1 optional)
Module map (LOC)
Top-level (src/): fx.rs 974, screen_effects.rs 278, hud.rs 293, cutscene/mod.rs 238, platformer_presentation.rs 209, dialog_ui.rs 198, ui_fonts.rs 187, quality.rs 65, lib.rs 27.

src/rendering/ (the bulk, ~7.8k): world.rs 1109, actors/mod.rs 586, nameplates.rs 550, hit_flash.rs 541, item_visuals.rs 457, projectile_visuals.rs 413, deep_dream.rs 400, shrine_visuals.rs 339, health.rs 332, parallax.rs 331, mod.rs 328, morph_ball.rs 324, slash_visuals.rs 295, actors/overlays.rs 295, actors/animation.rs 268, bubble_shield.rs 237, pirate_weapon.rs 234, actors/worn_binder_tests.rs 203, actors/boss.rs 200, camera.rs 200, primitives.rs 197, gate_portal_visuals.rs 158, gravity_visuals.rs 126, foreground.rs 118, features.rs 72, mark_beacon.rs 61, sheet_atlas.rs 52.

Per-dep classification (use-site count → verdict)
Dep	sites	What render names	Class
ambition_sim_view	45	BodyPoseView, FeatureViewIndex, ActorRenderIndex, BossRenderIndex, BossFrameIndex, NameplateIndex, PlayerHudFacts, ShrinesView, GroundItemsView, WorldItemsView, WieldedGunSwordsView, ProjectileView, camera_snapshot::*, BlinkPreviewFact…	(a) read-model — the seam. Correct.
ambition_platformer_primitives	101	lifecycle::{SessionWorldRef, SessionSpawnScope, ActiveSessionScope…}, physics::PhysicsSandboxSettings, markers::{MainCamera, PrimaryPlayer, PlayerEntity}, schedule::SandboxSet, gravity::GravityCtx, camera_layers::MainCamera	Engine primitive vocabulary. Fine (shared lifecycle/schedule/marker types).
ambition_engine_core	95	RoomGeometry, config::world_to_bevy, config::WORLD_Z_*, Vec2/AabbExt	Core math/config vocabulary. Fine.
ambition_sprite_sheet	33	game_assets::GameAssets, character::CharacterAnimator/CharacterAnim, boss::BossAnimator, SheetRegistry/FrameRect	Render-facing asset/art vocabulary. Fine.
ambition_world	24	rooms::{RoomSet, RoomSpec, Authored<T>, HazardVolumeSpec, ChestSpec, InteractableSpec, GatePortalRegistry, LoadingZone…}	(b/c) Authoring/IR read directly. world.rs::spawn_room_visuals pulls SessionWorldRef<RoomSet> and iterates RoomSpec placements to build static visuals — this path bypasses sim_view entirely. Legal-ish (authored static data, not live sim), but it is why this dep can't be cut.
ambition_time	15	WorldTime, ProperTimeScale	Time vocabulary. Fine.
ambition_portal_presentation	13	PortalPresentationPlugin/Set, PortalCameraContinuity*, PortalViewRig, PortalCaptureQualityBudget	Optional (portal_render feature). Presentation sub-renderer. Fine.
ambition_combat	7	events::{FeatureView, FeatureVisualKind, BoundFeatureKind}	Vocabulary type — FeatureView is the shape of the read-model, but its home crate is combat, and render reads instances via sim_view::FeatureViewIndex. Explicitly allow-listed. Severable only if the type moved to sim_view.
ambition_dev_tools	7	DeveloperTools, SandboxDevState	Dev-overlay gating. Fine.
ambition_characters	5	actor::Health (as data), actor::WornCharacter (query filter Without<WornCharacter>), actor::BodyCombat (doc only)	(b-adjacent) "Health-as-data" is explicitly allowed, but WornCharacter is used as a live query filter in actors/mod.rs. Borderline: it names a characters ECS component directly. Severable if folded into a pose/identity view.
ambition_entity_catalog	5	placements::PlacementSchema::{Hazard,Pickup,Chest,Breakable,Interactable}	Paired with ambition_world: world.rs pattern-matches placement schema directly. Authoring IR.
ambition_persistence	5	settings::{UserSettings, VisualQualityBudget, ParallaxBudget, ScreenShaderSettings, TextureResolutionScale}	Settings/config vocabulary. Fine.
ambition_asset_manager	5	AssetId, sandbox_assets::{ids, SandboxAssetCatalog}	Font/asset-id resolution (ui_fonts). Fine.
ambition_vfx	3	vfx::{VfxMessage, SlashKind} (re-exported by fx)	Message vocabulary. Fine.
ambition_projectiles	3	ProjectileVisualKind	Visual-enum vocabulary. Fine.
ambition_dialog	3	DialogState, DialogChoiceSlot (dialog_ui.rs mirrors DialogState)	Read-model-ish, but read directly, not via a sim_view DialogView. Severable if a dialog view existed.
ambition_cutscene	2	ActiveCutscene, CutsceneAdvanceRequest	Cutscene format/state the cutscene UI plays back. Fine.
ambition_sfx	1	SfxMessage, SfxWriter	Message vocabulary. Fine.
ambition_input	0	— (one doc-comment mention only)	DEAD DEP. No use ambition_input / ambition_input:: anywhere in code.
ambition_interaction	0	—	DEAD DEP. Zero references. (BreakableState flows through combat::FeatureView but render never names it.)
Also: leafwing-input-manager (optional, input feature) has zero src uses; the render input feature = ["dep:leafwing-input-manager", "ambition_sim_view/input"] — the leafwing half looks vestigial.

Which deps could be severed if the read-model seam were complete
Immediately (already dead): ambition_interaction, ambition_input (and the unused leafwing-input-manager optional dep). These are cost-free removals today.
With modest read-model work:
ambition_combat — move FeatureView/FeatureVisualKind/BoundFeatureKind into ambition_sim_view (where the index that carries them already lives).
ambition_characters — replace the Health-as-data reads and the Without<WornCharacter> query filter with sim_view facts (a worn/identity field already partly exists in ActorRenderIndex).
ambition_dialog — introduce a DialogView read-model so dialog_ui.rs stops reading DialogState directly.
Structurally harder (the static-world path): ambition_world + ambition_entity_catalog are only severable if room/placement visual facts are mirrored into a view (e.g. a RoomVisualsView). Today world.rs reads the authoring IR directly — the single largest "render reads non-view data" surface (1,109 LOC).
Genuine vocabulary, keep: sim_view, platformer_primitives, engine_core, sprite_sheet, time, persistence, asset_manager, vfx, sfx, cutscene, projectiles, dev_tools, portal_presentation.
Net: render's 20 deps could realistically drop to ~13-15, and 2 are removable this afternoon.

2. ambition_sim_view (3,018 LOC) — read-models + the actors dependency
Module map
view_index.rs 893, camera_snapshot.rs 767, facts.rs 706, anim_index.rs 314, pose_view.rs 256, lib.rs 82.

Read-models it publishes (the SimView vocabulary)
Geometry/state indexes (view_index.rs): FeatureViewIndex, ActorRenderIndex/ActorRenderView, BossRenderIndex/BossRenderView, NameplateIndex/NameplateFact.
Pose/anim (pose_view.rs, anim_index.rs): BodyPoseView, ShieldRingsView/ShieldRingFact, ActorAnimIndex/ActorAnimFrame/ActorSpriteData, BossFrameIndex/BossFrameView, HazardLaneFact.
Fact resources (facts.rs): PlayerHudFacts, HeldItemView, GroundItemsView, WorldItemsView, HeldShotsView, MarkBeaconsView, GravitySwitchesView, ShrinesView, WieldedGunSwordsView, ProjectileView, DynamicFeatureViews, BlinkPreviewFact.
Camera (camera_snapshot.rs): CameraSnapshot2d, SceneCaptureRequest, CameraFocus2d, ResolvedCameraSnapshot, CameraViewport, CameraObservationPlugin.
All are plain-data, rebuilt once per tick in SandboxSet::FeatureViewSync by FeatureViewSyncSchedulePlugin. The doc frames render, RL observation, netcode, and shaders as the same kind of consumer.

Why it depends on ambition_actors (and the inversion question)
sim_view is the extraction / pull side of the seam. The rebuild_* systems query the sim heart's live components to produce the snapshots:

facts.rs: Query<&ambition_actors::features::HeldItem>, items::pickup::GroundItem, items::world_item::WorldItem, abilities::traversal::mark_recall::PlayerMark, gravity::GravityFlipSwitch, shrine::HealShrine, features::ActorDisposition, actor::{BodyKinematics, BodyMana, PlayerEntity, PrimaryPlayer}.
view_index.rs: features::{HazardFeature, BossConfig, BossClusterRef}, platformer_runtime::orientation::ActorRoll.
anim_index.rs: reads the full Body* cluster (BodyGroundState, BodyFlightState, BodyModeState, BodyShieldState, BodyAnimFacts…) and calls character_sprites::pick_actor_anim, boss_encounter::sprites::*.
lib.rs even schedules ambition_actors::features::advance_actor_anim_overlays (an actor-owned mutation).
So sim_view depends on actors because it reads actors — that is inherent to a pull model, and it correctly quarantines that dependency to ONE crate so render never touches actors.

On the inversion ("could actors publish into a view vocabulary instead?"): yes, in principle — a ambition_sim_view_vocab crate could own the plain-data view types (with no actor dep), and ambition_actors would depend on it and populate the resources. That would flip the arrow (actors → view-vocab, view-vocab ⊥ actors), letting the view types sit below actors. But note the cost/benefit: the extraction logic (which knows actor cluster shapes) must live somewhere that can see both; today that's sim_view. Inversion buys you a vocab crate below actors at the price of splitting types from their builders. Worth flagging for the planning session as a "types-vs-builders" tradeoff rather than a clear win — the current arrangement already achieves the north-star property (render adds nothing to actors, and a new game reuses the same view types).

3. ambition_touch_input (2,899 LOC) — input? overlay? both. Should split.
Module map
bevy_plugin.rs 1352, tests.rs 491, state.rs 310, menu_bridge.rs 260, layout.rs 287, exclusion.rs 116, lib.rs 83.

It is two crates wearing one coat
The crate itself documents the split: an always-built pure fold (state.rs: TouchInputState, apply_deadzone, fold_touch_into_control_frame → ControlFrame; deps: only ambition_input) and a mobile_touch-gated Bevy overlay HUD (bevy_plugin.rs + layout/exclusion/menu_bridge; draws virtual joysticks, touch buttons, glyphs).

Why the actors + render deps (both live only in the gated plugin layer)
ambition_render — fonts only. The entire render coupling is ambition_render::ui_fonts::{UiFonts, UiFontWeight} plus .after(load_ui_fonts) ordering in bevy_plugin.rs. That's it. A dedicated ambition_ui_fonts (or moving ui_fonts down) would sever render from touch_input completely.
ambition_actors — schedule ordering + affordance glyphs + gravity + player marker. In bevy_plugin.rs: .after/.before on schedule::{populate_menu_control_frame_from_actions, apply_menu_frame_to_cutscene_request, MenuNavConsume, populate_control_frame_from_actions}, control::populate_slot_controls, affordances::AffordancesSystemSet::Compute; reads affordances::{PlayerAffordances, ActiveInputMethod, glyph_for, InteractVariant, VariantLabel} (to label the context button), physics::{GravityField, gravity_dir_or_default} (to orient the stick under flipped gravity), and actor::PrimaryPlayer (query filter). This is the overlay HUD reaching into gameplay to render the right button prompt — exactly the kind of read that should go through sim_view/affordance read-models, not actors.
Recommendation
Split along the seam the crate already draws:

ambition_touch_input (pure): keep state.rs (fold to ControlFrame). Dep: ambition_input only. Reusable by RL/tests/any host, no Bevy.
ambition_touch_hud (or fold into a presentation crate): the mobile_touch overlay. Its actor reads should be re-pointed at read-models (affordance/glyph facts, gravity direction) and its font dep at a low ui_fonts crate — after which the overlay depends on ambition_input + sim_view + a ui/nav crate, and drops both ambition_actors and ambition_render. That makes an "input crate that depends on render+actors" stop being suspicious, because the input half no longer does.
4. ambition_portal_presentation (6,505 LOC > portal's 5,246) — what's in it
Module map
view_cones.rs 1145, view_cones/debug.rs 1078, view_cones/geometry/tests.rs 945, view_cones/geometry.rs 897, visuals.rs 439 (+visuals/tests.rs 342), gun_visuals.rs 355, camera_continuity.rs 351, clip_material.rs 164 (+tests 128), lib.rs 155, plugin.rs 138, effects.rs 108, view_cones/mesh.rs 102 (+tests 158).

Why it's bigger than the mechanic
The mechanic (ambition_portal) is headless geometry/state; the rendering of "you can see through a portal" is genuinely more code than the mechanic. The weight is concentrated in through-portal view windows (view_cones family = ~4,325 LOC, i.e. two-thirds of the crate): visibility cones, per-pane z-sorting with hysteresis, aperture LOS quality budgets, render-layer capture, and a very large debug.rs (1,078) + geometry test suite (945). The rest is texture-clip material for mid-transit body/gun pieces (clip_material, visuals, gun_visuals), and camera_continuity (roll/transit easing). The crate is a legitimately hard visual problem (wormhole compositing), not bloat — but note ~1,573 LOC is tests and 1,078 LOC is debug overlay, so "live" presentation is ~3,850 LOC. It cleanly depends on only bevy + engine_core + platformer_primitives + portal and "never names a host crate."

Content coupling: the lib doc admits "a sequestered compatibility module for Ambition's portal-gun sprites" — gun_visuals.rs + the PortalGunArt{blue,orange} seam. Art paths are pushed to the host (host loads the resource), so this is a seam, not hardcoded content.

5. UI cluster — 7 crates mapped
Crate	LOC	Role	ambition_* deps	Notes
ambition_ui_nav	728	Nav primitives (bottom layer): windowed-list math (list.rs), pointer row-activation (pointer.rs: MenuFocusState/Owner, resolve_selectable_row_interaction), drag-scroll accumulation (drag.rs). Pure over input.	ambition_input	Extracted from ambition_actors. Content-free. The shared substrate keyboard/gamepad/touch all fold into.
ambition_inventory_ui	149	Inventory navigation state only (InventoryUiState: selection/tab/scroll/focus; InventoryTab{Items,Map,Quests}). Holds no items (store is OwnedItems in ambition_items).	ambition_ui_nav	Tiny. Really just one model + enum on top of ui_nav.
ambition_menu	2,442	Menu framework: renderer-agnostic MenuPageModel/ItemsOnlyPageSpec data model + the flat render::bevy_ui tabbed renderer + a backend.rs switch for the kaleidoscope. Emits MenuActionActivated/MenuClosedRequested.	none (bevy only)	Content-free by policy (engine.menu-crate-*-no-actors). lib.rs 1074, render/bevy_ui/mod.rs 551.
ambition_settings_menu	2,243	Specific menu (settings): pure SettingsMenuModel/SettingsOption/apply_settings_option over persistence::settings::UserSettings, plus the System-menu layer (Radio/Video/Audio/Controls/Gameplay/Language/Reset/Quit/Developer). No bevy, no renderer.	ambition_persistence, ambition_input	Carved out of ambition_actors. system/mod.rs 665, settings/build.rs 555.
ambition_dialog	2,024	Dialogue runtime: DialogState view-model (typewriter reveal + option select), typewriter-SFX rules, input/reveal Bevy systems, bevy_yarnspinner ↔ DialogState bridge + generic Yarn binding installer.	ambition_engine_core, ambition_ui_nav, ambition_input, ambition_sfx, ambition_persistence	Extracted from ambition_actors (E1c). Content-free (host registers Yarn commands via installer). runtime.rs 511, bridge.rs 345. Consumed by render's dialog_ui.rs.
ambition_game_shell	5,099	Shell / routing (not really a "menu"): route graph, scoped experience lifecycle, neutral sequences, launcher, pause-menu (basic_presentation feature). Renderer-independent, no game-specific route names.	ambition_menu(opt), ambition_audio, ambition_load, ambition_platformer_primitives, ambition_sfx	tests.rs 611, basic_presentation.rs 646, router.rs 563, session.rs 560.
ambition_menu_kaleidoscope (game/)	2,500	Alternate menu renderer: bevy_lunex 3D OoT-style rotating-cube renderer for the same MenuPageModel. "The FIRST engine extension crate (E1e)."	ambition_menu	Deps: only ambition_menu + bevy + bevy_lunex. Names no game content.
Interdependency shape (already fairly layered)

ambition_input
   └─ ambition_ui_nav (nav primitives)
        ├─ ambition_inventory_ui (nav state)
        └─ ambition_dialog (+engine_core,sfx,persistence)
   ambition_persistence
        └─ ambition_settings_menu (settings IR)
ambition_menu (framework, bevy-only)
   └─ ambition_menu_kaleidoscope (alt renderer)
ambition_game_shell (routing) ──opt──> ambition_menu
This is already close to the "nav primitives / menu framework / specific menus" layering you're after. ambition_ui_nav is the nav-primitive layer; ambition_menu is the framework; ambition_settings_menu/ambition_inventory_ui are specific menus. The awkward parts:

ambition_inventory_ui (149 LOC) is barely a crate — one model + one enum over ui_nav. Strong consolidation candidate: fold into ambition_ui_nav (as a nav::inventory submodule) or into ambition_menu.
ambition_menu and ambition_settings_menu don't depend on each other even though a settings screen is "a menu." They meet only at the host, which builds a MenuPageModel from the SettingsMenuModel. That's arguably correct (data model vs specific-menu IR), but worth confirming in planning whether settings should render through ambition_menu's page model rather than being a parallel IR.
ambition_game_shell is misfiled with the UI cluster — it's routing/lifecycle, not a menu; its only UI edge is the optional ambition_menu (pause/launcher). It belongs in a "shell/runtime" layer, not the menu layer.
ambition_dialog is UI-framework-shaped (like a menu) but lives independently and is consumed by render. Fine as a peer of ambition_menu under ambition_ui_nav.
Consolidation verdict: don't mass-merge — the layering intent is mostly present. The concrete wins are (1) absorb ambition_inventory_ui into ui_nav or menu, (2) relabel game_shell out of the UI cluster, (3) decide whether settings renders through the MenuPageModel seam so there's one menu framework rather than two IRs.

6. Ambition-game-specific content baked into these "engine" crates
This is the material most relevant to the north-star oracle. Found the following hardcoded content inside supposedly reusable crates:

ambition_render — the biggest offender. Several modules and asset paths are named after specific Ambition content, compiled into the shared renderer regardless of game:

rendering/pirate_weapon.rs (234 LOC): "Pirate gun-sword", PirateWeaponVisual, hardcoded const WEAPON_SHEET_PATH = "sprites/lasersword_with_guns_spritesheet.png", PirateRaider/PirateHeavy sizing. Wholly Ambition content.
rendering/deep_dream.rs (400 LOC): "deep-dream shader experiment for the Puppy Slug enemy", PuppySlugDeepDreamMaterial, SHADER_ASSET_PATH = "shaders/puppy_slug_deep_dream.wgsl", add_puppy_slug_deep_dream_material_plugin. A named enemy's one-off shader in the engine renderer.
Hardcoded content asset paths throughout: item_visuals.rs (sprites/lasersword_spritesheet.png, sprites/props/{axe,javelin,gunsword,gauntlet_*}.png, doc ref to "Mary-O's milk carton"), shrine_visuals.rs (sprites/shrine_spritesheet.png), mark_beacon.rs (sprites/props/mark_beacon.png), slash_visuals.rs (robot_slash sheet).
Doc comments naming content: world.rs "Sanic-specific drawing" (chain), "Mary-O's bricks"; worn_binder_tests.rs spawns "sanic".
Ability-visual modules that are mechanic-generic but still always-compiled: bubble_shield.rs, morph_ball.rs, gravity_visuals.rs, gate_portal_visuals.rs. Less content-specific than pirate/puppy-slug, but still concrete mechanics a different platformer wouldn't have.
These directly threaten the oracle: a new platformer that adds a content crate still drags in the pirate gun-sword and puppy-slug shader systems. They read the seam correctly (sim_view / read-models), so they're seam-clean — they're just in the wrong crate. Candidate for extraction into per-mechanic presentation crates (like portal_presentation already is) or a game content-presentation crate.

ambition_portal_presentation: gun_visuals.rs + PortalGunArt — "Ambition's portal-gun sprites." Sequestered behind a host-loaded resource seam (paths are host-supplied), so this is a seam, not hardcoded content. Acceptable.

ambition_game_shell: ShellLauncherPresentation::default() hardcodes title: "Ambition" — but it's an overridable field (a default string, not a structural coupling). All "sanic"/"Sanic" occurrences in experience.rs are inside #[test] fixtures, not shipping code.

ambition_dialog: clean. The only content-ish string, "Shadow Oni Leader" in speech_sfx.rs:271, is in a test.

ambition_menu / ambition_settings_menu / ambition_ui_nav / ambition_inventory_ui / ambition_menu_kaleidoscope: clean. Settings labels ("Radio", "Quit to Title", "Developer") are generic system-menu vocabulary; the menu crates carry no character/story/Ambition-specific data (menu policy enforces content-freedom). Kaleidoscope "names no game content."

Content-coupling summary for the oracle
The UI cluster and portal_presentation largely pass the "add a content crate, don't edit core" test. ambition_render fails it: named-content modules (pirate_weapon, deep_dream/Puppy-Slug) and a spray of hardcoded sprites/... paths mean the shared renderer knows about specific Ambition entities. The cleanest lever for the decomposition session is to pull mechanic/content visuals out of ambition_render into content-owned presentation crates (mirroring how ambition_portal_presentation already sits outside the core renderer), leaving ambition_render as pure sim-view→Bevy plumbing.

Content-coupling summary for the oracle
The UI cluster and portal_presentation largely pass the "add a content crate, don't edit core" test. ambition_render fails it: named-content modules (pirate_weapon, deep_dream/Puppy-Slug) and a spray of hardcoded sprites/... paths mean the shared renderer knows about specific Ambition entities. The cleanest lever for the decomposition session is to pull mechanic/content visuals out of ambition_render into content-owned presentation crates (mirroring how ambition_portal_presentation already sits outside the core renderer), leaving ambition_render as pure sim-view→Bevy plumbing.

All six recon agents have reported. Before writing the plan doc, let me verify the highest-impact claims with quick spot checks (dead render deps, the ambition_items enum, and the repo's own doc-check scripts).

Bash Spot-check key agent claims
IN
echo "=== render: ambition_input / ambition_interaction use sites ==="; grep -rn "ambition_input\|ambition_interaction" crates/ambition_render/src --include='*.rs' | head; echo "=== items enum ==="; grep -n "PuppySlugGun\|PortalGun\|GunSword" crates/ambition_items/src/lib.rs | head -5; echo "=== app menu content grep ==="; grep -rln "cut_rope\|gnu_ton\|goblin\|puppy_slug" game/ambition_app/src/menu/ | head -5; echo "(empty = clean)"

OUT
=== render: ambition_input / ambition_interaction use sites ===
crates/ambition_render/src/cutscene/mod.rs:176:                        // `ambition_input::presets::ControlPreset::input_map`
=== items enum ===
69:    PortalGun = 0,
72:    GunSword = 3,
73:    PuppySlugGun = 4,
356:        Item::PortalGun,
359:        Item::GunSword,
=== app menu content grep ===
(empty = clean)
Bash Run the planning-docs consistency check
IN
python scripts/check_agent_kb.py 2>&1 | tail -15

OUT
/bin/bash: line 1: python: command not found
Bash Run KB check and look for stale smb1 path references
IN
python3 scripts/check_agent_kb.py 2>&1 | tail -8; echo "---"; grep -rn "ambition_demo_smb1" docs/planning/status.md docs/planning/tracks.md scripts/check_agent_kb.py 2>/dev/null | head -8; ls game/ | grep -i "smb1\|mary"

OUT
Agent KB warnings (non-fatal):
- docs/planning has 11520 lines (soft budget 10500); archive sections that are long done rather than trimming live plans
Agent KB check failed:
- inline-test review markers disagree with HEAD (unreviewed (add an inline-test marker): game/ambition_demo_mary_o/src/flag.rs, game/ambition_demo_mary_o/src/lib.rs; no longer >=200 lines (drop the marker): game/ambition_demo_smb1/src/flag.rs)
- module-size evidence stale (waivers, unwaived-violations, stale-waivers, invalid-waivers): status says (1, 0, 0, 0), source has (0, 0, 0, 0)
---
docs/planning/tracks.md:158:- `game/ambition_demo_smb1/src/flag.rs`
docs/planning/status.md:33:| Super Mary-O | **PARTIAL.** Equipment data/mechanism, scroll policy, and flag sequence exist. Pickup wiring, live body-scale read-fold, and the full game shell remain. | `game/ambition_demo_smb1`; [`demos/super-mary-o.md`] (demos/super-mary-o.md) | Pickup/equip path, collision/render scale consumption, enemies/HUD/results, and the headless 1-1 acceptance run. |
docs/planning/status.md:39:<!-- planning-evidence: inline-test path=game/ambition_demo_smb1/src/flag.rs kind=behavioral-local disposition=maintainer-review-pending -->
docs/planning/status.md:86:- **Super Mary-O** — corroborate: `cargo test -p ambition_demo_smb1`. Establishes
ambition_demo_mary_o
ambition_demo_mary_o_app
Bash Inspect KB checker marker rules and mary_o inline tests
IN
grep -n "inline-test\|kind=\|disposition" scripts/check_agent_kb.py | head -30; echo "---"; awk '/cfg\(test\)/,0' game/ambition_demo_mary_o/src/lib.rs | head -30; echo "==="; grep -c "" game/ambition_demo_mary_o/src/lib.rs game/ambition_demo_mary_o/src/flag.rs

OUT
195:    r"<!--\s*planning-evidence:\s*inline-test\s+path=(\S+)"
196:    r"\s+kind=(behavioral-local|guardrail)"
197:    r"\s+disposition=(maintainer-review-pending|maintainer-approved-inline|extract-pending)\s*-->"
854:    # `kind` finding and a `disposition`. 200 is a review PROXY (test-placement.md);
855:    # an agent records `kind=behavioral-local` + `disposition=maintainer-review-
859:    documented = {path for path, _kind, _disposition in reviewed}
866:            parts.append("unreviewed (add an inline-test marker): " + ", ".join(missing))
869:        fail(errors, "inline-test review markers disagree with HEAD (" + "; ".join(parts) + ")")
875:    for path, _kind, disposition in reviewed:
876:        if disposition == "maintainer-approved-inline" and path not in MAINTAINER_APPROVED_INLINE:
879:                f"inline-test marker for {path} claims maintainer-approved-inline but is not in "
966:        "<!-- planning-evidence: inline-test path=a/b.rs "
967:        "kind=behavioral-local disposition=maintainer-review-pending -->"
969:        fail(errors, "planning checker self-test failed: inline-test marker parser")
---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mary_o_demo_content_plugin_installs() {
        let mut app = App::new();
        add_demo_content(&mut app);
        let catalog = app
            .world()
            .resource::<ambition::characters::actor::character_catalog::CharacterCatalog>();
        assert!(catalog.get(provider::MARY_O_CHARACTER_ID).is_some());
        // Mary-O's authored grant list composes to her platformer moveset —
        // run+jump, air jump, wall mobility, fast fall — and NOTHING from the
        // full Ambition kit (blink/dash/fly/attack). This is her AbilityBase; the
        // session mask can only narrow it, never restore the sandbox kit.
        let mary_o_kit = catalog
            .ability_set(provider::MARY_O_CHARACTER_ID)
            .expect("Mary-O authors a grant list");
        assert_eq!(
            mary_o_kit,
            ambition::engine_core::AbilitySet::compose(&[
                ambition::engine_core::AbilityGrant::RunJump,
                ambition::engine_core::AbilityGrant::AirJump,
                ambition::engine_core::AbilityGrant::WallMobility,
                ambition::engine_core::AbilityGrant::FastFall,
            ]),
            "Mary-O composes to the classic platformer moveset"
        );
        assert!(
===
game/ambition_demo_mary_o/src/lib.rs:1019
game/ambition_demo_mary_o/src/flag.rs:430
Now I have everything. Synthesizing and writing the deliverables: the recon doc, the stale-docs fixes, and the ledger re-baseline.

Update Todos

Map crate landscape: LOC, deps, growth since last ledger (done inline)

Read existing planning docs, rulings, smells (done inline)

Collect 6 exploration-agent reports (actors, foundation, content-data, presentation, app/facade, midlayer)

Write recon+plan doc into docs/planning

Fix stale planning docs (smb1 paths, module-size marker, ledger re-baseline)

Verify doc edits, commit, summarize verdicts for Jon
