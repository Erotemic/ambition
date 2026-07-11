//! Super Mary-O's powerups — the M1 equipment chain, authored as A3 data.
//!
//! These are the M-track's proof that "powerups as equipment" is pure content on
//! the finished engine face (`docs/planning/demos/super-mary-o.md` §M1): a
//! mushroom-analog and a flower-analog authored entirely through the `ambition`
//! umbrella's re-exported A3 vocabulary, with **zero engine edits**. The engine's
//! `ambition::characters::equipment` module (A3) supplies the three mechanisms —
//! numeric modifiers, behavioral grants, on-hit armor — and this file just names
//! two rows that use them.
//!
//! Parody-original, like the rest of the demo (Q28): a "grow cap" and a "spark
//! blossom", homage in role, not a copy.

use ambition::characters::brain::action_set::RangedActionSpec;
use ambition::characters::equipment::{
    body_param, EquipmentGrant, EquipmentRow, ModifierOp, ModifierScope, OnHit, ParamModifier,
};

/// Row id of the grow-cap (mushroom-analog).
pub const GROW_CAP_ID: &str = "grow_cap";
/// Row id of the spark-blossom (flower-analog).
pub const SPARK_BLOSSOM_ID: &str = "spark_blossom";

/// The grow-cap: **size + armor**, the classic first powerup.
///
/// It scales the wearer's body 1.5× (a `Body`-scoped [`body_param::BODY_SCALE`]
/// modifier, folded wherever the body's size is read) and absorbs one hit — the
/// A3 [`OnHit::ConsumeAsArmor`] policy with `downgrade_to: None`, so a hit spends
/// the cap and the body reverts to its base size on the very next read, no
/// write-back. "Big → small on hit", as data.
pub fn grow_cap() -> EquipmentRow {
    EquipmentRow {
        id: GROW_CAP_ID.to_string(),
        modifiers: vec![ParamModifier {
            param: body_param::BODY_SCALE.to_string(),
            op: ModifierOp::Mul(1.5),
            scope: ModifierScope::Body,
        }],
        grants: Vec::new(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
    }
}

/// The spark-blossom: **a ranged verb + a damage buff**, the fireball powerup.
///
/// It grants a ranged bolt ([`EquipmentGrant::Ranged`]) — from which the moveset
/// derives a real fireable move on equip — and scales that shot's damage 1.5× at
/// fire (a `Verb("ranged")`-scoped [`ranged_param::DAMAGE`] modifier, folded in
/// [`ambition::characters::equipment::resolved_ranged`] at trigger-resolve).
///
/// It deliberately carries NO armor: an on-hit downgrade cannot re-run grant
/// application (A3 v1 — see [`OnHit`]), so a grant-bearing armor row would leave a
/// dangling verb. Layering the blossom over the [`grow_cap`] gives the two-hit feel
/// (lose fire, then lose size) without that gap: the cap is the armor, the blossom
/// the capability.
///
/// [`ranged_param::DAMAGE`]: ambition::characters::equipment::ranged_param::DAMAGE
pub fn spark_blossom() -> EquipmentRow {
    use ambition::characters::equipment::ranged_param;
    EquipmentRow {
        id: SPARK_BLOSSOM_ID.to_string(),
        modifiers: vec![ParamModifier {
            param: ranged_param::DAMAGE.to_string(),
            op: ModifierOp::Mul(1.5),
            scope: ModifierScope::Verb("ranged".to_string()),
        }],
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::Bolt {
            speed: 420.0,
            damage: 6,
        })],
        on_hit: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition::characters::equipment::{apply_equipment_grants, resolved_ranged, WornEquipment};

    /// The grow-cap absorbs a hit and reverts size — Mary-O's "big → small".
    /// Proven through the umbrella's A3 API: if `ambition` didn't re-export
    /// `characters::equipment`, this demo would not compile (the E9 oracle).
    #[test]
    fn grow_cap_scales_the_body_and_absorbs_one_hit() {
        let mut worn = WornEquipment::new(vec![grow_cap()]);

        // Worn: the body reads 1.5× its base size.
        let scaled = ambition::characters::equipment::resolved_param(
            100.0,
            &worn,
            body_param::BODY_SCALE,
            ambition::characters::equipment::ResolveScope::Body,
        );
        assert_eq!(scaled, 150.0, "the grow-cap scales the body 1.5×");

        // A hit spends the cap...
        assert_eq!(worn.consume_armor().as_deref(), Some(GROW_CAP_ID));
        // ...and the body is back to base size on the next read (no write-back).
        let reverted = ambition::characters::equipment::resolved_param(
            100.0,
            &worn,
            body_param::BODY_SCALE,
            ambition::characters::equipment::ResolveScope::Body,
        );
        assert_eq!(reverted, 100.0, "losing the cap reverts size, no bake");
        // The next hit finds no armor — it would reach HP.
        assert_eq!(worn.consume_armor(), None);
    }

    /// The spark-blossom grants a ranged verb and scales its shot's damage at fire.
    #[test]
    fn spark_blossom_grants_a_scaled_fireball() {
        use ambition::characters::brain::action_set::ActionSet;
        use ambition::combat::moveset::{build_actor_moveset, RANGED_VERB};

        let worn = WornEquipment::new(vec![spark_blossom()]);

        // The grant confers a ranged verb the moveset can fire.
        let mut actions = ActionSet::peaceful();
        assert!(actions.ranged.is_none());
        apply_equipment_grants(&mut actions, &worn);
        let moveset = build_actor_moveset(None, actions.melee.as_ref(), actions.ranged.as_ref())
            .expect("the blossom's ranged verb yields a moveset");
        assert!(
            moveset.move_for_verb(RANGED_VERB).is_some(),
            "the spark-blossom grants a fireable ranged move"
        );

        // The fireball leaves the barrel with folded (×1.5) damage.
        let base = actions.ranged.expect("blossom set a ranged spec");
        let shot = resolved_ranged(base, &worn, "ranged", RANGED_VERB);
        assert_eq!(shot.damage(), 9, "×1.5 on the blossom's 6-damage bolt");
        assert_eq!(shot.speed(), 420.0, "speed is unmodified");
    }

    /// Distinct ids so a body can wear both (the cap as armor, the blossom as
    /// capability) without one shadowing the other.
    #[test]
    fn the_two_powerups_are_distinct_rows() {
        assert_ne!(grow_cap().id, spark_blossom().id);
    }
}
