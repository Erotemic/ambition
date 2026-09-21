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

/// Per-attempt state a room rebuild CANNOT retract, and the one way to retract
/// it.
///
/// ⭐⭐ THE RULE THAT CREATES THIS CLASS, measured 2026-09-05: an admitted replay
/// records a transition back to the SAME room, and that rebuild despawns every
/// `RoomScopedEntity`. So ENTITY-shaped per-attempt state is retracted FOR FREE
/// — the entity is despawned and respawned whole. Nothing despawns a RESOURCE,
/// so resource-shaped per-attempt state has to retract ITSELF. Every known
/// instance is a resource feeding the collision overlay's `removed_block_names`.
///
/// ⛔⛔ AND THE OBVIOUS WRONG IMPLEMENTATION IS A SHIPPED, PLAYER-VISIBLE BUG.
/// Sanic's `SpentMonitors` re-armed on `RoomLoaded` only, and Sanic declares
/// `DeathRules::replay_level_after(0.0)`: a pit death replays the room IN PLACE
/// and never emits a load. A monitor broken before the death stayed broken after
/// the respawn and its grant was unreachable for the rest of the run. ⇒ this
/// trait names WHAT to re-arm and WHICH ROOM it belongs to, and leaves the
/// SIGNAL to [`rearm_attempt_scoped`], which asks
/// [`FreshAttempt`](ambition_combat::events::FreshAttempt). An implementor has
/// no way to spell "the load only", which is the whole defect.
pub trait AttemptScoped: Resource<Mutability = bevy::ecs::component::Mutable> {
    /// The room whose fresh attempt re-arms this, or `None` when ANY fresh
    /// attempt does.
    ///
    /// `None` is the right answer for state that is per-attempt but not
    /// per-room: you can only stand in one room, so any boundary re-arms
    /// everything. Name a room when the state is authored in that room alone.
    ///
    /// ⚠ It filters the LOAD leg only. A replay is always in the room you are
    /// in, so it re-arms whatever the constant says.
    const ROOM: Option<&'static str> = None;

    /// Return to the state a fresh attempt starts from.
    fn rearm(&mut self);
}

/// Re-arm one [`AttemptScoped`] resource when a fresh attempt begins.
///
/// ⚠ PREFER [`install_attempt_scoped`], which registers this in
/// [`ContentRoomReplayResetSet`] and creates the resource in one statement. Reach
/// for this function directly only when the resource is already in the world for
/// another reason — and then the set membership is yours to get right.
///
/// The host anchors that set BEFORE its generic replay consumer, so the re-arm
/// lands the same frame the request does. The set is the slot; this function is
/// what goes in it. Content still chooses the SCHEDULE and any mode gate, because
/// those genuinely differ per demo — what must not differ is which signal counts
/// as a fresh attempt.
pub fn rearm_attempt_scoped<T: AttemptScoped>(
    mut attempt: ambition_combat::events::FreshAttempt,
    mut state: ResMut<T>,
) {
    let began = match T::ROOM {
        Some(room) => attempt.began_in(room),
        None => attempt.began(),
    };
    if began {
        state.rearm();
    }
}

/// Put an [`AttemptScoped`] resource in the world AND on the retraction slot, in
/// one statement.
///
/// ⭐⭐ THE AUTHORITY THIS REMOVES: before it, a demo said "this state is
/// per-attempt" TWICE — once by `init_resource::<T>()` and once by an
/// `add_systems(rearm_attempt_scoped::<T>.in_set(ContentRoomReplayResetSet))`
/// two hundred lines away — and only the second one was load-bearing. A resource
/// with the impl and without the registration is exactly the shipped Sanic bug
/// ([`AttemptScoped`]'s own header): the state exists, nothing takes it back,
/// and the grant behind it is unreachable for the rest of the run. Through this
/// function that state is not expressible — you cannot get the resource without
/// the re-arm.
///
/// ⚠ THE CONDITION IS THE CALLER'S because it genuinely differs: a hosted demo
/// gates its systems on its mode, a rules-only harness runs unconditionally.
/// Pass `|| true` for the ungated case. What must NOT differ, and is therefore
/// not a parameter, is the SET and the SIGNAL.
pub fn install_attempt_scoped<T: AttemptScoped + FromWorld, M>(
    app: &mut App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
    when: impl bevy::ecs::schedule::SystemCondition<M>,
) {
    app.init_resource::<T>();
    app.add_systems(
        schedule,
        rearm_attempt_scoped::<T>
            .in_set(ContentRoomReplayResetSet)
            .run_if(when),
    );
}

