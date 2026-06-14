//! Sandbox-wide gameplay reset.
//!
//! Setting [`SandboxResetRequested::request`] clears gameplay progress and rebuilds
//! runtime state so the player returns to the world's start room with encounters,
//! quests, switches, bosses, and flags reset.
//!
//! Reset replaces `SandboxSaveData`, resets encounter/boss/quest registries so
//! their populate systems rebuild from LDtk plus the empty save, despawns
//! `RoomScopedEntity` instances, warps/refills the player, and re-seeds authored
//! moving-platform state for the start room.
//!
//! It does **not** reset user settings, keyboard preset selection, or global app
//! preferences. Dev-tool gameplay flags stored on player clusters are reset with
//! the player so a manual reset gives a clean gameplay slate.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::engine_core as ae;

/// Room-transition slot for *content-side* reset work (named boss
/// arenas, story state). The app assembly places the content layer's
/// reset systems in this set; machinery that must run after content
/// resets (e.g. gravity reset-to-default) orders against the SET, so
/// generic plugins never name a content system.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRoomResetSet;

use crate::assets::game_assets::GameAssets;
use crate::boss_encounter::BossEncounterRegistry;
use crate::encounter::{EncounterMusicRequest, EncounterRegistry};
use crate::persistence::save::SandboxSave;
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::presentation::rendering::spawn_room_visuals;
use crate::quest::QuestRegistry;
use crate::rooms::RoomSet;
use crate::world::physics;
use crate::world::platforms;

/// Bundles sim-state resources so `process_sandbox_reset_request`
/// stays within Bevy's 16-SystemParam limit.
#[derive(SystemParam)]
pub struct ResetPlayState<'w> {
    sim_state: ResMut<'w, crate::SandboxSimState>,
    clock: ResMut<'w, crate::time::clock_state::ClockState>,
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
    *save.data_mut() = crate::persistence::save_data::SandboxSaveData::default();

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
    play_state.clock.time_scale = 1.0;
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
    #[cfg(feature = "portal")] transient: Query<
        Entity,
        Or<(
            With<crate::portal::PlacedPortal>,
            With<crate::portal::PortalShot>,
            With<crate::portal::PortalGunPickup>,
            With<crate::items::pickup::GroundItem>,
            With<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>,
        )>,
    >,
    #[cfg(not(feature = "portal"))] transient: Query<
        Entity,
        Or<(
            With<crate::items::pickup::GroundItem>,
            With<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>,
        )>,
    >,
    mut players: Query<
        (
            Entity,
            &mut crate::brain::ActionSet,
            Option<&crate::items::pickup::StashedActionSet>,
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
            .remove::<crate::items::pickup::StashedActionSet>();
        commands
            .entity(player)
            .remove::<crate::features::HeldItem>();
        #[cfg(feature = "portal")]
        commands.entity(player).remove::<crate::portal::PortalGun>();
        // Clear any Mark/Recall mark too, so re-equipping after a reset can't
        // recall to a position from before the room was rebuilt.
        commands
            .entity(player)
            .remove::<crate::abilities::traversal::mark_recall::PlayerMark>();
    }
}

/// Schedules [`process_sandbox_reset_request`] into [`SandboxSet::ResetProcessing`].
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
mod tests;
