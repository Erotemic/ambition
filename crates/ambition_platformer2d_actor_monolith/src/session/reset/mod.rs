//! Sandbox-wide gameplay reset.
//!
//! Setting [`NewGameResetRequested::request`] clears gameplay progress and rebuilds
//! runtime state so the player returns to the world's start room with encounters,
//! quests, switches, bosses, and flags reset.
//!
//! Reset replaces `AmbitionGameSaveData`, resets encounter/boss/quest registries so
//! their populate systems rebuild from LDtk plus the empty save, despawns
//! `RoomScopedEntity` instances, warps/refills the player, and re-seeds authored
//! moving-platform state for the start room.
//!
//! It does **not** reset user settings, keyboard preset selection, or global app
//! preferences. Dev-tool gameplay flags stored on player clusters are reset with
//! the player so a manual reset gives a clean gameplay slate.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::SessionCommands;

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

/// **The reset's preflight passed and the wipe is happening.**
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NewGameResetCommitted;

use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition_platformer2d_world::rooms::RoomSet;
use crate::world::physics;
use ambition_boss_encounter::BossEncounterRegistry;
use ambition_encounter::{EncounterMusicRequest, EncounterRegistry};
use ambition_persistence::quest::QuestRegistry;
use ambition_persistence::save::AmbitionGameSave;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Bundles sim-state resources so `process_new_game_reset_request`
/// stays within Bevy's 16-SystemParam limit.
#[derive(SystemParam)]
pub struct ResetPlayState<'w> {
    sim_state: ResMut<'w, ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown>,
    clock_resets: MessageWriter<'w, ambition_time::time_control::ClockResetRequest>,
    moving_platforms: ResMut<'w, ambition_platformer2d_world::collision::MovingPlatformSet>,
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    authored_sheets: Res<'w, ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    boss_catalog: Res<'w, ambition_boss_encounter::BossCatalog>,
    /// The installed placement-lowering authority — reset re-stages the start
    /// room's placements through the SAME registry setup/transition/restore use.
    placement_lowering: Res<'w, crate::world::placements::PlacementLoweringRegistry>,
    /// The installed room-content staging seam — same rule as the placement
    /// registry: reset re-stages content-staged occupants, one authority.
    content_staging: Res<'w, crate::features::RoomContentStagingRegistry>,
    /// The construction recipe table — reset re-plans the start room's planned
    /// families through the SAME recipes setup/transition/restore use.
    recipes: Res<'w, crate::construction::ActorConstructionRegistry>,
    /// `Option` like every other reader of it: a composition with no registered characters is
    /// the ordinary case.
    prepared_characters: Option<Res<'w, crate::character_runtime::PreparedCharacterRegistry>>,
    /// The session's live content binding, so a reset's plan states the SAME
    /// generation the session runs under instead of a default sentinel — the
    /// commit boundary refuses a mismatched plan as stale.
    active_binding: Option<Res<'w, crate::world::rooms::transaction::ActiveContentBinding>>,
    /// **The published controller policies**, so a placement that names a
    /// `brain_profile` still resolves it after a reset. Reset was the one road
    /// that carried the cast and not these — a room came IN with its authored
    /// policy and came back from every reset without it.
    brain_profiles:
        Option<Res<'w, ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
    /// Announced once the preflight has agreed. See [`NewGameResetCommitted`].
    committed: MessageWriter<'w, NewGameResetCommitted>,
    /// **What the world remembers about the occurrences it authored** — cleared
    /// by the reset, not read by it.
    ///
    /// **a reset is the EMPTY BASELINE.** A relocated occurrence's row names a
    /// room and a position in a world this reset is about to destroy; leaving it
    /// standing would put a moved object back at coordinates from the run that
    /// just ended, the first time the player walked into that room again. The
    /// custody leg would have retracted itself — the placement leg cannot,
    /// because "not in the world" is the ordinary condition of a row whose room
    /// is unloaded, so nothing but the reset can speak for it.
    ///
    /// `Option`, like every other reader: a composition without the item
    /// plugin remembers nothing and has nothing to clear.
    occurrences:
        Option<ResMut<'w, ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
}

/// Cross-system trigger for "wipe the save and rebuild the runtime."
/// Set `request = true` from anywhere; the next
/// `process_new_game_reset_request` tick consumes it.
#[derive(Resource, Clone, Default, Debug)]
pub struct NewGameResetRequested {
    pub request: bool,
}

impl NewGameResetRequested {
    pub fn request(&mut self) {
        self.request = true;
    }
}

/// **The set [`process_new_game_reset_request`] runs in.**
///
/// The only system that may DECLINE a new-game reset, so anything acting on the
/// decision waits for its commitment — `.after`, deliberately, not before.
///
/// ONE member: "the reset decision is made" is a single authority, and a
/// second member would mean two things can decline.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NewGameResetDecided;

