//! Apply typed flag, quest, switch, and audio effects.

use ambition_combat::events::{GameplaySfxRequested, SetFlagsRequested};
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;
use ambition_encounter::switches::SwitchActivated;
use ambition_persistence::quest::QuestAdvanceRequested;
use bevy::prelude::{Res, ResMut};

/// Save writes first so quest conditions that read flags see them this same
/// frame. `on == true` also mirrors into `QuestAdvanceEvent::FlagSet`, matching
/// the former monolithic router behavior.
///
/// A write from a room is logged with what it replaced
/// ([`ambition_combat::events::FlagWritesInRooms`]), and its quest advance is
/// caused by it, so a rewind of that room takes both back (ISL-2d). Without
/// the log (a bare fixture) the writes are only applied.
pub fn apply_flag_effects(
    mut replies: ambition_platformer2d_shared_tangle::action_results::ActionReplies,
    mut effects: ReplayMessageReader<SetFlagsRequested>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut log: Option<ResMut<ambition_combat::events::FlagWritesInRooms>>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let now = tick.map_or(0, |tick| tick.0);
    for effect in effects.read() {
        if !replies.is_pending(effect.reply.as_ref()) { continue; }
        for (id, on) in &effect.changes {
            write_logged_flag(&mut save, &mut quests, log.as_deref_mut(), id.clone(), *on, effect.written, now);
        }
        replies.publish(effect.reply.as_ref(), ambition_extension_sdk::action::ActionStage::Applied, true,
            "flags_written", "", effect.changes.iter().map(|(id, _)| ambition_extension_sdk::action::ActionSubject::named(id.clone())).collect());
    }
}

/// Write one flag, logged in [`ambition_combat::events::FlagWritesInRooms`]
/// with what it replaced when the log keeps it, and its quest advance caused
/// by the write: a flag a room's act sets, which a rewind of that room takes
/// back (ISL-2d).
pub fn write_logged_flag(
    save: &mut ambition_persistence::save::AmbitionGameSave,
    quests: &mut ambition_persistence::quest::QuestRegistry,
    log: Option<&mut ambition_combat::events::FlagWritesInRooms>,
    id: String,
    on: bool,
    written: Option<ambition_platformer2d_shared_tangle::lifecycle::RoomWrite>,
    now: u64,
) {
    let mut cause = None;
    if let Some(log) = log {
        let forgotten;
        (cause, forgotten) = log.log_write(&id, on, save.data().flag(&id), written, now);
        for old in forgotten {
            quests.forget_cause(&old);
        }
    }
    ambition_persistence::quest::write_flag_caused_by(save, quests, id, on, cause);
}

/// Record each NPC provocation as the NPC's persisted-hostile flag, written
/// in the NPC's room like any flag a room's act sets, so a rewind of that
/// room takes it back (ISL-2d).
pub fn apply_npc_provocations(
    mut changes: ReplayMessageReader<ambition_persistence::npc_provocation::NpcProvocationChanged>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut log: Option<ResMut<ambition_combat::events::FlagWritesInRooms>>,
    tick: Option<Res<ambition_time::SimTick>>,
) {
    let now = tick.map_or(0, |tick| tick.0);
    for change in changes.read() {
        let written = change.room.map(|room| ambition_platformer2d_shared_tangle::lifecycle::RoomWrite {
            room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::from_ordinal(room),
            tick: now,
        });
        let id = ambition_persistence::save_data::npc_hostile_flag(&change.id);
        write_logged_flag(&mut save, &mut quests, log.as_deref_mut(), id, change.provoked, written, now);
    }
}

/// `room` rewinds to `saved_at`: put back the flags it wrote after that tick,
/// and the quest steps they moved, except where a later write to the flag
/// stands (ISL-2d).
pub fn retract_room_flag_writes(
    world: &mut bevy::prelude::World,
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    saved_at: u64,
) {
    let Some(taken) = world
        .get_resource_mut::<ambition_combat::events::FlagWritesInRooms>()
        .map(|mut log| log.retract_room(room, saved_at))
        .filter(|taken| !taken.is_empty())
    else {
        return;
    };
    world.resource_scope(|world, mut save: bevy::prelude::Mut<ambition_persistence::save::AmbitionGameSave>| {
        let mut quests = world.resource_mut::<ambition_persistence::quest::QuestRegistry>();
        for (write, superseded) in taken {
            let cause = write.quest_cause();
            if superseded {
                if let Some(cause) = cause {
                    quests.forget_cause(&cause);
                }
                continue;
            }
            save.data_mut().set_flag(write.id, write.replaced);
            let Some(cause) = cause else { continue };
            for quest in quests.retract_caused_by(&cause) {
                if let Some(state) = quests.get(&quest) {
                    save.data_mut().set_quest(&quest, state.progression, state.step);
                }
            }
        }
    });
}