/// ASK for the ACTIVE room to be replayed: the controlled body back at the room
/// spawn, the room's scoped population rebuilt, progress outside the room
/// untouched. CONTENT emits this (a "try again" beat, a challenge retry, a
/// death); the engine's admission system decides whether it happens.
///
/// ⛔⛔ IT IS A REQUEST, NOT THE EVENT. Nothing may mutate authoritative state on
/// this message. A replay is a lifecycle operation and the one pending-commit
/// slot may already be owned by another one, in which case the replay does not
/// happen at all — so a listener that reset gravity, cleared combat state or
/// advanced a content cycle here would have changed the world for an operation
/// that was refused. React to
/// [`RoomReplayAdmitted`](ambition_combat::events::RoomReplayAdmitted) instead;
/// it is written by exactly one system, and only after the operation is in the
/// slot.
///
/// The REASON travels with the request because the policies downstream differ by
/// it — a death preserves the player's gun portals and a deliberate retry clears
/// them — and the only place that knows which this is, is the producer.
#[derive(Message, Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomReplayRequested {
    pub reason: ambition_combat::RoomResetReason,
}

impl RoomReplayRequested {
    /// A deliberate retry: a reset press, a "try again" beat, a level loop.
    pub fn manual() -> Self {
        Self {
            reason: ambition_combat::RoomResetReason::Manual,
        }
    }

    /// The body died or fell out of the world.
    pub fn player_death() -> Self {
        Self {
            reason: ambition_combat::RoomResetReason::PlayerDeath,
        }
    }
}

/// **The reset's preflight passed and the wipe is happening.**
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NewGameResetCommitted;

use crate::world::physics;
use ambition_boss_encounter::BossEncounterRegistry;
use ambition_encounter::{EncounterMusicRequest, EncounterRegistry};
use ambition_persistence::quest::QuestRegistry;
use ambition_persistence::save::AmbitionGameSave;
use ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_world::rooms::RoomSet;