/// Bevy system: drains a pending reset request and rebuilds the
/// sandbox state. Idempotent on `request = false` (early returns).
///
/// Schedule: runs in `Update` AFTER the player tick so a reset
/// triggered mid-frame doesn't race with in-flight gameplay
/// mutations, and BEFORE the populate systems so when they run on
/// the next frame the cleared registries see fresh state.
pub fn process_new_game_reset_request(
    mut request: ResMut<NewGameResetRequested>,
    mut save: ResMut<AmbitionGameSave>,
    mut encounter_registry: ResMut<EncounterRegistry>,
    mut boss_registry: ResMut<BossEncounterRegistry>,
    mut quest_registry: ResMut<QuestRegistry>,
    mut music_request: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        EncounterMusicRequest,
    >,
    mut play_state: ResetPlayState<'_>,
    mut room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<RoomSet>,
    mut world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        ambition_platformer2d_core::RoomGeometry,
    >,
    tuning: Res<ambition_platformer2d_core::ActiveMovementTuning>,
    mut respawn_visuals: MessageWriter<crate::session::RespawnRoomVisualsRequested>,
    mut commands: SessionCommands<'_, '_>,
    mut banner: ResMut<ambition_combat::events::GameplayBanner>,
    // **`With<RoomScopedEntity>` and NOT `RoomResident`, deliberately.** A room
    // CHANGE moves the room out from under its residents, so an object in a
    // body's custody rides across with whoever holds it. A reset DESTROYS the
    // world those residents live in — and this same function empties the hand a
    // few lines below (`remove::<HeldItem>`), so an object exempted here would
    // outlive both its room and the hand it was in, then reappear on the floor of
    // the rebuilt start room beside the freshly authored copy of itself. The two
    // sweeps ask different questions; unifying them is not the cleanup it looks
    // like.
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // E1: the live wave encounters are entities now; despawn them so
    // `populate_encounter_registry` (which the cleared `specs_loaded` flag
    // re-arms) respawns them fresh from the empty save next frame.
    encounter_entities: Query<Entity, With<ambition_encounter::Encounter>>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &mut crate::actor::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
            &mut crate::actor::BodyMelee,
            &mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState,
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
        &play_state.authored_sheets,
        &play_state.boss_catalog,
        session_scope,
        crate::features::ActorConstructionContext::for_room_construction(
            &play_state.recipes,
            ambition_platformer2d_core::ContentEpoch::default(),
            play_state.active_binding.as_deref(),
            play_state.prepared_characters.as_deref(),
            play_state.brain_profiles.as_deref(),
            // **A RESET STATES NO DISPOSITIONS, AND THAT IS THE WHOLE POINT
            // OF A RESET.** The ledger says which authored occurrences are
            // alive somewhere else; a reset destroys the world those
            // occurrences live in, hands included, and rebuilds the room from
            // the authored records alone. Handing it the ledger would make a
            // reset taken while carrying an authored object rebuild the room
            // WITHOUT that object — the one path where "remember what happened"
            // is exactly wrong.
            None,
        ),
    );
    // DECLINE, do not die. The preflight runs before the wipe precisely so a
    // refusal costs nothing — and a reset that cannot be prepared is a reason to
    // keep playing the game that is running, not to kill the process holding it.
    //
    // (Initial session setup still panics on the same failure, and that is a
    // different judgement: there is no game yet, so a silent partial start would
    // be worse than a loud stop. Same error, different stakes.)
    let room_plan = match room_plan {
        Ok(plan) => plan,
        Err(error) => {
            bevy::log::error!(
                target: "ambition_platformer2d::reset",
                "sandbox reset declined: room preflight failed ({error}). The \
                 running session is untouched."
            );
            // The request was already consumed above, so this cannot spin:
            // leaving it armed would retry the same failing preflight forever.
            return;
        }
    };

    info!(
        target: "ambition_platformer2d::reset",
        "sandbox reset requested — wiping save, registries, and runtime"
    );
    // Past the point of refusal. Every OTHER teardown system waits for this
    // rather than for the request, so a declined reset costs nothing anywhere —
    // not just in this function.
    play_state.committed.write(NewGameResetCommitted);

    // 1. Wipe the persisted save. Change-detection will trigger the
    //    autosave system to write the empty save to disk this tick.
    *save.data_mut() = ambition_persistence::save_data::AmbitionGameSaveData::default();

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
    // **AND WHAT THE WORLD REMEMBERED ABOUT ITS OWN OCCURRENCES.** The plan
    // above was prepared against NO dispositions on purpose; this is the other
    // half of the same statement, and without it the rooms this reset is not
    // rebuilding would still be carrying rows that place a moved object at
    // coordinates from the run that just ended. See the field's own note.
    if let Some(occurrences) = play_state.occurrences.as_mut() {
        occurrences.forget_everything();
    }

    // 3-5. The same artifact drives transition, hot reload, and restore.
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
        .write(ambition_time::time_control::ClockResetRequest::sim_clock(
            ambition_time::time_control::ClockRequester::Engine,
            "sandbox_reset",
        ));
    play_state.sim_state.remaining = 0.0;
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
            tuning.air_jumps,
        );
        clusters.mana.meter.refill_full();
        anim.reset();
        combat.reset();
        combat.hit_flash = 0.18;
        // ONE CALL, and it is the reason this system needs no camera test of its
        // own: `reset_to_spawn` clears the blink and keeps the snap together, so
        // the ordering hazard that produced Jon's 440px pan is unspellable here.
        blink_cam.reset_to_spawn(crate::ROOM_DOOR_CAMERA_SNAP_TIME);
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

