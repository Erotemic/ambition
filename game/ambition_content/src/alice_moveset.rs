//! Tests of the moves the game ships for `npc_alice`.
//!
//! The table is content: `assets/data/movesets/alice.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::MovesetContract;


#[cfg(test)]
mod tests {
    use super::*;

    /// The pad is armoured during the wind-up and not after. The end of the
    /// armour is the test: armour over the active frames would win every
    /// simultaneous exchange, and a check for `WindowTag::Armor` alone would
    /// pass that.
    #[test]
    fn her_one_time_pad_is_armoured_only_while_she_winds_up() {
        use ambition_entity_catalog::WindowTag;
        let pad = crate::authored_movesets::shipped("npc_alice")
            .move_by_id("one_time_pad")
            .expect("one_time_pad exists")
            .clone();
        let armor = pad
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Armor)
            .expect("the unbreakable cipher is armoured");
        let active = pad
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("it is still a strike");
        assert!(
            armor.end_s <= active.start_s,
            "armour must close as the hitbox opens, or the trade is free: \
             armour ends {}, hitbox opens {}",
            armor.end_s,
            active.start_s,
        );
        assert!(
            armor.start_s > 0.0,
            "a fast enough answer must still beat it, so it cannot open on frame 0"
        );
    }

    /// `hash_collision` holds twice and launches once. The holds are
    /// `VolumeReaction::Autolink`; the finisher carries no reaction.
    ///
    /// The gap between pulses matters: the re-hit rule refuses a second hit
    /// across a contiguous track, so touching windows would land once.
    #[test]
    fn her_hash_collision_holds_twice_and_launches_once() {
        use ambition_entity_catalog::VolumeReaction;
        let collision = crate::authored_movesets::shipped("npc_alice")
            .move_by_id("hash_collision")
            .expect("hash_collision exists")
            .clone();
        let striking: Vec<_> = collision
            .windows
            .iter()
            .filter(|w| !w.volumes.is_empty())
            .collect();
        assert_eq!(
            striking.len(),
            3,
            "two inputs and one output: {} striking window(s)",
            striking.len()
        );
        for (index, window) in striking[..2].iter().enumerate() {
            assert!(
                window
                    .volumes
                    .iter()
                    .all(|v| matches!(v.reaction, Some(VolumeReaction::Autolink(_)))),
                "input {index} must HOLD, or the finisher swings at nobody"
            );
        }
        assert!(
            striking[2].volumes.iter().all(|v| v.reaction.is_none()),
            "the output must launch: a held finisher is a move that never lets go"
        );
        for pair in striking.windows(2) {
            assert!(
                pair[0].end_s < pair[1].start_s,
                "the windows must be SEPARATED or the runtime lands one hit: \
                 {} then {}",
                pair[0].end_s,
                pair[1].start_s,
            );
        }
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// Alice is not Bob with different names. The pair's split is the
    /// design: she reaches further and recovers sooner, he hits harder and
    /// commits longer. A table copied between them would pass every other test.
    #[test]
    fn alice_reaches_further_than_bob_and_bob_hits_harder() {
        let alice = crate::authored_movesets::shipped("npc_alice");
        let bob = crate::authored_movesets::shipped("npc_bob");
        let reach = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| match v.shape {
                    ambition_entity_catalog::VolumeShape::Rect {
                        offset,
                        half_extents,
                    } => offset.0.abs() + half_extents.0,
                    _ => 0.0,
                })
                .fold(0.0f32, f32::max)
        };
        let damage = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        assert!(
            reach(&alice, "cipher_sweep") > reach(&bob, "wrench_swing"),
            "the sender reaches further"
        );
        assert!(
            damage(&bob, "rivet_smash") > damage(&alice, "brute_force"),
            "and the one who builds things hits harder when he connects"
        );
    }
}

#[cfg(test)]
mod portal_recovery_tests {
    use ambition_entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};
    use ambition_entity_catalog::MoveEventKind;

    /// Her up-B opens a portal pair and does not also throw an impulse. With both,
    /// she would recover on the impulse and the portals would be scenery, while
    /// "it opens a portal" still passed.
    #[test]
    fn the_up_special_recovers_through_a_portal_rather_than_an_arc() {
        let kit = crate::authored_movesets::shipped("npc_alice");
        let up_b = kit
            .moves
            .iter()
            .find(|m| m.id == "elliptic_curve")
            .expect("Alice authors her up-special");

        let pair = up_b
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == PORTAL_PAIR => Some(effect),
                _ => None,
            })
            .expect("the up-special opens a portal pair");
        let params: PortalPairParams = pair.params.hydrate().expect("portal params hydrate");
        assert!(
            params.rise > 0.0,
            "the pair's exit is not above its entrance, so falling in returns \
             her where she started"
        );

        assert!(
            up_b.start_impulse.is_none(),
            "the up-special still throws a start impulse alongside its portal \
             pair, so the arc is the recovery and the portals are scenery"
        );
        // An impulse is its own event kind (`MoveEventKind::Impulse`), not an
        // `Effect` with a telling key.
        let thrown: Vec<(f32, &ambition_entity_catalog::ImpulseMode)> = up_b
            .events
            .iter()
            .filter_map(|ev| match &ev.kind {
                MoveEventKind::Impulse { mode, .. } => Some((ev.at_s, mode)),
                _ => None,
            })
            .collect();
        assert!(
            thrown.is_empty(),
            "the up-special throws {thrown:?} beside its portal pair, so the arc \
             is the recovery and the portals are scenery"
        );
    }
}
