//! Tests of the moves the game ships for `pugnacious_polygon`.
//!
//! The table is content: `assets/data/movesets/pugnacious_polygon.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.



use ambition_entity_catalog::smash_capture::CaptureAttemptParams;


use ambition_entity_catalog::MovesetContract;

#[cfg(test)]
mod tests {
    /// The launch conditions a CONTENT claim is about: a fresh reference body
    /// under the undeclared ruleset.
    ///
    /// Named, not implied: an authoring claim is about this world, and a scorer
    /// is not, so each must say which conditions it uses.
    fn fresh(victim_damage: i32) -> ambition_entity_catalog::launch::LaunchConditions {
        ambition_entity_catalog::launch::LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY
            .at_damage(victim_damage)
    }

    /// His two smashes cross, so a kill question must not read base knockback.
    ///
    /// Forward smash is `(162, 3.25)` and up smash is `(158, 5.83)`. The forward
    /// smash has the bigger base; past a few points of damage the up smash has
    /// the bigger launch. A brain feature folded to `max(base)` ranks his
    /// finisher backwards.
    ///
    /// The test is about the order, not the numbers: it asserts that the lines
    /// cross and which side wins on each side. Retuning either smash keeps it
    /// green while the roster still has a base/growth tradeoff; it fails if the
    /// derivation discards growth.
    #[test]
    fn his_up_smash_out_launches_his_forward_smash_once_the_opponent_is_worn() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        let frames = |id: &str| {
            set.moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} is on his table"))
                .frame_data()
        };
        let forward = frames("polygon_brawler_smash_forward").launch;
        let up = frames("polygon_brawler_smash_up").launch;

        assert!(
            forward.at(fresh(0)) > up.at(fresh(0)),
            "against a FRESH opponent the forward smash is the harder launch — \
             if this flips, the arm below is no longer measuring a crossing"
        );
        assert!(
            up.at(fresh(120)) > forward.at(fresh(120)),
            "against a worn one the up smash is, and a scorer reading base \
             knockback alone would never pick it: forward={}, up={}",
            forward.at(fresh(120)),
            up.at(fresh(120))
        );
        assert!(up.grows_under(fresh(0)) && forward.grows_under(fresh(0)));
    }

    /// The slam's shock reaches ground the slam cannot. The two numbers are in
    /// two authoring vocabularies, so only this test holds them together.
    ///
    /// It must stay the weaker half: a follow-up that hit harder would make the
    /// slam a delivery mechanism for its own tail.
    #[test]
    fn his_ground_slam_sends_a_shock_that_outreaches_the_fists_and_hits_softer() {
        use ambition_entity_catalog::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};

        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        let slam = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_ground_slam")
            .expect("his grounded down-B");

        let shock: RiposteStrikeParams = slam
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == RIPOSTE_STRIKE =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("his slam sends a shock");

        // The fists: the widest volume the move authors on its own timeline.
        let fists_reach = slam
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            // `leading_edge_x`, not `offset.0 + half_extents.0`:
            // `test_the_grab_reach_is_one_formula` requires the one formula, and this
            // file names `CaptureAttemptParams`, so that guard covers it.
            .map(|volume| volume.shape.leading_edge_x())
            .fold(f32::MIN, f32::max);
        assert!(
            fists_reach > f32::MIN,
            "the slam has no hitbox of its own any more, so this test is \
             comparing the shock against nothing",
        );
        assert!(
            shock.reach + shock.half_extents.0 > fists_reach,
            "the shock reaches {}px and the fists {fists_reach}px — a follow-up \
             that covers no new ground is a second hit on the same square",
            shock.reach + shock.half_extents.0,
        );

        let fists_damage = slam
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            .map(|volume| volume.damage)
            .max()
            .expect("the slam deals damage");
        assert!(
            (shock.damage as i32) < fists_damage,
            "the shock deals {} against the fists' {fists_damage}: the tail \
             must not outhit the impact it follows",
            shock.damage,
        );
        assert!(
            shock.problems().is_empty(),
            "the shock is authored unusably: {}",
            shock.problems().join("; "),
        );
    }

    /// His punch charges and does not store.
    ///
    /// The Projectile Polygon's neutral-B is authored to store, at the
    /// maintainer's request. A ranged fighter banks a shot; a brawler commits.
    /// This test fails if either fighter is retuned toward the other.
    #[test]
    fn his_haymaker_charges_and_deliberately_does_not_store() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        let haymaker = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_haymaker")
            .expect("his neutral-B");
        let charge = haymaker
            .smash_charge
            .as_ref()
            .expect("his neutral-B charges");
        assert!(
            !charge.stores,
            "his haymaker stores its charge, which makes the commitment a \
             resource and takes the read out of the move",
        );
        assert!(
            charge.roots,
            "a charge that does not root him is a threat with no commitment",
        );
        assert!(
            haymaker.smash_charge_mult > 1.0,
            "charging his punch pays {}x, so holding it is strictly worse than \
             throwing it",
            haymaker.smash_charge_mult,
        );

        // The other half of the contrast, asserted rather than described.
        let hers = crate::authored_movesets::shipped("projectile_polygon");
        let shot = hers
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_charge_shot")
            .expect("her neutral-B");
        assert!(
            shot.smash_charge.as_ref().is_some_and(|c| c.stores),
            "her charge shot stopped storing, so the brawler's not-storing says \
             nothing any more — the storing was asked for on THAT move \
             specifically",
        );
    }

    /// Every capture attempt a move authors, by move id.
    fn capture_of(set: &MovesetContract, id: &str) -> CaptureAttemptParams {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not in this table"))
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| effect.key == ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT)
            .and_then(|effect| effect.params.hydrate().ok())
            .unwrap_or_else(|| panic!("`{id}` carries no capture attempt"))
    }

    /// His side-B is a command grab.
    ///
    /// Both grabs feed the same four throws, so the hold must match: a captive
    /// held elsewhere would make the throws read differently per grab.
    #[test]
    fn his_side_b_is_a_command_grab_that_shares_the_hold_with_his_standing_one() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        let standing = capture_of(&set, "polygon_brawler_grab");
        let command = capture_of(&set, "polygon_brawler_collar");

        assert_eq!(
            command.hold_offset, standing.hold_offset,
            "the command grab holds captives at {:?} and the standing grab at \
             {:?} — the throws are shared, so they would read differently \
             depending on which grab caught you",
            command.hold_offset, standing.hold_offset,
        );
        assert!(
            command.reach_x() > standing.reach_x(),
            "the command grab reaches {}px and the standing grab {}px — a \
             command grab that closes no distance is a worse standing grab",
            command.reach_x(),
            standing.reach_x(),
        );
        let travels = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_collar")
            .and_then(|m| m.start_impulse);
        assert!(
            travels.is_some_and(|(x, _)| x > 0.0),
            "his command grab does not travel ({travels:?}), so it is a standing \
             grab on a different button",
        );
    }

    /// The brain can see how far the command grab reaches. A capture rides an
    /// Active window's `sustain_effect`, not its volume list, so `frame_data`
    /// folded over `volumes` alone gave every grab no region. Only the neutral
    /// grab (through `GRAB_VERB`) was patched; this one is bound to `attack_side`,
    /// so it was offered at every gap and priced at zero.
    ///
    /// The standing grab is the control: a derivation that fixes only the broken
    /// move is another special case.
    #[test]
    fn both_of_his_grabs_tell_the_brain_the_distance_they_close() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        for id in ["polygon_brawler_collar", "polygon_brawler_grab"] {
            let spec = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is on his table"));
            let frames = spec.frame_data();
            let authored = capture_of(&set, id).reach_x();
            assert_eq!(
                frames.reach, authored,
                "`{id}` reaches {authored}px and its frame data says {}",
                frames.reach,
            );
            let coverage = frames
                .coverage
                .unwrap_or_else(|| panic!("`{id}` has no region, so no gap can miss it"));
            assert_eq!(coverage.max.0, authored, "`{id}`'s region stops short of its reach");
            assert!(
                frames.ignores_guard,
                "`{id}` is a capture, and a raised shield is not the answer to one",
            );
        }
    }
    use super::*;

    /// His up-B opens a parasol that outlives it; the duration is the assertion,
    /// not the presence.
    ///
    /// A `WindowTag` would satisfy "the move slows his fall" and fail this. The
    /// regime must still run after the move's timeline ends.
    #[test]
    fn the_uppercut_leaves_him_floating_for_longer_than_the_move_lasts() {
        use ambition_entity_catalog::MoveEventKind;
        let moves = crate::authored_movesets::shipped("pugnacious_polygon");
        let up = moves
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_uppercut")
            .expect("his up-B is in the table");
        let (at_s, scale, seconds) = up
            .events
            .iter()
            .find_map(|e| match e.kind {
                MoveEventKind::GravityModifier { scale, seconds } => Some((e.at_s, scale, seconds)),
                _ => None,
            })
            .expect(
                "his up-B authors no gravity modifier, so the roster's dullest \
                 recovery is a bare uppercut again",
            );
        assert!(
            scale < 1.0 && scale > 0.0,
            "a modifier of {scale} does not SLOW a fall — 1.0 is a no-op and \
             0.0 is a hover, and neither is a parasol"
        );
        let move_ends = up.duration_s;
        assert!(
            at_s + seconds > move_ends,
            "the float ({seconds}s from {at_s}s) is spent by the time the move \
             ends at {move_ends}s, so it can only slow a fall he is not having \
             yet — the whole reason this is an event and not a window is that it \
             has to outlast the move"
        );
    }

    #[test]
    fn the_reference_brawler_answers_the_complete_typed_repertoire() {
        let moves = crate::authored_movesets::shipped("pugnacious_polygon");
        for id in [
            "polygon_brawler_jab",
            "polygon_brawler_tilt_forward",
            "polygon_brawler_tilt_up",
            "polygon_brawler_tilt_down",
            "polygon_brawler_smash_forward",
            "polygon_brawler_smash_up",
            "polygon_brawler_smash_down",
            "polygon_brawler_air_neutral",
            "polygon_brawler_air_forward",
            "polygon_brawler_air_back",
            "polygon_brawler_air_up",
            "polygon_brawler_air_down",
            "polygon_brawler_haymaker",
            "polygon_brawler_collar",
            "polygon_brawler_uppercut",
            "polygon_brawler_ground_slam",
            "polygon_brawler_body_drop",
            "polygon_brawler_grab",
            "polygon_brawler_pummel",
            "polygon_brawler_throw_forward",
            "polygon_brawler_throw_back",
            "polygon_brawler_throw_up",
            "polygon_brawler_throw_down",
            "pugnacious_polygon_taunt",
            "pugnacious_polygon_dash_attack",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}
