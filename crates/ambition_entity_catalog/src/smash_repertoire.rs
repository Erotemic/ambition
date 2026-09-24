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

use crate::{MoveGates, MoveSpec, MovesetContract, RecoveryUse};

/// Ground moves are grounded-only, so an airborne body falls through them to
/// its aerials and does not throw a tilt in mid-air.
const GROUNDED: MoveGates = MoveGates {
    grounded: Some(true),
    // A posture knows nothing about recoveries. The lowering loop discards
    // this neutral value for what the move or slot says (see the destructure
    // in `into_contract`).
    recovery: RecoveryUse::None,
    // The same for costs: a move's price is its own statement.
    costs: Vec::new(),
    // The same for being held: whether a move refuses to start from a saddle
    // is the move's own statement (`call_the_shark` makes it).
    forbidden_while_held: false,
    // A grounded attack roots its owner. Otherwise a quick forward smash
    // accelerates the fighter to the full run cap during its own startup, so
    // it dashes first and smashes second. This is the one place posture gates
    // are applied, so the steering rule for the posture belongs here. A dash
    // attack keeps its slide, which is the move's own impulse, not steering.
    roots_steering: true,
    recovery_route: None,
    when_refused: None,
};
/// Aerials are airborne-only for the mirror reason: a grounded press must not
/// reach a move whose whole design is that landing costs you.
const AIRBORNE: MoveGates = MoveGates {
    grounded: Some(false),
    // Move-owned fields: neutral here and discarded on lowering (see
    // `GROUNDED`).
    recovery: RecoveryUse::None,
    costs: Vec::new(),
    forbidden_while_held: false,
    // An aerial keeps its drift: the genre trades ground control for air
    // control, and a fighter that could not steer a forward air would lose
    // every edgeguard.
    roots_steering: false,
    recovery_route: None,
    when_refused: None,
};
/// The specials: a move that answers its button from the ground OR the air.
const EITHER: MoveGates = MoveGates {
    grounded: None,
    // Move-owned fields: neutral here and discarded on lowering (see
    // `GROUNDED`).
    recovery: RecoveryUse::None,
    costs: Vec::new(),
    forbidden_while_held: false,
    // A special answers from either stance, so it has no stance rule. A
    // special's effect on its owner's motion is authored on its windows.
    roots_steering: false,
    recovery_route: None,
    when_refused: None,
};

/// The neutral special, or a stated reason there is no authored one.
///
/// Only the player robot abstains: its Hadouken comes from the
/// charged-projectile kit its body already derives, and a `special` binding
/// would replace it. The abstention is a slot value, not an omission, so it
/// cannot happen by accident and the reason travels with it.
pub enum NeutralSpecial {
    /// This fighter authors its neutral-B.
    Authored(MoveSpec),
    /// The press is answered by the move the body's action set derives, which
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
    /// answers `special_air_down`, which is ahead of it in
    /// `directional_verb_chain`. So an airborne press reaches the air form and
    /// a grounded press falls past it to the ground form.
    ///
    /// The pair is one slot and not two: a special gated to one posture and
    /// unanswered in the other lets the chain fall through to the neutral
    /// special, so the player presses down-B and gets something else.
    ByPosture {
        /// Feet down.
        grounded: MoveSpec,
        /// Feet up.
        airborne: MoveSpec,
    },
}

