//! Tests of the moves the game ships for `pointed_polygon`.
//!
//! The table is content: `assets/data/movesets/pointed_polygon.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.





use ambition_entity_catalog::AutolinkVolume;


#[cfg(test)]
mod tests {

    /// The thrust's tip is authored first. The strike seam takes the first
    /// authored volume that reaches (`StrikeRank { window, volume }` order), so a
    /// tip appended after the base would lose every exchange where both reach.
    #[test]
    fn his_thrusts_tip_outranks_its_base_and_is_worth_spacing_for() {
        let set = crate::authored_movesets::shipped("pointed_polygon");
        let thrust = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_point")
            .expect("his neutral-B");
        let window = thrust
            .windows
            .iter()
            .find(|w| {
                w.tag == ambition_entity_catalog::WindowTag::Active
                    && !w.volumes.is_empty()
            })
            .expect("the thrust has an active window");
        assert_eq!(
            window.volumes.len(),
            2,
            "the thrust authors {} volume(s); a tipper is two — a tip and the \
             base it outranks",
            window.volumes.len(),
        );
        let tip = &window.volumes[0];
        let base = &window.volumes[1];
        assert!(
            tip.shape.leading_edge_x() > base.shape.leading_edge_x(),
            "the FIRST-authored volume reaches {}px and the second {}px — the \
             tip is the far one, so authoring them the other way round makes the \
             sourspot win every exchange where both reach",
            tip.shape.leading_edge_x(),
            base.shape.leading_edge_x(),
        );
        assert!(
            tip.damage > base.damage && tip.knockback > base.knockback,
            "the tip ({} dmg / {} kb) is not stronger than the base ({} / {}), \
             so the spacing it asks the player to learn buys them nothing",
            tip.damage,
            tip.knockback,
            base.damage,
            base.knockback,
        );
    }

    /// Jon: *"Swordies will get a counter."* This test names her so it stays
    /// true for this fighter.
    ///
    /// It checks the answer, not just the stance: a retuned harmless response
    /// would still pass a check for `smash.counter` alone.
    #[test]
    fn his_down_b_is_a_counter_that_answers_with_the_blade() {
        use ambition_entity_catalog::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};

