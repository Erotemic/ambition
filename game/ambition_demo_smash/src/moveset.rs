//! Tests of the stand-in duelists' table.
//!
//! The stand-ins' table is content: `assets/data/movesets/smash_duelist_a.ron`,
//! which `smash_duelist_b` borrows. The tests read it through
//! [`crate::smash_pack::shipped_moveset`], the table the demo plays.

mod tests {
    use ambition_entity_catalog::{AttackDir, MoveSpec};

    /// What a press answers, not what a verb list binds.
    ///
    /// `directional_verb_chain` falls back to the base verb, so a missing
    /// `attack_forward` still answers with the `jab`. This test enumerates
    /// every press instead of inspecting keys.
    #[test]
    fn the_only_presses_this_fighter_cannot_answer_are_specials() {
        use ambition_entity_catalog::AttackDir;
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let dirs = [
            ("neutral", AttackDir::Neutral),
            ("forward", AttackDir::Forward),
            ("up", AttackDir::Up),
            ("down", AttackDir::Down),
            ("back", AttackDir::Back),
        ];

        let mut silent: Vec<String> = Vec::new();
        for base in ["attack", "smash", "special"] {
            for (dir_name, dir) in dirs {
                for (stance, grounded) in [("ground", true), ("air", false)] {
                    if set.move_for_directional_verb(base, dir, grounded).is_none() {
                        silent.push(format!("{base}_{dir_name}_{stance}"));
                    }
                }
            }
        }

        // Every silent press is a `smash` or a `special`; the `attack` family
        // answers all ten.
        assert!(
            silent.iter().all(|p| !p.starts_with("attack_")),
            "an `attack` press went unanswered: {silent:?} — the base-verb \
             fallback is what makes a jab answer a forward tilt, and if it has \
             stopped, every planning claim about this fighter's reach is stale"
        );

        // Two of the ten special presses are silent: `special_neutral_air`
        // and `special_back_air`. `special_forward` and `special_down` answer
        // in both stances, and the neutral special answers the remaining ground
        // presses through the fallback. The aerial column is left because the
        // neutral special is `grounded_only`.
        let specials: Vec<&String> = silent.iter().filter(|p| p.starts_with("special_")).collect();
        assert_eq!(
            specials.len(),
            2,
            "the special gap changed size: {specials:?}. If a special was \
             AUTHORED this is good news and the number wants updating here and \
             in `awaiting-maintainer-decision.md`; if one was LOST, that is a \
             regression the roster question was about."
        );
        assert!(
            !silent.iter().any(|p| p.starts_with("special_forward")),
            "the command grab stopped answering a forward special: {silent:?}"
        );
    }

    /// Every `(base, direction, stance)` press, and which of them a contract
    /// answers with nothing. Shared so both fighters use one instrument.
    fn silent_presses(
        set: &ambition_entity_catalog::MovesetContract,
    ) -> Vec<String> {
        let dirs = [
            ("neutral", AttackDir::Neutral),
            ("forward", AttackDir::Forward),
            ("up", AttackDir::Up),
            ("down", AttackDir::Down),
            ("back", AttackDir::Back),
        ];
        let mut silent = Vec::new();
        for base in ["attack", "smash", "special"] {
            for (dir_name, dir) in dirs {
                for (stance, grounded) in [("ground", true), ("air", false)] {
                    if set.move_for_directional_verb(base, dir, grounded).is_none() {
                        silent.push(format!("{base}_{dir_name}_{stance}"));
                    }
                }
            }
        }
        silent
    }

    /// The two fighters' silent presses are a subset and a difference.
    ///
    /// Siblings pin each fighter's count (the stand-in above, **15**, and
    /// `the_presses_george_leaves_unanswered_are_the_ones_the_genre_lacks`,
    /// **7**). Two counts do not say one set is inside the other, so this
    /// asserts the relation: George's silent set is a strict subset of the
    /// stand-in's, and the rest is exactly eight `special` presses. The
    /// stand-in is George's genre shape without the special button.
    ///
    /// A ratchet. Authoring a stand-in special fails the second assertion (lower
    /// the number in the same commit). Losing a George special breaks the subset,
    /// which is a regression.
    #[test]
    fn the_stand_in_is_george_s_genre_shape_with_the_special_button_removed() {
        let stand_in = silent_presses(&crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID));
        let george = silent_presses(&crate::smash_pack::shipped_moveset(crate::SMASH_GEORGE_BOOUL));

