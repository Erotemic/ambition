//! Participant death interlude and level-reset policy.
//!
//! Death opens [`DeathInterlude`] and marks the participant [`OutOfPlay`]; the
//! interlude advances on simulation time, then [`GoverningDeathRules`] decides
//! whether the active room should replay. Body restart is owned elsewhere and
//! clears `OutOfPlay` through the shared `BodyRestarted` observer.

use bevy::prelude::*;

use ambition_combat::death_rules::{
    DeathInterlude, DeathRules, DeclaredDeathRules, LevelReset, OutOfPlay,
};
use ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_platformer2d_world::rooms::ActiveRoomMetadata;

use crate::session::reset::RoomReplayRequested;
use ambition_combat::death_rules::ActorDiedMessage;

#[cfg(test)]
mod tests;

/// Resolve death rules from the active room's mode declaration.
///
/// Missing declarations or session state use the conservative engine default.
#[derive(bevy::ecs::system::SystemParam)]
pub struct GoverningDeathRules<'w, 's> {
    declared: Option<Res<'w, DeclaredDeathRules>>,
    active_room: Option<SessionWorldRef<'w, 's, ActiveRoomMetadata>>,
}

impl GoverningDeathRules<'_, '_> {
    /// The rules in force for the active room.
    pub fn get(&self) -> DeathRules {
        let Some(declared) = self.declared.as_ref() else {
            return DeathRules::default();
        };
        let mode = self
            .active_room
            .as_ref()
            .and_then(|active| active.0.mode.as_deref());
        declared.governing(mode)
    }
}

/// A participant died: mark it out of play and open its window.
///
/// scoped to PARTICIPANT bodies. An enemy's death is answered by the
/// authored respawn policy (ADR 0022) and has nothing to do with a run ending.
///
/// The `Without<OutOfPlay>` filter is what makes a repeated report harmless: a body already out of
/// play cannot re-open its own window, so the death that a still-falling corpse might report costs
/// nothing.
pub fn open_death_interlude(
    mut commands: Commands,
    mut deaths: MessageReader<ActorDiedMessage>,
    rules: GoverningDeathRules,
    participants: Query<(Entity, Option<&SimId>), (With<PlayerEntity>, Without<OutOfPlay>)>,
    confirmed_boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    mut pending_lifecycle: ResMut<crate::session::lifecycle_commit::PendingLifecycleCommit>,
) {
    // Drained unconditionally so a death reported during a load cannot be
    // re-read later and charged to the next attempt — the same rule every other
    // reader of this channel follows.
    let victims: Vec<Entity> = deaths.read().map(|death| death.victim).collect();
    let interlude = rules.get().interlude;
    for victim in victims {
        let Ok((_, sim_id)) = participants.get(victim) else {
            continue;
        };

        // A rollback host is deliberately different. Absence from rollback state
        // is not confirmed merely because the current prediction contains a death,
        // so its confirmed lifecycle path remains the authority for retiring an
        // already-open transaction. Bodies without a stable `SimId` cannot have
        // recorded a transition in the first place.
        if confirmed_boundary.is_none() {
            if let Some(sim_id) = sim_id {
                pending_lifecycle.retract_transition_for_subject(sim_id);
            }
        }

        commands.entity(victim).try_insert((
            OutOfPlay,
            DeathInterlude {
                remaining: interlude,
                consequence_pending: true,
            },
        ));

        // A dead body does not answer input, and it says so by CLAIMING the
        // sequence hold rather than by stamping the marker. `ScriptedControl`
        // is the engine's existing word for "normal input does not reach this
        // body", and its doc already names Mary-O's death as the case that
        // reinvented it badly — she blanked the control frame a full phase
        // after everything that reads it. Its one caveat is a FEATURE here:
        // *"gravity will happily walk an undriven body out from under its
        // pose"* is precisely the classic pit death.
        //
        // stamping the marker directly was a real breach, not a style
        // point. The marker is DERIVED — its presence means `ControlHolds`
        // is non-empty — so a death that set it without a bit left the two
        // disagreeing, and the disagreement is resolved by whoever releases
        // NEXT: a captor letting go of a body that died in its grip found an
        // empty claim set, concluded nobody was holding it, and took
        // `ScriptedControl` off a corpse mid-interlude. Claiming a bit makes
        // that release arithmetic instead of a guess.
        //
        // `Sequence` and not a bit of its own: a death fall is the first
        // case its doc names, and the level beats that share it — a flagpole
        // slide, a goal brake, an act clear — are states the SAME body cannot
        // also be in. A second bit is what two OVERLAPPING owners need, and
        // these do not overlap.
        ambition_characters::control::claim_control_hold(
            &mut commands,
            victim,
            ambition_characters::control::ControlHold::Sequence,
        );
    }
}


