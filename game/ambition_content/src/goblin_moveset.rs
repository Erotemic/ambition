//! Tests of the moves the game ships for `goblin`.
//!
//! The table is content: `assets/data/movesets/goblin.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::MovesetContract;



#[cfg(test)]
mod dirt_tests {

    /// The kick wins where both reach, and that order is the move.
    ///
    /// The strike seam takes the first authored volume that reaches, so the
    /// damaging volume is first: standing in both means you were kicked. A wake
    /// ranked first would shove the people the move was about to hit. This
    /// asserts the order and each side's damage.
    #[test]
    fn the_dirt_kick_hits_what_it_reaches_and_shoves_what_it_misses() {
        use ambition_entity_catalog::{VolumeReaction, WindowTag};
        let kick = crate::authored_movesets::shipped("goblin")
            .move_by_id("dirt_kick")
            .expect("dirt_kick exists")
            .clone();
        let window = kick
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("it still strikes");
        assert_eq!(window.volumes.len(), 2, "a kick and its dust");
        assert!(window.volumes[0].damage > 0, "the KICK is ranked first");
        assert_eq!(window.volumes[1].damage, 0, "dirt does not wound");
        assert!(
            matches!(
                window.volumes[1].reaction,
                Some(VolumeReaction::Windbox(_))
            ),
            "the dust must be a push, not a weak second hitbox"
        );
        assert!(
            window.volumes[1].shape.leading_edge_x()
                > window.volumes[0].shape.leading_edge_x(),
            "the dust travels BEYOND the boot"
        );
    }

    /// The brain is told where the boot is, not where the dust is.
    ///
    /// `MoveFrameData::reach`/`coverage` are all a fighter brain knows about where
    /// a move lands. As the union of every Active volume, they would report this
    /// move reaching to the dust, and a goblin would press `dirt_kick` where only
    /// the shove lands. Here the boot ends at 48 and the dust at 82.
    #[test]
    fn the_brains_reach_for_the_dirt_kick_is_the_boot_and_not_the_dust() {
        let frames = crate::authored_movesets::shipped("goblin")
            .move_by_id("dirt_kick")
            .expect("dirt_kick exists")
            .frame_data();
        // No `expect` above the claim, so a missing region fails at the assertion
        // that names it.
        let hit = frames.coverage.map(|c| c.max.0);
        let push = frames.push_coverage.map(|c| c.max.0);
        assert_eq!(frames.reach, 48.0, "offset 18 + half-extent 30 — the boot");
        assert_eq!(hit, Some(48.0), "the hittable region ends at the boot");
        assert_eq!(push, Some(82.0), "offset 58 + half-extent 24 — the dust");
        assert!(push > hit, "the whole point of a wake: {push:?} vs {hit:?}");
    }
}

