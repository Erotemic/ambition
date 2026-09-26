//! Tests of George Booul's moves.
//!
//! George is a heavy commitment fighter with three fast pokes and otherwise
//! slow, high-damage attacks; the startup gap between those groups is part of
//! his character contract. His table is content: `smash_moveset.ron` beside his
//! fighter facet in the character-authoring submodule, which the demo's pack
//! selects. These tests read it through [`crate::smash_pack::shipped_moveset`],
//! the table the demo plays.

use ambition_entity_catalog::MovesetContract;

/// The table the demo plays for George.
fn shipped() -> MovesetContract {
    crate::smash_pack::shipped_moveset(crate::SMASH_GEORGE_BOOUL)
}

mod tests {
    use super::*;

    /// The widest startup a POKE may have, and the narrowest a COMMITMENT may
    /// have.
    ///
    /// These define the character. The gap between them is the excluded middle,
    /// and the guard asserts no move lands inside it. Retune by moving a move to
    /// one side, never into the band.
    const POKE_MAX_STARTUP_S: f32 = 0.08;
    const COMMIT_MIN_STARTUP_S: f32 = 0.15;
    use ambition_entity_catalog::{MoveSpec, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// The tell before a move becomes dangerous, or `None` for a move with no
    /// dangerous moment. Pummels and throws have no Active window: their
    /// target was selected when the capture began.
    fn startup(m: &MoveSpec) -> Option<f32> {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| w.start_s)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    // `SmashRepertoire` owns the verb strings and has no `Default` or private
    // fields, so a missing or renamed slot is a compile error here. That every
    // press is answered in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The excluded middle, as an assertion: every move is a poke or a
    /// commitment, and the band between them is empty. A move in the band would
    /// be a reasonable tilt and would make George somebody else.
    #[test]
    fn no_move_lives_between_the_pokes_and_the_commitments() {
        let george = shipped();

        // Exempt by name: six moves have no tell and reach for nobody (a
        // pummel and four throws, whose target is already selected, and the
        // taunt). Pinning the list means a strike that lost its Active window
        // fails here. The grab is not exempt: it reaches, so it has a tell.
        let mut telless: Vec<&str> = george
            .moves
            .iter()
            .filter(|m| startup(m).is_none())
            .map(|m| m.id.as_str())
            .collect();
        telless.sort_unstable();
        assert_eq!(
            telless,
            vec![
                "george_booul_taunt",
                "george_bthrow",
                "george_dthrow",
                "george_fthrow",
                "george_pummel",
                "george_uthrow",
            ],
            "the set of moves with no Active window changed"
        );

        for m in &george.moves {
            let Some(s) = startup(m) else { continue };
            assert!(
                s <= POKE_MAX_STARTUP_S || s >= COMMIT_MIN_STARTUP_S,
                "`{}` starts at {s}s, inside the band this fighter does not have \
                 ({POKE_MAX_STARTUP_S}..{COMMIT_MIN_STARTUP_S})",
                m.id
            );
        }

        // The two halves also differ by payoff, not only timing. Pinned by
        // name like the tell exemption, so a smash that lost its volumes
        // fails instead of becoming the softest commitment.
        let mut payless: Vec<&str> = george
            .moves
            .iter()
            .filter(|m| startup(m).is_some() && damage(m) == 0)
            .map(|m| m.id.as_str())
            .collect();
        payless.sort_unstable();
        assert_eq!(
            payless,
            // The running grab is derived by the capture kit from George's
            // grab, so the startup band also constrains a derived move. If the
            // grab starts near `POKE_MAX_STARTUP_S`, the derived wind-up can
            // land in the band, and the assertion above reports it.
            vec!["george_grab", "george_grab_dash"],
            "the set of moves that reach and deal no damage changed"
        );

        let (pokes, commits): (Vec<_>, Vec<_>) = george
            .moves
            .iter()
            // A move with no tell is neither poke nor commitment, and a move
            // with no damage is outside this claim.
            .filter(|m| startup(m).is_some() && damage(m) > 0)
            .partition(|m| startup(m).unwrap_or_default() <= POKE_MAX_STARTUP_S);
        let hardest_poke = pokes.iter().map(|m| damage(m)).max().expect("pokes exist");
        let softest_commit = commits
            .iter()
            .map(|m| damage(m))
            .min()
            .expect("commitments exist");
        assert!(
            hardest_poke < softest_commit,
            "the fast half must be the weak half ({hardest_poke} vs {softest_commit})"
        );

        // The poison: the shared table has a real middle (tilts at
        // 0.06–0.07, aerials at 0.09, 0.10, 0.12). If this passed for both
        // tables, the band would describe nothing.
        let shared = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        assert!(
            shared.moves.iter().any(|m| {
                startup(m).is_some_and(|s| s > POKE_MAX_STARTUP_S && s < COMMIT_MIN_STARTUP_S)
            }),
            "the shared repertoire is supposed to HAVE a middle; if it does not, \
             this whole test is asserting a property of the threshold rather \
             than a property of George"
        );
    }

    /// Comparative, as for the goblin and the admiral: a table copied and
    /// renumbered would pass every other test here.
    #[test]
    fn george_commits_longer_and_hits_harder_than_the_shared_repertoire() {
        let george = shipped();
        let shared = crate::smash_pack::shipped_moveset(crate::SMASH_CHARACTER_ID);
        for id in ["smash_forward", "smash_up", "smash_down"] {
            let (g, s) = (find(&george, id), find(&shared, id));
            // `expect`, not a filter: these are strikes, so a missing Active
            // window is a defect.
            let (gs, ss) = (
                startup(&g).expect("a smash has an active window"),
                startup(&s).expect("a smash has an active window"),
            );
            assert!(gs > ss, "`{id}`: the heavy commits longer ({gs} vs {ss})");
            assert!(
                damage(&g) > damage(&s),
                "`{id}`: and is paid for it ({} vs {})",
                damage(&g),
                damage(&s)
            );
        }

        // And nowhere is he faster; otherwise he would just be stronger.
        // The count stops the filter from emptying the loop.
        let mut compared = 0;
        for m in &george.moves {
            let Some(s) = shared.moves.iter().find(|other| other.id == m.id) else {
                continue;
            };
            compared += 1;
            // Both are shared-table strikes; `None` means one lost its Active
            // window.
            let (gs, ss) = (
                startup(m).expect("a shared-table move has an active window"),
                startup(s).expect("a shared-table move has an active window"),
            );
            assert!(
                gs >= ss,
                "`{}` is quicker than the shared table's ({gs} vs {ss})",
                m.id
            );
        }
        assert!(
            compared >= 11,
            "only {compared} moves were comparable; the two tables have stopped \
             overlapping and this test is asserting nothing"
        );
    }
    // ── the specials ─────────────────────────────────────────────────────────

    /// The ascent is a save, not a flight.
    ///
    /// This lets the Up-B exist with no cooldown, no per-airtime counter and no
    /// rollback state. With no `Cancelable` window the body cannot re-press
    /// while the move plays, and the move outlasts its arc, so one full cycle
    /// cannot gain height.
    #[test]
    fn the_ascent_is_a_save_and_not_a_flight() {
        let g = ambition_platformer2d::engine_core::DEFAULT_TUNING.gravity;
        let up_b = find(&shipped(), "excluded_middle");
        let frames = up_b.frame_data();
        let to_apex = frames.lift_speed / g;
        // The move lets go at its end, so the tail runs from the burst to there.
        let tail = up_b.duration_s - frames.lift_at_s;
        assert!(
            tail > 2.0 * to_apex,
            "the ascent climbs for {to_apex:.3}s and is handed back {tail:.3}s \
             after the burst; anything at or under {:.3}s returns George higher \
             than it found him, every press, which is flight",
            2.0 * to_apex
        );
        // The windup is real: a recovery with no tell is a free escape.
        assert!(frames.lift_at_s >= COMMIT_MIN_STARTUP_S);
        // Landing out of it costs, so it is a bad panic button on the stage.
        assert!(up_b.landing_lag_s.unwrap_or(0.0) > 0.0);
    }

    /// The rise is commanded (`Set`), not added.
    ///
    /// Under `ImpulseMode::Add` a falling George would get only what was left
    /// over. `lift_speed` is derived from `Set` impulses only, so this also
    /// asserts that the brain and the recovery probe can see the move.
    #[test]
    fn the_ascent_commands_its_rise_and_advertises_it() {
        use ambition_entity_catalog::{ImpulseMode, MoveEventKind};
        let up_b = find(&shipped(), "excluded_middle");
        let burst = up_b
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, mode } => Some((e.at_s, *local, *mode)),
                _ => None,
            })
            .expect("the recovery special displaces its owner");
        assert_eq!(burst.2, ImpulseMode::Set);
        assert!(burst.1 .1 < 0.0, "the burst must point AGAINST gravity");

        // The derived affordance the brain and recovery probe read is the
        // authored burst. At zero the CPU cannot see its recovery.
        let frames = up_b.frame_data();
        assert!(frames.lift_speed > 0.0, "the recovery advertises no lift");
        assert_eq!(frames.lift_speed, -burst.1 .1);
        assert_eq!(frames.lift_at_s, burst.0);

        // The poison: nothing else advertises a lift, or the assertion above
        // would tell a policy layer nothing.
        let table = shipped();
        let others: Vec<&str> = table
            .moves
            .iter()
            .filter(|m| m.id != "excluded_middle" && m.frame_data().lift_speed > 0.0)
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            others.is_empty(),
            "these moves also claim to be ways home: {others:?}"
        );
    }

    /// Four specials, four mechanisms: not rotated or mirrored clones of one
    /// base melee. One commands a rise, one a plunge that rebounds off what it
    /// hits, one an unsteerable horizontal charge, and one lands twice on one
    /// press.
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_entity_catalog::{ImpulseMode, MoveEventKind, WindowTag};
        let set = shipped();
        let commanded = |id: &str| -> Option<(f32, f32)> {
            find(&set, id).events.iter().find_map(|e| match &e.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } => Some(*local),
                _ => None,
            })
        };
        // Up: a rise only.
        let up = commanded("excluded_middle").expect("the Up-B displaces");
        assert!(up.1 < 0.0 && up.0 == 0.0);
        // Down: a plunge that rebounds off a body.
        let down = commanded("reductio").expect("the dive displaces");
        assert!(down.1 > 0.0);
        assert!(find(&set, "reductio")
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .any(|v| v.on_hit.is_some()));
        // Side: a horizontal charge with an unsteerable tail.
        let side = commanded("modus_ponens").expect("the side special travels");
        assert!(side.0 > 0.0);
        assert!(
            find(&set, "modus_ponens")
                .windows
                .iter()
                .any(|w| matches!(w.tag, WindowTag::Recovery) && w.motion_scale == 0.0),
            "a charge you can steer out of is not a commitment"
        );
        // Neutral: no displacement; it lands twice instead.
        assert!(commanded("bivalence").is_none());
        assert_eq!(
            find(&set, "bivalence")
                .windows
                .iter()
                .filter(|w| matches!(w.tag, WindowTag::Active))
                .count(),
            2,
            "the neutral special's whole idea is the second window"
        );
    }

    /// Every press a body can make reaches a move, in both postures.
    #[test]
    fn both_postures_reach_at_least_eight_distinct_moves() {
        use ambition_entity_catalog::AttackDir;
        let set = shipped();
        let reachable = |grounded: bool| -> std::collections::BTreeSet<String> {
            let mut ids = std::collections::BTreeSet::new();
            for base in ["attack", "smash", "special"] {
                for dir in [
                    AttackDir::Neutral,
                    AttackDir::Forward,
                    AttackDir::Back,
                    AttackDir::Up,
                    AttackDir::Down,
                ] {
                    if let Some(m) = set.move_for_directional_verb(base, dir, grounded) {
                        ids.insert(m.id.clone());
                    }
                }
            }
            ids
        };
        let on_ground = reachable(true);
        let airborne = reachable(false);
        assert!(
            on_ground.len() >= 8,
            "a grounded George reaches only {:?}",
            on_ground
        );
        assert!(
            airborne.len() >= 8,
            "an airborne George reaches only {:?}",
            airborne
        );
        // The recovery is reachable from both postures, so it can be practised
        // on stage.
        assert!(on_ground.contains("excluded_middle"));
        assert!(airborne.contains("excluded_middle"));
        // The forward press does not fall through to the jab.
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|m| m.id.as_str()),
            Some("tilt_forward")
        );
    }

    /// The feedback is differentiated and resolvable. Two claims in one test
    /// because they fail together: identical sounds give no feedback, and an
    /// effect no shipped spritesheet carries never plays.
    #[test]
    fn important_moves_sound_and_look_like_themselves() {
        use ambition_entity_catalog::MoveEventKind;
        let set = shipped();
        let mut effects = std::collections::BTreeSet::new();
        let mut cues = std::collections::BTreeSet::new();
        // Accumulate problems across every move, then assert once, so one run
        // lists every move that references a renamed effect.
        let mut problems: Vec<String> = Vec::new();
        for m in &set.moves {
            problems.extend(m.presentation_problems(
                ambition_platformer2d::sprite_sheet::fx::is_authored_effect,
            ));
            for ev in &m.events {
                match &ev.kind {
                    MoveEventKind::Vfx { effect, .. } => {
                        effects.insert(effect.clone());
                    }
                    MoveEventKind::Sfx { cue } => {
                        cues.insert(cue.clone());
                    }
                    _ => {}
                }
            }
        }
        // Before the palette checks below, which also fail on a renamed
        // effect with a less useful message.
        //
        // The message names the other cause. `fx::is_authored_effect` reads a
        // table that `ambition_sprite_sheet`'s `build.rs` bakes at compile time
        // from the generated, gitignored `assets/sprites`. If they were never
        // rendered, the table is empty and every move is listed. The count is
        // the diagnosis.
        assert!(
            problems.is_empty(),
            "{} move(s) name an unknown cosmetic effect.\n{problems:?}\n\
             ⇒ IF NEARLY EVERY MOVE IS LISTED, the baked FX sheet table is EMPTY \
             and this is not a content bug: the sheets are generated and \
             gitignored. Rebuild them, then re-run:\n\
             \x20   ./scripts/regen/sprites.sh          # or one target: --target george_booul_vfx\n\
             \x20   scripts/setup/generated_content.sh  # everything, fonts included\n\
             \x20   find crates/ambition_platformer2d_actor_monolith/assets/sprites \\\n\
             \x20        -name '*_spritesheet.ron' | wc -l   # 0 means the bake is empty\n\
             ⇒ IF ONLY ONE OR TWO ARE LISTED, a sheet row really was renamed or \
             removed and this moveset still names the old row.",
            problems.len()
        );
        assert!(
            effects.len() >= 4,
            "a jab, a smash, a launcher, a special and a recovery cannot all \
             look the same: {effects:?}"
        );
        assert!(cues.len() >= 3, "{cues:?}");
        // The recovery activating has its own burst.
        let up_b = find(&set, "excluded_middle");
        assert!(up_b.events.iter().any(|e| matches!(
            &e.kind,
            MoveEventKind::Vfx { effect, .. } if effect == "classic_burst"
        )));
        // A heavy landing sounds different from a poke landing.
        let heavy_hit = |id: &str| -> Option<String> {
            find(&set, id)
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .find_map(|v| v.hit_sfx.clone())
        };
        assert_ne!(heavy_hit("smash_forward"), heavy_hit("jab"));
        assert!(heavy_hit("smash_forward").is_some());
        assert!(heavy_hit("jab").is_none(), "a jab does not clang");
    }

    /// The jab's two cancels are different promises. The string continues on
    /// a whiff (`Always`); the route across George's gap rewards connecting
    /// (`OnHit`). Windows are read by what they name, not by their order.
    #[test]
    fn the_jab_strings_on_a_whiff_and_opens_the_commitments_only_when_it_lands() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let jab = find(&shipped(), "jab");
        let cancels: Vec<(Vec<String>, CancelCondition)> = jab
            .windows
            .iter()
            .filter_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition } => Some((into.clone(), *condition)),
                _ => None,
            })
            .collect();
        let named = |target: &str| {
            cancels
                .iter()
                .find(|(into, _)| into.iter().any(|t| t == target))
                .unwrap_or_else(|| panic!("no cancel window names `{target}`"))
        };
        assert_eq!(
            named("jab2").1,
            CancelCondition::Always,
            "a whiffed jab must still string"
        );
        let route = named("smash");
        assert_eq!(
            route.1,
            CancelCondition::OnHit,
            "George's route across the gap is bought by connecting"
        );
        assert!(route.0.iter().any(|t| t == "special"));
        // The string is named first: the chain takes the first successor it
        // can resolve by move id, so an undirected follow-up reaches `jab2`,
        // not a smash.
        assert_eq!(
            cancels[0].0.first().map(String::as_str),
            Some("jab2"),
            "the string has to be the first thing the jab nominates"
        );
    }

    /// What George leaves unanswered is the genre's shape, not a gap.
    ///
    /// The sibling guard in `moveset.rs` pins the stand-in's silent presses.
    /// Both enumerate every `(base, direction, stance)` press, because
    /// `move_for_directional_verb` falls back to the base verb.
    ///
    /// George is silent on seven presses, all `smash`: no neutral smash, no back
    /// smash, and no aerial smashes. That matches the genre, which uses the
    /// `attack` family in the air. The stand-in's silent presses are specials,
    /// which the genre has. See `awaiting-maintainer-decision.md`.
    ///
    /// The two halves are different claims:
    ///
    /// - The `smash` set is structural: `SmashRepertoire` has only
    ///   `forward_smash` / `up_smash` / `down_smash`, and aerials answer `attack`
    ///   presses. This arm is a schema guard.
    /// - The special arm is authored. George authors four specials (neutral,
    ///   side, up, down); the back press falls through to the neutral one.
    #[test]
    fn the_presses_george_leaves_unanswered_are_the_ones_the_genre_lacks() {
        use ambition_entity_catalog::AttackDir;
        let set = shipped();
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

        // The load-bearing half: George answers all ten attack and all ten
        // special presses. If a special falls silent, the maintainer decision
        // changes shape.
        let non_smash: Vec<&String> = silent.iter().filter(|p| !p.starts_with("smash_")).collect();
        assert!(
            non_smash.is_empty(),
            "George stopped answering a non-`smash` press: {non_smash:?}. The roster question in `awaiting-maintainer-decision.md` rests on George answering all ten specials while the stand-ins answer two."
        );

        // The seven are exactly the genre's missing presses, asserted as a
        // set so a swap cannot pass by keeping the count.
        let mut got = silent.clone();
        got.sort();
        let mut want = vec![
            "smash_neutral_ground",
            "smash_neutral_air",
            "smash_back_ground",
            "smash_back_air",
            "smash_forward_air",
            "smash_up_air",
            "smash_down_air",
        ];
        want.sort();
        assert_eq!(
            got,
            want,
            "George's silent presses moved. Gaining one is likely good news (an authored move) and losing one is a regression; either way the claim in `smash-parity-inventory.md` wants re-deriving, not editing to match."
        );
    }

    /// A shielded jab buys a grab (`OnBlock`): a blocked jab is when the
    /// defender is committed to shield. Without it the `OnHit` route is closed,
    /// because nothing connected.
    ///
    /// It must not widen the other two: the string stays `Always` and the
    /// route stays `OnHit`. `MoveContact` carries three facts for this.
    #[test]
    fn a_shielded_jab_buys_george_a_grab() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let george = shipped();
        let jab = find(&george, "jab");
        let blocked: Vec<&Vec<String>> = jab
            .windows
            .iter()
            .filter_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition }
                    if *condition == CancelCondition::OnBlock =>
                {
                    Some(into)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            blocked.len(),
            1,
            "the jab should nominate exactly one on-block continuation"
        );
        assert!(
            blocked[0].iter().any(|t| t == "grab"),
            "a shielded jab buys a GRAB — the option that beats the shield that \
             just ate it; got {:?}",
            blocked[0]
        );

        // The name must resolve: `grab` is a verb, not a move id, so the
        // contract's verb map must bind it.
        let targets = george.cancel_targets(blocked[0]);
        assert!(
            !targets.is_empty(),
            "`grab` named a continuation nothing in George's table answers to; \
             the window would open onto nothing"
        );
    }
}

