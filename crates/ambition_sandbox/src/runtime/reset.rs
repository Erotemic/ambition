//! Sandbox-wide reset: wipe the save and rebuild the runtime so the
//! player ends up in the world's start room with every NPC alive,
//! every encounter armed, and every persisted flag cleared.
//!
//! Triggered by setting [`SandboxResetRequested::request`] from
//! anywhere — today the pause-menu "Reset Sandbox" item is the only
//! caller, but the seam is generic so future debug hotkeys / dev
//! tools can reuse it.
//!
//! ## What gets reset
//!
//! - **Save**: replaced with `SandboxSaveData::default()` (encounters,
//!   switches, bosses, quests, flags all cleared). Bevy's
//!   change-detection picks up the mutation and `autosave_sandbox_save`
//!   writes the empty save to disk on the same tick, so the reset
//!   survives a fresh game launch.
//! - **Encounter / boss / quest registries**: replaced with their
//!   `Default` values, which sets `specs_loaded` / `initialized` to
//!   false. The populate Update systems
//!   (`populate_encounter_registry`, etc.) re-run on the next frame
//!   and rebuild the registries from the LDtk project + the now-empty
//!   save.
//! - **`RoomScopedEntity` entities** (including all `RoomVisual`s):
//!   despawned. The room-visual respawn path runs after the active
//!   room flips back to the start.
//! - **Player entity**: warped to the start room's spawn via
//!   `player.reset_to(world.spawn)` — restores movement resources and
//!   refills mana. Immediately afterward, the moving-platform state
//!   is re-seeded from the start room's LDtk-authored `MovingPlatform`,
//!   falling back to the legacy test/reference platform only for rooms
//!   that have no authored platform yet.
//! - **Active room**: `room_set.active` resets to `room_set.start`
//!   (captured at `RoomSet::from_parts` time) so the player ends up
//!   wherever a fresh game would start, not wherever they happened
//!   to be when they hit reset.
//!
//! ## What does NOT get reset
//!
//! - User settings (audio mix, controls, video, gameplay tuning) —
//!   those live in `crate::persistence::settings::UserSettings` and are not part
//!   of the sandbox state. Reset is about gameplay progress only.
//! - Keyboard preset selection.
//! - Dev-tool toggles (the F3 stats editor's invincible flag etc.)
//!   live on `PlayerOffense` / `PlayerMana` and ARE reset by
//!   `reset_player_clusters`, because the caller also runs
//!   `mana.refill_full()` — that's actually a feature: a player who
//!   accidentally enabled invincibility and wants to play "for real"
//!   gets a clean slate.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::engine_core as ae;

use crate::assets::game_assets::GameAssets;
use crate::boss_encounter::BossEncounterRegistry;
use crate::content::quest::QuestRegistry;
use crate::encounter::{EncounterMusicRequest, EncounterRegistry};
use crate::persistence::save::SandboxSave;
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::presentation::rendering::spawn_room_visuals;
use crate::rooms::RoomSet;
use crate::world::physics;
use crate::world::platforms;

/// Bundles sim-state resources so `process_sandbox_reset_request`
/// stays within Bevy's 16-SystemParam limit.
#[derive(SystemParam)]
pub struct ResetPlayState<'w> {
    sim_state: ResMut<'w, crate::SandboxSimState>,
    physics_settings: Res<'w, crate::world::physics::PhysicsSandboxSettings>,
    moving_platforms: ResMut<'w, crate::MovingPlatformSet>,
}

/// Cross-system trigger for "wipe the save and rebuild the runtime."
/// Set `request = true` from anywhere; the next
/// `process_sandbox_reset_request` tick consumes it.
#[derive(Resource, Default, Debug)]
pub struct SandboxResetRequested {
    pub request: bool,
}

impl SandboxResetRequested {
    pub fn request(&mut self) {
        self.request = true;
    }
}