/// Bundles sim-state resources so `process_new_game_reset_request`
/// stays within Bevy's 16-SystemParam limit.
#[derive(SystemParam)]
pub struct ResetPlayState<'w, 's> {
    character_catalog: Res<'w, ambition_characters::actor::character_catalog::CharacterCatalog>,
    /// The mechanics of the generation this session was activated under — the
    /// ONLY construction source a reset has.
    ///
    /// ⛔⛤ **SIX App REGISTRIES USED TO TRAVEL BESIDE IT AND THEY ARE GONE.**
    /// `AuthoredSheets`, `BossCatalog`, `PreparedCharacterRegistry`,
    /// `AuthoredBrainOverride`, `AuthoredPopulationCap` and the
    /// `SessionGatedSimulation` flag that chose between them were this
    /// system's half of `DUP-GENERATION-MECHANICS`: a reset in a composition
    /// with no activated generation rebuilt the start room out of whatever the
    /// App happened to be holding. The 2026-09-19 composition ruling closed
    /// that road — *"no anonymous App-global fallback state returns"* — so a
    /// reset with no generation declines instead. See
    /// `GenerationMechanics::for_live_session`.
    generation: Option<Res<'w, crate::session::mechanics::SessionMechanics>>,
    /// The installed placement-lowering authority — reset re-stages the start
    /// room's placements through the SAME registry setup/transition/restore use.
    placement_lowering: Res<'w, crate::world::placements::PlacementLoweringRegistry>,
    /// The installed room-content staging seam — same rule as the placement
    /// registry: reset re-stages content-staged occupants, one authority.
    content_staging: Res<'w, crate::features::RoomContentStagingRegistry>,
    /// The construction recipe table — reset re-plans the start room's planned
    /// families through the SAME recipes setup/transition/restore use.
    recipes: Res<'w, crate::construction::ActorConstructionRegistry>,
    /// The session's live content binding, so a reset's plan states the SAME
    /// generation the session runs under instead of a default sentinel — the
    /// commit boundary refuses a mismatched plan as stale.
    /// ⛔ ON THE SESSION ROOT, not a process global — see `ActiveContentBinding`.
    /// `Option<Single<..>>` rather than a bare `Single`, because a bare one would
    /// SKIP this whole system in a composition that has no session root, and a
    /// reset in a direct-entry fixture is legitimate.
    active_binding: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            'w,
            's,
            crate::world::rooms::transaction::ActiveContentBinding,
        >,
    >,
    /// **The published controller policies**, so a placement that names a
    /// `brain_profile` still resolves it after a reset. Reset was the one road
    /// that carried the cast and not these — a room came IN with its authored
    /// policy and came back from every reset without it.
    brain_profiles:
        Option<Res<'w, ambition_characters::actor::character_catalog::BrainProfileRegistry>>,
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
    // ⛔⛤ **THE PARAMS THIS SYSTEM NO LONGER TAKES ARE THE POINT OF THE CHANGE.**
    // `AmbitionGameSave`, the three registries, `EncounterMusicRequest`,
    // `GameplayBanner`, the player query, the clock writer, the occurrence ledger
    // and `NewGameResetCommitted` were all `&mut` here. They are written by the
    // staged closure below instead — at a command flush, which is exclusive world
    // access, so nothing is lost to parallelism and the SIGNATURE no longer
    // claims a reset that has not been verified.
    mut request: ResMut<NewGameResetRequested>,
    play_state: ResetPlayState<'_, '_>,
    // ⛤ READ ONLY, AND THE TYPE SAYS SO SINCE 2026-09-18. It is bound without
    // `mut`, which in Rust already meant nothing here could write through it —
    // `start` is read and the value is passed on as `&room_set` — but the
    // accessor still asked for `&mut`, which is an exclusive borrow of a
    // session-world component and an entry in the multi-writer census for a
    // system that only reads.
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>,
    // ⛔ A GUARD, NOT A WRITE TARGET — and `Single` is what makes it one: this
    // system does not run unless the live session root carries room geometry. The
    // reset no longer WRITES it (`replace_live_world` stages that behind the
    // room's verdict), but a reset in a world with no room authority is still
    // nothing this should attempt.
    //
    // ⛤ SO IT ASKS FOR `Ref`. Both aliases are `Single<_, With<SessionRoot>>`,
    // so the refusal is identical and the `&mut` bought nothing.
    _room_geometry: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    tuning: Res<ambition_platformer2d_core::ActiveMovementTuning>,
    mut commands: SessionCommands<'_, '_>,
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

