//! The kit a body wears: what a character id resolves to when a body puts it on.
//!
//! One resolver for spawn, runtime re-wear and match seating, so no two roads can
//! disagree about what a character IS. Every field is a deterministic function of
//! the identity, the body's `AbilitySet`, and (only for a seated fighter) the
//! stage's borrowed repertoire — never of the body's prior kit.
//!
//! This is content compilation, and it used to live in the actor kernel's
//! `avatar` module. The kernel now consumes the [`WornKit`] value and writes it
//! onto components; it no longer decides what a character's kit is.

use ambition_characters::brain::action_set::IdentityKit;
use ambition_characters::brain::{ActionSet, RangedExecution};
use ambition_characters::prepared::{overlay_authored_moves, PreparedCharacterDefinition};
use ambition_characters::move_damage::{move_damage_over, DamageScale};
use ambition_entity_catalog::MovesetContract;

use crate::moveset::build_actor_moveset;

/// What a MATCH says about the kit a seated fighter wears.
///
/// `Default` is a body in no match: its character's own kit, dealing its
/// moveset's own damage. That is every body outside a match.
#[derive(Clone, Copy, Debug, Default)]
pub struct SeatTerms<'a> {
    /// The stage's borrowed repertoire. It replaces the action set; see
    /// [`WornKit::of`].
    pub action_set: Option<&'a ActionSet>,
    /// The damage scale the fighter's moves deal on this stage; `None` is
    /// each moveset's own damage.
    pub move_damage: Option<DamageScale>,
}

impl<'a> SeatTerms<'a> {
    /// A seat whose stage lends it `action_set` and says nothing else.
    pub fn borrowing(action_set: &'a ActionSet) -> Self {
        Self {
            action_set: Some(action_set),
            ..Self::default()
        }
    }
}

/// What a body carries once it wears a character.
#[derive(Clone, Debug)]
pub struct WornKit {
    pub action_set: ActionSet,
    pub moveset: MovesetContract,
    /// The un-granted baseline the brain reads: the action set and the moveset
    /// it was built beside, before equipment and granted verbs overlay them.
    pub identity: IdentityKit,
    /// HOW the resolved persona fires. The kernel's ECS derive synchronizes the
    /// charge marker and its mutable state from this.
    pub execution: RangedExecution,
}

impl WornKit {
    /// The kit the prepared character `prepared` puts on a body. The body's
    /// abilities are not an input: what the character IS does not depend on
    /// what this body may currently do (that is the per-frame action scheme's
    /// question).
    ///
    /// The kit is the row's `PreparedKit::baseline` and its own
    /// `ranged_execution` — the answer the spawn grant writes. Authored
    /// repertoire is what a character IS; what a ruleset currently permits it
    /// to use is the per-frame action scheme over `BodyAbilities`, not a
    /// narrowing of the kit (census DUP-CHARACTER-KIT, decided 2026-09-23).
    ///
    /// ⛔ ONLY A PREPARED CHARACTER HAS A KIT (Q103, 2026-10-03). The input is
    /// the prepared definition, not an id, so no road can ask for the kit of an
    /// id the cast does not hold. A character admitted into simulation was
    /// prepared for that generation: there is no engine-default kit and no read
    /// of another catalog. A caller that holds only an id looks it up in the
    /// cast, and for an id that is not there it leaves the body as it is and
    /// reports the refusal (`wear_character`).
    ///
    /// A MATCH OUTRANKS THE PERSONA, and only a match: `match_kit` is a rule of
    /// the stage the fighter stands on, not another opinion about who the
    /// character is, so it replaces the action set outright. How the borrower
    /// FIRES is still the character's own fact — a robot seated with a stage's
    /// generic set still charges if the robot charges — and a character's own
    /// authored timelines still overlay the borrowed set's derived moves.
    ///
    /// A MATCH ALSO SAYS WHICH DAMAGE ITS HITS DEAL (`terms.move_damage`): a
    /// platform-fighter stage reads the character's `smash_fighter` damage over
    /// the same moves. Resolved here and not at seating, so a fighter that
    /// re-wears its character during the match keeps the stage's damage.
    pub fn of(prepared: &PreparedCharacterDefinition, terms: SeatTerms<'_>) -> Self {
        let execution = prepared.ranged_execution;

        let (set, derived) = match terms.action_set {
            Some(kit) => {
                let derived =
                    derive_persona_moveset(kit, execution, prepared.authored_moveset.clone());
                (kit.clone(), derived)
            }
            // The prepared baseline, exactly as the spawn grant writes it.
            None => prepared.kit.baseline(),
        };
        let scaled = terms
            .move_damage
            .and_then(|scale| prepared.scaled_move_damage.get(&scale))
            .filter(|damage| !damage.is_empty());
        let derived = match scaled {
            Some(damage) => {
                match move_damage_over(damage, derived.clone()) {
                    Ok(moveset) => moveset,
                    // Preparation already reported this on the published value
                    // (`unresolved_references`). The body keeps one game's damage
                    // for every move rather than a mix of two.
                    Err(problems) => {
                        bevy::log::error!(
                            "character '{}' keeps its moveset damage on this stage: {problems:?}",
                            prepared.id.as_str(),
                        );
                        derived
                    }
                }
            }
            None => derived,
        };
        Self {
            identity: IdentityKit::of(set.clone(), derived.clone()),
            moveset: derived,
            action_set: set,
            execution,
        }
    }
}

