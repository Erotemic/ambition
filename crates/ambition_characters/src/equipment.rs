//! A3 — equipment→params.
//!
//! Worn equipment changes a body in exactly three ways, and **never a fourth**
//! (`docs/planning/engine/combat-model.md` §8 "A3 design"):
//!
//! 1. **Numeric modifiers** ([`ParamModifier`]) fold into a resolved value at the
//!    moment it is READ — a move's damage as its fire event dispatches, a body's
//!    size as the render/collision face reads it. The fold is [`resolved_param`],
//!    a pure `(base, worn, key, scope) -> f32`. Modified values are **never baked
//!    into stored state**: equip/unequip changes what the next read sees, with no
//!    write-back to reconcile.
//! 2. **Behavioral grants** ([`EquipmentGrant`]) are ordinary capabilities applied
//!    on equip and revoked on unequip — a ranged verb overlaid onto the wearer's
//!    `ActionSet`, from which the moveset auto-derives its move. Never a bespoke
//!    third mechanism.
//! 3. **On-hit armor** ([`OnHit::ConsumeAsArmor`]) intercepts a hit inside the ONE
//!    victim-side resolver before body damage: the row is spent (removed, or
//!    downgraded to a lesser row it carries), the victim takes zero HP damage and
//!    gains the same brief i-frames any hit arms.
//!
//! The whole worn set lives on one body component, [`WornEquipment`]. This module
//! is content-free and ECS-light on purpose: the fold and the armor spend are pure
//! functions the combat resolver, the moveset dispatch, and the body-size reads all
//! call, so a CPU/RL policy wearing a mushroom is scaled through the same seam a
//! human is (the relativity principle).

use crate::brain::action_set::{ActionSet, MeleeActionSpec, RangedActionSpec};
use bevy::ecs::component::Component;

/// Body-param keys A3 folds worn modifiers against (scope [`ModifierScope::Body`]).
/// A move/verb param uses its own authored key namespace; these name the handful of
/// body-level values equipment can move.
pub mod body_param {
    /// Multiplies/adds onto the body's base collision + render size.
    pub const BODY_SCALE: &str = "body_scale";
    /// Adds onto the body's maximum health.
    pub const MAX_HEALTH: &str = "max_health";
}

/// One authored equipment row. The rows a body currently wears live in
/// [`WornEquipment`]; a content game authors rows as RON and equips them through
/// the ordinary wear seam.
///
/// `Deserialize`-only, matching the codebase's authored-content convention
/// (`HeldItemSpec`): rows are read from content, never written back.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct EquipmentRow {
    /// Stable authoring id (`"mushroom"`, `"fire_flower"`). Also what
    /// [`WornEquipment::consume_armor`] reports when this row absorbs a hit.
    pub id: String,
    /// Numeric modifiers this row folds into resolved params ([`resolved_param`]).
    #[serde(default)]
    pub modifiers: Vec<ParamModifier>,
    /// Capabilities this row grants on equip and revokes on unequip.
    #[serde(default)]
    pub grants: Vec<EquipmentGrant>,
    /// What happens when the wearer is hit while this row is worn. Absent = the hit
    /// damages HP as usual.
    #[serde(default)]
    pub on_hit: Option<OnHit>,
}

/// A numeric modifier one worn row contributes to a resolved param. See
/// [`resolved_param`] for the fold rule (all Adds, then all Muls).
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ParamModifier {
    /// The param key this modifier moves — a body-param key ([`body_param`]) for a
    /// [`ModifierScope::Body`] modifier, or a move's own authored param key for a
    /// [`ModifierScope::Move`]/[`ModifierScope::Verb`] modifier.
    pub param: String,
    /// Add or multiply.
    pub op: ModifierOp,
    /// Which resolution context this modifier applies in. Defaults to the body.
    #[serde(default)]
    pub scope: ModifierScope,
}

/// Add or multiply. Adds apply before Muls (see [`resolved_param`]).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum ModifierOp {
    Add(f32),
    Mul(f32),
}

/// Where a modifier applies. A `Body` modifier folds at body-param reads; a
/// `Move`/`Verb` modifier folds only when THAT move/verb resolves its params.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub enum ModifierScope {
    /// Body-level params (size, max HP). The default, so a terse body modifier
    /// authors no `scope`.
    #[default]
    Body,
    /// A specific move, matched by its move id.
    Move(String),
    /// Any move triggered by this input verb.
    Verb(String),
}