mod limit_payoff_tests {
    use super::shipped;

    fn spec(id: &str) -> ambition_entity_catalog::MoveSpec {
        shipped()
            .move_by_id(id)
            .unwrap_or_else(|| panic!("the contract carries `{id}`"))
            .clone()
    }

    fn top_damage(spec: &ambition_entity_catalog::MoveSpec) -> i32 {
        spec.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .expect("the special hits")
    }

    /// An `id` with no move behind it is a dead button. `when_refused` resolves
    /// with `move_by_id` against the carried moves; a fallback never pushed into
    /// `moves` resolves to `None`, so neutral-B on an empty meter does nothing.
    /// The id is a `String`, so the compiler cannot check it.
    #[test]
    fn the_metered_special_falls_back_to_a_move_the_contract_actually_carries() {
        let payoff = spec("bivalence");
        let fallback_id = payoff
            .gates
            .when_refused
            .clone()
            .expect("the metered special names a fallback");
        assert!(
            shipped().move_by_id(&fallback_id).is_some(),
            "`bivalence` falls back to `{fallback_id}`, which the contract does \
             not carry — on an empty meter the press resolves to nothing and the \
             button is dead"
        );
    }

    /// The order is the mechanic: a fallback cloned after the buff would be
    /// the expensive move, and the meter would buy nothing while every other
    /// test passes. This compares the two specs one press can give.
    #[test]
    fn a_full_meter_buys_a_strictly_harder_answer_from_the_same_press() {
        let payoff = spec("bivalence");
        let unmetered = spec("bivalence_unmetered");
        assert!(
            top_damage(&payoff) > top_damage(&unmetered),
            "the metered neutral special ({}) must hit harder than the one an \
             empty meter gives ({}), or blocking all match bought nothing",
            top_damage(&payoff),
            top_damage(&unmetered),
        );
        assert!(
            unmetered.gates.costs.is_empty(),
            "the fallback must be free; a priced fallback is refused by the same \
             affordance that refused the payoff, and the press dies"
        );
    }

    /// The price is the cap: `afford_meter` refuses anything less. A lower
    /// price would be a different mechanic (a chargeable resource).
    #[test]
    fn the_price_is_the_whole_meter() {
        assert_eq!(
            spec("bivalence").gates.costs,
            vec![ambition_resource_spec::ResourceCost::new(
                ambition_entity_catalog::smash_limit::LIMIT,
                ambition_entity_catalog::smash_limit::LimitMeterFill::JONS_BASELINE.cap,
            )],
            "the payoff must cost exactly the match's Limit cap, in Limit"
        );
    }

    /// The fallback is carried, not pressable. With a verb, a player could
    /// reach the cheap version directly.
    #[test]
    fn the_fallback_is_bound_to_no_input() {
        assert!(
            !shipped()
                .verbs
                .iter()
                .any(|(_, id)| id == "bivalence_unmetered"),
            "the unmetered fallback has been bound to an input; it is reachable \
             only by refusal, or the meter buys nothing"
        );
    }
}