    let start_index = room_set.start();
    // ⛔ THE GENERATION'S VALUES WHEN THERE IS ONE. See `GenerationMechanics`.
    // ⛔⛤ **A RESET REBUILDS A LIVE ROOM, so a shell session that has lost its
    // generation DECLINES rather than rebuilding from the App's registries.** See
    // `GenerationMechanics::for_live_session`; the decline below is this
    // function's existing *"DECLINE, do not die"* road.
    let Some(mechanics) = crate::session::mechanics::GenerationMechanics::for_live_session(
        play_state.generation.as_deref(),
    ) else {
        bevy::log::error!(
            target: "ambition_platformer2d::reset",
            "sandbox reset declined: a reset rebuilds a LIVE room, so it is built \
             from the generation this session is running and there is no \
             `SessionMechanics` to build from. A composition that means to reset \
             rooms declares one. The running session is untouched."
        );
        return;
    };
    let room_plan = crate::rooms::RoomConstructionPlan::prepare_from_parts(
        &room_set,
        start_index,
        &play_state.placement_lowering,
        &play_state.content_staging,
        mechanics.bosses(),
        session_scope,
        crate::features::ActorConstructionContext::for_live_room_construction(
            &play_state.recipes,
            &play_state.character_catalog,
            &mechanics,
            // ⛔⛤ THE LIVE GENERATION, FOR BOTH HALVES. A reset rebuilds the room
            // the session is already living in, so that generation is what the
            // boundary compares against AND what the rebuilt roots are made of.
            // This stated `content_unstated(ContentEpoch::default())` for the
            // second half — the default epoch, on a road that has a real one —
            // so a reset erased the prepared content identity from every root it
            // rebuilt.
            crate::world::rooms::transaction::ActiveContentBinding::live_or(
                play_state
                    .active_binding
                    .as_deref()
                    .map(|binding| &**binding),
                // ⚠ A fixture resetting outside any session has no generation to
                // name, which is the one case this default was ever right for.
                ambition_platformer2d_shared_tangle::construction::ContentBinding::content_unstated(
                    ambition_platformer2d_core::ContentEpoch::default(),
                ),
            ),
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

    // ⛔⛤ **A10: THE RESET IS STAGED, NOT PERFORMED.** Everything below — the
    // save wipe, the registries, the remembered occurrences, the player's own
    // position and state, the commit message every other teardown system waits
    // for — happens ONLY if the start room publishes.
    //
    // ⚠ **THE PREVIOUS SHAPE HAD A NAME FOR THIS AND STOPPED ONE STEP SHORT.**
    // The line it replaced read *"Past the point of refusal. Every OTHER teardown
    // system waits for this rather than for the request, so a declined reset
    // costs nothing anywhere."* True of a declined PREFLIGHT, which was the only
    // refusal that existed when it was written. The room transaction can refuse
    // too, and under the old order that refusal arrived after the save was gone,
    // the registries were cleared and the player had been warped to the spawn of
    // a room that was never built. ⇒ `NewGameResetCommitted` now means what its
    // doc says: the reset HAPPENED.
    //
    // ⭐ The roster is captured HERE and despawned THERE, for the same reason
    // `replace_live_world` captures the outgoing room: by the time the verdict
    // runs, the start room's OWN encounters exist, and a fresh
    // `With<Encounter>` query would sweep the room this reset just built.
    let doomed_encounters: Vec<Entity> = encounter_entities.iter().collect();
    let spawn = room_plan.spec().world.spawn;
    let air_jumps = tuning.air_jumps;
    let start_room_id = room_plan.room_id().to_string();

    // 1-3. The same artifact drives transition, hot reload, and restore.
    let publication = room_plan.replace_live_world(
        &mut commands,
        room_visuals
            .iter()
            .map(|(entity, physics_entity)| (entity, physics_entity.is_some())),
        None,
        None,
        // ⚠ THE RESET PLACES ITS PLAYER ITSELF, in the closure below, with
        // `reset_body_clusters` — a fuller operation than an arrival (mana,
        // animation, combat, camera). Both are behind the same verdict; they are
        // different operations, not two spellings of one.
        None,
    );

    // ⛔⛤ **AND THE RECEIPT IS RETIRED BY ITS OWNER, IN ITS OWN STATEMENT.** A
    // separate trailing closure rather than a line at the end of the reader
    // below, because the reader RETURNS EARLY on a refusal — and a retirement
    // that only happens on one branch is a leak on the other. Queued after every
    // reader of this publication, so "the last reader has run" is an ordering
    // this road states rather than one a future reader has to remember.
    let retire = publication;
    commands.queue(move |world: &mut World| {
        // ⛔ **THIS EXACT PUBLICATION, not "the last verdict for a room with this
        // name".** `LastConstructionVerification` is last-writer-wins and cannot
        // tell two operations on one room apart; a reset that wiped the save on
        // somebody else's success is the shape that makes possible.
        if !crate::world::rooms::publication_succeeded(world, publication) {
            bevy::log::error!(
                target: "ambition_platformer2d::reset",
                "sandbox reset ABANDONED: the start room `{start_room_id}` failed \
                 construction verification, so nothing was wiped and the running \
                 session is untouched."
            );
            return;
        }
        info!(
            target: "ambition_platformer2d::reset",
            "sandbox reset committed — wiping save, registries, and runtime"
        );
        // Every OTHER teardown system waits for this rather than for the request.
        world.write_message(NewGameResetCommitted);

        // 4. Wipe the persisted save. Change-detection will trigger the
        //    autosave system to write the empty save to disk this tick.
        if let Some(mut save) = world.get_resource_mut::<AmbitionGameSave>() {
            *save.data_mut() = ambition_persistence::save_data::AmbitionGameSaveData::default();
        }

        // 5. Clear registries. Setting them to Default flips
        //    `specs_loaded` / `initialized` back to false so the populate
        //    Update systems re-run on the next frame.
        if let Some(mut registry) = world.get_resource_mut::<EncounterRegistry>() {
            *registry = EncounterRegistry::default();
        }
        for entity in doomed_encounters {
            if let Ok(entity) = world.get_entity_mut(entity) {
                entity.despawn();
            }
        }
        if let Some(mut registry) = world.get_resource_mut::<BossEncounterRegistry>() {
            *registry = BossEncounterRegistry::default();
        }
        if let Some(mut registry) = world.get_resource_mut::<QuestRegistry>() {
            *registry = QuestRegistry::default();
        }
        if let Some(mut music) = ambition_platformer2d_shared_tangle::lifecycle::
            session_world_component_mut::<EncounterMusicRequest>(world)
        {
            *music = EncounterMusicRequest::default();
        }
        // **AND WHAT THE WORLD REMEMBERED ABOUT ITS OWN OCCURRENCES.** The plan
        // was prepared against NO dispositions on purpose; this is the other half
        // of the same statement, and without it the rooms this reset is not
        // rebuilding would still carry rows that place a moved object at
        // coordinates from the run that just ended.
        //
        // ⚠ Safe AFTER the rebuild: every writer of this ledger is a SYSTEM, and
        // no system runs inside a command flush — so the room the verdict just
        // published has authored no rows for this to erase.
        if let Some(mut occurrences) = world.get_resource_mut::<
            ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences,
        >() {
            occurrences.forget_everything();
        }

        // 6. Reset the player to the start room's spawn point.
        world.write_message(ambition_time::time_control::ClockResetRequest::sim_clock(
            ambition_time::time_control::ClockRequester::Engine,
            "sandbox_reset",
        ));
        if let Some(mut cooldown) = world.get_resource_mut::<
            ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown,
        >() {
            cooldown.remaining = 0.0;
        }
        // Reset the ECS authority directly so the next player tick frame starts
        // from the spawn position. Also zero animation state so post-reset frames
        // don't continue a mid-air slash or dash-startup pose.
        let mut player = world.query_filtered::<
            (
                ae::BodyClusterQueryData,
                &mut ambition_platformer2d_core::movement::MotionModel,
                &mut ambition_characters::actor::BodyAnimFacts,
                &mut ambition_characters::actor::BodyCombat,
                &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
                &mut ambition_combat::BodyMelee,
                &mut ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState,
            ),
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
        >();
        if let Ok((
            mut cluster_item,
            mut motion_model,
            mut anim,
            mut combat,
            mut blink_cam,
            mut attack,
            mut safety,
        )) = player.single_mut(world)
        {
            let mut clusters = cluster_item.as_clusters_mut();
            ae::reset_body_clusters(&mut motion_model, &mut clusters, spawn, air_jumps);
            clusters.mana.meter.refill_full();
            anim.reset();
            combat.reset();
            combat.hit_flash = 0.18;
            // ONE CALL, and it is the reason this system needs no camera test of
            // its own: `reset_to_spawn` clears the blink and keeps the snap
            // together, so the ordering hazard that produced Jon's 440px pan is
            // unspellable here.
            blink_cam.reset_to_spawn(crate::ROOM_DOOR_CAMERA_SNAP_TIME);
            attack.clear();
            // ⛔ THE PLAN'S SPAWN, NOT THE LIVE GEOMETRY'S — the same value
            // `reset_body_clusters` above already read, named at its source.
            safety.last_safe_pos = spawn;
        }

        // 7. Respawn the static world visuals + parallax for the start room.
        //    Without this, the sweep above leaves the scene empty until something
        //    else (LDtk reload, room transition) rebuilds it. The visual respawn
        //    is a PRESENTATION concern, so the sim only emits the request — the
        //    render layer's `respawn_room_visuals_on_request` consumes it and
        //    reads the active room from `RoomSet`. A headless build has no
        //    consumer and correctly skips the (purely visual) respawn.
        world.write_message(ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested);

        // 8. User feedback: surface a banner so the reset is visibly confirmed.
        //    The HUD's banner channel is the same one used for "ARENA CLEAR" etc.
        if let Some(mut banner) = world.get_resource_mut::<ambition_combat::events::GameplayBanner>()
        {
            banner.show("SANDBOX RESET", 3.0);
        }
    });

    commands.queue(move |world: &mut World| {
        crate::world::rooms::retire_publication(world, retire);
    });
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
/// Filtering here loses nothing: [`process_new_game_reset_request`] sweeps
/// `RoomScopedEntity` unconditionally, so a room-scoped transient (a thrown
/// item, a placed portal) is destroyed by the stricter of the two sweeps either
/// way. ⚠ THAT IS THE RESET SWEEP, NOT THE ROOM-TRANSITION ONE, and this
/// sentence named a `retire_outgoing` until 2026-09-19 — a method A10 deleted  <!-- cite-ok: names the method A10 DELETED on 2026-09-14; this sentence RECORDS the dead name, so a resolvable citation here would mean the deletion did not happen -->
/// on 2026-09-14, and one that would have made the claim FALSE if it had
/// existed, because the transition sweeps the narrower `RoomResident` roster.
/// The parameter list above says why the two differ.
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
                With<ambition_held_items::GroundItem>,
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
                With<ambition_held_items::GroundItem>,
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
            Option<&ambition_held_items::StashedActionSet>,
        ),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
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
            .remove::<ambition_held_items::StashedActionSet>();
        commands
            .entity(player)
            .remove::<ambition_combat::held_items::HeldItem>();
        #[cfg(feature = "portal")]
        commands
            .entity(player)
            .remove::<ambition_portal2d::PortalGun>();
        // Clear any Mark/Recall mark too, so re-equipping after a reset can't
        // recall to a position from before the room was rebuilt.
        commands
            .entity(player)
            .remove::<ambition_abilities::traversal::mark_recall::PlayerMark>();
    }
}

/// Schedules [`process_new_game_reset_request`] into [`Platformer2dSimulationPhaseMonolith::ResetProcessing`].
pub struct NewGameResetPlugin;

impl Plugin for NewGameResetPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The request this plugin's processor is the sole consumer of. It was
        // initialised by the runtime's `sim_core_resources` while the system that
        // reads it was scheduled here, which made the composition layer the
        // owner of a fact only this plugin uses.
        app.init_resource::<NewGameResetRequested>();
        app.add_message::<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>();
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
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::ResetProcessing),
        );
    }
}

#[cfg(test)]
mod tests;
