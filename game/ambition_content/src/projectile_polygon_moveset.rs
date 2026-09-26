//! Tests of the moves the game ships for `projectile_polygon`.
//!
//! The table is content: `assets/data/movesets/projectile_polygon.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.






/// When the bomb reaches the floor.
const BOMB_LAID_AT_S: f32 = 0.18;

/// When the shot leaves, measured from the move's start. Everything before it is
/// windup that plays out on release.
const CHARGE_FIRE_AT_S: f32 = 0.26;

/// When the tail leaves her hand. Late enough to read as a wind-up and be
/// punishable on reaction.
const PONYTAIL_THROWN_AT_S: f32 = 0.16;

/// How far her line reaches, in world px.
///
/// One number for two moves: her grab is a tether
/// (`offset.0 + half_extents.0` = 86 + 64) and her up-B throws the same line
/// at a ledge. `the_tether_reaches_as_far_as_her_grab` keeps them equal,
/// because the grab is authored as a box, not a distance.
const THE_TETHERS_REACH: f32 = 150.0;

#[cfg(test)]
mod tests {

    /// Her neutral game has no hit volume.
    ///
    /// The boomerang and the charge shot author no Active volume (the projectile
    /// is the damage), so `MoveFrameData::coverage` is `None` for both. An option
    /// layer that read `None` as "reaches nobody" would remove both from her
    /// attack menu. She is admitted through `hazard_reach`, from the
    /// `MoveEventKind::Ranged` event.
    ///
    /// This tests the join, not the number: 1000px is a stage-crossing
    /// placeholder, because the body owns the shot's real speed and flight.
    /// Retune this test if that changes; do not delete it.
    #[test]
    fn her_two_neutral_projectiles_tell_the_brain_they_cross_the_stage() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        for id in [
            "polygon_ponytail_boomerang",
            "polygon_projectile_charge_shot",
        ] {
            let m = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her moveset"));
            let frames = m.frame_data();
            // Premise: if one of these gets a hit volume, the test below means
            // nothing.
            assert!(
                frames.coverage.is_none(),
                "`{id}` now authors a hit volume, so this test no longer \
                 guards the road it was written for",
            );
            // A request, not a distance. The shot's speed, flight and lifetime are on
            // the body, so the catalog asks and the kit builder answers with her
            // `RangedActionSpec`. A reader that never joins a body gets the standing
            // `RANGED_ACTION_REACH` through `MoveHazard::reach`, asserted beside it.
            assert_eq!(
                frames.hazard,
                Some(ambition_entity_catalog::MoveHazard::OwnersRangedAction),
                "`{id}` fires the body's ranged action but tells the option \
                 layer it reaches nowhere, so it would never be offered",
            );
            assert_eq!(
                frames.hazard.expect("checked above").reach(),
                ambition_entity_catalog::RANGED_ACTION_REACH,
            );
        }
    }

    /// Control: a move that reaches through something it spawns itself answers
    /// that thing's flight, not the placeholder. Without this, a catalog that
    /// answered every hitless move 1000px would pass the test above.
    #[test]
    fn her_bomb_answers_its_own_flight_rather_than_the_ranged_placeholder() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let bomb = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_lay_bomb")
            .expect("she lays a bomb");
        let hazard = bomb
            .frame_data()
            .hazard
            .expect("her bomb puts a hazard in the world");
        let reach = hazard.reach();
        // Not live when it lands: laying the bomb is not a hit, and the fuse is
        // four seconds. Pricing the drop as an immediate blast would price a trap
        // as a strike.
        assert_eq!(
            hazard.detonates_by_s(),
            4.0,
            "her bomb's fuse is not the four seconds it authors: {}s",
            hazard.detonates_by_s()
        );
        // Its travel is zero: it is placed, so there is nothing to aim. See
        // `ThreatTravel::live_at_s`.
        assert_eq!(hazard.travel_to(reach - 1.0), Some(0.0));
        assert_eq!(
            hazard.travel_to(reach + 1.0),
            None,
            "her bomb answered a distance outside its own blast"
        );
        assert!(
            reach > 0.0 && reach < ambition_entity_catalog::RANGED_ACTION_REACH,
            "her bomb reaches {reach}px, which is either nothing or the \
             ranged placeholder — neither is its own arc",
        );
    }

    /// Her grab's comment says 86 + 64 is the tether's reach. Check it: retuning
    /// either move alone fails this.
    #[test]
    fn the_tether_reaches_as_far_as_her_grab() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let grab = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_grab")
            .expect("she has a grab");
        // A grab's reach is not on its timeline. `author_standing_grab` hangs the
        // capture on the active window as a `sustain_effect`, so a scan of `events`
        // finds nothing.
        let capture: ambition_entity_catalog::smash_capture::CaptureAttemptParams = grab
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| effect.key == ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT)
            .and_then(|effect| effect.params.hydrate().ok())
            .expect("her grab captures");

        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_recoil_lift")
            .expect("she has an up-B");
        let tether: ambition_entity_catalog::smash_tether::TetherPullParams = lift
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_tether::TETHER_PULL =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("her up-B throws the tether");

        assert_eq!(
            tether.reach,
            capture.reach_x(),
            "her up-B throws a {}px line while her grab reaches {}px — one \
             fiction, two numbers",
            tether.reach,
            capture.reach_x(),
        );
        assert_eq!(tether.reach, THE_TETHERS_REACH);
    }
    use super::*;

    #[test]
    fn the_reference_projectile_fighter_answers_the_complete_typed_repertoire() {
        let moves = crate::authored_movesets::shipped("projectile_polygon");
        for id in [
            "polygon_projectile_jab",
            "polygon_projectile_tilt_forward",
            "polygon_projectile_tilt_up",
            "polygon_projectile_tilt_down",
            "polygon_projectile_smash_forward",
            "polygon_projectile_smash_up",
            "polygon_projectile_smash_down",
            "polygon_projectile_air_neutral",
            "polygon_projectile_air_forward",
            "polygon_projectile_air_back",
            "polygon_projectile_air_up",
            "polygon_projectile_air_down",
            // The charge shot is the neutral special.
            "polygon_projectile_charge_shot",
            "polygon_ponytail_boomerang",
            "polygon_projectile_recoil_lift",
            "polygon_lay_bomb",
            "polygon_projectile_downward_vector",
            "polygon_projectile_grab",
            "polygon_projectile_pummel",
            "polygon_projectile_throw_forward",
            "polygon_projectile_throw_back",
            "polygon_projectile_throw_up",
            "polygon_projectile_throw_down",
            "projectile_polygon_taunt",
            "projectile_polygon_dash_attack",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }

    /// The mine is an addition and the swing is unchanged. Both in one test: a
    /// check for the mine alone would pass a down smash that lost its hitbox.
    #[test]
    fn her_down_smash_still_swings_and_now_also_plants_a_mine() {
        let moves = crate::authored_movesets::shipped("projectile_polygon");
        let down_smash = moves
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_smash_down")
            .expect("she has a down smash");

        // The swing, unchanged.
        assert_eq!(down_smash.smash_charge_mult, 1.75, "still a charged smash");
        assert!(
            down_smash
                .windows
                .iter()
                .flat_map(|window| window.volumes.iter())
                .any(|volume| volume.damage > 0),
            "the down smash still has a hit volume that hurts"
        );

        // The mine, added.
        let params = down_smash
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_mine::PLACE_MINE =>
                {
                    Some(
                        effect
                            .params
                            .hydrate::<ambition_entity_catalog::smash_mine::PlaceMineParams>()
                            .expect("place-mine params hydrate"),
                    )
                }
                _ => None,
            })
            .expect("her down smash plants a mine");

        // Plant-and-detonate must never be one continuous input: the mine is still
        // arming when the planting move ends.
        assert!(
            params.arm_s > down_smash.duration_s,
            "the mine arms in {}s but the move lasts {}s, so she could plant and \
             detonate without ever letting go",
            params.arm_s,
            down_smash.duration_s,
        );

        // The object must be pickable, so its held item must be registered. Its art
        // is checked in `items::held_visuals` (`register` is
        // `pub(in crate::items)`).
        assert!(
            ambition_characters::brain::held_item_by_id(&params.item_id).is_some(),
            "`{}` is not a registered held item, so nobody could pick the mine up",
            params.item_id,
        );
    }
}