/// Bevy system: drains a pending reset request and rebuilds the
/// sandbox state. Idempotent on `request = false` (early returns).
///
/// Schedule: runs in `Update` AFTER the player tick so a reset
/// triggered mid-frame doesn't race with in-flight gameplay
/// mutations, and BEFORE the populate systems so when they run on
/// the next frame the cleared registries see fresh state.
pub fn process_sandbox_reset_request(
    mut request: ResMut<SandboxResetRequested>,
    mut save: ResMut<SandboxSave>,
    mut encounter_registry: ResMut<EncounterRegistry>,
    mut boss_registry: ResMut<BossEncounterRegistry>,
    mut quest_registry: ResMut<QuestRegistry>,
    mut music_request: ResMut<EncounterMusicRequest>,
    mut play_state: ResetPlayState<'_>,
    mut room_set: ResMut<RoomSet>,
    mut world: ResMut<crate::GameWorld>,
    tuning: Res<crate::dev::dev_tools::EditableMovementTuning>,
    assets: Option<Res<GameAssets>>,
    mut commands: Commands,
    mut banner: ResMut<crate::features::GameplayBanner>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerBlinkCameraState,
            &mut crate::player::ActivePlayerAttack,
            &mut crate::player::PlayerSafetyState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    if !request.request {
        return;
    }
    request.request = false;

    info!(
        target: "ambition::reset",
        "sandbox reset requested — wiping save, registries, and runtime"
    );

    // 1. Wipe the persisted save. Change-detection will trigger the
    //    autosave system to write the empty save to disk this tick.
    *save.data_mut() = crate::save::SandboxSaveData::default();

    // 2. Clear registries. Setting them to Default flips
    //    `specs_loaded` / `initialized` back to false so the populate
    //    Update systems re-run on the next frame.
    *encounter_registry = EncounterRegistry::default();
    *boss_registry = BossEncounterRegistry::default();
    *quest_registry = QuestRegistry::default();
    music_request.desired_track = None;

    // 3. Despawn all room visuals (and their physics colliders if
    //    Avian2D installed any). The room-visual respawn path that
    //    the player tick / room-load already use will rebuild them
    //    once the active room flip below kicks in.
    for (entity, physics_entity) in &room_visuals {
        if physics_entity.is_some() {
            physics::retire_physics_entity(&mut commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // 5. Warp the active room back to the start room (where the
    //    player begins on a fresh game). `RoomSet::start` was
    //    captured at construction.
    let start_index = room_set.start;
    let start_spec = room_set.set_active(start_index).clone();
    world.0 = start_spec.world.clone();

    // 6. Reset the player to the start room's spawn point.
    play_state.sim_state.time_scale = 1.0;
    play_state.sim_state.room_transition_cooldown = 0.0;
    // Reset the ECS authority directly so the next player tick frame
    // starts from the spawn position. Also zero animation state so post-reset
    // frames don't continue a mid-air slash or dash-startup pose.
    if let Ok((mut cluster_item, mut anim, mut combat, mut blink_cam, mut attack, mut safety)) =
        player_q.single_mut()
    {
        let mut clusters = cluster_item.as_clusters_mut();
        ae::reset_player_clusters(&mut clusters, world.0.spawn);
        // reset_player_clusters uses DEFAULT_TUNING for the post-reset
        // dash/jump refresh; redo with the live tuning so a F3
        // editable-tuning session sees its overridden air_jumps /
        // dash_charge_count immediately after a reset.
        ae::refresh_movement_resources_clusters(
            clusters.abilities,
            clusters.dash,
            clusters.jump,
            tuning.as_engine(),
        );
        clusters.mana.meter.refill_full();
        anim.reset();
        combat.reset();
        combat.flash_timer = 0.18;
        blink_cam.reset();
        attack.clear();
        safety.last_safe_pos = world.0.spawn;
    }
    crate::features::spawn_room_feature_entities(&mut commands, &start_spec);
    play_state.moving_platforms.0 = platforms::moving_platforms_for_room(&start_spec);

    // 7. Respawn the static world visuals + moving platform for the
    //    start room. Without this, the despawn in step 4 leaves the
    //    scene empty until something else (LDtk reload, room transition)
    //    triggers a fresh `spawn_room_visuals`. Mirrors the pattern in
    //    `app::world_flow::load_room` and the LDtk hot-reload path.
    crate::presentation::rendering::spawn_parallax_layers(
        &mut commands,
        &world.0,
        &start_spec.metadata,
        assets.as_deref(),
    );
    spawn_room_visuals(
        &mut commands,
        &start_spec,
        *play_state.physics_settings,
        assets.as_deref(),
    );
    platforms::spawn_moving_platforms(&mut commands, &world.0, &play_state.moving_platforms.0);

    // 8. User feedback: surface a banner so the reset is visibly
    //    confirmed. The HUD's banner channel is the same one used
    //    for "ARENA CLEAR" etc.
    banner.show("SANDBOX RESET", 3.0);
}

/// On a sandbox reset, despawn the transient world items the registry/room
/// reset doesn't touch — placed portals + in-flight shots, the portal-gun
/// pickup, thrown/dropped ground items, and summoned puppy-slug allies — and
/// strip the player's held state (`HeldItem` / `StashedActionSet` / `PortalGun`),
/// restoring its base `ActionSet`. Runs BEFORE
/// [`process_sandbox_reset_request`] consumes the request flag, so it sees the
/// same reset tick (Jon: "portals and held items don't reset on sandbox reset —
/// they should").
#[allow(clippy::type_complexity)]
pub fn clear_transient_on_sandbox_reset(
    request: Res<SandboxResetRequested>,
    mut commands: Commands,
    transient: Query<
        Entity,
        Or<(
            With<crate::portal::Portal>,
            With<crate::portal::PortalProjectile>,
            With<crate::portal::PortalGunPickup>,
            With<crate::item_pickup::GroundItem>,
            With<crate::puppy_slug_gun::PuppySlugAlly>,
        )>,
    >,
    mut players: Query<
        (
            Entity,
            &mut crate::brain::ActionSet,
            Option<&crate::item_pickup::StashedActionSet>,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    if !request.request {
        return;
    }
    for entity in &transient {
        commands.entity(entity).despawn();
    }
    for (player, mut action_set, stashed) in &mut players {
        if let Some(stash) = stashed {
            *action_set = stash.0.clone();
        }
        commands
            .entity(player)
            .remove::<crate::item_pickup::StashedActionSet>();
        commands
            .entity(player)
            .remove::<crate::features::HeldItem>();
        commands.entity(player).remove::<crate::portal::PortalGun>();
        // Clear any Mark/Recall mark too, so re-equipping after a reset can't
        // recall to a position from before the room was rebuilt.
        commands
            .entity(player)
            .remove::<crate::mark_recall::PlayerMark>();
    }
}

/// Module-local Bevy plugin: schedules
/// [`process_sandbox_reset_request`] into [`SandboxSet::ResetProcessing`].
///
/// Carved out of `app/plugins.rs::register_reset_processing_systems`
/// per OVERNIGHT-TODO #6. The reset machinery lives entirely in this
/// module, so its schedule registration belongs here too.
pub struct SandboxResetSchedulePlugin;

impl Plugin for SandboxResetSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            // Clear transient portals/held-items/summons BEFORE the request flag
            // is consumed by the main reset processor.
            (
                clear_transient_on_sandbox_reset,
                process_sandbox_reset_request,
            )
                .chain()
                .in_set(crate::app::SandboxSet::ResetProcessing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::dev_tools::EditableMovementTuning;
    use crate::player::PlayerBlinkCameraState;
    use crate::GameWorld;

    /// Pin the request resource's defaults: a fresh app starts with
    /// no reset queued. Important because the reset processor must
    /// be a no-op when nothing has been requested.
    #[test]
    fn request_default_is_idle() {
        let req = SandboxResetRequested::default();
        assert!(!req.request);
    }

    /// `request()` sets the flag; the processor consumes it.
    #[test]
    fn request_helper_sets_the_flag() {
        let mut req = SandboxResetRequested::default();
        req.request();
        assert!(req.request);
    }

    #[test]
    fn sandbox_reset_clears_portals_held_items_and_summons() {
        let mut app = App::new();
        app.insert_resource(SandboxResetRequested::default());
        app.add_systems(Update, clear_transient_on_sandbox_reset);

        let ground = app
            .world_mut()
            .spawn(crate::item_pickup::GroundItem {
                spec: crate::item_pickup::axe_spec(),
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                half_extent: ae::Vec2::splat(18.0),
            })
            .id();
        let ally = app
            .world_mut()
            .spawn(crate::puppy_slug_gun::PuppySlugAlly)
            .id();
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerEntity,
                crate::brain::ActionSet::default(),
                crate::item_pickup::StashedActionSet(crate::brain::ActionSet::default()),
                crate::features::HeldItem::new(crate::item_pickup::axe_spec()),
                crate::portal::PortalGun::default(),
            ))
            .id();

        // No reset queued → nothing changes.
        app.update();
        assert!(app
            .world()
            .get::<crate::item_pickup::GroundItem>(ground)
            .is_some());
        assert!(app
            .world()
            .get::<crate::features::HeldItem>(player)
            .is_some());

        // Reset requested → transient entities despawn + player held-state stripped.
        app.world_mut()
            .resource_mut::<SandboxResetRequested>()
            .request = true;
        app.update();
        assert!(
            app.world()
                .get::<crate::item_pickup::GroundItem>(ground)
                .is_none(),
            "ground item despawned on reset"
        );
        assert!(
            app.world()
                .get::<crate::puppy_slug_gun::PuppySlugAlly>(ally)
                .is_none(),
            "summoned ally despawned on reset"
        );
        assert!(
            app.world()
                .get::<crate::features::HeldItem>(player)
                .is_none(),
            "held item removed from player"
        );
        assert!(
            app.world()
                .get::<crate::portal::PortalGun>(player)
                .is_none(),
            "portal gun removed from player"
        );
        assert!(
            app.world()
                .get::<crate::item_pickup::StashedActionSet>(player)
                .is_none(),
            "stashed action set cleared"
        );
    }

    fn dummy_world() -> ae::World {
        ae::World::new(
            "test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 1500.0),
                ae::Vec2::new(2000.0, 32.0),
            )],
        )
    }

    /// Build a minimal Bevy app wired with the reset processor and
    /// just enough resources for it to run: the request resource,
    /// the save, the three registries it clears, the music request,
    /// runtime + world + room set + tuning, and the relevant entity
    /// queries (empty here — no controllers / no room visuals to
    /// despawn in this synthetic harness).
    fn min_app() -> App {
        let mut app = App::new();
        let world = dummy_world();
        app.insert_resource(SandboxResetRequested::default());
        app.insert_resource(SandboxSave::default());
        app.insert_resource(EncounterRegistry::default());
        app.insert_resource(BossEncounterRegistry::default());
        app.insert_resource(QuestRegistry::default());
        app.insert_resource(EncounterMusicRequest::default());
        app.insert_resource(crate::features::GameplayBanner::default());
        // Spawn the player entity so process_sandbox_reset_request can query it.
        // Uses the full simulation bundle so every cluster component lands
        // — the reset path queries `PlayerClusterQueryData` which needs all
        // of them present.
        {
            let mut initial =
                crate::player::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all());
            ae::refresh_movement_resources_clusters(
                &initial.abilities,
                &mut initial.dash,
                &mut initial.jump,
                ae::DEFAULT_TUNING,
            );
            let health = crate::actor::Health::new(20);
            app.world_mut()
                .spawn(crate::player::PlayerSimulationBundle::from_scratch(
                    initial, health,
                ));
            let _ = PlayerBlinkCameraState::default();
        }
        app.insert_resource(crate::world::physics::PhysicsSandboxSettings::default());
        app.insert_resource(crate::MovingPlatformSet::default());
        app.insert_resource(crate::SandboxSimState::default());
        app.insert_resource(crate::SandboxDevState::default());
        app.insert_resource(GameWorld(world.clone()));
        // Construct a minimal RoomSet with one room so `start` and
        // `active` are both valid indices.
        let room_spec = crate::rooms::RoomSpec {
            id: "test".into(),
            world: world.clone(),
            loading_zones: Vec::new(),
            metadata: crate::rooms::RoomMetadata::default(),
            camera_zones: Vec::new(),
            kinematic_paths: Vec::new(),
            moving_platforms: Vec::new(),
            props: Vec::new(),
            ground_items: Vec::new(),
            portal_gun_spawns: Vec::new(),
            portals: Vec::new(),
            shrines: Vec::new(),
            gravity_zones: Vec::new(),
            hazards: Vec::new(),
            interactables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            breakables: Vec::new(),
            enemy_spawns: Vec::new(),
            boss_spawns: Vec::new(),
            debug_labels: Vec::new(),
        };
        app.insert_resource(crate::rooms::RoomSet::from_parts(
            "test",
            vec![room_spec],
            Vec::new(),
        ));
        app.insert_resource(EditableMovementTuning::default());
        app.add_systems(Update, process_sandbox_reset_request);
        app
    }

    /// Sanity: with no request, the processor leaves state alone.
    /// Set a save flag, run a tick, confirm it's still set.
    #[test]
    fn processor_is_a_noop_without_request() {
        let mut app = min_app();
        {
            let mut save = app.world_mut().resource_mut::<SandboxSave>();
            save.data_mut().set_flag("npc_test_hostile", true);
        }
        app.update();
        let save = app.world().resource::<SandboxSave>();
        assert!(save.data().flag("npc_test_hostile"));
    }

    /// The headline behavior: a queued request wipes the save flags
    /// (the thing the user noticed — NPCs persisting as dead) and
    /// flips registries back to "specs not loaded" so the populate
    /// systems repopulate on the next frame.
    #[test]
    fn processor_wipes_save_flags_and_clears_registries() {
        let mut app = min_app();
        // Pre-populate the state the user is trying to reset:
        // - a save flag remembering an NPC turned hostile
        // - a save flag remembering an encounter chest was looted
        // - "specs already loaded" on the registries
        {
            let mut save = app.world_mut().resource_mut::<SandboxSave>();
            save.data_mut().set_flag("npc_kira_hostile", true);
            save.data_mut()
                .set_flag("encounter_goblin_encounter_reward_dropped", true);
            save.data_mut().set_encounter(
                "goblin_encounter",
                crate::save::PersistedEncounterState::Cleared,
            );
        }
        {
            let mut reg = app.world_mut().resource_mut::<EncounterRegistry>();
            reg.specs_loaded = true;
        }
        {
            let mut reg = app.world_mut().resource_mut::<BossEncounterRegistry>();
            reg.specs_loaded = true;
        }
        {
            let mut reg = app.world_mut().resource_mut::<QuestRegistry>();
            reg.initialized = true;
        }
        // Queue the reset.
        {
            let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
            req.request();
        }
        app.update();

        // Save is wiped.
        let save = app.world().resource::<SandboxSave>();
        assert!(!save.data().flag("npc_kira_hostile"));
        assert!(!save
            .data()
            .flag("encounter_goblin_encounter_reward_dropped"));
        assert_eq!(
            save.data().encounter("goblin_encounter"),
            crate::save::PersistedEncounterState::Untouched
        );
        // Registries flag-flipped back so populate Update systems
        // will re-run on the next frame.
        let enc = app.world().resource::<EncounterRegistry>();
        assert!(!enc.specs_loaded);
        let boss = app.world().resource::<BossEncounterRegistry>();
        assert!(!boss.specs_loaded);
        let quest = app.world().resource::<QuestRegistry>();
        assert!(!quest.initialized);
        // Banner surfaces the action so the player can see it.
        assert_eq!(
            app.world()
                .resource::<crate::features::GameplayBanner>()
                .text,
            "SANDBOX RESET"
        );
        // Request consumed.
        let req = app.world().resource::<SandboxResetRequested>();
        assert!(!req.request);
    }

    /// After reset, the player is warped to the start room's spawn
    /// regardless of where they were before the reset. This is the
    /// "back to a fresh game" guarantee.
    #[test]
    fn processor_warps_player_to_start_spawn() {
        let mut app = min_app();
        {
            let mut q = app
                .world_mut()
                .query_filtered::<&mut crate::player::PlayerKinematics, With<crate::player::PlayerEntity>>(
                );
            if let Ok(mut kin) = q.single_mut(app.world_mut()) {
                kin.pos = ae::Vec2::new(1234.0, 1234.0);
            }
        }
        {
            let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
            req.request();
        }
        app.update();
        let world = app.world().resource::<GameWorld>();
        let expected_spawn = world.0.spawn;
        let mut q = app
            .world_mut()
            .query_filtered::<&crate::player::PlayerKinematics, With<crate::player::PlayerEntity>>(
            );
        let player_pos = q.single(app.world()).map(|k| k.pos).unwrap();
        assert_eq!(player_pos, expected_spawn);
    }

    /// Reset must restore the moving platform from the start room's
    /// authored LDtk platform, not from the old procedural fallback.
    #[test]
    fn processor_restores_authored_start_room_platform() {
        let mut app = min_app();
        let authored = crate::world::platforms::MovingPlatformState::from_authored(
            ae::Vec2::new(512.0, 900.0),
            ae::Vec2::new(128.0, 16.0),
            192.0,
            75.0,
        );
        {
            let mut room_set = app.world_mut().resource_mut::<RoomSet>();
            room_set.rooms[0].moving_platforms = vec![authored.clone()];
        }
        {
            let mut platform_set = app.world_mut().resource_mut::<crate::MovingPlatformSet>();
            platform_set.0 = vec![crate::world::platforms::MovingPlatformState::from_authored(
                ae::Vec2::new(10.0, 20.0),
                ae::Vec2::new(32.0, 8.0),
                64.0,
                10.0,
            )];
        }
        {
            let mut req = app.world_mut().resource_mut::<SandboxResetRequested>();
            req.request();
        }
        app.update();
        let platform_set = app.world().resource::<crate::MovingPlatformSet>();
        assert_eq!(platform_set.0[0].pos, authored.pos);
        assert_eq!(platform_set.0[0].size, authored.size);
    }
}
