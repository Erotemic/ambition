//! Actor death withdrawal and participant death consequences.
//!
//! Death opens [`DeathInterlude`] and marks the participant [`OutOfPlay`]; the
//! interlude advances on simulation time, then the death rules that govern the
//! dying body's live room ([`RulesOf`](ambition_platformer2d_world::governing_rules::RulesOf)) decide
//! whether the active room should replay. Body restart is owned elsewhere and
//! clears `OutOfPlay` through the shared `BodyRestarted` observer.

use bevy::prelude::*;

use ambition_combat::death_rules::{DeathInterlude, DeathRules, LevelReset, OutOfPlay};
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use crate::reset::RoomReplayRequested;
use ambition_combat::death_rules::ActorDiedMessage;

#[cfg(test)]
mod tests;

/// A death withdraws the crossing of that exact body on this frame.
/// Read before settlement can remove the victim's stable identity.
pub fn withdraw_crossings_on_actor_death(
    mut deaths: MessageReader<ActorDiedMessage>,
    identities: Query<(&SimId,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>)>,
    boundary: Option<Res<ambition_platformer2d_core::ConfirmedFrameBoundary>>,
    mut pending: ResMut<crate::lifecycle_commit::PendingLifecycleCommit>,
) {
    if pending.peek().is_none() { deaths.clear(); return; }
    let now = boundary.map_or(0, |boundary| boundary.current);
    for death in deaths.read() {
        let Ok((id, stamp, root)) = identities.get(death.victim) else { continue };
        pending.retract_transition_for_subject(&ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(
            id.clone(), ambition_platformer2d_shared_tangle::lifecycle::live_room_of(stamp, root)), now);
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
    // The victim's own room's rules (OW1): with two rooms live, THE live
    // room's rules were the rules of no room.
    rules: ambition_platformer2d_world::governing_rules::RulesOf<DeathRules>,
    participants: Query<Entity, (With<PlayerEntity>, Without<OutOfPlay>)>,
) {
    // Drained unconditionally so a death reported during a load cannot be
    // re-read later and charged to the next attempt — the same rule every other
    // reader of this channel follows.
    let victims: Vec<Entity> = deaths.read().map(|death| death.victim).collect();
    // Who plays on. Victims of this tick are still in the query (their marks
    // are deferred), so two that fall together each see the other: both
    // wait, and the wipe is read when the first window closes.
    let in_play: Vec<Entity> = participants.iter().collect();
    for victim in victims {
        let Ok(victim) = participants.get(victim) else { continue };
        let death = rules.of(victim).copied().unwrap_or_default();
        // A partner still plays: wait, so that its fall in that time is a wipe.
        let partner_plays = in_play.iter().any(|body| *body != victim);
        let interlude = death.interlude + if partner_plays { death.recovery_wait } else { 0.0 };

        commands.entity(victim).try_insert((
            OutOfPlay,
            DeathInterlude {
                remaining: interlude,
                consequence_pending: true,
            },
        ));

        // A dead body does not answer input, and it says so by CLAIMING the
        // sequence hold. `ControlHolds`
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
        // `ControlHolds` off a corpse mid-interlude. Claiming a bit makes
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
    // Each closing body's own room's rules (OW1).
    rules: ambition_platformer2d_world::governing_rules::RulesOf<DeathRules>,
    mut closing: Query<(Entity, &mut DeathInterlude, Option<&ambition_characters::control::DrivingParticipant>)>,
    still_playing: Query<Entity, (With<PlayerEntity>, Without<OutOfPlay>)>,
    // The restore's subject. Its room is the only room a death sends back.
    primary: Query<Entity, With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>>,
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
    // One second seat's own recovery: it fell alone in its room while
    // another participant plays (COOP-RECOVERY).
    mut recover: Option<MessageWriter<crate::checkpoint::RecoverAtCheckpoint>>,
) {
    // Nobody left in play IN THAT ROOM: the level goes back. In co-op this is
    // false while a teammate still plays the level, which is why the condition
    // is a query and not a flag on the death. A participant in another live
    // room does not hold it back, because a death is local to its room (Q151).
    let rooms_with_a_participant: Vec<_> = still_playing.iter().map(|body| rules.room_of(body)).collect();
    // The restore and the replay put the PRIMARY body back, so only the
    // primary's room can go back this way. A second seat alone in its room
    // asks for its own recovery (`RecoverAtCheckpoint`). A participant whose
    // room a living one still holds goes back alone, with no room reset
    // (COOP-RECOVERY, see `return_a_fallen_participant`). With nobody in play
    // it is a wipe. With no primary, as in a match, any room with nobody left
    // in play goes back.
    let primary = primary.iter().next();
    // The live rooms where a body's window closed under rules that send the
    // level back when nobody remains.
    let mut resetting_rooms = Vec::new();
    let mut lone_seats = Vec::new();
    for (body, mut window, driver) in &mut closing {
        let death = rules.of(body).copied().unwrap_or_default();
        // The partner wait is for a partner's fall to make the wipe. With
        // nobody in play (the partner fell, or left the session) there is no
        // one to wait for, and only the beat is owed.
        if rooms_with_a_participant.is_empty() && death.recovery_wait > 0.0 && window.remaining > death.interlude {
            window.remaining = death.interlude;
        }
        if window.open() || !window.consequence_pending {
            continue;
        }
        let room = rules.room_of(body);
        let resets = death.level_reset == LevelReset::WhenNoParticipantRemains;
        // Spent, not removed. The window lives on until the body restarts, so a
        // rewind across the frame this fired can answer "did it already?" from
        // state rather than from a component that is no longer there.
        window.consequence_pending = false;
        if resets {
            resetting_rooms.push(room);
            if primary.is_some_and(|primary| primary != body)
                && !rooms_with_a_participant.is_empty()
                && !rooms_with_a_participant.contains(&room)
            {
                if let Some(driver) = driver {
                    lone_seats.push(driver.0);
                }
            }
        }
    }
    if let Some(recover) = recover.as_mut() {
        for seat in lone_seats {
            recover.write(crate::checkpoint::RecoverAtCheckpoint(seat));
        }
    }
    // Nobody in play anywhere: a full-party wipe, whoever's window closed.
    let wipe = rooms_with_a_participant.is_empty() && !resetting_rooms.is_empty();
    let goes_back = |room: &Option<_>| resetting_rooms.contains(room) && !rooms_with_a_participant.contains(room);
    let back = wipe
        || match primary {
            Some(primary) => goes_back(&rules.room_of(primary)),
            None => resetting_rooms.iter().any(goes_back),
        };
    if !back {
        return;
    }
    // ⛔⛔ ONE LIFECYCLE OPERATION, NOT TWO. This used to write BOTH
    // `ResetToCheckpoint` and `RoomReplayRequested`, and the two competed for
    // the single pending-commit slot: `resume_at_checkpoint_on_reset` recorded
    // its transition first and the replay's record silently no-op'd, while the
    // replay's consequences ran anyway. That is the defect in miniature — a
    // world half-reset for an operation that never happened — and it was
    // invisible until the slot started reporting refusals.
    //
    // ⭐ THE CHECKPOINT RESUME IS THE REPLAY, on this road. It knows the target
    // (the saved room, which is not necessarily the active one), it records the
    // intent, and it announces the admitted replay so the death's consequences —
    // the body back at the arrival, the attempt's residue retired, content's
    // per-attempt state re-armed, the gun portals PRESERVED because this is a
    // death and not a deliberate retry — all hang off the one fact.
    let Some(restore) = restore.as_mut() else {
        // No checkpoint horizon in this composition: fall back to asking for a
        // plain replay of the active room, which is what a game with no
        // checkpoints means by "the level goes back".
        replay.write(RoomReplayRequested::player_death());
        return;
    };
    restore.write(ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint);
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