        let escaped: Vec<&String> = george.iter().filter(|p| !stand_in.contains(p)).collect();
        assert!(
            escaped.is_empty(),
            "a press George cannot answer is one the STAND-IN can: {escaped:?}. That breaks the subset, so the stand-in is no longer George minus the specials and the roster question needs re-deriving rather than re-counting."
        );

        let mut extra: Vec<&String> = stand_in.iter().filter(|p| !george.contains(p)).collect();
        extra.sort();
        assert!(
            extra.iter().all(|p| p.starts_with("special_")),
            "the stand-in's surplus silence is no longer all specials: {extra:?}"
        );
        assert_eq!(
            extra.len(),
            2,
            "the stand-in/George special gap moved to {}: {extra:?}. If a special was AUTHORED on the stand-in this is the good failure — lower the number here in the same commit. If George LOST one, the subset assertion above would have fired first.",
            extra.len()
        );
    }

    /// No grab this demo authors reaches further than 96px.
    ///
    /// Nothing else bounds an authored grab's reach (not the params schema,
    /// `acquire_captures`, or the content pass), so a typo in `half_extents`
    /// could catch across the stage. A ceiling, not an engine clamp: a clamp
    /// would silently truncate a deliberate long reach.
    ///
    /// It covers only this crate's two movesets. `ambition_content`'s fighters
    /// (including tethers) are checked by the guard of the same name in
    /// `ambition_content::authored_movesets`, which holds the tether allowlist.
    ///
    /// The platform is 480px wide, so 96 is a fifth of it.
    #[test]
    fn no_grab_this_demo_authors_reaches_further_than_the_stage_allows() {
        use ambition_entity_catalog::smash_capture::{
            CaptureAttemptParams, CAPTURE_ATTEMPT,
        };

        /// A fifth of the shipped platform's width.
        const MAX_REACH_PX: f32 = 96.0;

        let mut seen = 0usize;
        for (who, set) in [
            ("the stand-in fighter", crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID)),
            ("George", crate::smash_pack::shipped_moveset(crate::SMASH_GEORGE_BOOUL)),
        ] {
            for spec in &set.moves {
                for window in &spec.windows {
                    let Some(effect) = window.sustain_effect.as_ref() else {
                        continue;
                    };
                    if effect.key != CAPTURE_ATTEMPT {
                        continue;
                    }
                    let params: CaptureAttemptParams = effect
                        .params
                        .hydrate()
                        .expect("an authored capture attempt must hydrate");
                    seen += 1;
                    // The far edge of the reach box, along the captor's facing.
                    let reach = params.offset.0.abs() + params.half_extents.0.abs();
                    assert!(
                        reach <= MAX_REACH_PX,
                        "{who}'s `{}` reaches {reach}px (offset {:?} + half {:?}), past the \
                         {MAX_REACH_PX}px ceiling. If this is a deliberate tether, raise \
                         MAX_REACH_PX here in the same commit; if it is a typo, this is the \
                         only thing that would have caught it",
                        spec.id, params.offset, params.half_extents
                    );
                    assert!(
                        params.half_extents.0 > 0.0 && params.half_extents.1 > 0.0,
                        "{who}'s `{}` has a non-positive grab box {:?}, so it can never catch \
                         anybody",
                        spec.id, params.half_extents
                    );
                }
            }
        }

        // Population floor: a census that walked nothing also finds no
        // offender.
        assert!(
            seen >= 3,
            "found only {seen} authored capture attempt(s); the demo has at least three (two \
             stand-in grabs and George's), so this census is measuring nothing rather than \
             passing"
        );
    }

    /// The side special is a real capture, not a strike with the name.
    ///
    /// It asserts the special's own move captures and is a different move from
    /// the standing grab. Either claim alone passes on a wrong binding.
    #[test]
    fn the_side_special_is_a_command_grab_and_not_the_standing_grab_renamed() {
        use ambition_entity_catalog::WindowTag;
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);

        let special = set
            .move_for_verb("special_forward")
            .expect("the stand-in fighter binds a side special");
        let standing = set
            .move_for_verb("grab")
            .expect("the stand-in fighter binds a standing grab");
        assert_ne!(
            special.id, standing.id,
            "the side special resolves to the STANDING grab, so the special \
             button is an alias and the command grab does not exist"
        );

        // Live during `Active` only. Sustained through startup, it would catch
        // bodies before the lunge commits.
        let live: Vec<&WindowTag> = special
            .windows
            .iter()
            .filter(|w| w.sustain_effect.is_some())
            .map(|w| &w.tag)
            .collect();
        assert_eq!(
            live,
            vec![&WindowTag::Active],
            "the command grab's capture attempt is live on {live:?} — it must be \
             live on exactly the Active window, or it is either a move that \
             cannot catch anybody or one that catches during its own startup"
        );
        assert_eq!(
            special
                .windows
                .iter()
                .find(|w| w.tag == WindowTag::Active)
                .and_then(|w| w.sustain_effect.as_ref())
                .map(|e| e.key.as_str()),
            Some(ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT),
            "the special's live window sustains some OTHER effect, so it is not \
             a capture at all"
        );

        // It travels: a command grab that closes no distance is worse than
        // the standing grab.
        let (dx, _) = special
            .start_impulse
            .expect("a command grab that does not lunge is a slower standing grab");
        assert!(
            dx > 0.0,
            "the command grab's impulse is {dx}, so it lunges backwards or \
             stands still"
        );

        // The committed grab: with equal startup it would be a strictly better
        // standing grab.
        assert!(
            special.windows.iter().any(|w| w.tag == WindowTag::Active)
                && standing.windows.iter().any(|w| w.tag == WindowTag::Active),
            "one of the two grabs has no active window"
        );
        let first_active = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .find(|w| w.tag == WindowTag::Active)
                .map(|w| w.start_s)
                .expect("checked above")
        };
        assert!(
            first_active(special) > first_active(standing),
            "the command grab goes live at {}s and the standing grab at {}s — \
             a command grab that is not slower is a free upgrade and retires \
             the button it is supposed to complement",
            first_active(special),
            first_active(standing)
        );
    }

    /// Every verb resolves to a move that exists. A verb pointing at a
    /// missing id is a press that silently does nothing.
    #[test]
    fn every_authored_verb_resolves() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        for (verb, id) in &set.verbs {
            assert!(
                set.move_by_id(id).is_some(),
                "verb `{verb}` names move `{id}`, which is not in the contract"
            );
        }
    }

    /// A forward smash is not a renamed jab: it commits longer, hurts more,
    /// throws harder, and scales with the victim's damage.
    #[test]
    fn the_forward_smash_is_a_real_smash_and_not_the_jab_renamed() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let jab = set.move_for_verb("attack").expect("a fighter has a jab");
        let smash = set
            .move_for_verb("smash_forward")
            .expect("a fighter has a forward smash");

        let launch = |mv: &MoveSpec| {
            mv.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                // No authored growth defers to the stage (its fraction of the
                // base), the comparable number. Do not rely on `Option`
                // ordering: `None < Some(_)` would read "states nothing" as
                // "grows least".
                .map(|v| {
                    (
                        v.damage,
                        v.knockback,
                        v.knockback_growth
                            .unwrap_or(v.knockback * crate::SMASH_KNOCKBACK_GROWTH),
                    )
                })
                .next()
                .expect("a strike has a volume")
        };
        let (jab_damage, jab_kb, jab_growth) = launch(jab);
        let (smash_damage, smash_kb, smash_growth) = launch(smash);

        assert!(
            smash.duration_s > jab.duration_s * 2.0,
            "the smash commits {:.2}s against the jab's {:.2}s, which is not a \
             commitment",
            smash.duration_s,
            jab.duration_s
        );
        assert!(smash_damage >= jab_damage * 3);
        assert!(smash_kb >= jab_kb * 2.0);
        assert!(
            smash_growth > jab_growth,
            "the smash does not scale harder with percent than the jab, so a \
             stock never ends on it"
        );
        assert!(
            smash.smash_charge_mult > 1.0,
            "holding the smash pays nothing, so there is no reason to charge it"
        );
        // The payoff is reachable. This roster authors each smash's charge
        // pose (see `CHARGE_POSE_AT_S`); a smash with no policy fires at once
        // and the multiplier is unpayable. This keeps the engine fallback
        // (`CHARGE_POSE_FRACTION`) from becoming the contract.
        assert!(
            smash.smash_charge.is_some(),
            "this smash derives its charge pose from the engine fallback \
             instead of authoring one"
        );
        let policy = smash
            .charge_policy()
            .expect("the smash resolves no charge policy, so it cannot be held");
        assert!(
            policy.hold_at_s > 0.0,
            "the hold sits at the very first instant of the move, so there is \
             no windup to commit to before the charge"
        );
        // Strictly before the first strike: Active membership is
        // `start_s <= t < end_s`, so a hold on that instant would charge with
        // the hitbox out.
        let first_active = smash
            .windows
            .iter()
            .filter(|w| {
                matches!(
                    w.tag,
                    ambition_entity_catalog::WindowTag::Active
                )
            })
            .map(|w| w.start_s)
            .fold(f32::MAX, f32::min);
        assert!(
            policy.hold_at_s < first_active,
            "the charge freezes at {} and this smash goes live at {first_active}",
            policy.hold_at_s
        );
    }

    /// Every authored growth equals the stage's own declaration, in the
    /// stage's units.
    ///
    /// Guards a unit mismatch. A volume's `knockback_growth` is absolute px/s per
    /// point; the ruleset's is a fraction of base. Both are `f32` "growth", and
    /// an authored move outranks the ruleset, so a fraction-shaped number would
    /// grow about 40x slower with nothing failing. A move may differ on purpose,
    /// but by a visible factor, not a unit.
    #[test]
    fn an_authored_growth_is_the_stage_declaration_in_the_stage_units() {
        for mv in &crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID).moves {
            for volume in mv.windows.iter().flat_map(|w| w.volumes.iter()) {
                // No growth defers to the stage, and fixed knockback
                // (`Some(0.0)`) is deliberate. Only a stated non-zero growth
                // can carry the slip.
                let Some(authored) = volume.knockback_growth.filter(|g| *g > 0.0) else {
                    continue;
                };
                let expected = volume.knockback * crate::SMASH_KNOCKBACK_GROWTH;
                assert!(
                    (authored - expected).abs() < 0.01,
                    "`{}` launches at {} and grows {}/point, but the stage \
                     declares {} of base = {expected}/point. A growth that is \
                     off by a FACTOR is the fraction-vs-absolute unit slip, and \
                     it silently opts this move out of the percent loop",
                    mv.id,
                    volume.knockback,
                    authored,
                    crate::SMASH_KNOCKBACK_GROWTH,
                );
            }
        }
    }

    /// The aerials commit, and the auto-cancel window is real.
    ///
    /// `autocancel_after_s` is ignored unless `landing_lag_s` is authored, so
    /// an aerial with a window and no lag is inert.
    #[test]
    fn every_aerial_authors_both_halves_of_the_landing_rule() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let mut checked = 0;
        for verb in [
            "attack_air",
            "attack_air_forward",
            "attack_air_back",
            "attack_air_up",
            "attack_air_down",
        ] {
            let mv = set.move_for_verb(verb).expect("authored above");
            checked += 1;
            let lag = mv.landing_lag_s.unwrap_or(0.0);
            let cancel = mv
                .autocancel_after_s
                .expect("an aerial with lag and no cancel window can only be paid");
            assert!(lag > 0.0, "{verb} lands free, so it is not a commitment");
            assert!(
                cancel < mv.duration_s,
                "{verb}'s auto-cancel opens at {cancel:.2}s of a {:.2}s move, so it \
                 never opens at all",
                mv.duration_s
            );
        }
        assert_eq!(checked, 5, "the loop did not reach every aerial");
    }

    /// A grounded press cannot reach an aerial, and the reverse. The gates make
    /// one button eleven moves.
    #[test]
    fn the_directional_chain_lands_on_the_right_move_for_the_posture() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|mv| mv.id.as_str()),
            Some("jab"),
            "a grounded forward press should fall through to the jab: there is no \
             forward tilt, and the aerial is gated off the ground"
        );
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, false)
                .map(|mv| mv.id.as_str()),
            Some("air_forward"),
        );
        assert_eq!(
            set.move_for_directional_verb("smash", AttackDir::Forward, true)
                .map(|mv| mv.id.as_str()),
            Some("smash_forward"),
        );
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Up, true)
                .map(|mv| mv.id.as_str()),
            Some("tilt_up"),
        );
    }
}

