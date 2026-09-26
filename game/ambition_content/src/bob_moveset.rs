//! Tests of the moves the game ships for `npc_bob`.
//!
//! The table is content: `assets/data/movesets/bob.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::MovesetContract;


#[cfg(test)]
mod tests {
    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// He commits for longer than she does, on every press they both have.
    /// The pair's other half is asserted in `alice_moveset`; this is the axis
    /// that is his.
    #[test]
    fn bob_is_slower_to_start_than_alice_on_every_shared_press() {
        let bob = crate::authored_movesets::shipped("npc_bob");
        let alice = crate::authored_movesets::shipped("npc_alice");
        let startup = |set: &MovesetContract, verb: &str| {
            set.move_for_verb(verb)
                .unwrap_or_else(|| panic!("{verb} is bound"))
                .windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_entity_catalog::WindowTag::Active
                    )
                })
                .expect("a strike has an active window")
                .start_s
        };
        for verb in ["attack", "attack_forward", "smash_forward", "attack_air"] {
            assert!(
                startup(&bob, verb) > startup(&alice, verb),
                "`{verb}` comes out at least as fast for the engineer as for the \
                 cryptographer, so the pair is one table twice"
            );
        }
    }

    /// "Not one hit, the tool running": the neutral multi-hits. One contiguous
    /// window lands once (see `Pulse`).
    #[test]
    fn his_rivet_gun_runs_rather_than_landing_once() {
        let set = crate::authored_movesets::shipped("npc_bob");
        let gun = set
            .moves
            .iter()
            .find(|m| m.id == "rivet_gun")
            .expect("his neutral special");

        let mut hitting: Vec<(f32, f32)> = gun
            .windows
            .iter()
            .filter(|w| w.volumes.iter().any(|v| v.damage > 0))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        hitting.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(
            hitting.len() >= 3,
            "a tool that runs has more than {} hitting window(s)",
            hitting.len()
        );

        // The gaps are the move, not the count: touching windows land once, so
        // assert the separation.
        for pair in hitting.windows(2) {
            let gap = pair[1].0 - pair[0].1;
            assert!(
                gap > 0.0,
                "windows {:?} and {:?} touch, so the runtime hands the hit set \
                 forward and the tool lands once",
                pair[0],
                pair[1]
            );
        }

        // The pulses hold: an intermediate hit that launches throws the victim out
        // of the later windows.
        let holding = gun
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter(|v| v.reaction.is_some())
            .count();
        assert!(holding >= 3, "only {holding} of the pulses hold their victim");

        // The finisher is unchanged: an addition, not a rebalance.
        assert!(
            gun.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.reaction.is_none() && v.damage == 12),
            "the rivet no longer drives home at its authored 12"
        );
    }

    /// The plate is an addition and the slam is unchanged. Both in one test: a
    /// check for the plate alone would pass a down-B that lost its hitbox.
    #[test]
    fn his_bulkhead_drop_still_slams_and_now_leaves_the_plate_it_names() {
        let set = crate::authored_movesets::shipped("npc_bob");
        let drop = set
            .moves
            .iter()
            .find(|m| m.id == "bulkhead_drop")
            .expect("his grounded down special");

        // The slam, unchanged.
        assert!(
            drop.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage == 12),
            "the slam lost its authored damage"
        );

        let plate: ambition_entity_catalog::smash_spring::PlaceSpringParams = drop
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key
                        == ambition_entity_catalog::smash_spring::PLACE_SPRING =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("he drops a plate, which his comment has always said");

        // It throws upward. Up is negative y.
        assert!(plate.launch.1 < 0.0, "the plate throws downward: {:?}", plate.launch);
        assert!(plate.uses > 0, "a plate nobody can use is an invisible object");

        // Short-lived: a plate that outlived its exchange would be terrain.
        assert!(
            plate.lifetime_s <= 12.0,
            "the plate lasts {}s, which is stage geometry rather than a move",
            plate.lifetime_s
        );

        // It must not out-launch his own recovery.
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "steam_lift")
            .expect("his recovery");
        let rise = lift
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Impulse { local, .. } => {
                    Some(local.1.abs())
                }
                _ => None,
            })
            .unwrap_or(f32::INFINITY);
        assert!(
            plate.launch.1.abs() < rise,
            "the plate ({}) throws harder than his recovery ({rise})",
            plate.launch.1.abs(),
        );
    }
}