/// The window closed: ask the roster whether the level goes back.
///
/// the question is about the ROSTER, never about the death. Hanging a
/// level reset off an individual death is player-centric and invisible in single
/// player, where the two are the same event. Asking "is anybody still in play"
/// makes NSMB co-op and a one-participant platformer the same rule.
pub fn close_death_interlude(
    rules: GoverningDeathRules,
    mut closing: Query<&mut DeathInterlude>,
    still_playing: Query<Entity, (With<PlayerEntity>, Without<OutOfPlay>)>,
    mut replay: MessageWriter<RoomReplayRequested>,
    // the horizon half of the same consequence. `RoomReplayRequested`
    // says "rebuild the active room"; this says "and rebuild it from the last
    // committed checkpoint rather than from the world that just killed you".
    //
    // they are two channels because `RoomReplayRequested` is ALSO how
    // content announces a level COMPLETION — Mary-O's flag, Sanic's act
    // clear, a dialogue's "try again". Restoring a reset baseline when the
    // player just won would take the reward back off them, and a single
    // channel makes that indistinguishable from a death.
    mut restore: Option<
        MessageWriter<ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint>,
    >,
) {
    let mut any_closed = false;
    for mut window in &mut closing {
        if window.open() || !window.consequence_pending {
            continue;
        }
        // Spent, not removed. The window lives on until the body restarts, so a
        // rewind across the frame this fired can answer "did it already?" from
        // state rather than from a component that is no longer there.
        window.consequence_pending = false;
        any_closed = true;
    }
    if !any_closed {
        return;
    }
    let level_reset = rules.get().level_reset;
    if level_reset != LevelReset::WhenNoParticipantRemains {
        return;
    }
    // Nobody left in play — the run is over, so the level goes back. In co-op
    // this is false while a teammate is still running the level, which is the
    // entire reason the condition is a query rather than a flag on the death.
    if still_playing.iter().next().is_some() {
        return;
    }
    // ORDER OF WRITES IS IRRELEVANT; ORDER OF READS IS NOT. Both land in
    // this frame's channels and the schedule decides which consumer sees its
    // own first — `CheckpointRestore` is configured before `RoomReplayApplied`
    // precisely so the ledger is back to the baseline before anything rebuilds
    // a room against it.
    if let Some(restore) = restore.as_mut() {
        restore.write(ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint);
    }
    replay.write(RoomReplayRequested);
}

/// A body that restarted is back IN play.
///
/// An observer on the derived restart announcement rather than a line in each
/// respawn: every caller of `reset_body_clusters` raises it, so the level
/// replay, a room arrival and a ruleset's own respawn all clear this without
/// naming it.
pub fn clear_out_of_play_on_restart(
    restart: On<ambition_platformer2d_core::BodyRestarted>,
    mut commands: Commands,
    out: Query<Entity, With<OutOfPlay>>,
) {
    if out.get(restart.entity).is_ok() {
        commands
            .entity(restart.entity)
            .remove::<(OutOfPlay, DeathInterlude)>();
        // a RESET, not a release: a body that restarted cannot still be
        // mid-sequence for anybody, so the whole claim set goes rather than one
        // bit of it.
        ambition_characters::control::clear_control_holds(&mut commands, restart.entity);
    }
}