mod hit_confirm_tests {
    use ambition_entity_catalog::{FlowNode, FlowSignal};

    /// The neutral special confirms: it waits on a connect and answers with a
    /// grab.
    ///
    /// The signal is the assertion. `Overlapped` is set by a shielded poke,
    /// so a confirm on it would grab through a guard.
    #[test]
    fn the_neutral_special_confirms_on_a_connect_and_not_on_a_shield() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let id = set
            .verbs
            .get("special")
            .expect("the contract binds a neutral special");
        let spec = set
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the neutral special names a move the contract carries");
        let flow = spec
            .flow
            .as_ref()
            .expect("the neutral special is a hit confirm, so it authors a flow");
        assert_eq!(
            flow.problems(),
            Vec::<String>::new(),
            "the shipped confirm's flow is invalid: {:?}",
            flow.problems()
        );

        let waits: Vec<FlowSignal> = flow
            .nodes
            .iter()
            .filter_map(|n| match n {
                FlowNode::Wait { on, .. } => Some(*on),
                _ => None,
            })
            .collect();
        assert_eq!(
            waits,
            vec![FlowSignal::Connected],
            "the confirm waits on {waits:?}. `Overlapped` is set by a BLOCKED \
             strike, so a confirm on it grabs through a shield"
        );

        // It must also emit something; otherwise it commits to a read and
        // gets nothing.
        let emitted: Vec<&str> = flow
            .nodes
            .iter()
            .filter_map(|n| match n {
                FlowNode::Emit { effect, .. } => Some(effect.key.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            emitted,
            vec![ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT],
            "the confirm's payoff is {emitted:?} rather than the grab"
        );
    }

    /// The confirm's wait outlasts the window it confirms.
    ///
    /// A timeout shorter than `startup + active` means the flow gives up before
    /// the strike can report a connect, so the grab never comes out. The wait
    /// starts at move start.
    #[test]
    fn the_confirms_wait_outlasts_the_window_it_confirms() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let id = set.verbs.get("special").expect("a neutral special is bound");
        let spec = set.moves.iter().find(|m| &m.id == id).expect("it names a move");
        let flow = spec.flow.as_ref().expect("the confirm authors a flow");

        let timeout = flow
            .nodes
            .iter()
            .find_map(|n| match n {
                FlowNode::Wait { timeout_s, .. } => Some(*timeout_s),
                _ => None,
            })
            .expect("the confirm waits");
        // The last moment a strike can report a connect: the end of the
        // authored Active window.
        let active_ends = spec
            .windows
            .iter()
            .filter(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
            .map(|w| w.end_s)
            .fold(0.0_f32, f32::max);
        assert!(
            active_ends > 0.0,
            "the confirm has no Active window, so there is nothing to confirm"
        );
        assert!(
            timeout > active_ends,
            "the confirm gives up at {timeout}s but its strike stays live until \
             {active_ends}s — the flow stops waiting before the hit can report, \
             so the grab never comes out however clean the confirm was"
        );
    }

    /// Every authored flow in this contract is valid.
    ///
    /// A population check, so the next flow authored here is covered. A flow
    /// with a dangling transition is silent at runtime.
    #[test]
    fn every_authored_flow_in_this_contract_is_valid() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let mut seen = 0usize;
        for spec in &set.moves {
            if let Some(flow) = spec.flow.as_ref() {
                seen += 1;
                assert_eq!(
                    flow.problems(),
                    Vec::<String>::new(),
                    "`{}` authors an invalid flow",
                    spec.id
                );
            }
        }
        assert!(
            seen >= 1,
            "no move in this contract authors a flow, so this guard is measuring \
             nothing rather than passing"
        );
    }
}

