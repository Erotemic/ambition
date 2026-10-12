//! The trail of each fist of GNU-ton's fight.
//!
//! The pair of the punch (phase 1) is a REAL pair of portals, and the line
//! at each shoulder and behind each wrist (after phase 1) is a LOOK. Neither
//! is here: his conductor (the module
//! `ambition_content_modules::gnu_ton::conductor`) asks the portal domain
//! for each one each tick (`ambition.portal.held_pair` and
//! `ambition.portal.light_line`). This puts the start of each fist's trail
//! on the fist, from the conductor's record.

use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_content_modules::gnu_ton::conductor::Conductor;
use ambition_platformer2d::platformer::lifecycle::{LiveRoomInstance, LiveRooms};

/// A living GNU-ton's scholar: his body, his encounter, his conductor's
/// record.
type Scholars<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        BossClusterRef,
        &'static ambition_extension_host::BodyRecords,
        &'static ambition_characters::actor::BodyHealth,
    ),
>;

/// The conductor's record of each living GNU-ton, with his scholar, his
/// placement id and his live room.
fn conductors_of(scholars: &Scholars, live: &LiveRooms) -> Vec<(Entity, String, LiveRoomInstance, Conductor)> {
    scholars
        .iter()
        .filter(|(_, boss, _, health)| super::boss_is_gnu_ton(&boss.as_boss_ref()) && health.alive())
        .filter_map(|(scholar, boss, records, _)| {
            let conductor = Conductor::from_record(records.get(&Conductor::KEY)?).ok()?;
            Some((scholar, boss.as_boss_ref().config.id.clone(), live.of(scholar)?, conductor))
        })
        .collect()
}

/// The trail of each fist is part of his spell: it joins the two lines the
/// arm is in. It starts at the shoulder's line (after phase 1), or at the
/// near line of the punch for the fist that is through the pair. A fist on
/// its arm has its trail from the giant, where its body is: no start of its
/// own. The start is on the fist (`LimbBondStart`), so each fist has its
/// own, and a fist of a scholar who is dead has none.
pub fn join_gnu_ton_lines(
    mut commands: Commands,
    live: LiveRooms,
    scholars: Scholars,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    starts: Query<&ambition_characters::actor::LimbBondStart>,
) {
    use ambition_characters::actor::{LimbBondStart, LimbSlot};
    let living = conductors_of(&scholars, &live);
    for (scholar, boss, ..) in &scholars {
        if !super::boss_is_gnu_ton(&boss.as_boss_ref()) {
            continue;
        }
        let Some(rig) = riders.get(scholar).ok().and_then(|riding| rigs.get(riding.mount).ok()) else {
            continue;
        };
        let conductor = living.iter().find(|(s, ..)| *s == scholar).map(|(.., conductor)| conductor);
        for (index, slot) in [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].into_iter().enumerate() {
            let Some(fist) = rig.get(slot) else {
                continue;
            };
            let want = conductor.and_then(|conductor| {
                // The right fist is the one that goes through the pair of the punch.
                let through = conductor.punch_pair().filter(|_| index == 1).map(|[near, _]| near);
                conductor.look_lines()[index].map(|(at, _)| at).or(through)
            });
            let now = starts.get(fist).ok().map(|start| start.0);
            match want.map(|at| Vec2::new(at.x, at.y)) {
                Some(at) if now != Some(at) => {
                    commands.entity(fist).try_insert(LimbBondStart(at));
                }
                None if now.is_some() => {
                    commands.entity(fist).try_remove::<LimbBondStart>();
                }
                _ => {}
            }
        }
    }
}