/// The up-B slot, and what it costs the fighter's airtime.
///
/// Most characters can use their up-B more than once without freefall only
/// by exception; the general rule is one use per airtime, then helpless.
/// `MoveGates::recovery` is authored per move in general, but in a smash
/// repertoire the `up_special` field is the recovery by definition, so this
/// slot is where the default can be applied.
///
/// The default must not be opt-in: a rule every author must remember gets
/// applied to only some of them.
///
/// Shaped like [`NeutralSpecial`]: the ordinary case is the shortest to
/// write, and each exception is a separate variant that carries its reason.
pub enum UpSpecial {
    /// The genre's rule: one use per airtime, helpless once the move ends.
    /// This is what the obvious form gives.
    Standard(MoveSpec),
    /// Spends the airtime's recovery but leaves the fighter able to act: a
    /// recovery that gives a vehicle, not an arc (the pirate's burning
    /// shark).
    NoFreefall(MoveSpec),
    /// Not a recovery: this fighter's up-B is repeatable, and `because` says
    /// why that is a design choice.
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
    /// The slot overrides the move here, the opposite of the posture-gate
    /// rule. A posture cannot know whether a move is a recovery, but the up-B
    /// slot can, and a moveset must not quietly disagree with it.
    ///
    /// Public, so a fighter that replaces a borrowed table's up-B pays what
    /// this slot says (for example the Director, who uses the Pointed
    /// Polygon's repertoire and swaps one slot). An up-B that costs nothing is
    /// flight. Guarded by `the_replacement_still_spends_the_airtimes_recovery`.
    pub fn into_spec(self) -> MoveSpec {
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
    /// `special_up` — and what it costs. See [`UpSpecial`].
    pub up_special: UpSpecial,
    /// `special_down`, and possibly `special_air_down`. See [`DownSpecial`].
    pub down_special: DownSpecial,
    /// The capture kit — grab, pummel, throws. See
    /// [`SmashCaptureRepertoire`](crate::smash_capture::SmashCaptureRepertoire).
    ///
    /// Required, so the compiler checks that every fighter has a grab.
    pub capture: crate::smash_capture::SmashCaptureRepertoire,
    /// `taunt` — the move that buys nothing. Required like every other slot,
    /// so a fighter with nothing to say has to say so; `crate::authoring::taunt`
    /// is the one-liner for a fighter whose taunt is not yet designed.
    pub taunt: MoveSpec,
    /// `attack_dash` — the move a body already moving forward throws.
    /// Required: the engine selects `AttackIntent::DashForward` for a dashing
    /// swing, and an unauthored dash attack reads as the forward tilt, which
    /// looks correct and is wrong. `crate::authoring::dash_attack` owns the
    /// shape.
    pub dash_attack: MoveSpec,
}

/// Every verb a [`SmashRepertoire`] can bind.
///
/// [`SmashRepertoire::into_contract`] is the only thing that binds a verb to a
/// move. This constant is that table's verb set, held equal to it by
/// `the_bound_table_binds_exactly_the_declared_vocabulary`, so downstream code
/// (device tables, press lists, `ENGINE_ACTIONS`, censuses) can ask and does
/// not have to repeat the list.
///
/// It is the whole set, including conditional slots. `special` is bound only
/// by an authored neutral special and `special_air_down` only by a `ByPosture`
/// down special, but a verb a repertoire can bind is authoring vocabulary
/// whether or not a fighter uses it.
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
    // The one capture verb with a stance, not a direction: the grab from a
    // run. Derived from each fighter's standing grab in
    // `SmashCaptureRepertoire::bound`, so every fighter has it.
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
    /// This is the only place the posture gates are applied and the table is
    /// checked. The verb set is [`REPERTOIRE_VERBS`].
    ///
    /// # Panics
    ///
    /// If two slots were given moves with the same id.
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
            // Grounded: a dash is a ground stance, and `move_for_attack` only
            // asks for this verb when the body is on the floor.
            ("attack_dash", dash_attack, GROUNDED),
        ];
        if let NeutralSpecial::Authored(spec) = neutral_special {
            bound.push(("special", spec, EITHER));
        }
        bound.push(("special_forward", side_special, EITHER));
        // The slot decides the recovery here, not in the loop below, so the
        // loop only sets posture fields. The loop then leaves the resulting
        // `gates.recovery` alone.
        bound.push(("special_up", up_special.into_spec(), EITHER));
        match down_special {
            DownSpecial::OneForm(spec) => bound.push(("special_down", spec, EITHER)),
            DownSpecial::ByPosture { grounded, airborne } => {
                bound.push(("special_down", grounded, GROUNDED));
                bound.push(("special_air_down", airborne, AIRBORNE));
            }
        }

        // Capture moves are grounded for now. Aerial and command grabs are
        // future techniques.
        {
            for (verb, spec) in capture.bound() {
                bound.push((verb, spec, GROUNDED));
            }
        }

        let mut contract = MovesetContract::default();
        for (verb, mut spec, gates) in bound {
            // Set the posture fields only; keep every statement the move made
            // about its own gates.
            //
            // The split is by authority. A posture knows whether a slot
            // answers from the ground and whether that stance roots its owner.
            // It does not know whether a move is a recovery
            // (`MoveGates::recovery` is authored, not inferred). For the up-B
            // the slot has already decided (`UpSpecial::into_spec`), so
            // `recovery` is dropped here.
            //
            // An exhaustive destructure, so a new gate is a compile error here
            // and its author must choose a side.
            let MoveGates {
                grounded,
                roots_steering,
                recovery: _,
                // The move's own, like `recovery`: `call_the_shark` sets it.
                forbidden_while_held: _,
                // The move's own: only the move can say it summons a vehicle
                // or teleports.
                recovery_route: _,
                // The move's own: a stance template cannot know a move's price.
                costs: _,
                // The move's own, like the price it answers for.
                when_refused: _,
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
    use crate::{AttackDir, ClipBinding};

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
            // Deliberately the wrong gate for every slot: the seam must set it.
            gates: MoveGates {
                grounded: Some(true),
                roots_steering: false,
                recovery_route: None,
                // Move-owned fields, neutral here (see `GROUNDED`).
                recovery: RecoveryUse::None,
                costs: Vec::new(),
                forbidden_while_held: false,
                // A posture names no fallback either: which move answers a
                // refused press is the move's own statement.
                when_refused: None,
            },
            start_impulse: None,
            smash_charge_mult: 1.0,
            charge_gesture: crate::ChargeGesture::default(),
            smash_charge: None,
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
            equips: None,
            flow: None,
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
            taunt: crate::authoring::taunt("taunt", 0.9),
            dash_attack: crate::authoring::dash_attack(
                "dash_attack",
                crate::authoring::DashAttackShape::GENRE,
                9,
                320.0,
            ),
            // A real, minimal kit, because the slot is required. The fixture
            // exercises the verb table, so the capture verbs are tested here.
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
    /// It runs a maximal repertoire: every conditional slot takes the branch
    /// that binds the most verbs. A fixture with one `None` would only show
    /// the constant has no extra words, not that it misses none.
    ///
    /// The floor check matters: an empty contract would make both sets equal
    /// and this test would pass.
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
    /// asked in, resolved the way a body resolves it (through the directional
    /// chain), not by checking that a verb key exists.
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

    /// The posture comes from the slot, not from what the fighter set.
    ///
    /// The fixture gives every slot `grounded: Some(true)`, which is wrong for
    /// most slots.
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
        // 21: the ordinary slots, the capture verbs, the taunt, the dash
        // attack and the derived running grab. Abstaining from the neutral
        // special binds nothing; this count catches a slot that binds
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

    /// The genre's rule is what the obvious form gives. A `Standard` that
    /// lowered to anything but `SpendAndFreefall` would let most fighters
    /// press their recovery forever.
    #[test]
    fn a_standard_up_b_spends_the_recovery_and_ends_in_freefall() {
        assert_eq!(
            lowered(UpSpecial::Standard(spec("uspecial"))),
            RecoveryUse::SpendAndFreefall,
            "the ordinary up-B costs nothing, so the roster has no recovery              budget and the stage has no bottom blastzone"
        );
    }

    /// The two exceptions survive the lowering. An exception flattened back
    /// to the default is worse than none, because the content still reads as
    /// exempt.
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

    /// The posture does not decide. `EITHER` carries `RecoveryUse::None`, and
    /// the lowering loop drops it. If the posture won, the slot's statement
    /// would be dead code.
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
    use crate::MoveSpec;

    fn kit() -> SmashRepertoire {
        repertoire(
            DownSpecial::OneForm(spec("dspecial")),
            NeutralSpecial::Authored(spec("nspecial")),
        )
    }

    /// The taunt reaches the contract, grounded, under its own verb.
    ///
    /// A taunt bound to no verb does nothing, and an ungated taunt answers an
    /// airborne press, which would stop a fighter dead in mid-air.
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

    /// A taunt threatens nobody, and it costs you the floor.
    #[test]
    fn an_authored_taunt_has_no_volume_and_roots_the_body() {
        let spec = crate::authoring::taunt("t", 0.9);
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
