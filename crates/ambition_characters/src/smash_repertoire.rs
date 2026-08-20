//! **THE STANDARD SMASH REPERTOIRE — the vocabulary and the bookkeeping, once.**
//!
//! ⭐⭐ **Jon's rule for this seam, 2026-08-16**: *"Centralize the vocabulary and
//! validation, not the fighter design."* Every fighter supplies its own bespoke
//! [`MoveSpec`]s. What it no longer supplies is a hand-copied verb map, a
//! hand-set posture gate on every move, and a private test asserting that the
//! two agree — fourteen copies of the same seventeen strings, which is fourteen
//! places for a typo to become silence at the press.
//!
//! And his acceptance criterion, which is what fixes the shape below: *"Adding
//! the next standard Smash action should require extending the Smash grammar
//! once, rather than manually updating fourteen copies of infrastructure."*
//!
//! ## Why a struct literal and not a builder
//!
//! A builder can only refuse an incomplete repertoire at RUN time. This is a
//! plain struct with no `Default` and no private fields, so **omitting a slot or
//! misspelling one is a compile error in the fighter's own file** — the
//! strongest available form of "a missing slot fails at authoring time". It is
//! also why `SmashRoster`'s completeness ratchet has nothing left to catch here:
//! a verb cannot dangle when there is no string to get wrong.
//!
//! ## Sixteen PRESSES, seventeen VERBS
//!
//! ⚠ `special_air_down` is not a seventeenth press. It is the AIRBORNE FORM of
//! the down-B — Jon's Bowser ruling: *"A down-b that has special airborne
//! properties should also have an effect on ground. Think of bowser down b. In
//! the air he just does a downward slam, but on the ground, it causes him to jump
//! in an arc and then slam. Specials can have different effects in different
//! contexts."* One slot, two moves; see [`DownSpecial`].
//!
//! ## The posture follows the SLOT
//!
//! ```text
//!   jab, tilts, smashes        grounded    an airborne body falls THROUGH to its aerials
//!   aerials                    airborne    a grounded press must not reach a landing cost
//!   neutral / side / up-B      either      a recovery you can only press standing is not one
//!   down-B, one form           either
//!   down-B, two forms          one each
//! ```
//!
//! ⭐ that table was not decided here — it was MEASURED off all fourteen tables
//! before this module existed, and all fourteen already agreed with it, every
//! move, with no exception. So the gate is not a per-move decision any more; it
//! is what the slot MEANS. A fighter that one day needs an unusual gate extends
//! this grammar (which is one edit) rather than opting out of it silently.
//!
//! ## ⛔ This is SMASH's vocabulary, and it lives here on purpose
//!
//! `ForwardSmash` / `NeutralAir` are NOT universal engine concepts and must not
//! become ones — Jon was explicit. They stop at the smash-shaped layer: this
//! module lowers the whole repertoire into the generic [`MovesetContract`] the
//! engine already speaks, and the engine learns nothing new. It sits in
//! `ambition_characters` beside [`crate::brain::smash`] and
//! [`crate::moveset_authoring`] because that is the crate every provider and
//! demo already has, so a fighter in any game can reach it with no new dep edge.
//!
//! ⚠ **and the sixteen slots are NOT what D166's first facet migrated.**
//! [`crate::smash_fighter`] carries the CAPTURE kit as authored values, because
//! that kit is pure numbers. These slots are not: a fighter builds each one by
//! COMPOSING helpers (`strike`, `impulse`, `on_hit`, `committed_tail`, `feel`),
//! and George's file states a law about the shape of his whole table in a
//! `debug_assert` beside them. That composition IS the design, and flattening it
//! into RON would trade authored reasoning for a wall of numbers.

use ambition_entity_catalog::{MoveGates, MoveSpec, MovesetContract};

/// Ground moves are grounded-only so an airborne body falls THROUGH them to its
/// aerials rather than throwing a tilt in mid-air.
const GROUNDED: MoveGates = MoveGates {
    grounded: Some(true),
};
/// Aerials are airborne-only for the mirror reason: a grounded press must not
/// reach a move whose whole design is that landing costs you.
const AIRBORNE: MoveGates = MoveGates {
    grounded: Some(false),
};
/// The specials: a move that answers its button from the ground OR the air.
const EITHER: MoveGates = MoveGates { grounded: None };

/// **The neutral special, or a stated reason there is no authored one.**
///
/// ⚠ exactly one fighter abstains today — the player robot, whose Hadouken comes
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

