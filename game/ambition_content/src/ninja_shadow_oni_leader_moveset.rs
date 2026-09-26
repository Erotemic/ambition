//! Tests of the moves the game ships for `npc_ninja_shadow_oni_leader`.
//!
//! The table is content: `assets/data/movesets/ninja_shadow_oni_leader.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::MovesetContract;



#[cfg(test)]
mod answer_tests {

    /// The answer confirms on a hit and not on a block. A block-cancel would let
    /// him skip the recovery, and `iaijutsu` already escapes when blocked. A check
    /// that only found a `Cancelable` window would pass either version.
    #[test]
    fn the_shadow_answer_confirms_on_a_hit_and_not_on_a_block() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let answer = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader")
            .move_by_id("shadow_answer")
            .expect("shadow_answer exists")
            .clone();
        let active = answer
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("it still cuts");
        let cancel = answer
            .windows
            .iter()
            .find_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition } => Some((w, into, condition)),
                _ => None,
            })
            .expect("the answer has a second half");
        assert_eq!(
            cancel.2,
            &CancelCondition::OnHit,
            "a block-cancel stacks two escapes and makes the fastest button free"
        );
        // Ask the runtime's question: what the press offers after
        // `base_verb_of`, not the authored list compared with itself.
        let base = ambition_entity_catalog::base_verb_of("special_forward");
        let offered = ambition_entity_catalog::cancel_names_for(base, false);
        assert!(
            cancel.1.iter().any(|verb| offered.contains(&verb.as_str())),
            "it confirms into the DRAW: the window names {:?} and a \
             `special_forward` press offers {offered:?}",
            cancel.1
        );
        assert!(
            cancel.0.start_s >= active.end_s,
            "the window opens once the verdict is in ({} vs {}), not as a buffer \
             held from the press",
            cancel.0.start_s,
            active.end_s,
        );
    }
}

