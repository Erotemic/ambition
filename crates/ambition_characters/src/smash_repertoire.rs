//! Standard Smash action grammar and repertoire bookkeeping.
//!
//! Fighters author their own [`MoveSpec`]s; this module centralizes the standard
//! slots, their posture gates, and validation. [`SmashRepertoire`] is a struct with
//! no `Default`, so a missing standard slot is a compile-time error at the fighter
//! definition.
//!
//! Ground attacks are grounded-only, aerials are airborne-only, and specials are
//! available in either posture unless the down special provides distinct grounded
//! and airborne forms. The contextual down-special forms share one input slot.
//!
//! Smash vocabulary stops here and lowers into the generic [`MovesetContract`];
//! engine-level move execution does not depend on Smash-specific action names.

use ambition_entity_catalog::{MoveGates, MoveSpec, MovesetContract, RecoveryUse};

/// Ground moves are grounded-only so an airborne body falls THROUGH them to its
/// aerials rather than throwing a tilt in mid-air.
const GROUNDED: MoveGates = MoveGates {
    grounded: Some(true),
    // ⛔ A POSTURE KNOWS NOTHING ABOUT RECOVERIES. This is the neutral value the
    // lowering loop then DISCARDS in favour of whatever the move or the slot
    // said; see the destructure in `into_contract`.
    recovery: RecoveryUse::None,
    // A posture says nothing about being HELD. Whether a move refuses to
    // start from a saddle is that move's own statement -- `call_the_shark`
    // makes it -- and a stance default answering for every move would be
    // this file deciding a question it cannot see.
    forbidden_while_held: false,
    // ⭐ A GROUNDED ATTACK ROOTS ITS OWNER. Jon, W8 playtest: *"When I quickly
    // perform a Forward Smash, the fighter currently travels noticeably before
    // the Forward Smash takes over... I should not effectively dash first and
    // then Smash."* Measured through the real key stack: the smash STARTED on
    // the press tick — recognition was never late — and then the fighter
    // accelerated from a standstill to the full run cap, 64 world px, while its
    // own startup played.
    //
    // ⛔ so this is not an ordering fix and it is not a per-move number. It is
    // the one place the posture gates are applied, which makes it the one place
    // the posture's steering rule belongs: every fighter, every grounded slot,
    // by the same statement. A dash attack keeps its slide, which is the move's
    // own impulse and never was steering.
    roots_steering: true,
};
/// Aerials are airborne-only for the mirror reason: a grounded press must not
/// reach a move whose whole design is that landing costs you.
const AIRBORNE: MoveGates = MoveGates {
    grounded: Some(false),
    // ⛔ A POSTURE KNOWS NOTHING ABOUT RECOVERIES. This is the neutral value the
    // lowering loop then DISCARDS in favour of whatever the move or the slot
    // said; see the destructure in `into_contract`.
    recovery: RecoveryUse::None,
    // A posture says nothing about being HELD. Whether a move refuses to
    // start from a saddle is that move's own statement -- `call_the_shark`
    // makes it -- and a stance default answering for every move would be
    // this file deciding a question it cannot see.
    forbidden_while_held: false,
    // ⭐ AND AN AERIAL KEEPS ITS DRIFT, which is the other half of the same
    // rule: the genre trades ground control for air control, and a fighter that
    // could not steer a forward air would lose every edgeguard it has.
    roots_steering: false,
};
/// The specials: a move that answers its button from the ground OR the air.
const EITHER: MoveGates = MoveGates {
    grounded: None,
    // ⛔ A POSTURE KNOWS NOTHING ABOUT RECOVERIES. This is the neutral value the
    // lowering loop then DISCARDS in favour of whatever the move or the slot
    // said; see the destructure in `into_contract`.
    recovery: RecoveryUse::None,
    // A posture says nothing about being HELD. Whether a move refuses to
    // start from a saddle is that move's own statement -- `call_the_shark`
    // makes it -- and a stance default answering for every move would be
    // this file deciding a question it cannot see.
    forbidden_while_held: false,
    // A special answers from either stance, so it cannot state a stance rule.
    // What a special does to its owner's motion is the SPECIAL's own business
    // and is authored on its windows.
    roots_steering: false,
};