#[cfg(test)]
mod tests {
    /// The Limit costs the whole meter, so it is usable exactly when full with no
    /// new gate.
    ///
    /// The equality is the assertion. Below the cap it could be used twice at
    /// 50% (a resource move, not a Limit); above it, never, because the meter
    /// stops at the cap. Both would fail silently.
    #[test]
    fn the_goblins_limit_dive_costs_exactly_the_matchs_full_meter() {
        use ambition_entity_catalog::smash_limit::LimitMeterFill;
        let set = crate::authored_movesets::shipped("goblin");
        let dive = set
            .moves
            .iter()
            .find(|m| m.id == "dive_stomp")
            .expect("its air down-B is in the table");
        let cap = LimitMeterFill::JONS_BASELINE.cap;
        assert_eq!(
            dive.gates.costs,
            vec![ambition_resource_spec::ResourceCost::new(
                ambition_entity_catalog::smash_limit::LIMIT,
                cap,
            )],
            "the Limit dive must cost exactly the cap of {cap}, in Limit — below \
             the cap it is a resource move you can use twice, above it is a move \
             nobody can ever afford, and the meter stops filling at the cap \
             either way",
        );

        // It must hit harder than the move it replaces.
        let hardest = dive
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .expect("the dive has a volume");
        assert!(
            hardest > 20,
            "the Limit dive's hardest volume does {hardest} — the whole meter \
             bought a poke"
        );

        // Every other special must stay free.
        let priced: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| !m.gates.costs.is_empty())
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            priced,
            vec!["dive_stomp"],
            "more than one goblin move costs meter ({priced:?}) — an uncharged \
             goblin would be missing part of its kit rather than one button"
        );
    }

    /// The goblin's flow has the opposite shape to the oni's: it waits for a
    /// success (`Connected`) to commit, where the oni branches on a failure
    /// (`Blocked`) to escape. The same `Wait`/`Emit` pair needed no new node,
    /// signal or engine change.
    ///
    /// It must wait on `Connected`, not `Overlapped`: an overlap is also true of a
    /// blocked charge.
    #[test]
    fn the_goblins_charge_grabs_on_a_connect_and_not_on_a_mere_overlap() {
        use ambition_entity_catalog::{FlowNode, FlowSignal};
        let set = crate::authored_movesets::shipped("goblin");
        let charge = set
            .moves
            .iter()
            .find(|m| m.id == "headlong_charge")
            .expect("its side-B is in the table");
        let flow = charge
            .flow
            .as_ref()
            .expect("the charge authors no flow, so it bounces off and stands there");
        assert!(
            flow.problems().is_empty(),
            "the goblin's flow does not validate: {:?}",
            flow.problems()
        );

        let waited_on = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Wait { on, .. } => Some(*on),
            _ => None,
        });
        assert_eq!(
            waited_on,
            Some(FlowSignal::Connected),
            "the tackle waits on {waited_on:?} — an `Overlapped` charge includes \
             one a guard ate, so the goblin would be rewarded for running into a \
             shield"
        );

        // The follow-up is its own grab, not a new technique.
        let emitted = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Emit { effect, .. } => Some(effect.key.clone()),
            _ => None,
        });
        assert_eq!(
            emitted.as_deref(),
            Some(ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT),
            "the charge follows up with {emitted:?} rather than the capture the \
             goblin already authors"
        );

        // The whiff stays punishable: the wait gives up inside the move.
        let timeout = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Wait { timeout_s, .. } => Some(*timeout_s),
            _ => None,
        });
        let active_ends = 0.14 + 0.10;
        assert!(
            timeout.is_some_and(|t| t > active_ends && t < charge.duration_s),
            "the wait times out at {timeout:?}, outside ({active_ends}, {}) — a \
             charge that hit nothing must not still be waiting to grab",
            charge.duration_s
        );
    }

    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The goblin is not the robot with different numbers.
    ///
    /// A copied table would pass every other test here. This pins the module
    /// doc's identity: shorter reach, faster jab, weaker kill.
    #[test]
    fn the_goblin_is_shorter_faster_and_weaker_than_the_robot() {
        let goblin = crate::authored_movesets::shipped("goblin");
        let robot = crate::authored_movesets::shipped("player_robot_v3");
        let find = |set: &MovesetContract, id: &str| {
            set.moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .clone()
        };

        let (g_jab, r_jab) = (find(&goblin, "jab"), find(&robot, "jab"));
        let startup = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
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
        assert!(
            startup(&g_jab) < startup(&r_jab),
            "the goblin's jab comes out faster"
        );

        let reach = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
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
        assert!(
            reach(&g_jab) < reach(&r_jab),
            "and it reaches less far, which is what makes it have to get close"
        );

        let damage = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        assert!(
            damage(&find(&goblin, "smash_forward")) < damage(&find(&robot, "smash_forward")),
            "and its kill move hits softer — a small fighter trades reach and \
             power for speed, or it is just the robot in a different sheet"
        );
    }

    /// A carry, not a throw, and not both. Leaving `author_throw` beside
    /// `author_carry` would make down both hoist and launch, which plays as a
    /// carry that randomly fails.
    #[test]
    fn the_goblins_down_throw_hauls_instead_of_launching() {
        let moves = crate::authored_movesets::shipped("goblin");
        let beat = moves
            .moves
            .iter()
            .find(|m| m.id == "goblin_dthrow")
            .expect("the goblin has a down-throw slot");
        let keys: Vec<&str> = beat
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect) => {
                    Some(effect.key.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(
            keys.contains(&ambition_entity_catalog::smash_capture::CAPTURE_CARRY),
            "the goblin's down press does not take the weight: {keys:?}"
        );
        assert!(
            !keys.contains(&ambition_entity_catalog::smash_capture::CAPTURE_THROW),
            "the goblin's down press both hauls AND throws: {keys:?}"
        );
    }
}
