//! What a body can do right now: one projection from its durable facts.
//!
//! A body's repertoire is a function of three things and nothing else — the
//! [`IdentityKit`] its character resolved to, the [`WornEquipment`] it is
//! wearing, and what its [`Hand`] holds. [`effective_repertoire`] is the only
//! place that fold is written, so spawn, provocation, a brain-command switch, a
//! dismount, a pickup, a throw and the equipment reconcile cannot answer it
//! differently.
//!
//! `ActionSet` and `ActorMoveset` are both DERIVED from that fold. Writing one
//! without the other leaves a body whose verbs and whose move timelines
//! disagree, which is why this returns the pair.

use ambition_entity_catalog::{is_melee_verb, MovesetContract};

use crate::brain::action_set::{ActionSet, IdentityKit};
use crate::brain::HeldItemSpec;
use crate::equipment::{apply_equipment_grants, WornEquipment};
use crate::moveset_prefabs::build_actor_moveset;

/// What the body's hand contributes.
///
/// The hand's verbs ARE the item's verbs: a held object replaces both attack
/// slots rather than overlaying only the ones it names, so picking up a
/// gun-sword puts the laser where the swing was instead of leaving the body
/// holding a gun and still swinging its fists.
#[derive(Clone, Copy, Debug, Default)]
pub enum Hand<'a> {
    #[default]
    Empty,
    Holding(&'a HeldItemSpec),
    /// The portal gun equips through its own component and names no verbs of
    /// its own; it occupies the hand, so the body's melee goes with it.
    PortalGun,
}

/// The pair a body's live repertoire consists of.
#[derive(Clone, Debug)]
pub struct EffectiveRepertoire {
    pub action_set: ActionSet,
    pub moveset: MovesetContract,
}

/// Fold identity, worn equipment and the hand into the body's live repertoire.
///
/// The order is the precedence: identity is the baseline a body returns to,
/// equipment grants overlay the verbs a row confers, and the hand replaces the
/// attack slots outright. Rebuilding from the baseline every time is what makes
/// a grant REVOCABLE — a row removed disappears instead of lingering because
/// nothing subtracted it.
///
/// Specials are identity policy: no row and no held item confers one.
pub fn effective_repertoire(
    identity: &IdentityKit,
    worn: Option<&WornEquipment>,
    hand: Hand<'_>,
) -> EffectiveRepertoire {
    let mut action_set = identity.action_set.clone();
    if let Some(worn) = worn {
        apply_equipment_grants(&mut action_set, worn);
    }
    match hand {
        Hand::Empty => {}
        Hand::Holding(spec) => {
            action_set.melee = spec.melee;
            action_set.ranged = spec.ranged.clone();
        }
        Hand::PortalGun => action_set.melee = None,
    }
    // A FOLD WITH NOTHING TO FOLD IS THE BASELINE, and saying so is load-bearing
    // rather than an optimisation. `build_actor_moveset` DERIVES a swing from a
    // melee spec and inserts it over whatever the contract already had, so
    // re-deriving an unchanged verb replaces an AUTHORED move with a generated
    // one — a smash fighter re-derived from its own identity loses the jab
    // string its character authored. The rebuild is for a verb the overlay
    // actually changed.
    //
    // A HELD WEAPON IS THE WHOLE MELEE VOCABULARY. The wearer's melee verbs are
    // released before the weapon's are derived, so a directional or smash move
    // the character authored cannot answer a press the weapon owns. A hand that
    // brings no melee releases nothing: its Attack press is spent by the item's
    // own road, and the wearer's verbs are what keep that slot offered.
    let weapon_in_hand = matches!(hand, Hand::Holding(spec) if spec.melee.is_some());
    let moveset = if !weapon_in_hand
        && action_set.melee == identity.action_set.melee
        && action_set.ranged == identity.action_set.ranged
    {
        identity.moveset.clone()
    } else {
        let mut base = identity.moveset.clone();
        if weapon_in_hand {
            release_melee_verbs(&mut base);
        }
        build_actor_moveset(
            Some(&base),
            action_set.melee.as_ref(),
            action_set.ranged.as_ref(),
            None,
        )
        .unwrap_or(base)
    };
    EffectiveRepertoire {
        action_set,
        moveset,
    }
}

/// Unbind every melee verb, and drop the moves only those verbs used.
fn release_melee_verbs(contract: &mut MovesetContract) {
    let released: Vec<String> = contract
        .verbs
        .iter()
        .filter(|(verb, _)| is_melee_verb(verb))
        .map(|(_, id)| id.clone())
        .collect();
    contract.verbs.retain(|verb, _| !is_melee_verb(verb));
    let verbs = &contract.verbs;
    contract
        .moves
        .retain(|m| !released.contains(&m.id) || verbs.values().any(|id| *id == m.id));
}