/// The neutral special, or a stated reason there is no authored one.
///
///  exactly one fighter abstains today — the player robot, whose Hadouken comes
/// from the CHARGED-PROJECTILE kit its body already derives. Authoring a
/// `special` binding here would overlay and replace it. The abstention is a
/// slot value rather than an omission so that it is impossible to do by
/// accident, and so the reason travels with it.
pub enum NeutralSpecial {
    /// This fighter authors its neutral-B.
    Authored(MoveSpec),
    /// The press is answered by the move the BODY's action set derives, which
    /// authoring here would replace. `because` says which move and why.
    FromBodyKit {
        /// Prose, for the next reader — e.g. `"the charged Hadouken"`.
        because: &'static str,
    },
}

/// The down special — one move, or the Bowser pair.
pub enum DownSpecial {
    /// One move that answers the press in both postures. The ordinary case for a
    /// down-B that means the same thing wherever you are standing.
    OneForm(MoveSpec),
    /// Two forms, one slot. `grounded` answers `special_down`, `airborne`
    /// answers `special_air_down` — which sits AHEAD of it in
    /// `directional_verb_chain`, so an airborne press reaches the air form and a
    /// grounded one falls past it to the ground form.
    ///
    ///  this is why the pair is a single slot and not two: a special gated to
    /// ONE posture and left unanswered in the other is not "a move with a
    /// restriction" — the chain walks past it to the NEUTRAL special, and the
    /// player pressed down-B and got something else.
    ByPosture {
        /// Feet down.
        grounded: MoveSpec,
        /// Feet up.
        airborne: MoveSpec,
    },
}

/// THE UP-B SLOT, and what it costs the fighter's airtime.
///
/// ⭐⭐ THE GENRE'S DEFAULT LIVES HERE BECAUSE THIS SLOT ALREADY MEANS "the
/// Up-B". Jon, 2026-08-25: *"characters can often use their up b more than once
/// without going into freefall. only a few should be exempt from that general
/// rule."* The engine cannot state that rule — `MoveGates::recovery` is
/// deliberately authored per move, because an up-special that does not lift and
/// a side-special that does are both ordinary things to write. But a SMASH
/// repertoire is not a generic moveset: its `up_special` field is the fighter's
/// recovery by definition, so this is the one place the default is expressible
/// at all.
///
/// ⛔⛔ AND IT WAS OPT-IN UNTIL 2026-08-26, WHICH MEANT IT WAS OFF. Every Up-B
/// was bound through the `EITHER` posture gate, which says `RecoveryUse::None`;
/// exactly one fighter in the tree had written the opt-in by hand. Asking
/// fourteen authors to remember one field is how a rule ends up applying to one
/// of them, and the census that would have caught it is the grep nobody ran.
///
/// ⭐ SHAPED AFTER [`NeutralSpecial`], which is the same problem solved once
/// already in this file: the ordinary case is the shortest thing to write, the
/// exception is a different variant, and the exceptional variant carries a
/// `because` so the reason travels with it instead of living in a commit
/// message.
pub enum UpSpecial {
    /// The genre's rule: one use per airtime, helpless once the move ends.
    ///
    /// This is what a fighter gets by writing the obvious thing, which is the
    /// whole point.
    Standard(MoveSpec),
    /// Spends the airtime's recovery but leaves the fighter able to act — a
    /// recovery that hands over a VEHICLE rather than an arc. The pirate's
    /// burning shark is the case this exists for.
    NoFreefall(MoveSpec),
    /// ⛔ NOT A RECOVERY AT ALL: this fighter's up-B is repeatable, and
    /// `because` says why that is a design and not an oversight.
    NotARecovery {
        /// The move.
        spec: MoveSpec,
        /// Prose, for the next reader — e.g. `"it does not rise"`.
        because: &'static str,
    },
}

impl UpSpecial {
    /// The move, with `gates.recovery` set to what this slot says it costs.
    ///
    /// ⛔ THE SLOT OVERWRITES THE MOVE HERE, and that is the opposite of the
    /// rule the posture gates follow one function down. It is right for this one
    /// field: a posture cannot know whether a move is a recovery, but a slot
    /// whose NAME is "the up-B" can, and letting a moveset quietly disagree with
    /// its own repertoire slot would put the default back where it started.
    fn into_spec(self) -> MoveSpec {
        let (mut spec, recovery) = match self {
            Self::Standard(spec) => (spec, RecoveryUse::SpendAndFreefall),
            Self::NoFreefall(spec) => (spec, RecoveryUse::SpendWithoutFreefall),
            Self::NotARecovery { spec, .. } => (spec, RecoveryUse::None),
        };
        spec.gates.recovery = recovery;
        spec
    }
}

