//! What the save says an authored body's state is when it is BUILT.
//!
//! ADR 0022: a persisted death holds for EVERY persistent actor — including a
//! killed but never-provoked peaceful NPC — and ONLY for a body whose policy
//! keeps a death record at all.
use super::PersistedFates;
use ambition_entity_catalog::placements::RespawnPolicy;
use ambition_persistence::save_data::{AmbitionGameSaveData, PersistedEncounterState};
use ambition_platformer2d_actor_spawn::RecordedFate;

fn fates(edit: impl FnOnce(&mut AmbitionGameSaveData)) -> PersistedFates {
    let mut save = ambition_persistence::save::AmbitionGameSave::default();
    edit(save.data_mut());
    PersistedFates::from_save(save.data())
}

#[test]
fn a_killed_unprovoked_npc_is_built_dead() {
    let fates = fates(|save| {
        // The kill hook wrote the DeadStaysDead flag; the NPC was never
        // provoked, so its `npc_<id>_hostile` flag is absent.
        save.set_flag(crate::features::enemy_dead_flag("kernel_guide"), true);
    });
    assert_eq!(fates.npc_fate("kernel_guide"), RecordedFate::Dead);
    assert_eq!(
        fates.npc_fate("other_guide"),
        RecordedFate::AsAuthored,
        "an NPC with no death record is built alive"
    );
}

/// ⭐⭐ A BODY WHOSE KIND NEVER WRITES A FLAG MUST NOT READ ONE.
///
/// The death path writes `enemy_<id>_dead` only for `DeadStaysDead` and
/// `enemy_<id>_dead_until_rest` only for `OnRest`. A SUMMONED body shares one id
/// with every instance ever made of it (the pirate's recovery shark is always
/// `smash_ride_shark`), so a save already carrying a flag for that id must not
/// kill every later summon of it.
///
/// ⛔ THE TWO BODIES CARRY THE SAME FLAG and differ ONLY in policy, so a pass
/// cannot be bought by the flag being absent.
#[test]
fn a_body_that_never_persists_its_death_ignores_a_flag_bearing_its_name() {
    let fates = fates(|save| {
        save.set_flag(crate::features::enemy_dead_flag("guide"), true);
    });
    assert_eq!(
        fates.enemy_fate_under("guide", RespawnPolicy::OnRoomReenter),
        RecordedFate::AsAuthored,
        "an `OnRoomReenter` body was built dead by a flag its own death path never writes"
    );
    assert_eq!(
        fates.enemy_fate_under("guide", RespawnPolicy::DeadStaysDead),
        RecordedFate::Dead,
        "a `DeadStaysDead` body stopped honouring its own death flag"
    );
}

/// The read is wider than the write: a placement re-authored from `OnRest` to
/// `DeadStaysDead` after the save was written still reads its old record.
#[test]
fn a_persisting_body_honours_either_death_record() {
    let fates = fates(|save| {
        save.set_flag(crate::features::enemy_dead_until_rest_flag("sentry"), true);
    });
    assert_eq!(
        fates.enemy_fate_under("sentry", RespawnPolicy::DeadStaysDead),
        RecordedFate::Dead
    );
    assert_eq!(fates.enemy_fate_under("sentry", RespawnPolicy::OnRest), RecordedFate::Dead);
}

#[test]
fn a_cleared_boss_placement_is_built_dead() {
    let fates = fates(|save| save.set_boss("warden", PersistedEncounterState::Cleared));
    assert_eq!(fates.boss_fate("warden"), RecordedFate::Dead);
    assert_eq!(
        fates.boss_fate("other_warden"),
        RecordedFate::AsAuthored,
        "cleared is keyed by PLACEMENT, not archetype"
    );
}

#[test]
fn a_commit_no_record_reaches_builds_every_body_as_authored() {
    let fates = PersistedFates::unrecorded();
    assert_eq!(fates.npc_fate("kernel_guide"), RecordedFate::AsAuthored);
    assert_eq!(fates.boss_fate("warden"), RecordedFate::AsAuthored);
}