#[cfg(test)]
mod tests {
    /// The oni's flow validates, and this checks the shape, not the node count.
    ///
    /// `TechniqueFlow::problems()` exists because each failure is silent at
    /// runtime (a transition past the end, no reachable `Finish`, a `Wait` that
    /// never times out): a move that does nothing, or a fighter stuck in a
    /// special.
    #[test]
    fn the_onis_iaijutsu_authors_a_flow_that_validates_and_escapes_only_on_block() {
        use ambition_entity_catalog::{FlowNode, FlowSignal};
        let set = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let side_b = set
            .moves
            .iter()
            .find(|m| m.id == "iaijutsu")
            .expect("his side-B is in the table");
        let flow = side_b
            .flow
            .as_ref()
            .expect("the iaijutsu authors no flow, so a shielded dash is a free punish again");
        assert!(
            flow.problems().is_empty(),
            "the oni's flow does not validate: {:?}",
            flow.problems()
        );

        // Branch on `Blocked`, not `Overlapped`. The wait uses `Overlapped` because
        // it is also true of a blocked strike; a branch on it would escape on every
        // connect, making the move safe on everything, not just on shield.
        let branch_signal = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Branch { on, .. } => Some(*on),
            _ => None,
        });
        assert_eq!(
            branch_signal,
            Some(FlowSignal::Blocked),
            "the escape branches on {branch_signal:?} — only a BLOCK may buy it, \
             or the dash becomes safe on hit as well"
        );

        // The wait must time out before the move ends: a whiffed dash must be
        // punishable.
        let timeout = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Wait { timeout_s, .. } => Some(*timeout_s),
            _ => None,
        });
        let active_ends = 0.05 + 0.05;
        assert!(
            timeout.is_some_and(|t| t > active_ends && t < side_b.duration_s),
            "the wait times out at {timeout:?}, which is not between the end of \
             the active window ({active_ends}) and the end of the move \
             ({}) — a whiff must not reach the escape",
            side_b.duration_s
        );
    }

    use super::*;
    use ambition_entity_catalog::{MoveSpec, MoveWindow, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// Only a strike has one. Pummels and throws have no Active window, so the
    /// swing tests iterate `strikes()`, not every move.
    fn active(m: &MoveSpec) -> &MoveWindow {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
    }

    /// Every move that actually swings.
    fn strikes(set: &MovesetContract) -> Vec<&MoveSpec> {
        set.moves
            .iter()
            .filter(|m| m.windows.iter().any(|w| matches!(w.tag, WindowTag::Active)))
            .collect()
    }

    fn startup(m: &MoveSpec) -> f32 {
        active(m).start_s
    }

    fn active_len(m: &MoveSpec) -> f32 {
        let w = active(m);
        w.end_s - w.start_s
    }

    fn recovery(m: &MoveSpec) -> f32 {
        m.duration_s - active(m).end_s
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// He starts faster than the fastest existing body and finishes answering
    /// sooner.
    ///
    /// Compared against the goblin, the fast one; comparing with the admiral or
    /// the clerk would only show he is not a heavyweight.
    #[test]
    fn he_answers_faster_and_for_less_time_than_the_goblin() {
        let oni = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let goblin = crate::authored_movesets::shipped("goblin");

        assert!(
            startup(&find(&oni, "jab")) < startup(&find(&goblin, "jab")),
            "the shadow answers first"
        );

        let longest = |set: &MovesetContract| {
            strikes(set)
                .into_iter()
                .map(|m| active_len(m))
                .fold(0.0f32, f32::max)
        };
        assert!(
            longest(&oni) < longest(&goblin),
            "and his widest window is still narrower than the goblin's ({} vs {}) \
             — one breath, and you are either in it or you are not",
            longest(&oni),
            longest(&goblin)
        );
    }

    /// Every move recovers for more than three times its active window.
    ///
    /// This is his axis, and what separates him from a goblin with smaller
    /// numbers. The goblin must fail this, or the ratio is a property of
    /// `strike`'s shape, not of him.
    #[test]
    fn every_swing_costs_more_than_three_times_the_moment_it_buys() {
        let oni = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        // Swings only: a pummel or throw holds no window. The count is the zero
        // floor: a filter that removed everything would pass trivially.
        let swings = strikes(&oni);
        assert!(
            swings.len() >= 16,
            "only {} of his moves swing at all — this is being asserted over a \
             population that shrank",
            swings.len()
        );
        for m in swings {
            assert!(
                recovery(m) > active_len(m) * 3.0,
                "`{}` recovers {}s for an active window of {}s — under 3x, which \
                 is a swing he could throw casually",
                m.id,
                recovery(m),
                active_len(m)
            );
        }

        let goblin = crate::authored_movesets::shipped("goblin");
        assert!(
            goblin
                .moves
                .iter()
                .any(|m| recovery(m) <= active_len(m) * 3.0),
            "the goblin is supposed to have cheap swings; if every table passes \
             this, the ratio describes `strike` rather than the oni leader"
        );
    }

    /// He is not simply better: his kill move commits longer than the admiral's,
    /// and the admiral is the slow one.
    #[test]
    fn his_kill_move_commits_longer_than_the_admirals() {
        let oni = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let admiral = crate::authored_movesets::shipped("npc_pirate_admiral");
        let (o, a) = (find(&oni, "smash_forward"), find(&admiral, "smash_forward"));
        assert!(
            startup(&o) < startup(&a),
            "he starts his finisher first ({} vs {})",
            startup(&o),
            startup(&a)
        );
        assert!(
            recovery(&o) > recovery(&a),
            "and stands in it longer afterwards ({} vs {}) — the reply is instant \
             and the price is paid at the other end",
            recovery(&o),
            recovery(&a)
        );
    }

    /// The seal is a counter, and it still wears the cues (`counter_ring`,
    /// `faction.ninja.parry_flash`) that always said so.
    #[test]
    fn the_command_seal_parries_and_keeps_the_cues_that_always_said_so() {
        let set = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let seal = find(&set, "command_seal");

        let params: ambition_entity_catalog::smash_counter::CounterParams = seal
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| {
                effect.key == ambition_entity_catalog::smash_counter::COUNTER
            })
            .expect("the seal holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        // It no longer strikes: a counter that also swung would put its own strike
        // among the things its parry catches.
        assert!(
            !seal
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage > 0),
            "the seal still carries a damaging volume, so it is a strike wearing \
             a parry ring"
        );

        // The answer is smoke, not a grab (George's) or a teleport (the
        // Director's): the response is an arbitrary technique.
        assert_eq!(
            params.response,
            ambition_entity_catalog::smash_sleep::SLEEP,
            "the seal must answer with the sleep pulse"
        );
        let sleep: ambition_entity_catalog::smash_sleep::SleepParams =
            params.response_params.hydrate().expect("sleep params hydrate");

        // Shorter than the Performer's: she earns 1.4s by standing rooted next to
        // someone, while this comes from a parry, which is already a full punish.
        let monologue = crate::authored_movesets::shipped("performer");
        let hers: ambition_entity_catalog::smash_sleep::SleepParams = monologue
            .moves
            .iter()
            .find(|m| m.id == "performer_monologue")
            .expect("her neutral special")
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_sleep::SLEEP =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("she sings");
        assert!(
            sleep.duration_s < hers.duration_s,
            "the ninja's guaranteed sleep ({}s) outlasts the Performer's earned \
             one ({}s)",
            sleep.duration_s,
            hers.duration_s,
        );

        // The cues survived: without them the counter would no longer look like
        // one.
        let cues: Vec<&str> = seal
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Vfx { effect, .. } => {
                    Some(effect.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(
            cues.contains(&"counter_ring"),
            "the parry ring is gone: {cues:?}"
        );
    }

    /// The dive must not wear the parry's cues. `falling_seal` is a fast-fall
    /// spike with no defensive frame; parry cues on it would punish a player for
    /// reading them.
    #[test]
    fn his_falling_seal_does_not_wear_the_counters_cues() {
        let set = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let dive = find(&set, "falling_seal");
        let cues: Vec<String> = dive
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Vfx { effect, .. } => {
                    Some(effect.clone())
                }
                ambition_entity_catalog::MoveEventKind::Sfx { cue } => {
                    Some(cue.clone())
                }
                _ => None,
            })
            .collect();
        for lie in ["counter_ring", "faction.ninja.parry_flash"] {
            assert!(
                !cues.iter().any(|c| c == lie),
                "the dive still announces `{lie}`, which only the counter does: {cues:?}"
            );
        }
        // It is still a dive: a cue swap, not a nerf.
        assert!(
            dive.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage > 0),
            "the dive lost its hitbox along with its borrowed cues"
        );
    }
}