/// The standard smash repertoire, as sixteen presses of bespoke moves.
///
/// See the module doc. Fill every field; the compiler enforces that.
pub struct SmashRepertoire {
    /// `attack` — neutral, grounded.
    pub jab: MoveSpec,
    /// `attack_forward`.
    pub forward_tilt: MoveSpec,
    /// `attack_up`.
    pub up_tilt: MoveSpec,
    /// `attack_down`.
    pub down_tilt: MoveSpec,
    /// `smash_forward`.
    pub forward_smash: MoveSpec,
    /// `smash_up`.
    pub up_smash: MoveSpec,
    /// `smash_down`.
    pub down_smash: MoveSpec,
    /// `attack_air`.
    pub neutral_air: MoveSpec,
    /// `attack_air_forward`.
    pub forward_air: MoveSpec,
    /// `attack_air_back`.
    pub back_air: MoveSpec,
    /// `attack_air_up`.
    pub up_air: MoveSpec,
    /// `attack_air_down`.
    pub down_air: MoveSpec,
    /// `special`.
    pub neutral_special: NeutralSpecial,
    /// `special_forward`.
    pub side_special: MoveSpec,
    /// `special_up` — and WHAT IT COSTS. See [`UpSpecial`].
    pub up_special: UpSpecial,
    /// `special_down`, and possibly `special_air_down`. See [`DownSpecial`].
    pub down_special: DownSpecial,
    /// The capture kit — grab, pummel, throws. See
    /// [`SmashCaptureRepertoire`](crate::smash_capture::SmashCaptureRepertoire).
    ///
    /// All fourteen author one now, so the compiler resumes doing here what it does for the other
    /// sixteen slots.
    ///
    ///  this replaces a grep. A goal check read the movesets looking for
    /// `capture: Some`, which is the kind of guard that answers a question the
    /// compiler can answer better: a new fighter that forgets a grab no longer
    /// ships and gets noticed, it does not build.
    pub capture: crate::smash_capture::SmashCaptureRepertoire,
    /// `taunt` — the move that buys nothing. Required like every other slot,
    /// so a fighter with nothing to say has to say so; `moveset_authoring::taunt`
    /// is the one-liner for a fighter whose taunt is not yet designed.
    pub taunt: MoveSpec,
    /// `attack_dash` — the move a body already moving forward throws.
    /// Required like every other slot: the engine has selected
    /// `AttackIntent::DashForward` for a dashing swing since long before any
    /// fighter could answer it, and an unauthored dash attack does not read as
    /// missing — it reads as the forward tilt, which is worse than a gap because
    /// nothing looks wrong. `moveset_authoring::dash_attack` owns the shape.
    pub dash_attack: MoveSpec,
}

/// Every verb a [`SmashRepertoire`] can bind.
///
///  this exists because [`SmashRepertoire::into_contract`]'s own doc was FALSE. It says
/// it is *"the ONE place the verb strings exist"* — and it was not.
///
///  both halves of that pair have now been missed, in three days. `taunt` reached the
/// device table, the human brain's press list, `ENGINE_ACTIONS` and its rollback codec, and
/// every fighter authored one — while the vocabulary did not know the word, and nineteen
/// characters shipped reporting *"unknown input verb `taunt`"*. The lesson is not "remember the
/// fifth list"; it is that a list nobody can derive gets remembered four times out of five.
///
///  not a registry and not a new authority. The table in `into_contract`
/// is still the only thing that BINDS a verb to a move. This is that table's
/// verb SET, held to it by `the_bound_table_binds_exactly_the_declared_vocabulary`,
/// so a downstream census can ASK instead of remember.
///
///  the whole set, including the conditional slots. `special` is bound
/// only by an authored neutral special and `special_air_down` only by a
/// `ByPosture` down special — but a verb a repertoire CAN bind is authoring
/// vocabulary whether or not one fighter uses it, and a vocabulary that shrank
/// with the cast would call a real verb unknown the day the last fighter using
/// it changed shape.
pub const REPERTOIRE_VERBS: &[&str] = &[
    // The grounded normals.
    "attack",
    "attack_forward",
    "attack_up",
    "attack_down",
    "smash_forward",
    "smash_up",
    "smash_down",
    // The aerials.
    "attack_air",
    "attack_air_forward",
    "attack_air_back",
    "attack_air_up",
    "attack_air_down",
    // The two that were missed.
    "taunt",
    "attack_dash",
    // The specials, conditional slots included.
    "special",
    "special_forward",
    "special_up",
    "special_down",
    "special_air_down",
    // The capture kit — flat, never directional. A throw is not `grab_forward`.
    "grab",
    // ⭐ the one capture verb with a STANCE rather than a direction: the same
    // grab out of a run. Derived from each fighter's standing grab in
    // `SmashCaptureRepertoire::bound`, so no fighter authors it and every
    // fighter has it.
    "grab_dash",
    "capture_pummel",
    "capture_throw_forward",
    "capture_throw_back",
    "capture_throw_up",
    "capture_throw_down",
];

