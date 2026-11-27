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

use ambition_entity_catalog::{is_melee_verb, is_ranged_verb, MovesetContract};

use crate::brain::action_set::{ActionSet, IdentityKit};
use crate::brain::HeldItemSpec;
use crate::equipment::{apply_equipment_grants, EquipmentGrant, WornEquipment};
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
    }
    // EACH ATTACK FAMILY IS SELECTED BY ITS SOURCE, then only the selected
    // families are materialized. `build_actor_moveset` DERIVES a swing from a
    // melee spec and inserts it over whatever the contract already had, so
    // re-deriving a family nothing replaced swaps an AUTHORED move for a
    // generated one: a smash fighter wearing a ranged-only row would lose the
    // jab string its character authored. Selection is by source, not by
    // payload: a ranged grant equal to a charging identity's latent preset is
    // still the grant's shot, and a melee-only row must not turn that latent
    // preset into a moveset `ranged` verb that takes the press from charge.
    //
    // A HELD WEAPON IS THE WHOLE MELEE VOCABULARY. The wearer's melee verbs are
    // released before the weapon's are derived, so a directional or smash move
    // the character authored cannot answer a press the weapon owns. A hand that
    // brings no melee releases nothing: its Attack press is spent by the item's
    // own road, and the wearer's verbs are what keep that slot offered.
    //
    // A HELD ITEM IS THE WHOLE RANGED VOCABULARY, whatever it brings: its shot
    // replaces the wearer's, and an item with none leaves the body without one,
    // exactly as `ActionSet.ranged` says.
    let holding = matches!(hand, Hand::Holding(_));
    let weapon_in_hand = matches!(hand, Hand::Holding(spec) if spec.melee.is_some());
    let granted = |family: fn(&EquipmentGrant) -> bool| {
        worn.is_some_and(|worn| worn.rows.iter().any(|row| row.grants.iter().any(family)))
    };
    let melee_selected = weapon_in_hand || granted(|grant| matches!(grant, EquipmentGrant::Melee(_)));
    let ranged_selected = holding || granted(|grant| matches!(grant, EquipmentGrant::Ranged(_)));
    let kit = stance_kit(&identity.moveset, worn);
    let moveset = if !melee_selected && !ranged_selected {
        kit
    } else {
        let mut base = kit;
        if weapon_in_hand {
            release_verbs(&mut base, is_melee_verb);
        }
        if holding {
            release_verbs(&mut base, is_ranged_verb);
        }
        build_actor_moveset(
            Some(&base),
            action_set.melee.as_ref().filter(|_| melee_selected),
            action_set.ranged.as_ref().filter(|_| ranged_selected),
            None,
        )
        .unwrap_or(base)
    };
    EffectiveRepertoire {
        action_set,
        moveset,
    }
}

/// The body's own moveset with each worn row's verb rebinds applied, in the
/// worn order. A rebinding to a move the table does not author is skipped,
/// so a stance row on a body without its moves changes nothing.
fn stance_kit(own: &MovesetContract, worn: Option<&WornEquipment>) -> MovesetContract {
    let mut kit = own.clone();
    for (verb, id) in worn.into_iter().flat_map(|worn| &worn.rows).flat_map(|row| &row.rebinds) {
        if own.move_by_id(id).is_some() {
            kit.verbs.insert(verb.clone(), id.clone());
        }
    }
    kit
}

