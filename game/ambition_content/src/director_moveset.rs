//! Tests of the moves the game ships for `director`.
//!
//! The table is content: `assets/data/movesets/director.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.

#[cfg(test)]
mod tests {

    /// The swap is complete: the verb points at the new move, the move is in the
    /// table, and the archetype's rise is removed, not left unreachable.
    #[test]
    fn the_director_recovers_by_teleporting_and_the_archetypes_rise_is_gone() {
        let set = crate::authored_movesets::shipped("director");
        assert_eq!(
            set.verbs.get("special_up").map(String::as_str),
            Some("director_revision"),
            "his up-B must be the teleport"
        );
        assert!(
            set.moves.iter().any(|m| m.id == "director_revision"),
            "…and the move it names must be in the table"
        );
        assert!(
            !set.moves.iter().any(|m| m.id == "director_rising_edge"),
            "…and the archetype's spinning rise must not be left behind \
             unreachable, where every census that walks `moves` reports it as \
             part of his kit"
        );
    }

    /// He must be free before the thought fades, or a whiff pins him through his
    /// own punish window. The guard states the relationship between the two
    /// numbers, not either number.
    #[test]
    fn his_thought_outlives_the_move_that_threw_it() {
        let set = crate::authored_movesets::shipped("director");
        assert_eq!(
            set.verbs.get("special_forward").map(String::as_str),
            Some("director_train_of_thought"),
            "his side-B must be the steered thought"
        );
        let move_spec = set
            .moves
            .iter()
            .find(|m| m.id == "director_train_of_thought")
            .expect("…and the move it names must be in the table");
        let bolt: ambition_entity_catalog::smash_bolt::SteeredBoltParams = move_spec
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key
                        == ambition_entity_catalog::smash_bolt::STEERED_BOLT =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("his side-B throws a bolt");
        assert!(
            bolt.lifetime_s > move_spec.duration_s,
            "the thought fades at {}s inside a {}s move, so he is pinned watching \
             nothing",
            bolt.lifetime_s,
            move_spec.duration_s,
        );

        // It carries him: `self_launch` must beat the bolt's speed, or it would be
        // slower than walking.
        assert!(
            bolt.self_launch > bolt.speed,
            "the thunder jacket ({}) is slower than the bolt ({})",
            bolt.self_launch,
            bolt.speed,
        );

        // The archetype's lunge is removed, not left unreachable.
        assert!(
            !set.moves.iter().any(|m| m.id == "director_vector_lunge"),
            "the displaced lunge is still in the table"
        );
    }

    /// The counter is his: the response must be a teleport that arrives behind
    /// the attacker. A check for `smash.counter` alone would pass a copy of
    /// George's riposte.
    #[test]
    fn the_directors_counter_answers_by_arriving_behind_whoever_swung() {
        let set = crate::authored_movesets::shipped("director");
        assert_eq!(
            set.verbs.get("special_down").map(String::as_str),
            Some("director_second_draft"),
            "his down-B must be the counter"
        );
        let counter = set
            .moves
            .iter()
            .find(|m| m.id == "director_second_draft")
            .expect("…and the move it names must be in the table");

        let params: ambition_entity_catalog::smash_counter::CounterParams = counter
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| {
                effect.key == ambition_entity_catalog::smash_counter::COUNTER
            })
            .expect("the move holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        assert_eq!(
            params.response,
            ambition_entity_catalog::smash_teleport::TELEPORT,
            "his counter must answer with the teleport, not a grab"
        );
        let teleport: ambition_entity_catalog::smash_teleport::TeleportParams =
            params.response_params.hydrate().expect("teleport params hydrate");
        assert!(
            teleport.behind_nearest_foe,
            "the response is an AIMED teleport, so he answers a parry by leaving \
             rather than by arriving behind the swing"
        );
        // Not a recovery: a positive `ledge_assist` would let a ledge catch the
        // ambush arrival, and would give him a second recovery.
        assert_eq!(teleport.ledge_assist, 0.0);
        assert!(
            params.absorbs_projectiles,
            "he deletes shots rather than returning them — the difference from \
             the riposte, and what stops this being strictly better than it"
        );
    }

    /// The down swap took the grounded half and left the aerial one. His
    /// borrowed down-special is `DownSpecial::ByPosture` (two moves on two
    /// verbs), so replacing `special_down` must keep his air-down.
    #[test]
    fn the_counter_displaced_the_ground_low_arc_and_spared_the_falling_edge() {
        let set = crate::authored_movesets::shipped("director");
        // Name the id the borrowed table currently provides, not a historical one;
        // a renamed id would make this check pass without testing anything.
        assert!(
            !set.moves.iter().any(|m| m.id == "director_riposte"),
            "the displaced grounded down-B is still in the table, where every \
             census that walks `moves` reports it as part of his kit"
        );
        assert!(
            set.moves.iter().any(|m| m.id == "director_falling_edge"),
            "his AERIAL down-special went with it — that verb was never replaced"
        );
    }

    /// It is still a recovery. `UpSpecial::Standard` sets `gates.recovery` on
    /// the move it lowers; a replacement inserted after that must carry the cost,
    /// or he gets an unlimited teleport.
    #[test]
    fn the_replacement_still_spends_the_airtimes_recovery() {
        let set = crate::authored_movesets::shipped("director");
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "director_revision")
            .expect("his up-B is in the table");
        assert_ne!(
            up.gates.recovery,
            ambition_entity_catalog::RecoveryUse::None,
            "an up-B that costs nothing is flight"
        );
    }
}