impl SmashRepertoire {
    /// Lower the repertoire into the generic move contract the engine speaks.
    ///
    /// This is the ONE place the verb strings exist, the ONE place the posture
    /// gates are applied, and the ONE place the table is checked — which is the
    /// whole point of the type.
    ///
    /// # Panics
    ///
    /// If two slots were given moves with the SAME id.
    pub fn into_contract(self) -> MovesetContract {
        let Self {
            jab,
            forward_tilt,
            up_tilt,
            down_tilt,
            forward_smash,
            up_smash,
            down_smash,
            neutral_air,
            forward_air,
            back_air,
            up_air,
            down_air,
            neutral_special,
            side_special,
            up_special,
            down_special,
            capture,
            taunt,
            dash_attack,
        } = self;

        let mut bound: Vec<(&'static str, MoveSpec, MoveGates)> = vec![
            ("attack", jab, GROUNDED),
            ("attack_forward", forward_tilt, GROUNDED),
            ("attack_up", up_tilt, GROUNDED),
            ("attack_down", down_tilt, GROUNDED),
            ("smash_forward", forward_smash, GROUNDED),
            ("smash_up", up_smash, GROUNDED),
            ("smash_down", down_smash, GROUNDED),
            ("attack_air", neutral_air, AIRBORNE),
            ("attack_air_forward", forward_air, AIRBORNE),
            ("attack_air_back", back_air, AIRBORNE),
            ("attack_air_up", up_air, AIRBORNE),
            ("attack_air_down", down_air, AIRBORNE),
            ("taunt", taunt, GROUNDED),
            //  GROUNDED: a dash is a ground stance, and `move_for_attack` only
            // asks for this verb when the body is on the floor.
            ("attack_dash", dash_attack, GROUNDED),
        ];
        if let NeutralSpecial::Authored(spec) = neutral_special {
            bound.push(("special", spec, EITHER));
        }
        bound.push(("special_forward", side_special, EITHER));
        // ⭐ THE SLOT DECIDES THE RECOVERY, and it decides it HERE rather than in
        // the loop below, so the loop's rule stays *"a posture sets posture
        // fields and nothing else"*. What comes out is an ordinary `MoveSpec`
        // whose `gates.recovery` the loop then leaves alone.
        bound.push(("special_up", up_special.into_spec(), EITHER));
        match down_special {
            DownSpecial::OneForm(spec) => bound.push(("special_down", spec, EITHER)),
            DownSpecial::ByPosture { grounded, airborne } => {
                bound.push(("special_down", grounded, GROUNDED));
                bound.push(("special_air_down", airborne, AIRBORNE));
            }
        }

        // Capture moves are GROUNDED for v1 — aerial and command grabs are named
        // future techniques, and a capture that answered an airborne press would
        // be one of them by accident.
        {
            for (verb, spec) in capture.bound() {
                bound.push((verb, spec, GROUNDED));
            }
        }

        let mut contract = MovesetContract::default();
        for (verb, mut spec, gates) in bound {
            // ⛔⛔ THE POSTURE FIELDS ONLY. This was `spec.gates = gates`, a
            // wholesale overwrite, and it silently threw away every statement a
            // MOVE made about its own gates. Measured 2026-08-26: the pointed
            // polygon was the one fighter in the tree that had opted into the
            // recovery budget, and the line below deleted the opt-in on the way
            // into the contract.
            //
            // ⭐ THE SPLIT IS BY AUTHORITY, and the comment on `GROUNDED` above
            // already states it: a POSTURE knows whether a slot answers from the
            // ground and whether that stance roots its owner, and it is right
            // that one place decides those for every fighter. It knows nothing
            // about whether a particular move is a recovery — `MoveGates::
            // recovery`'s own doc says so: *"AUTHORED, not inferred from a name
            // or an impulse."* For the up-B the author is the SLOT, and it has
            // already spoken by the time a spec reaches this loop
            // (`UpSpecial::into_spec`), which is why `recovery` is dropped here
            // rather than carried through.
            //
            // ⚠ an exhaustive destructure rather than two assignments, so a new
            // gate is a compile error here and somebody has to say which side of
            // this line it falls on.
            let MoveGates {
                grounded,
                roots_steering,
                recovery: _,
                // ⛔ THE MOVE'S OWN, NOT THE POSTURE'S — same side of the line as
                // `recovery` above. `call_the_shark` sets it; a stance template
                // overwriting it would delete the rule.
                forbidden_while_held: _,
            } = gates;
            spec.gates.grounded = grounded;
            spec.gates.roots_steering = roots_steering;
            if let Some(clash) = contract.verbs.iter().find(|(_, id)| **id == spec.id) {
                panic!(
                    "smash repertoire: `{verb}` and `{}` were both given the move id `{}`. \
                     A move id is looked up by first match, so one of the two presses would \
                     silently swing the other's timeline — give each slot its own id.",
                    clash.0, spec.id
                );
            }
            contract.verbs.insert(verb.to_string(), spec.id.clone());
            contract.moves.push(spec);
        }
        contract
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::{AttackDir, ClipBinding};

    pub(super) fn spec(id: &str) -> MoveSpec {
        MoveSpec {
            display_name: None,
            id: id.to_string(),
            clip: ClipBinding {
                clip: "attack".to_string(),
                fallbacks: Vec::new(),
            },
            duration_s: 0.2,
            windows: Vec::new(),
            events: Vec::new(),
            // Deliberately the WRONG gate for every slot: the seam must set it.
            gates: MoveGates {
                grounded: Some(true),
                roots_steering: false,
                // ⛔ A POSTURE KNOWS NOTHING ABOUT RECOVERIES. This is the neutral value the
                // lowering loop then DISCARDS in favour of whatever the move or the slot
                // said; see the destructure in `into_contract`.
                recovery: RecoveryUse::None,
                // A posture says nothing about being HELD. Whether a move refuses to
                // start from a saddle is that move's own statement -- `call_the_shark`
                // makes it -- and a stance default answering for every move would be
                // this file deciding a question it cannot see.
                forbidden_while_held: false,
            },
            start_impulse: None,
            smash_charge_mult: 1.0,
            charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
            smash_charge: None,
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
        }
    }

    pub(super) fn repertoire(
        down_special: DownSpecial,
        neutral_special: NeutralSpecial,
    ) -> SmashRepertoire {
        SmashRepertoire {
            jab: spec("jab"),
            forward_tilt: spec("ftilt"),
            up_tilt: spec("utilt"),
            down_tilt: spec("dtilt"),
            forward_smash: spec("fsmash"),
            up_smash: spec("usmash"),
            down_smash: spec("dsmash"),
            neutral_air: spec("nair"),
            forward_air: spec("fair"),
            back_air: spec("bair"),
            up_air: spec("uair"),
            down_air: spec("dair"),
            neutral_special,
            side_special: spec("sspecial"),
            up_special: UpSpecial::Standard(spec("uspecial")),
            taunt: crate::moveset_authoring::taunt("taunt", 0.9),
            dash_attack: crate::moveset_authoring::dash_attack(
                "dash_attack",
                crate::moveset_authoring::DashAttackShape::GENRE,
                9,
                320.0,
            ),
            //  a real kit, because the slot is required now. The fixture's
            // job is to exercise the VERB TABLE, so the smallest catchable grab
            // that reaches `bound()` is the honest fixture — not a placeholder
            // that would make the capture verbs untested here.
            capture: crate::smash_capture::SmashCaptureRepertoire {
                cues: crate::smash_capture::CaptureCues::GENERIC,
                grab: crate::smash_capture::author_standing_grab(
                    crate::smash_capture::grab_shell("grab", "attack", 0.07, 0.05, 0.2),
                    crate::smash_capture::CaptureAttemptParams {
                        offset: (12.0, 1.0),
                        half_extents: (18.0, 15.0),
                        hold_offset: (13.0, 3.0),
                    },
                ),
                pummel: crate::smash_capture::author_pummel(
                    crate::smash_capture::capture_beat("pummel", "attack", 0.18),
                    0.08,
                    crate::smash_capture::CapturePummelParams { damage: 3 },
                ),
                forward_throw: crate::smash_capture::author_throw(
                    crate::smash_capture::capture_beat("fthrow", "attack", 0.26),
                    0.14,
                    crate::smash_capture::CaptureThrowParams {
                        damage: 8,
                        knockback: 120.0,
                        knockback_growth: 2.0,
                        launch_dir: (0.85, -0.55),
                    },
                ),
                back_throw: None,
                up_throw: None,
                down_throw: None,
            },
            down_special,
        }
    }

    /// The declared vocabulary is exactly what the table binds — both
    /// directions, so neither list can grow without the other.
    ///
    ///  it runs a MAXIMAL repertoire, every conditional slot taking the
    /// branch that binds the most verbs: an authored neutral special, a
    /// `ByPosture` down special, and all three optional throws present. A
    /// fixture that left one `None` would prove the const has no EXTRA words and
    /// say nothing about the ones it is missing — which is the direction that
    /// has failed twice.
    ///
    ///  the floor is not decoration. `into_contract` returning an empty
    /// contract would make the two sets equal to each other and to nothing, and
    /// this test would pass while every fighter lost every move.
    #[test]
    fn the_bound_table_binds_exactly_the_declared_vocabulary() {
        use std::collections::BTreeSet;

        let mut maximal = repertoire(
            DownSpecial::ByPosture {
                grounded: spec("dspecial_ground"),
                airborne: spec("dspecial_air"),
            },
            NeutralSpecial::Authored(spec("nspecial")),
        );
        for (slot, id) in [
            (&mut maximal.capture.back_throw, "bthrow"),
            (&mut maximal.capture.up_throw, "uthrow"),
            (&mut maximal.capture.down_throw, "dthrow"),
        ] {
            *slot = Some(crate::smash_capture::author_throw(
                crate::smash_capture::capture_beat(id, "attack", 0.26),
                0.14,
                crate::smash_capture::CaptureThrowParams {
                    damage: 8,
                    knockback: 120.0,
                    knockback_growth: 2.0,
                    launch_dir: (0.85, -0.55),
                },
            ));
        }

        let bound: BTreeSet<String> = maximal.into_contract().verbs.keys().cloned().collect();
        let declared: BTreeSet<String> = REPERTOIRE_VERBS.iter().map(|v| (*v).to_owned()).collect();

        assert!(
            bound.len() >= 20,
            "a maximal repertoire bound only {} verbs — the table produced almost \
             nothing, and comparing two nearly-empty sets proves nothing",
            bound.len(),
        );
        assert_eq!(
            bound, declared,
            "`REPERTOIRE_VERBS` and `into_contract`'s table disagree. Whichever \
             gained a verb, give it to the other: preparation reads the const to \
             decide whether an authored verb is a real word, and a verb the table \
             binds but the const omits ships as a move on a button the runtime \
             says does not exist.",
        );
    }

    /// Every press this vocabulary names is answered, in every posture it is
    /// asked in — resolved the way a BODY resolves it, through the directional
    /// chain, rather than by asking whether a verb key exists.
    #[test]
    fn every_press_is_answered_in_every_posture_it_is_asked_in() {
        for down in [
            DownSpecial::OneForm(spec("dspecial")),
            DownSpecial::ByPosture {
                grounded: spec("dspecial_ground"),
                airborne: spec("dspecial_air"),
            },
        ] {
            let set = repertoire(down, NeutralSpecial::Authored(spec("nspecial"))).into_contract();
            let ground_only = [
                ("attack", AttackDir::Neutral),
                ("attack", AttackDir::Forward),
                ("attack", AttackDir::Up),
                ("attack", AttackDir::Down),
                ("smash", AttackDir::Forward),
                ("smash", AttackDir::Up),
                ("smash", AttackDir::Down),
            ];
            for (base, dir) in ground_only {
                assert!(
                    set.move_for_directional_verb(base, dir, true).is_some(),
                    "{base}/{dir:?} unanswered on the ground"
                );
            }
            for dir in [
                AttackDir::Neutral,
                AttackDir::Forward,
                AttackDir::Back,
                AttackDir::Up,
                AttackDir::Down,
            ] {
                assert!(
                    set.move_for_directional_verb("attack", dir, false)
                        .is_some(),
                    "aerial {dir:?} unanswered in the air"
                );
            }
            for dir in [
                AttackDir::Neutral,
                AttackDir::Forward,
                AttackDir::Up,
                AttackDir::Down,
            ] {
                for grounded in [true, false] {
                    assert!(
                        set.move_for_directional_verb("special", dir, grounded)
                            .is_some(),
                        "special {dir:?} unanswered (grounded={grounded})"
                    );
                }
            }
        }
    }

    /// The two-form down-B maps ground and air postures to their corresponding moves.
    #[test]
    fn the_two_form_down_b_answers_each_posture_with_its_own_move() {
        let set = repertoire(
            DownSpecial::ByPosture {
                grounded: spec("slam_from_a_hop"),
                airborne: spec("plunge"),
            },
            NeutralSpecial::Authored(spec("nspecial")),
        )
        .into_contract();
        let reached = |grounded| {
            set.move_for_directional_verb("special", AttackDir::Down, grounded)
                .map(|m| m.id.clone())
        };
        assert_eq!(reached(true).as_deref(), Some("slam_from_a_hop"));
        assert_eq!(reached(false).as_deref(), Some("plunge"));
    }

    /// The posture comes from the SLOT, not from what the fighter set.
    ///
    /// the fixture hands every slot a `grounded: Some(true)` spec — the gate that would be wrong
    /// for eleven of the sixteen.
    #[test]
    fn the_slot_owns_the_posture_gate() {
        let set = repertoire(
            DownSpecial::OneForm(spec("dspecial")),
            NeutralSpecial::Authored(spec("nspecial")),
        )
        .into_contract();
        let gate = |id: &str| set.move_by_id(id).expect("defined").gates.grounded;
        assert_eq!(gate("jab"), Some(true));
        assert_eq!(gate("fsmash"), Some(true));
        assert_eq!(gate("nair"), Some(false));
        assert_eq!(gate("dair"), Some(false));
        assert_eq!(gate("nspecial"), None);
        assert_eq!(gate("dspecial"), None);
    }

    /// An abstaining neutral-B binds nothing, so the body's derived move
    /// keeps the press instead of being overlaid by an authored one.
    #[test]
    fn abstaining_from_the_neutral_special_binds_nothing() {
        let set = repertoire(
            DownSpecial::OneForm(spec("dspecial")),
            NeutralSpecial::FromBodyKit {
                because: "the charged projectile the body derives",
            },
        )
        .into_contract();
        assert!(!set.verbs.contains_key("special"));
        // ⚠ **19, and every step of the count is a VERB ARRIVING rather than a
        // retune.** 15 → 18 on 2026-08-19, when `capture` stopped being `Option`
        // because every fighter gained a grab and this fixture became a fighter
        // WITH one, binding the three capture verbs beside its ordinary slots.
        // 18 → 19 on 2026-08-20 with the taunt, 19 → 20 with the dash attack,
        // 20 → 21 on 2026-08-22 with the RUNNING GRAB — which every fighter has
        // without authoring one, because the capture kit derives it. The claim
        // above is untouched either way: abstaining from the neutral special
        // still binds nothing, and this number exists to catch a slot binding
        // something it should not.
        assert_eq!(set.verbs.len(), 21);
    }

    /// Two slots cannot share a move id. The one integrity defect this shape
    /// cannot rule out by construction, so it is named at preparation time.
    #[test]
    #[should_panic(expected = "were both given the move id")]
    fn two_slots_sharing_a_move_id_is_refused() {
        let _ = repertoire(
            DownSpecial::OneForm(spec("jab")),
            NeutralSpecial::Authored(spec("nspecial")),
        )
        .into_contract();
    }
}

#[cfg(test)]
mod up_special_recovery_tests {
    use super::tests::{repertoire, spec};
    use super::*;