/// Unbind every verb in `family`, and drop the moves only those verbs used.
fn release_verbs(contract: &mut MovesetContract, family: fn(&str) -> bool) {
    let released: Vec<String> = contract
        .verbs
        .iter()
        .filter(|(verb, _)| family(verb))
        .map(|(_, id)| id.clone())
        .collect();
    contract.verbs.retain(|verb, _| !family(verb));
    let verbs = &contract.verbs;
    contract
        .moves
        .retain(|m| !released.contains(&m.id) || verbs.values().any(|id| *id == m.id));
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::action_set::{MeleeActionSpec, RangedActionSpec, SwipeSpec};
    use crate::equipment::EquipmentRow;
    use ambition_entity_catalog::{ATTACK_VERB, RANGED_VERB};

    fn wearing(grant: EquipmentGrant) -> WornEquipment {
        WornEquipment::new(vec![EquipmentRow {
            exclusive_slot: None,
            id: "row".into(),
            modifiers: vec![],
            grants: vec![grant],
            on_hit: None,
            withholds: vec![],
            lends: Vec::new(),
            rebinds: Default::default(),
        }])
    }

    /// An identity with an authored jab on `attack` and, like the charging
    /// robot, a ranged spec that no moveset verb executes.
    fn charging_brawler() -> IdentityKit {
        let mut action_set = ActionSet::peaceful();
        action_set.melee = Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT));
        action_set.ranged = Some(RangedActionSpec::bolt(400.0, 5));
        let mut moveset = build_actor_moveset(None, action_set.melee.as_ref(), None, None).unwrap();
        let jab = moveset.verbs[ATTACK_VERB].clone();
        moveset.verbs.insert(ATTACK_VERB.into(), "authored_jab".into());
        moveset.moves.iter_mut().find(|m| m.id == jab).unwrap().id = "authored_jab".into();
        IdentityKit::of(action_set, moveset)
    }

    #[test]
    fn a_ranged_grant_keeps_the_authored_jab() {
        let live = effective_repertoire(
            &charging_brawler(),
            Some(&wearing(EquipmentGrant::Ranged(RangedActionSpec::arrow(300.0, 2)))),
            Hand::Empty,
        );
        assert_eq!(live.moveset.verbs.get(ATTACK_VERB).map(String::as_str), Some("authored_jab"));
        assert!(live.moveset.move_for_verb(RANGED_VERB).is_some(), "the grant's shot is a verb");
    }

    #[test]
    fn a_melee_grant_leaves_the_latent_ranged_preset_to_charge() {
        let live = effective_repertoire(
            &charging_brawler(),
            Some(&wearing(EquipmentGrant::Melee(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)))),
            Hand::Empty,
        );
        assert!(live.moveset.move_for_verb(RANGED_VERB).is_none(), "a melee row made a ranged verb");
        assert!(live.action_set.ranged.is_some());
    }

    #[test]
    fn a_ranged_grant_equal_to_the_latent_preset_is_still_the_grants_shot() {
        let identity = charging_brawler();
        let same = identity.action_set.ranged.clone().unwrap();
        let live = effective_repertoire(&identity, Some(&wearing(EquipmentGrant::Ranged(same))), Hand::Empty);
        assert!(live.moveset.move_for_verb(RANGED_VERB).is_some(), "an equal grant was read as no grant");
        assert_eq!(live.moveset.verbs.get(ATTACK_VERB).map(String::as_str), Some("authored_jab"));
    }

    fn stance(rebinds: &[(&str, &str)]) -> WornEquipment {
        WornEquipment::new(vec![EquipmentRow {
            exclusive_slot: Some("stance".into()),
            id: "stance".into(),
            rebinds: rebinds.iter().map(|(verb, id)| (verb.to_string(), id.to_string())).collect(),
            ..Default::default()
        }])
    }

    /// A brawler whose table also authors a lunge no verb names.
    fn brawler_with_a_lunge() -> IdentityKit {
        let mut identity = charging_brawler();
        let mut lunge = identity.moveset.move_by_id("authored_jab").unwrap().clone();
        lunge.id = "lunge".into();
        identity.moveset.moves.push(lunge);
        identity
    }

    /// A worn stance binds a verb to another move of the body's own table; a
    /// rebinding to a move the table lacks changes nothing; off, the verb is
    /// the body's own again.
    #[test]
    fn a_worn_stance_rebinds_the_kit_and_gives_it_back() {
        let identity = brawler_with_a_lunge();
        let worn = stance(&[(ATTACK_VERB, "lunge"), ("attack_up", "no_such_move")]);
        let live = effective_repertoire(&identity, Some(&worn), Hand::Empty);
        assert_eq!(live.moveset.verbs.get(ATTACK_VERB).map(String::as_str), Some("lunge"));
        assert_eq!(live.moveset.verbs.get("attack_up"), identity.moveset.verbs.get("attack_up"), "a missing move was bound");
        assert!(live.moveset.move_for_verb(ATTACK_VERB).is_some());

        let bare = effective_repertoire(&identity, Some(&WornEquipment::default()), Hand::Empty);
        assert_eq!(bare.moveset.verbs.get(ATTACK_VERB).map(String::as_str), Some("authored_jab"));
        let elsewhere = effective_repertoire(&charging_brawler(), Some(&worn), Hand::Empty);
        assert_eq!(elsewhere.moveset.verbs, charging_brawler().moveset.verbs, "a stance changed a body without its moves");
        assert!(!worn.rows[0].changes_nothing());
    }
}
