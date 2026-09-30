//! New Game and the room-replay vocabulary.
//!
//! A New Game returns the player to the world's start room with encounters,
//! quests, switches, bosses, flags, the bag and the occurrence ledger reset. It
//! is a checkpoint restore to the fresh baseline: [`NewGameRequested`] is
//! admitted by the checkpoint coordinator, the room is rebuilt by the
//! confirmed-frame lifecycle commit, and the commit runs this module's fresh-run
//! reducers with every other domain's restore.
//!
//! It does **not** reset user settings, keyboard preset selection, or global app
//! preferences.

use bevy::prelude::*;

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

use ambition_boss_encounter::BossEncounterRegistry;
use ambition_encounter::EncounterMusicRequest;
use ambition_persistence::quest::QuestRegistry;
use ambition_persistence::save::AmbitionGameSave;
use ambition_platformer2d_shared_tangle::lifecycle::{FreshRunRestore, RoomScopedEntity};

/// A host asks for a New Game. (host intent)
///
/// A menu writes this through `HostIntentWriter`. On the stamped tick the
/// checkpoint coordinator (`session::checkpoint::resume_at_checkpoint_on_reset`)
/// admits it as a checkpoint restore to the fresh baseline at the start room.
///
/// ⭐ A NEW GAME HAS NO REBUILD OF ITS OWN. It had one until 2026-09-29: a
/// staged room construction in the simulation that despawned every body on a
/// speculative frame. A rewind across that frame brought the bodies back without
/// the components derived from them, and the rollback sync test failed on every
/// New Game. Now the room is rebuilt by the same confirmed-frame commit as a
/// death, and this module keeps only the fresh-run reducers that commit runs.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NewGameRequested;

/// Wipe the run: the save, the registries built from it, and the live
/// encounters. (fresh-run reducer, `CheckpointDomainApply`)
///
/// The room, the player's body, the occurrence ledger and the bag are not here:
/// the checkpoint restore puts each of them back from the pinned fresh
/// baseline, the same way a death puts them back from a checkpoint.
pub fn begin_fresh_run(world: &mut World) {
    if !world.contains_resource::<FreshRunRestore>() {
        return;
    }
    info!(
        target: "ambition_platformer2d::reset",
        "new game committed — wiping the save, the registries and the encounters"
    );
    // The save. Change detection makes the autosave write the empty save.
    if let Some(mut save) = world.get_resource_mut::<AmbitionGameSave>() {
        *save.data_mut() = ambition_persistence::save_data::AmbitionGameSaveData::default();
    }
    // The registries. `Default` clears `specs_loaded` / `initialized`, so the
    // populate systems build them again from the empty save on the next tick.
    if let Some(mut registry) = world.get_resource_mut::<BossEncounterRegistry>() {
        *registry = BossEncounterRegistry::default();
    }
    if let Some(mut registry) = world.get_resource_mut::<QuestRegistry>() {
        *registry = QuestRegistry::default();
    }
    // The live wave encounters: their outcomes came from the save that was
    // just wiped. `project_live_encounter_occurrences` builds each live room's
    // again from the empty save.
    let encounters: Vec<Entity> = world
        .query_filtered::<Entity, With<ambition_encounter::Encounter>>()
        .iter(world)
        .collect();
    for entity in encounters {
        world.despawn(entity);
    }
    if let Some(mut music) =
        ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<
            EncounterMusicRequest,
        >(world)
    {
        *music = EncounterMusicRequest::default();
    }
    if let Some(mut banner) = world.get_resource_mut::<ambition_combat::events::GameplayBanner>() {
        banner.show("SANDBOX RESET", 3.0);
    }
}

/// On a New Game, despawn the transient world items **the room rebuild does not own** —
/// placed portals + in-flight shots, a dropped weapon, a summoned puppy-slug ally — and strip
/// the player's held state (`HeldItem` / `PortalGun`), re-deriving its repertoire for the
/// emptied hand. (fresh-run reducer, `CheckpointDomainApply`)
///
/// **AND NOTHING THAT IS ROOM-SCOPED, because the room is already rebuilt by
/// the time this runs.** The commit runs `CheckpointDomainApply` after it
/// publishes the start room, so every room-scoped ground item this query can
/// see is a FRESHLY AUTHORED one. A blanket `With<GroundItem>` sweep despawned
/// exactly those, and a reset taken in a room with an authored pickup rebuilt
/// that room permanently one pickup short of itself. The room plan owns ROOM
/// scope; this system owns the residue that outlives a room and has no other
/// retirement — an enemy's dropped weapon is `spawn_session_scoped` and nothing
/// else takes it back.
///
/// It runs after the custody restore, which has already emptied the hand of
/// every authored occurrence (the fresh custody baseline holds nothing). What
/// is left in a hand here is the item state custody does not record.
#[allow(clippy::type_complexity)]
pub fn clear_transient_on_sandbox_reset(
    fresh: Option<Res<FreshRunRestore>>,
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
        (Entity, ambition_combat::hand::RepertoireQuery),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
) {
    if fresh.is_none() {
        return;
    }
    for entity in &transient {
        commands.entity(entity).despawn();
    }
    for (player, mut repertoire) in &mut players {
        // Both hand components go below, so the hand this refolds for is empty.
        repertoire.refold(ambition_characters::repertoire::Hand::Empty);
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

/// Installs the New Game: its host intent, and its fresh-run reducers in the
/// checkpoint commit's domain-apply schedule. The admission is the checkpoint
/// coordinator's.
pub struct NewGameResetPlugin;

impl Plugin for NewGameResetPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>();
        app.add_message::<RoomReplayRequested>();
        app.add_plugins(crate::session::host_intents::HostIntentPlugin::<NewGameRequested>::default());
        // After the item domain's restore, so the custody restore has emptied
        // the hand of authored occurrences before the transient clear strips
        // what is left.
        app.add_systems(
            ambition_platformer2d_shared_tangle::lifecycle::CheckpointDomainApply,
            (begin_fresh_run, clear_transient_on_sandbox_reset)
                .chain()
                .after(crate::items::pickup::minted_horizon::start_the_item_domain_fresh),
        );
    }
}

#[cfg(test)]
mod tests;