/// The context a param is being resolved IN — passed to [`resolved_param`], which
/// folds only the modifiers whose declared [`ModifierScope`] matches. Borrowed, so
/// a resolution site names its move/verb without allocating.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolveScope<'a> {
    /// Resolving a body-level param.
    Body,
    /// Resolving a param for a specific move, optionally triggered by a verb.
    Move { id: &'a str, verb: Option<&'a str> },
}

impl ModifierScope {
    /// Whether a modifier declared with this scope applies while resolving in
    /// `ctx`. `Body`↔`Body`, `Move(id)` matches the move id, `Verb(v)` matches the
    /// triggering verb; the body and move contexts are otherwise disjoint.
    fn applies_in(&self, ctx: ResolveScope<'_>) -> bool {
        match (self, ctx) {
            (ModifierScope::Body, ResolveScope::Body) => true,
            (ModifierScope::Move(m), ResolveScope::Move { id, .. }) => m == id,
            (ModifierScope::Verb(v), ResolveScope::Move { verb: Some(vb), .. }) => v == vb,
            _ => false,
        }
    }
}

/// A capability a row confers on equip and revokes on unequip. Grants are ordinary
/// `ActionSet` verbs — the flower grants a ranged verb, from which the moveset
/// derives its `simple_ranged` move — never a bespoke mechanism.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum EquipmentGrant {
    /// Grant a ranged verb (overlaid onto `ActionSet.ranged`).
    Ranged(RangedActionSpec),
    /// Grant a melee verb (overlaid onto `ActionSet.melee`).
    Melee(MeleeActionSpec),
}

impl EquipmentGrant {
    /// Overlay this grant onto an action set, exactly as `HeldItemSpec` does for a
    /// wielded weapon. Applied on equip; the caller re-derives the moveset after.
    pub fn apply_to_action_set(&self, actions: &mut ActionSet) {
        match self {
            EquipmentGrant::Ranged(r) => actions.ranged = Some(*r),
            EquipmentGrant::Melee(m) => actions.melee = Some(*m),
        }
    }
}

/// What being hit does to a worn row.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum OnHit {
    /// Spend this row to absorb the hit: the wearer takes zero HP damage and gains
    /// the normal brief i-frames. The row is removed, or replaced by `downgrade_to`
    /// (Mary-O's mushroom big→small is `downgrade_to: None`).
    ///
    /// A downgrade row is expected to be grant-free (modifiers/armor only): the
    /// armor spend happens inside the victim-side resolver, which can rewrite the
    /// worn set but cannot run equip-time grant application. A grant-bearing
    /// downgrade is out of v1 scope.
    ConsumeAsArmor {
        #[serde(default)]
        downgrade_to: Option<Box<EquipmentRow>>,
    },
}

/// The equipment a body currently wears — the single per-body worn set. Its rows'
/// modifiers fold at read time and its armor rows are spent by the victim-side
/// resolver; nothing here is baked into other state.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct WornEquipment {
    pub rows: Vec<EquipmentRow>,
}

impl WornEquipment {
    pub fn new(rows: Vec<EquipmentRow>) -> Self {
        Self { rows }
    }

    /// True iff a row with this id is worn.
    pub fn wears(&self, id: &str) -> bool {
        self.rows.iter().any(|r| r.id == id)
    }

    /// Equip a row (append). Grant application is the caller's job (it needs the
    /// `ActionSet` + a moveset rebuild); this only records the worn state its
    /// modifiers and armor are read from.
    pub fn equip(&mut self, row: EquipmentRow) {
        self.rows.push(row);
    }

    /// Remove the first row with this id, returning it. Grant revocation is the
    /// caller's job.
    pub fn unequip(&mut self, id: &str) -> Option<EquipmentRow> {
        let idx = self.rows.iter().position(|r| r.id == id)?;
        Some(self.rows.remove(idx))
    }

    /// A3 armor-on-hit, called inside the ONE victim-side resolver before body
    /// damage. If any worn row absorbs hits ([`OnHit::ConsumeAsArmor`]), spend the
    /// first such row — remove it, or replace it in place with its downgrade — and
    /// return the spent row's id. `None` means no armor was worn: the hit proceeds
    /// to HP.
    pub fn consume_armor(&mut self) -> Option<String> {
        let idx = self
            .rows
            .iter()
            .position(|r| matches!(r.on_hit, Some(OnHit::ConsumeAsArmor { .. })))?;
        let consumed = self.rows.remove(idx);
        let id = consumed.id.clone();
        if let Some(OnHit::ConsumeAsArmor {
            downgrade_to: Some(downgrade),
        }) = consumed.on_hit
        {
            self.rows.insert(idx, *downgrade);
        }
        Some(id)
    }
}