    fn lowered(up_special: UpSpecial) -> RecoveryUse {
        let mut kit = repertoire(
            DownSpecial::OneForm(spec("dspecial")),
            NeutralSpecial::Authored(spec("nspecial")),
        );
        kit.up_special = up_special;
        let contract = kit.into_contract();
        let id = contract
            .verbs
            .get("special_up")
            .expect("the up-B slot bound no verb, so the button is dead")
            .clone();
        contract
            .moves
            .into_iter()
            .find(|m| m.id == id)
            .expect("the up-B verb names a move the contract does not carry")
            .gates
            .recovery
    }

    /// ⭐⭐ THE GENRE'S RULE IS WHAT A FIGHTER GETS FOR WRITING THE OBVIOUS
    /// THING, and that is the whole of D204.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS: the rule was OPT-IN, and one fighter in the
    /// tree had opted in. Every up-B went through the `EITHER` posture gate,
    /// which says `RecoveryUse::None`, so most of the roster could press its
    /// recovery forever and could only be killed by a launch that outran it —
    /// which is precisely the behaviour Jon asked to remove. A `Standard` that
    /// lowered to anything but `SpendAndFreefall` would put it straight back,
    /// silently, for fourteen fighters at once.
    #[test]
    fn a_standard_up_b_spends_the_recovery_and_ends_in_freefall() {
        assert_eq!(
            lowered(UpSpecial::Standard(spec("uspecial"))),
            RecoveryUse::SpendAndFreefall,
            "the ordinary up-B costs nothing, so the roster has no recovery              budget and the stage has no bottom blastzone"
        );
    }