/// Structured quest events from gameplay (NPC talked, item collected, etc.).
///
/// A caused event is queued under its cause, so a room rewind can retract it
/// ([`ambition_combat::events::QuestCausesInRooms`]); the causes no rewind
/// can reach any longer are forgotten here.
pub fn apply_quest_effects(
    mut effects: ReplayMessageReader<QuestAdvanceRequested>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    (causes, tick): (
        Option<ResMut<ambition_combat::events::QuestCausesInRooms>>,
        Option<Res<ambition_time::SimTick>>,
    ),
) {
    if let (Some(mut causes), Some(tick)) = (causes, tick) {
        for old in causes.prune(tick.0) {
            quests.forget_cause(&old);
        }
    }
    for effect in effects.read() {
        match &effect.1 {
            Some(cause) => quests.push_event_caused_by(effect.0.clone(), cause.clone()),
            None => quests.push_event(effect.0.clone()),
        }
    }
}

/// `room` rewinds to `saved_at`: put back the quest steps the events it
/// queued after that tick moved, except where a later event of the same
/// kind still stands (ISL-2d, [`ambition_combat::events::QuestCausesInRooms`]).
pub fn retract_room_quest_causes(
    world: &mut bevy::prelude::World,
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    saved_at: u64,
) {
    let Some(taken) = world
        .get_resource_mut::<ambition_combat::events::QuestCausesInRooms>()
        .map(|mut causes| causes.retract_room(room, saved_at))
        .filter(|taken| !taken.is_empty())
    else {
        return;
    };
    world.resource_scope(|world, mut save: bevy::prelude::Mut<ambition_persistence::save::AmbitionGameSave>| {
        let mut quests = world.resource_mut::<ambition_persistence::quest::QuestRegistry>();
        for (cause, superseded) in taken {
            if superseded {
                quests.forget_cause(&cause);
                continue;
            }
            for quest in quests.retract_caused_by(&cause) {
                if let Some(state) = quests.get(&quest) {
                    save.data_mut().set_quest(&quest, state.progression, state.step);
                }
            }
        }
    });
}

/// Switch activations are gameplay events too. The activation is already
/// typed by the LDtk-to-ECS spawn path (see [`ambition_encounter::switches::SwitchFeature`]),
/// so this consumer just forwards it to the encounter queue and emits the
/// click SFX.
pub fn apply_switch_effects(
    mut effects: ReplayMessageReader<SwitchActivated>,
    mut switch_activations: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        ambition_encounter::switches::SwitchActivationQueue,
    >,
    mut sfx: ambition_sfx::SfxWriter,
) {
    for effect in effects.read() {
        switch_activations
            .0
            .push(ambition_encounter::switches::QueuedSwitchActivation {
                subject: effect.subject.clone(),
                activation: effect.activation.clone(),
                room: effect.room,
            });
        sfx.in_space(ambition_vfx::sfx_space(effect.room)).write(ambition_sfx::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_SWITCH_TOGGLE,
            emitter: (effect.pos).into(),
        });
    }
}

/// Standalone audio-only gameplay events. Presentation-shaped cues with richer
/// semantics should use their own typed messages; this reader only handles
/// bare SFX requests.
pub fn apply_gameplay_sfx_effects(
    mut effects: ReplayMessageReader<GameplaySfxRequested>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    for effect in effects.read() {
        sfx.in_space(ambition_vfx::sfx_space(effect.room)).write(ambition_sfx::SfxMessage::Play {
            id: effect.id,
            emitter: (effect.pos).into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core as ae;

    #[test]
    fn gameplay_effect_messages_remain_typed() {
        let flag = SetFlagsRequested {
            changes: vec![("flag".into(), true)],
            reply: None,
            written: None,
        };
        let quest = QuestAdvanceRequested::new(
            ambition_persistence::quest::QuestAdvanceEvent::NpcTalked("guide".into()),
        );
        let switch = SwitchActivated {
            subject: None,
            activation: ambition_encounter::SwitchActivation {
                id: "goblin_encounter".into(),
                action: "ResetEncounter".into(),
                target_encounter: "goblin_encounter".into(),
            },
            pos: ae::Vec2::new(1.0, 2.0),
            room: None,
        };
        let sfx = GameplaySfxRequested {
            id: ambition_sfx::ids::PLAYER_DAMAGE,
            pos: ae::Vec2::new(5.0, 6.0),
            room: None,
        };

        assert!(flag.changes[0].1);
        assert!(matches!(
            quest.0,
            ambition_persistence::quest::QuestAdvanceEvent::NpcTalked(_)
        ));
        assert_eq!(switch.pos, ae::Vec2::new(1.0, 2.0));
        assert_eq!(sfx.pos, ae::Vec2::new(5.0, 6.0));
    }
}
