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

use ambition_engine_core as ae;
use ambition_platformer_primitives::lifecycle::SessionCommands;

/// Room-transition slot for *content-side* reset work (named boss
/// arenas, story state). Content plugins register their reset systems in
/// this set; the host anchors the set into the room-transition chain, and
/// machinery that must run after content resets (e.g. gravity
/// reset-to-default) orders against the SET — generic plugins never name
/// a content system.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRoomResetSet;

/// Player-input-phase slot for content systems that FOLLOW UP a closed
/// dialogue with a request (e.g. emit [`RoomReplayRequested`] after a
/// "try again" conversation ends). Content plugins register emitters in
/// this set; the host anchors it before the replay consumer so a request
/// lands the same frame it is emitted.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentDialogueFollowupSet;

/// Player-input-phase slot for content systems that reset *content-named*
/// per-attempt state when a [`RoomReplayRequested`] fires (e.g. clear a named
/// boss's persisted "cleared" record before the room replays). Content plugins
/// register their reset systems here; the host anchors the set before its
/// generic replay consumer so the content reset lands the same frame the
/// request does — the consumer never names content.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRoomReplayResetSet;

/// Replay the ACTIVE room in place: reset the controlled player to the
/// room spawn and tear down + respawn the room's scoped state (bosses,
/// features), leaving progress outside the room untouched. CONTENT emits
/// this (a "try again" beat, a challenge retry); the host's replay
/// consumer drains it. Engine-generic vocabulary — the message names no
/// content.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomReplayRequested;

use crate::boss_encounter::BossEncounterRegistry;
use crate::encounter::{EncounterMusicRequest, EncounterRegistry};
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::rooms::RoomSet;
use crate::world::physics;
use ambition_persistence::quest::QuestRegistry;
use ambition_persistence::save::SandboxSave;
use ambition_platformer_primitives::schedule::SimScheduleExt;

/// Bundles sim-state resources so `process_sandbox_reset_request`
/// stays within Bevy's 16-SystemParam limit.
#[derive(SystemParam)]
pub struct ResetPlayState<'w> {
    sim_state: ResMut<'w, crate::SandboxSimState>,
    clock_resets: MessageWriter<'w, crate::time::time_control::ClockResetRequest>,
    moving_platforms: ResMut<'w, ambition_world::collision::MovingPlatformSet>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    character_roster: Res<'w, crate::features::CharacterRoster>,
    boss_catalog: Res<'w, crate::boss_encounter::BossCatalog>,
    /// The installed placement-lowering authority — reset re-stages the start
    /// room's placements through the SAME registry setup/transition/restore use.
    placement_lowering: Res<'w, crate::world::placements::PlacementLoweringRegistry>,
    /// The installed room-content staging seam — same rule as the placement
    /// registry: reset re-stages content-staged occupants, one authority.
    content_staging: Res<'w, crate::features::RoomContentStagingRegistry>,
}