    /// AND THE TWO EXCEPTIONS SURVIVE THE LOWERING.
    ///
    /// ⛔ THE DEFECT THIS PINS is the mirror of the one above and it has already
    /// happened once: `into_contract` used to do `spec.gates = gates`, which
    /// threw away everything a move said about itself. An exception that the
    /// lowering flattens back to the default is worse than no exception,
    /// because the content still READS as though the fighter is exempt.
    #[test]
    fn the_declared_exceptions_reach_the_contract_as_themselves() {
        assert_eq!(
            lowered(UpSpecial::NoFreefall(spec("uspecial"))),
            RecoveryUse::SpendWithoutFreefall,
            "a vehicle recovery was flattened into the ordinary one, so the              pirate is helpless on the shark it is supposed to be riding"
        );
        assert_eq!(
            lowered(UpSpecial::NotARecovery {
                spec: spec("uspecial"),
                because: "a fixture, exercising the third arm",
            }),
            RecoveryUse::None,
            "a fighter declared exempt from the recovery budget was charged for              it anyway"
        );
    }

    /// ⛔ AND THE POSTURE DOES NOT GET A VOTE. `EITHER` carries
    /// `RecoveryUse::None` like every other posture constant, and the lowering
    /// loop drops it: if the posture won instead, the slot's whole statement
    /// would be dead code and the test above would be measuring the fixture.
    #[test]
    fn the_posture_gate_does_not_overwrite_the_slots_statement() {
        assert_eq!(
            EITHER.recovery,
            RecoveryUse::None,
            "the posture constant now states a recovery, so the assertion that              the SLOT decides it can no longer fail"
        );
        assert_ne!(
            lowered(UpSpecial::Standard(spec("uspecial"))),
            EITHER.recovery,
            "the up-B came out of the lowering wearing the POSTURE's answer"
        );
    }
}

#[cfg(test)]
mod taunt_slot_tests {
    use super::tests::{repertoire, spec};
    use super::*;
    use ambition_entity_catalog::MoveSpec;

