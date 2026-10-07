//! Fold the active room's declared combat rules over the world's baseline. (AE6)
//!
//! The type this produces —
//! [`ResolvedCombatTuning`](ambition_combat::rules::ResolvedCombatTuning) — lives
//! in `ambition_combat`, because `on_hit`, `hitbox` and the damage paths are its
//! readers and a type must sit at or below its readers. The FOLD lives here,
//! one layer up, because its inputs do not both live down there: friendly fire
//! is combat's own baseline, and `di_max_angle` belongs to this crate's feel
//! tuning. Ownership travels down with the type; the projection happens where
//! the facts are visible.
//!
//! this is a DERIVED resource — rebuilt every tick from inputs that are
//! themselves either rollback state (the active room) or authored constants
//! (the declarations), so a rewind does not need to restore it and must not
//! try to.

use bevy::prelude::{Commands, Res};

/// Rebuild each live room's [`RoomCombatTuning`] from that room's declared
/// rules and the baseline, and [`ResolvedCombatTuning`] from the rules that
/// govern no room.
///
/// Per live room, not from THE live room (OW1): with two rooms live, the
/// one-room read gave no room, so both rooms played under the rules of none
/// (a 3-damage robot hit in Alice's Ambition room was classed light because
/// Bob held another room).
///
/// [`RoomCombatTuning`]: ambition_combat::rules::RoomCombatTuning
///
/// Runs in `Platformer2dSimulationPhase::WorldPrep`, which is before every reader: the damage
/// paths are in `PlayerSimulation`/`Combat`, and a resolution landing after them
/// would give the hit kernel last tick's rules on the tick a match opens — the
/// one tick where they differ.
pub fn project_combat_rules(
    mut commands: Commands,
    declared: crate::session::governing_rules::GoverningRules<ambition_combat::rules::CombatRules>,
    // The room's line between light and heavy hits: a rule of its own, so a
    // game that declares no combat ruleset can still draw it.
    strike_weight: crate::session::governing_rules::GoverningRules<
        ambition_combat::strike_weight::StrikeWeightRules,
    >,
    baseline_feel: Option<Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>>,
    baseline_ff: Option<Res<ambition_combat::targeting::FriendlyFire>>,
    rule_rooms: crate::session::governing_rules::LiveRuleRooms,
    roots: bevy::prelude::Query<
        (
            bevy::prelude::Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        ),
        bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
) {
    // `Option` on both baselines: a minimal headless world that never stands
    // up the tuning resources still resolves, and `resolve_over` says what an
    // absent baseline stands at.
    let resolve = |room: ambition_combat::scoped_rules::ActiveRoom<'_>| {
        ambition_combat::rules::ResolvedCombatTuning::resolve_over(
            declared.in_room(room),
            baseline_feel.as_deref(),
            baseline_ff.as_deref(),
        )
        .with_strike_weight(strike_weight.in_room(room))
    };
    commands.insert_resource(resolve(ambition_combat::scoped_rules::ActiveRoom::NoRoom));
    for (root, room) in &roots {
        commands
            .entity(root)
            .insert(ambition_combat::rules::RoomCombatTuning(resolve(rule_rooms.of(*room))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::rules::RoomCombatTuning;
    use ambition_combat::scoped_rules::{DeclaredRules, RulesScope};
    use ambition_combat::strike_weight::StrikeWeightRules;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        session_world_component, LiveRoomInstance, RoomInstanceRoot, SessionRoot, SessionScopeId,
    };
    use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};
    use bevy::prelude::{App, Update};

    fn room(id: &str, mode: Option<&str>) -> RoomSpec {
        let mut room = RoomSpec::new(
            id,
            ambition_platformer2d_core::World::new(
                id,
                ambition_platformer2d_core::Vec2::new(320.0, 240.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        room.metadata.mode = mode.map(str::to_owned);
        room
    }

    /// Two live rooms of two games hold two lines between light and heavy
    /// hits at once: Ambition's room is heavy at 3, Smash's at 12, and the
    /// rules of no room draw no line. When the rules were resolved from THE
    /// live room, both rooms held the rules of no room.
    #[test]
    fn two_live_rooms_of_two_games_hold_their_own_heavy_lines() {
        let mut app = App::new();
        let mut lines = DeclaredRules::<StrikeWeightRules>::default();
        lines.declare(RulesScope::UntaggedRooms, StrikeWeightRules::heavy_at(3));
        lines.declare(RulesScope::Mode("smash"), StrikeWeightRules::heavy_at(12));
        app.insert_resource(lines)
            .add_systems(Update, project_combat_rules);
        app.world_mut().spawn((
            SessionRoot(SessionScopeId(1)),
            RoomSet::from_parts_or_panic(
                "hall",
                vec![room("hall", None), room("stage", Some("smash"))],
                Vec::new(),
            ),
        ));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("the fixture set holds its room");
        let first = *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
            LiveRoomInstance,
        >(app.world())
        .expect("the hall is live");
        let stage = session_world_component::<RoomSet>(app.world())
            .and_then(|rooms| rooms.definition_by_id("stage"))
            .expect("the fixture set holds the stage");
        app.world_mut().spawn((RoomInstanceRoot, first.next(), stage));
        app.update();
        let world = app.world_mut();
        let mut rooms: Vec<(LiveRoomInstance, Option<StrikeWeightRules>)> = world
            .query::<(&LiveRoomInstance, &RoomCombatTuning)>()
            .iter(world)
            .map(|(room, tuning)| (*room, tuning.0.strike_weight))
            .collect();
        rooms.sort_by_key(|(room, _)| *room);
        let roomless = world
            .resource::<ambition_combat::rules::ResolvedCombatTuning>()
            .strike_weight;
        assert_eq!(
            (rooms, roomless),
            (
                vec![
                    (first, Some(StrikeWeightRules::heavy_at(3))),
                    (first.next(), Some(StrikeWeightRules::heavy_at(12))),
                ],
                None,
            ),
            "(each live room's line, the line of no room)"
        );
    }
}