/// **The down special — one move, or the Bowser pair.**
pub enum DownSpecial {
    /// One move that answers the press in both postures. The ordinary case for a
    /// down-B that means the same thing wherever you are standing.
    OneForm(MoveSpec),
    /// **Two forms, one slot.** `grounded` answers `special_down`, `airborne`
    /// answers `special_air_down` — which sits AHEAD of it in
    /// `directional_verb_chain`, so an airborne press reaches the air form and a
    /// grounded one falls past it to the ground form.
    ///
    /// ⛔ this is why the pair is a single slot and not two: a special gated to
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

/// **The standard smash repertoire, as sixteen presses of bespoke moves.**
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
    /// `special_up`.
    pub up_special: MoveSpec,
    /// `special_down`, and possibly `special_air_down`. See [`DownSpecial`].
    pub down_special: DownSpecial,
    /// **The capture kit — grab, pummel, throws.** See
    /// [`SmashCaptureRepertoire`](crate::smash_capture::SmashCaptureRepertoire).
    ///
    /// ✔ **REQUIRED, as of 2026-08-19 — the migration is over and the `Option`
    /// is gone.** It was the one optional slot on this struct, suspending the
    /// type's whole argument (a missing field is a compile error in the
    /// fighter's own file) while the relationship architecture was proven on two
    /// fighters. All fourteen author one now, so the compiler resumes doing here
    /// what it does for the other sixteen slots.
    ///
    /// ⭐ **this replaces a grep.** A goal check read the movesets looking for
    /// `capture: Some`, which is the kind of guard that answers a question the
    /// compiler can answer better: a new fighter that forgets a grab no longer
    /// ships and gets noticed, it does not build.
    pub capture: crate::smash_capture::SmashCaptureRepertoire,
    /// **`taunt` — the move that buys nothing.** Required like every other slot,
    /// so a fighter with nothing to say has to say so; `moveset_authoring::taunt`
    /// is the one-liner for a fighter whose taunt is not yet designed.
    pub taunt: MoveSpec,
}

impl SmashRepertoire {
    /// **Lower the repertoire into the generic move contract the engine speaks.**
    ///
    /// This is the ONE place the verb strings exist, the ONE place the posture
    /// gates are applied, and the ONE place the table is checked — which is the
    /// whole point of the type.
    ///
    /// # Panics
    ///
    /// If two slots were given moves with the SAME id. `move_by_id` takes the
    /// first match, so a reused id silently makes one of the two presses swing
    /// the other's timeline — the one defect this shape cannot rule out by
    /// construction, so it is ruled out here, at preparation time, by name.
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
        ];
        if let NeutralSpecial::Authored(spec) = neutral_special {
            bound.push(("special", spec, EITHER));
        }
        bound.push(("special_forward", side_special, EITHER));
        bound.push(("special_up", up_special, EITHER));
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
            spec.gates = gates;
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
            },
            start_impulse: None,
            smash_charge_mult: 1.0,
            landing_lag_s: None,
            autocancel_after_s: None,
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
            up_special: spec("uspecial"),
            taunt: crate::moveset_authoring::taunt("taunt", 0.9),
            // ⚠ a real kit, because the slot is required now. The fixture's
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

    /// **Every press this vocabulary names is answered, in every posture it is
    /// asked in** — resolved the way a BODY resolves it, through the directional
    /// chain, rather than by asking whether a verb key exists.
    ///
    /// ⭐ this is the check that used to be fourteen private copies of
    /// `every_bound_verb_names_a_move_that_exists`. It is stronger than they
    /// were: they asked whether a bound id was defined, and this asks whether a
    /// PRESS reaches a move — which is the question the fourteen copies were
    /// standing in for.
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

    /// **The two-form down-B answers each posture with ITS OWN move**, which is
    /// the whole content of Jon's Bowser ruling: not "the press works", but "the
    /// press does the right thing in both places".
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

    /// **The posture comes from the SLOT, not from what the fighter set.**
    ///
    /// ⭐ the fixture hands every slot a `grounded: Some(true)` spec — the gate
    /// that would be wrong for eleven of the sixteen. If the seam merely
    /// *defaulted*, the aerials would still be grounded-only and unreachable in
    /// the air, which is exactly the defect the census found nine fighters
    /// carrying before the slot owned this.
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

    /// **An abstaining neutral-B binds nothing**, so the body's derived move
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
        // 18 → 19 on 2026-08-20 with the taunt. The claim above is untouched
        // either way: abstaining from the neutral special still binds nothing,
        // and this number exists to catch a slot binding something it should not.
        assert_eq!(set.verbs.len(), 19);
    }

    /// **Two slots cannot share a move id.** The one integrity defect this shape
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

    /// **THE TAUNT REACHES THE CONTRACT, GROUNDED, UNDER ITS OWN VERB.**
    ///
    /// ⛔ the two halves that matter: a taunt bound to no verb is a button that
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

    /// **A TAUNT THREATENS NOBODY, AND IT COSTS YOU THE FLOOR.**
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