#[cfg(test)]
mod threat_timing_tests {
    use super::*;

    /// A projectile goes live when it is thrown, not when the move ends.
    ///
    /// `startup_s` is "time until the first Active window" and falls back to the
    /// whole move duration when there is none, as for every ranged move. Leading
    /// a target by it overshoots: `charge_shot` fires at 0.26s but reports
    /// `startup_s` 0.58s, which at 200px/s closing speed is 64px of error against
    /// an `ADMISSION_SLACK_PX` of 24.
    ///
    /// Control: [`a_strike_threatens_when_its_hitbox_opens`], where the two
    /// numbers are equal. Without it, a field that is always early would pass.
    #[test]
    fn a_projectile_threatens_when_it_is_thrown_not_when_the_move_ends() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        // Read the authored trigger times from the constants the moves are built
        // from, not from the events the derivation reads, so the test does not
        // restate the implementation.
        for (id, thrown_at) in [
            ("polygon_projectile_charge_shot", CHARGE_FIRE_AT_S),
            ("polygon_ponytail_boomerang", PONYTAIL_THROWN_AT_S),
            ("polygon_lay_bomb", BOMB_LAID_AT_S),
        ] {
            let spec = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} is on the reference projectile fighter"));
            let f = spec.frame_data();
            let live = f.threat_live_at_s.unwrap_or_else(|| {
                panic!(
                    "{id} threatens nobody at any time, so it is off the attack \
                     menu entirely — see `hazard_reach`"
                )
            });
            assert!(
                live < f.startup_s,
                "{id} throws at {live:.2}s and reports `startup_s` {:.2}s; \
                 equal means a consumer leading by the threat time is leading \
                 by the whole move duration after all",
                f.startup_s
            );
            // And it is the authored throw time, not just something smaller.
            assert!(
                (live - thrown_at).abs() < 1e-4,
                "{id} is authored to throw at {thrown_at:.3}s and reports its \
                 threat live at {live:.3}s"
            );
        }
    }

    /// Control: for a move whose threat is its hitbox, the two numbers are equal
    /// by construction.
    #[test]
    fn a_strike_threatens_when_its_hitbox_opens() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let jab = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_jab")
            .expect("she has a jab");
        let f = jab.frame_data();
        assert_eq!(
            f.threat_live_at_s,
            Some(f.startup_s),
            "an ordinary swing's threat is its first Active window, so the two \
             must agree — if they can differ here, the subject arm's \
             `live < startup_s` is a statement about the FIELD rather than \
             about ranged moves"
        );
    }

    /// A move that offers the opponent nothing reports `None`: the same
    /// population the attack menu's third arm refuses.
    #[test]
    fn a_move_that_threatens_nobody_names_no_time() {
        let set = crate::authored_movesets::shipped("projectile_polygon");
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_recoil_lift")
            .expect("she has a recoil lift");
        let f = lift.frame_data();
        if f.coverage.is_none() && f.push_coverage.is_none() && f.hazard.is_none() {
            assert_eq!(
                f.threat_live_at_s, None,
                "a move with no coverage, no shove and no hazard named a time \
                 at which it threatens somebody"
            );
        } else {
            assert!(
                f.threat_live_at_s.is_some(),
                "a move that covers, shoves or spawns a hazard must say WHEN"
            );
        }
    }
}