/// Derive a persona's moves from its action set, given HOW it fires.
///
/// Under `ChargedProjectile` the charge mechanic already owns the ranged press,
/// so the ranged preset contributes no move; under `MovesetVerb` the ranged
/// preset IS the ranged verb. The `authored` contract overlays the derivation
/// ([`overlay_authored_moves`]).
///
/// HOW A BODY FIRES SAYS NOTHING ABOUT HOW IT SOUNDS. A derived move carries the
/// builder's cues on this road and in preparation alike. A character with its
/// own sound family authors it in its moveset, as `player_robot.ron` does.
///
/// `pub` for fixtures: a body's swing is built HERE, at spawn, from its action
/// set, so a harness that mutates `ActionSet.melee` afterwards changes nothing
/// the runtime reads.
pub fn derive_persona_moveset(
    set: &ActionSet,
    execution: RangedExecution,
    authored: Option<MovesetContract>,
) -> MovesetContract {
    let (ranged, special) = match execution {
        RangedExecution::ChargedProjectile => (None, set.special.as_ref()),
        RangedExecution::MovesetVerb => (set.ranged.as_ref(), set.special.as_ref()),
    };
    let derived =
        build_actor_moveset(None, set.melee.as_ref(), ranged, special).unwrap_or_default();
    overlay_authored_moves(derived, authored)
}

/// The host-code action set, derived from a body's `AbilitySet`:
///
/// - `melee = Some(Swipe)` iff `abilities.attack` — with NO windup: the hand the
///   player is holding comes out on the press, unlike a Striker it is meant to
///   read;
/// - `ranged = Some(bolt)` always — the fireball path is itself gated by
///   `projectile`, and there is no separate ability flag for it;
/// - `special = Some(Special("bubble_shield"))` iff `abilities.shield`.
///
/// The resolver emits no request for a capability the body lacks, so effects
/// consumers can read the set as "what this body can actually do right now".
///
/// ⚠ A FIXTURE KIT. No shipped body is built with it: every body wears its
/// character's prepared kit. It stays public because other crates' tests build
/// the from-scratch home body with it; a runtime caller would be a fallback
/// inventing a kit the character never authored.
pub fn default_player_action_set(abilities: ambition_platformer2d_core::AbilitySet) -> ActionSet {
    use ambition_characters::brain::{
        MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SpecialActionSpec, SwipeSpec,
    };
    ActionSet {
        melee: abilities
            .attack
            .then_some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.0,
                active_s: 0.10,
                recover_s: 0.18,
                damage: 1,
                reach_px: 36.0,
            })),
        ranged: Some(RangedActionSpec::bolt(600.0, 1)),
        move_style: MoveStyleSpec::Walk,
        special: abilities
            .shield
            .then_some(SpecialActionSpec::Special("bubble_shield".to_string())),
    }
}