/// Fold worn-equipment numeric modifiers into `base` at READ time.
///
/// **Ordering (documented, load-bearing): ALL matching Adds first, then ALL
/// matching Muls.** So a `+10` and a `×2` on the same param yield `(base + 10) × 2`,
/// and two rows' Muls compose multiplicatively regardless of equip order — the
/// result is independent of the order rows were worn, which a fold that interleaved
/// Adds and Muls could not promise.
///
/// Only modifiers whose [`ModifierScope`] matches `scope` and whose `param` equals
/// `param` participate. This is a pure read: it never writes back, so the caller
/// must call it wherever the value is consumed rather than caching a folded result.
pub fn resolved_param(
    base: f32,
    worn: &WornEquipment,
    param: &str,
    scope: ResolveScope<'_>,
) -> f32 {
    let matching = || {
        worn.rows
            .iter()
            .flat_map(|r| r.modifiers.iter())
            .filter(|m| m.param == param && m.scope.applies_in(scope))
    };
    let mut value = base;
    for m in matching() {
        if let ModifierOp::Add(a) = m.op {
            value += a;
        }
    }
    for m in matching() {
        if let ModifierOp::Mul(x) = m.op {
            value *= x;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifier(param: &str, op: ModifierOp, scope: ModifierScope) -> ParamModifier {
        ParamModifier {
            param: param.to_string(),
            op,
            scope,
        }
    }

    fn row(id: &str, modifiers: Vec<ParamModifier>) -> EquipmentRow {
        EquipmentRow {
            id: id.to_string(),
            modifiers,
            grants: Vec::new(),
            on_hit: None,
        }
    }

    #[test]
    fn no_worn_equipment_returns_the_base_untouched() {
        let worn = WornEquipment::default();
        assert_eq!(
            resolved_param(42.0, &worn, body_param::MAX_HEALTH, ResolveScope::Body),
            42.0
        );
    }

    #[test]
    fn adds_apply_before_muls_regardless_of_authoring_order() {
        // A `+10` and a `×2` on max HP, authored Mul-first, must still be (base+10)*2.
        let worn = WornEquipment::new(vec![row(
            "gear",
            vec![
                modifier(
                    body_param::MAX_HEALTH,
                    ModifierOp::Mul(2.0),
                    ModifierScope::Body,
                ),
                modifier(
                    body_param::MAX_HEALTH,
                    ModifierOp::Add(10.0),
                    ModifierScope::Body,
                ),
            ],
        )]);
        assert_eq!(
            resolved_param(100.0, &worn, body_param::MAX_HEALTH, ResolveScope::Body),
            220.0
        );
    }

    #[test]
    fn two_rows_muls_compose_and_are_order_independent() {
        let a = row(
            "a",
            vec![modifier(
                body_param::BODY_SCALE,
                ModifierOp::Mul(1.5),
                ModifierScope::Body,
            )],
        );
        let b = row(
            "b",
            vec![modifier(
                body_param::BODY_SCALE,
                ModifierOp::Mul(2.0),
                ModifierScope::Body,
            )],
        );
        let ab = WornEquipment::new(vec![a.clone(), b.clone()]);
        let ba = WornEquipment::new(vec![b, a]);
        assert_eq!(
            resolved_param(1.0, &ab, body_param::BODY_SCALE, ResolveScope::Body),
            3.0
        );
        assert_eq!(
            resolved_param(1.0, &ba, body_param::BODY_SCALE, ResolveScope::Body),
            3.0
        );
    }

    #[test]
    fn a_body_modifier_does_not_leak_into_a_move_resolution() {
        // A body-scoped size buff must not scale a move's damage.
        let worn = WornEquipment::new(vec![row(
            "gear",
            vec![modifier(
                "damage",
                ModifierOp::Mul(2.0),
                ModifierScope::Body,
            )],
        )]);
        assert_eq!(
            resolved_param(
                50.0,
                &worn,
                "damage",
                ResolveScope::Move {
                    id: "fireball",
                    verb: Some("ranged")
                }
            ),
            50.0,
            "a Body-scoped modifier is inert in a Move context"
        );
    }

    #[test]
    fn a_move_scoped_mul_scales_only_its_own_move() {
        let worn = WornEquipment::new(vec![row(
            "fire_flower",
            vec![modifier(
                "damage",
                ModifierOp::Mul(1.5),
                ModifierScope::Move("fireball".to_string()),
            )],
        )]);
        // The named move is scaled...
        assert_eq!(
            resolved_param(
                10.0,
                &worn,
                "damage",
                ResolveScope::Move {
                    id: "fireball",
                    verb: None
                }
            ),
            15.0
        );
        // ...a different move is not.
        assert_eq!(
            resolved_param(
                10.0,
                &worn,
                "damage",
                ResolveScope::Move {
                    id: "iceball",
                    verb: None
                }
            ),
            10.0
        );
    }

    #[test]
    fn a_verb_scoped_modifier_matches_the_triggering_verb() {
        let worn = WornEquipment::new(vec![row(
            "gauntlet",
            vec![modifier(
                "speed",
                ModifierOp::Add(100.0),
                ModifierScope::Verb("ranged".to_string()),
            )],
        )]);
        // Same verb, any move id → applies.
        assert_eq!(
            resolved_param(
                300.0,
                &worn,
                "speed",
                ResolveScope::Move {
                    id: "anything",
                    verb: Some("ranged")
                }
            ),
            400.0
        );
        // A move with no verb, or a different verb → inert.
        assert_eq!(
            resolved_param(
                300.0,
                &worn,
                "speed",
                ResolveScope::Move {
                    id: "anything",
                    verb: Some("melee")
                }
            ),
            300.0
        );
    }

    #[test]
    fn consume_armor_removes_the_row_and_reports_its_id() {
        let mut worn = WornEquipment::new(vec![EquipmentRow {
            id: "mushroom".to_string(),
            on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
            ..Default::default()
        }]);
        assert_eq!(worn.consume_armor().as_deref(), Some("mushroom"));
        assert!(worn.rows.is_empty(), "the spent armor row is gone");
        // A second hit finds no armor: it proceeds to HP.
        assert_eq!(worn.consume_armor(), None);
    }

    #[test]
    fn consume_armor_downgrades_in_place_when_a_downgrade_is_authored() {
        // big → small: the first hit downgrades, the second (small has no armor)
        // proceeds to HP.
        let small = EquipmentRow {
            id: "mushroom_small".to_string(),
            ..Default::default()
        };
        let mut worn = WornEquipment::new(vec![EquipmentRow {
            id: "mushroom_big".to_string(),
            on_hit: Some(OnHit::ConsumeAsArmor {
                downgrade_to: Some(Box::new(small)),
            }),
            ..Default::default()
        }]);
        assert_eq!(worn.consume_armor().as_deref(), Some("mushroom_big"));
        assert!(worn.wears("mushroom_small"), "downgraded in place");
        assert_eq!(
            worn.consume_armor(),
            None,
            "the downgrade has no armor: the next hit hits HP"
        );
    }

    #[test]
    fn consume_armor_skips_non_armor_rows_to_find_the_armor() {
        let mut worn = WornEquipment::new(vec![
            row("plain", vec![]),
            EquipmentRow {
                id: "shell".to_string(),
                on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
                ..Default::default()
            },
        ]);
        assert_eq!(worn.consume_armor().as_deref(), Some("shell"));
        assert!(worn.wears("plain"), "a non-armor row is untouched by a hit");
    }

    #[test]
    fn a_grant_overlays_the_action_set_slot_it_names() {
        let mut actions = ActionSet::peaceful();
        assert!(actions.ranged.is_none());
        let grant = EquipmentGrant::Ranged(RangedActionSpec::Bolt {
            speed: 400.0,
            damage: 8,
        });
        grant.apply_to_action_set(&mut actions);
        assert!(
            matches!(actions.ranged, Some(RangedActionSpec::Bolt { .. })),
            "the flower's ranged verb lands in the action set"
        );
    }

    #[test]
    fn an_equipment_row_round_trips_from_ron() {
        // The authoring shape a content game writes: a mushroom (body size + armor)
        // and a flower (a ranged grant + a move-scoped damage buff).
        let mushroom: EquipmentRow = ron::from_str(
            r#"(
                id: "mushroom",
                modifiers: [ (param: "body_scale", op: Mul(1.4)) ],
                on_hit: Some(ConsumeAsArmor(downgrade_to: None)),
            )"#,
        )
        .expect("mushroom row parses");
        assert_eq!(mushroom.id, "mushroom");
        assert_eq!(mushroom.modifiers[0].scope, ModifierScope::Body);
        assert!(matches!(
            mushroom.on_hit,
            Some(OnHit::ConsumeAsArmor { downgrade_to: None })
        ));

        let flower: EquipmentRow = ron::from_str(
            r#"(
                id: "fire_flower",
                grants: [ Ranged(Bolt(speed: 420.0, damage: 6)) ],
                modifiers: [ (param: "damage", op: Mul(1.5), scope: Move("fireball")) ],
            )"#,
        )
        .expect("flower row parses");
        assert_eq!(flower.grants.len(), 1);
        assert_eq!(
            flower.modifiers[0].scope,
            ModifierScope::Move("fireball".to_string())
        );
    }
}