    fn kit() -> SmashRepertoire {
        repertoire(
            DownSpecial::OneForm(spec("dspecial")),
            NeutralSpecial::Authored(spec("nspecial")),
        )
    }

    /// THE TAUNT REACHES THE CONTRACT, GROUNDED, UNDER ITS OWN VERB.
    ///
    ///  the two halves that matter: a taunt bound to no verb is a button that
    /// does nothing, and a taunt left ungated answers an AIRBORNE press, which
    /// would make a fighter stop dead in mid-air.
    #[test]
    fn the_taunt_slot_binds_the_taunt_verb_and_is_grounded() {
        let contract = kit().into_contract();
        let id = contract
            .verbs
            .get("taunt")
            .expect("the taunt slot bound no verb, so the button is dead");
        let spec: &MoveSpec = contract
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the taunt verb names a move the contract does not carry");
        assert_eq!(spec.gates.grounded, Some(true));
    }

    /// A TAUNT THREATENS NOBODY, AND IT COSTS YOU THE FLOOR.
    #[test]
    fn an_authored_taunt_has_no_volume_and_roots_the_body() {
        let spec = crate::moveset_authoring::taunt("t", 0.9);
        assert!(spec.duration_s > 0.0);
        assert!(
            spec.windows.iter().all(|w| w.volumes.is_empty()),
            "a taunt carried a hitbox, which makes it an attack with a bad name"
        );
        assert!(
            spec.windows.iter().all(|w| w.motion_scale == 0.0),
            "a taunt you can walk out of is not a commitment"
        );
    }
}