/// On a sandbox reset, despawn the transient world items **the room rebuild does not own** —
/// placed portals + in-flight shots, a dropped weapon, a summoned puppy-slug ally — and strip
/// the player's held state (`HeldItem` / `StashedActionSet` / `PortalGun`), restoring its base
/// `ActionSet`.
///
/// **AND NOTHING THAT IS ROOM-SCOPED, because the room is already rebuilt by
/// the time this runs.** [`process_new_game_reset_request`] retires every
/// `RoomScopedEntity` and commits a fresh start-room plan in the same call, and
/// `.chain()` puts an auto-inserted `ApplyDeferred` between the two systems — so
/// every room-scoped ground item this query can see is a FRESHLY AUTHORED one,
/// spawned a sync point ago from the room's own records. A blanket
/// `With<GroundItem>` sweep despawned exactly those, and a reset taken in a room
/// with an authored pickup rebuilt that room permanently one pickup short of
/// itself. The room plan owns ROOM scope; this system owns
/// the residue that outlives a room and has no other retirement — an enemy's
/// dropped weapon is `spawn_session_scoped` and nothing else takes it back.
/// Filtering here loses nothing: `retire_outgoing` sweeps `RoomScopedEntity`
/// unconditionally, so a room-scoped transient (a thrown item, a placed portal)
/// is destroyed by the stricter of the two sweeps either way.
///
/// Runs AFTER [`process_new_game_reset_request`] and on [`NewGameResetCommitted`], not on the
/// request. Ordering costs nothing here: every despawn and removal below is a deferred command, so
/// it lands in the same flush either way, and the one immediate write (the `ActionSet` restore) is
/// exactly the one that must not happen speculatively.
#[allow(clippy::type_complexity)]
pub fn clear_transient_on_sandbox_reset(
    mut committed: MessageReader<NewGameResetCommitted>,
    mut commands: Commands,
    #[cfg(feature = "portal")] transient: Query<
        Entity,
        (
            Or<(
                With<ambition_portal2d::PlacedPortal>,
                With<ambition_portal2d::PortalShot>,
                With<ambition_portal2d::PortalGunPickup>,
                With<crate::items::pickup::GroundItem>,
                With<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>,
            )>,
            // the rebuilt room's own contents are NOT this system's business.
            Without<RoomScopedEntity>,
        ),
    >,
    #[cfg(not(feature = "portal"))] transient: Query<
        Entity,
        (
            Or<(
                With<crate::items::pickup::GroundItem>,
                With<crate::abilities::thrown::puppy_slug_gun::PuppySlugAlly>,
            )>,
            // the rebuilt room's own contents are NOT this system's business.
            Without<RoomScopedEntity>,
        ),
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
    if committed.read().count() == 0 {
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
            .remove::<ambition_portal2d::PortalGun>();
        // Clear any Mark/Recall mark too, so re-equipping after a reset can't
        // recall to a position from before the room was rebuilt.
        commands
            .entity(player)
            .remove::<crate::abilities::traversal::mark_recall::PlayerMark>();
    }
}

/// Schedules [`process_new_game_reset_request`] into [`Platformer2dSimulationPhaseMonolith::ResetProcessing`].
pub struct NewGameResetPlugin;

impl Plugin for NewGameResetPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_message::<crate::session::RespawnRoomVisualsRequested>();
        app.add_message::<RoomReplayRequested>();
        app.add_message::<NewGameResetCommitted>();
        app.add_systems(
            sim,
            // PREFLIGHT FIRST. The processor is the only system that may decline
            // a reset, so nothing may tear anything down ahead of it; the
            // transient clear waits for `NewGameResetCommitted` and therefore
            // never runs for a reset that was refused.
            (
                process_new_game_reset_request.in_set(NewGameResetDecided),
                clear_transient_on_sandbox_reset,
            )
                .chain()
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::ResetProcessing),
        );
    }
}

#[cfg(test)]
mod tests;