/// Cross-system trigger for "wipe the save and rebuild the runtime."
/// Set `request = true` from anywhere; the next
/// `process_sandbox_reset_request` tick consumes it.
#[derive(Resource, Clone, Default, Debug)]
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
    mut music_request: ambition_platformer_primitives::lifecycle::SessionWorldMut<
        EncounterMusicRequest,
    >,
    mut play_state: ResetPlayState<'_>,
    mut room_set: ambition_platformer_primitives::lifecycle::SessionWorldMut<RoomSet>,
    mut world: ambition_platformer_primitives::lifecycle::SessionWorldMut<
        ambition_engine_core::RoomGeometry,
    >,
    tuning: Res<ambition_dev_tools::dev_tools::EditableMovementTuning>,
    mut respawn_visuals: MessageWriter<crate::session::RespawnRoomVisualsRequested>,
    mut commands: SessionCommands<'_, '_>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // E1: the live wave encounters are entities now; despawn them so
    // `populate_encounter_registry` (which the cleared `specs_loaded` flag
    // re-arms) respawns them fresh from the empty save next frame.
    encounter_entities: Query<Entity, With<ambition_encounter::Encounter>>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut crate::features::MotionModel,
            &mut crate::actor::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut crate::avatar::PlayerBlinkCameraState,
            &mut crate::actor::BodyMelee,
            &mut crate::avatar::PlayerSafetyState,
        ),
        // PRIMARY-only: the reset warps THE player to the start-room spawn. A
        // brain-driven clone is a transient demo body; scoping to the primary keeps
        // the reset working once a second PlayerEntity exists (bare single_mut would Err).
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    if !request.request {
        return;
    }
    request.request = false;
    let Some(session_scope) = commands.spawn_scope() else {
        // A shell host may receive a late reset request after gameplay has
        // retired. With no active session there is no world to reset and no
        // scope that may own the replacement entities.
        return;
    };

    let start_index = room_set.start;
    let room_plan = crate::rooms::RoomConstructionPlan::prepare_from_parts(
        &room_set,
        start_index,
        &play_state.placement_lowering,
        &play_state.content_staging,
        &play_state.character_catalog,
        &play_state.character_roster,
        &play_state.boss_catalog,
        session_scope,
    )
    .unwrap_or_else(|error| panic!("sandbox reset room preflight failed: {error}"));

    info!(
        target: "ambition::reset",
        "sandbox reset requested — wiping save, registries, and runtime"
    );

    // 1. Wipe the persisted save. Change-detection will trigger the
    //    autosave system to write the empty save to disk this tick.
    *save.data_mut() = ambition_persistence::save_data::SandboxSaveData::default();

    // 2. Clear registries. Setting them to Default flips
    //    `specs_loaded` / `initialized` back to false so the populate
    //    Update systems re-run on the next frame.
    *encounter_registry = EncounterRegistry::default();
    for entity in &encounter_entities {
        commands.entity(entity).despawn();
    }
    *boss_registry = BossEncounterRegistry::default();
    *quest_registry = QuestRegistry::default();
    **music_request = EncounterMusicRequest::default();

    // 3-5. Commit the already-prepared canonical start-room construction.
    // Invalid authored content was rejected above, before save or registry
    // mutation. The same artifact drives transition, hot reload, and restore.
    room_plan.retire_outgoing(
        &mut commands,
        room_visuals
            .iter()
            .map(|(entity, physics_entity)| (entity, physics_entity.is_some())),
        None,
    );
    room_plan.commit_deferred(
        &mut commands,
        &mut room_set,
        &mut world,
        &mut play_state.moving_platforms.0,
    );

    // 6. Reset the player to the start room's spawn point.
    play_state
        .clock_resets
        .write(crate::time::time_control::ClockResetRequest::sim_clock(
            crate::time::time_control::ClockRequester::Engine,
            "sandbox_reset",
        ));
    play_state.sim_state.room_transition_cooldown = 0.0;
    // Reset the ECS authority directly so the next player tick frame
    // starts from the spawn position. Also zero animation state so post-reset
    // frames don't continue a mid-air slash or dash-startup pose.
    if let Ok((
        mut cluster_item,
        mut motion_model,
        mut anim,
        mut combat,
        mut blink_cam,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
    {
        let mut clusters = cluster_item.as_clusters_mut();
        ae::reset_body_clusters(
            &mut motion_model,
            &mut clusters,
            room_plan.spec().world.spawn,
        );
        // reset_body_clusters uses DEFAULT_TUNING for the post-reset
        // dash/jump refresh; redo with the live tuning so a F3
        // editable-tuning session sees its overridden air_jumps /
        // dash_charge_count immediately after a reset.
        ae::refresh_movement_resources_clusters(
            clusters.abilities,
            clusters.dash,
            clusters.jump,
            tuning.as_engine().air_jumps,
        );
        clusters.mana.meter.refill_full();
        anim.reset();
        combat.reset();
        combat.hit_flash = 0.18;
        blink_cam.reset();
        attack.clear();
        safety.last_safe_pos = world.0.spawn;
    }
    // 7. Respawn the static world visuals + parallax for the start room.
    //    Without this, the despawn in step 3 leaves the scene empty until
    //    something else (LDtk reload, room transition) rebuilds it. The visual
    //    respawn is a PRESENTATION concern, so the sim only emits the request —
    //    the render layer's `respawn_room_visuals_on_request` consumes it and
    //    reads the active room from `RoomSet`. A headless build has no consumer
    //    and correctly skips the (purely visual) respawn.
    respawn_visuals.write(crate::session::RespawnRoomVisualsRequested);
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
            With<ambition_portal::PlacedPortal>,
            With<ambition_portal::PortalShot>,
            With<ambition_portal::PortalGunPickup>,
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
            &mut ambition_characters::brain::ActionSet,
            Option<&crate::items::pickup::StashedActionSet>,
        ),
        With<crate::actor::PlayerEntity>,
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
        commands
            .entity(player)
            .remove::<ambition_portal::PortalGun>();
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
        let sim = app.sim_schedule();
        app.add_message::<crate::session::RespawnRoomVisualsRequested>();
        app.add_message::<RoomReplayRequested>();
        app.add_systems(
            sim,
            // Clear transient portals/held-items/summons BEFORE the request flag
            // is consumed by the main reset processor.
            (
                clear_transient_on_sandbox_reset,
                process_sandbox_reset_request,
            )
                .chain()
                .in_set(crate::schedule::SandboxSet::ResetProcessing),
        );
    }
}

#[cfg(test)]
mod tests;