        let set = crate::authored_movesets::shipped("pointed_polygon");
        let stance = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_riposte")
            .expect("his grounded down-B is the riposte");
        let counter: ambition_entity_catalog::smash_counter::CounterParams = stance
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| effect.key == ambition_entity_catalog::smash_counter::COUNTER)
            .and_then(|effect| effect.params.hydrate().ok())
            .expect("the stance carries a counter");

        assert_eq!(
            counter.response, RIPOSTE_STRIKE,
            "his counter answers with `{}` rather than the blade",
            counter.response,
        );
        let cut: RiposteStrikeParams = counter
            .response_params
            .hydrate()
            .expect("the cut's params hydrate");

        // The authoring check at test time. The ruleset refuses an unusable cut at
        // runtime and only logs it.
        assert!(
            cut.problems().is_empty(),
            "his riposte authors an unusable cut: {}",
            cut.problems().join("; "),
        );
        // The replaced swipe covered 18 + 34 = 52px. The counter must reach at
        // least that far: reach is this table's distinction.
        assert!(
            cut.reach + cut.half_extents.0 >= 52.0,
            "his counter reaches {}px, less than the low arc it replaced",
            cut.reach + cut.half_extents.0,
        );
    }
    use super::*;

    /// The authored fighter reaches the recovery budget, through the real
    /// moveset function, repertoire and lowering.
    ///
    /// Generic unit tests on `afford_recovery`, `start_move` and
    /// `body_is_helpless` cannot show that authored content reaches them; a
    /// fixture would miss a lowering that drops the field or a rule that is
    /// opt-in. So this uses `crate::authored_movesets::shipped("pointed_polygon")`.
    #[test]
    fn the_authored_up_b_costs_the_recovery_and_ends_in_freefall() {
        let set = crate::authored_movesets::shipped("pointed_polygon");
        let id = set
            .verbs
            .get("special_up")
            .expect("the pointed polygon bound no up-B verb");
        let up_b = set
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the up-B verb names a move the contract does not carry");
        assert_eq!(
            up_b.gates.recovery,
            ambition_entity_catalog::RecoveryUse::SpendAndFreefall,
            "the pointed polygon's rising spin costs nothing, so she can press \
             it forever and can only be killed by a launch that outruns her"
        );
    }

    /// The Up-B's holding pulses, as authored: `(offset, half_extents, autolink)`.
    fn rising_spin_pulses() -> Vec<((f32, f32), (f32, f32), AutolinkVolume)> {
        let spin = crate::authored_movesets::shipped("pointed_polygon")
            .moves
            .into_iter()
            .find(|m| m.id == "polygon_rising_edge")
            .expect("the up-special is authored");
        spin.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| {
                let link = v.autolink()?;
                match v.shape {
                    ambition_entity_catalog::VolumeShape::Rect {
                        offset,
                        half_extents,
                    } => Some((offset, half_extents, link)),
                    _ => None,
                }
            })
            .collect()
    }

    /// Is a victim standing at `local` (in Pointed's own frame) inside the
    /// holding pulse?
    fn caught_at(local: (f32, f32)) -> bool {
        rising_spin_pulses().iter().any(|(offset, half, _)| {
            (local.0 - offset.0).abs() <= half.0 && (local.1 - offset.1).abs() <= half.1
        })
    }

    /// The up-B is a disk, not a poke (D206).
    ///
    /// Jon: *"Pointed extends her swords approximately horizontally. The attack
    /// volume should form a broad disk / horizontal spinning envelope around her,
    /// rather than reading like a narrow ordinary strike."*
    ///
    /// The claim is "wider than tall", not a pair of numbers: an assertion on the
    /// literal extents would only restate the authoring.
    #[test]
    fn the_rising_spin_is_wider_than_it_is_tall() {
        let pulses = rising_spin_pulses();
        assert!(!pulses.is_empty(), "the up-special authored no held pulses");
        for (offset, half, _) in &pulses {
            assert!(
                half.0 > half.1,
                "a holding pulse is {}x{} — taller than it is wide, which reads \
                 as a strike rather than a spin",
                half.0 * 2.0,
                half.1 * 2.0,
            );
            assert_eq!(
                offset.0, 0.0,
                "the disk is offset sideways by {}, so it is in FRONT of her \
                 rather than around her — a spin has no front",
                offset.0,
            );
        }
    }

    /// It catches both sides.
    ///
    /// Jon: *"victim near Pointed's left side → multihit catches/carries; victim
    /// near Pointed's right side → multihit catches/carries; victim somewhat
    /// above/below center → broad disk still reads sensibly."*
    ///
    /// Far points are asserted too: a victim well past the swords is out, so a
    /// stage-wide hitbox would fail.
    #[test]
    fn the_rising_spin_gathers_from_either_side_and_stops_somewhere() {
        // Beside her, at torso height.
        assert!(
            caught_at((-34.0, -12.0)),
            "a victim on her BACK side is outside the spin"
        );
        assert!(
            caught_at((34.0, -12.0)),
            "a victim in FRONT of her is outside the spin"
        );
        // Somewhat above and below centre.
        assert!(
            caught_at((0.0, -30.0)),
            "a victim above her centre is outside the spin"
        );
        assert!(
            caught_at((0.0, 6.0)),
            "a victim at her feet is outside the spin"
        );
        // And it ends: about two body-widths out each way is outside.
        assert!(
            !caught_at((-120.0, -12.0)),
            "the spin reaches most of the stage to her left"
        );
        assert!(
            !caught_at((120.0, -12.0)),
            "the spin reaches most of the stage to her right"
        );
    }

    /// The gather point does not depend on her facing.
    ///
    /// `autolink_anchor_world` mirrors an anchor with facing, which is right for
    /// a poke and wrong for a spin. A non-zero x would gather victims to whichever
    /// side she faces.
    ///
    /// Asked through the engine's resolver, not by reading `anchor.0`, so a change
    /// to the mirroring rule is caught.
    #[test]
    fn the_gather_point_is_the_same_whichever_way_she_faces() {
        use ambition_platformer2d_core::hit_response::autolink_anchor_world;
        use ambition_platformer2d_core::Vec2;

        const HER: Vec2 = Vec2::new(300.0, 200.0);
        const DOWN: Vec2 = Vec2::new(0.0, 1.0);

        for (_, _, link) in rising_spin_pulses() {
            let authored = Vec2::new(link.anchor.0, link.anchor.1);
            let facing_right = autolink_anchor_world(authored, HER, 1.0, DOWN);
            let facing_left = autolink_anchor_world(authored, HER, -1.0, DOWN);
            assert_eq!(
                facing_right, facing_left,
                "the spin gathers to a different point depending on her facing"
            );
            assert!(
                facing_right.distance(HER) < 24.0,
                "the gather point is {:?}, which is not ON her — a spin pulls \
                 victims into itself",
                facing_right,
            );
        }
    }

    /// The finisher covers what the pulses gathered. If the pulses are wider than
    /// the launch, a victim carried in on her back side rides the climb and
    /// falls out unlaunched.
    #[test]
    fn the_launch_reaches_everything_the_pulses_held() {
        let spin = crate::authored_movesets::shipped("pointed_polygon")
            .moves
            .into_iter()
            .find(|m| m.id == "polygon_rising_edge")
            .expect("the up-special is authored");
        // The finisher is the one volume that authors a launch.
        let (offset, half) = spin
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .find(|v| v.autolink().is_none())
            .and_then(|v| match v.shape {
                ambition_entity_catalog::VolumeShape::Rect {
                    offset,
                    half_extents,
                } => Some((offset, half_extents)),
                _ => None,
            })
            .expect("the spin ends with a launching rect");

        for (pulse_offset, pulse_half, _) in rising_spin_pulses() {
            for side in [-1.0f32, 1.0] {
                // The anchor pulls victims in, so the finisher must cover the gathered
                // cloud, not the pulse's outer edge. Half the pulse reach is "held".
                let held = pulse_offset.0 + side * pulse_half.0 * 0.5;
                assert!(
                    (held - offset.0).abs() <= half.0,
                    "a victim gathered to x={held} is outside the launch box \
                     [{}, {}] — the spin catches it and then lets it go",
                    offset.0 - half.0,
                    offset.0 + half.0,
                );
            }
        }
    }

    #[test]
    fn the_reference_sword_fighter_answers_the_complete_typed_repertoire() {
        let moves = crate::authored_movesets::shipped("pointed_polygon");
        for id in [
            "polygon_jab",
            "pointed_polygon_dash_attack",
            "pointed_polygon_taunt",
            "polygon_tilt_forward",
            "polygon_tilt_up",
            "polygon_tilt_down",
            "polygon_smash_forward",
            "polygon_smash_up",
            "polygon_smash_down",
            "polygon_air_neutral",
            "polygon_air_forward",
            "polygon_air_back",
            "polygon_air_up",
            "polygon_air_down",
            "polygon_point",
            "polygon_vector_lunge",
            "polygon_rising_edge",
            "polygon_riposte",
            "polygon_falling_edge",
            "polygon_grab",
            "polygon_pummel",
            "polygon_throw_forward",
            "polygon_throw_back",
            "polygon_throw_up",
            "polygon_throw_down",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}