mod recovery_tests {
    use ambition_entity_catalog::smash_teleport::{TeleportParams, TELEPORT};
    use ambition_entity_catalog::MoveEventKind;

    /// These fighters can recover, and the recovery is aimed.
    ///
    /// Before it, `special_up_air` fell through to nothing, because every
    /// special was `grounded_only`. `behind_nearest_foe` must be false: it
    /// would teleport a recovering fighter next to the edgeguarder.
    #[test]
    fn the_up_special_is_an_aimed_airborne_recovery() {
        let set = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        let id = set
            .verbs
            .get("special_up")
            .expect("these fighters bind an up-special");
        let spec = set
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the up-special names a move the contract carries");

        assert_eq!(
            spec.gates.grounded,
            Some(false),
            "the recovery is not airborne-only, so it replaces the grounded \
             up-B — which already answers with the neutral special — with a \
             worse move"
        );

        let params: TeleportParams = spec
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == TELEPORT => {
                    Some(effect.params.hydrate().expect("teleport params hydrate"))
                }
                _ => None,
            })
            .expect("the recovery teleports");
        assert!(
            !params.behind_nearest_foe,
            "the recovery teleports BEHIND THE NEAREST FOE, so a fighter \
             recovering from offstage arrives next to their edgeguard"
        );
        assert!(
            params.ledge_assist > 0.0,
            "the recovery has no ledge assist ({}), which is the whole reason \
             this is a technique rather than an authored impulse — a blink that \
             drops you a pixel under the lip reads as a bug, not a miss",
            params.ledge_assist
        );
        assert!(
            params.distance > 0.0,
            "the recovery covers no distance ({})",
            params.distance
        );
    }
}
