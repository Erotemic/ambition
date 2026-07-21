//! `WorldItem` — a walk-into collectible that grants EQUIPMENT.
//!
//! The sibling of [`GroundItem`](super::pickup::GroundItem), split along the
//! collect TRIGGER the pickup module's `AMBITION_REVIEW(discrete_ok)` note
//! already anticipated: a `GroundItem` is a *held weapon* grabbed with a
//! deliberate `Attack` press; a `WorldItem` is *touched* — bare AABB overlap
//! auto-collects it, the way a mushroom / ring / heart is picked up by running
//! into it. Its payload is an A3 [`EquipmentRow`], so collecting it just RECORDS
//! the row in [`WornEquipment`]; any verb the row grants is derived from the worn
//! set by `reconcile_equipment_grants`, the one place a body's granted actions
//! come from.
//!
//! This is deliberately generic: "a thing in the world you collect to gain a
//! capability or effect" is universal (Super Mary-O's grow-cap / spark-blossom,
//! a heart, a power-up). What the row DOES is pure A3 data; this module only
//! owns "touch it → equip it → it's gone."

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use ambition_characters::equipment::{EquipmentRow, WornEquipment};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_primitives::markers::ControlledSubject;

/// A collectible resting in the world. Touch it (AABB overlap) and its
/// [`payload`](WorldItem::payload) is applied to the collecting body, then it
/// despawns. Unlike a [`GroundItem`](super::pickup::GroundItem) there is no
/// press gate and no held-weapon overlay — a `WorldItem` grants equipment.
#[derive(Component, Clone, Debug)]
pub struct WorldItem {
    pub payload: WorldItemPayload,
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    /// Optional ART id for the render layer to draw this pickup as a real sprite
    /// (e.g. a milk carton) instead of the row-tinted placeholder quad. It is a
    /// PRESENTATION key, deliberately separate from the equipment `row` id (art id
    /// ≠ equipment id): a game maps it to an image through its own `WorldItemArt`.
    /// `None` keeps the draw-blind quad.
    pub sprite: Option<String>,
}

impl WorldItem {
    /// A collectible that equips `row` when touched.
    pub fn equipping(row: EquipmentRow, pos: ae::Vec2, half_extent: ae::Vec2) -> Self {
        Self {
            payload: WorldItemPayload::Equip(row),
            pos,
            half_extent,
            sprite: None,
        }
    }

    /// Tag this item with a presentation art id the render layer resolves to a real
    /// sprite (via the game's `WorldItemArt`), falling back to the quad if unbound.
    pub fn with_sprite(mut self, sprite: impl Into<String>) -> Self {
        self.sprite = Some(sprite.into());
        self
    }

    /// This item's world-space box.
    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.half_extent)
    }
}

/// What collecting a [`WorldItem`] does. One variant today (equip a row); the
/// enum is the seam a heal / score / stat pickup extends into.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldItemPayload {
    /// Equip this A3 row on the collecting body (its modifiers/armor fold at
    /// read time; a granting row also rebuilds the moveset).
    Equip(EquipmentRow),
}

/// Spawn a `WorldItem` into the active session, room-scoped so it despawns with
/// the room (never leaks across a reload) — the same scoping a thrown
/// [`GroundItem`](super::pickup::GroundItem) uses.
pub fn spawn_world_item(commands: &mut Commands, item: WorldItem) {
    commands.spawn_room_scoped((item, Name::new("World item")));
}

/// **Touch-to-collect.** The [`ControlledSubject`] (the driven body — player or
/// possessed) collects a `WorldItem` it overlaps: the item's row is equipped
/// into [`WornEquipment`] (inserted if the body wore none), and the item is
/// despawned. Granted verbs are reconciled from the worn set, not applied here.
///
/// At most one item is collected per frame (`break` after the first, matching
/// [`pickup_held_item_system`](super::pickup::pickup_held_item_system)); the
/// demo's items are spatially separated, so "which one, if several overlap" —
/// the only query-order dependence here — never arises in practice.
pub fn collect_world_items(
    mut commands: Commands,
    controlled: Res<ControlledSubject>,
    mut bodies: Query<(&BodyKinematics, Option<&mut WornEquipment>)>,
    items: Query<(Entity, &WorldItem)>,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((kin, worn)) = bodies.get_mut(subject) else {
        return;
    };
    let body_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);

    for (item_entity, item) in &items {
        if !body_aabb.strict_intersects(item.aabb()) {
            continue;
        }
        match &item.payload {
            // Collecting RECORDS the row and nothing else. Any verb the row grants
            // is applied by `reconcile_equipment_grants`, which derives the live
            // action set + moveset from identity + worn equipment. Pickup used to
            // apply grants itself, which made it one of two places that could
            // change a body's action set — and the only one that could add a verb
            // but never remove one. Now there is exactly one derivation, and a hit
            // that spends a granting row revokes its verb on the same path this
            // pickup granted it.
            WorldItemPayload::Equip(row) => match worn {
                Some(mut worn) => worn.equip(row.clone()),
                None => {
                    commands
                        .entity(subject)
                        .insert(WornEquipment::new(vec![row.clone()]));
                }
            },
        }
        commands.entity(item_entity).despawn();
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::equipment::{EquipmentGrant, OnHit};

    fn kin(pos: ae::Vec2) -> BodyKinematics {
        BodyKinematics {
            pos,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(28.0, 32.0),
            facing: 1.0,
        }
    }

    /// A plain armor row (grow-cap shape): collecting equips it, the body ends
    /// up wearing it, and the item is gone.
    fn armor_row() -> EquipmentRow {
        EquipmentRow {
            id: "grow_cap".into(),
            modifiers: Vec::new(),
            grants: Vec::new(),
            on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        }
    }

    fn app_with_subject(pos: ae::Vec2) -> (App, Entity) {
        let mut app = App::new();
        let body = app
            .world_mut()
            .spawn(kin(pos))
            .id();
        app.insert_resource(ControlledSubject(Some(body)));
        app.add_systems(Update, collect_world_items);
        (app, body)
    }

    #[test]
    fn touching_a_world_item_equips_its_row_and_despawns_it() {
        let (mut app, body) = app_with_subject(ae::Vec2::ZERO);
        let item = app
            .world_mut()
            .spawn(WorldItem::equipping(
                armor_row(),
                ae::Vec2::ZERO,
                ae::Vec2::new(12.0, 12.0),
            ))
            .id();

        app.update();

        assert!(
            app.world().get_entity(item).is_err(),
            "a touched world item is collected (despawned)"
        );
        let worn = app
            .world()
            .get::<WornEquipment>(body)
            .expect("collecting inserts a worn set on a bare body");
        assert!(worn.wears("grow_cap"), "the row is now worn");
    }

    /// The presentation `sprite` tag threads onto the item (default `None`) so the
    /// render can bind a real image, while collect/equip ignores it entirely — art
    /// id is separate from equipment id.
    #[test]
    fn tagging_an_item_with_a_sprite_carries_the_art_id() {
        let item = WorldItem::equipping(armor_row(), ae::Vec2::ZERO, ae::Vec2::new(12.0, 12.0));
        assert_eq!(item.sprite, None, "a plain item carries no art override");
        let tagged = item.with_sprite("super_mary_o_milk_carton");
        assert_eq!(tagged.sprite.as_deref(), Some("super_mary_o_milk_carton"));
        assert!(
            matches!(tagged.payload, WorldItemPayload::Equip(_)),
            "the art tag leaves the equip payload untouched"
        );
    }

    #[test]
    fn a_world_item_out_of_reach_is_not_collected() {
        let (mut app, body) = app_with_subject(ae::Vec2::ZERO);
        let item = app
            .world_mut()
            .spawn(WorldItem::equipping(
                armor_row(),
                ae::Vec2::new(500.0, 0.0),
                ae::Vec2::new(12.0, 12.0),
            ))
            .id();

        app.update();

        assert!(
            app.world().get_entity(item).is_ok(),
            "an item the body doesn't overlap stays in the world"
        );
        assert!(
            app.world().get::<WornEquipment>(body).is_none(),
            "and nothing is equipped"
        );
    }

    /// A granting row is RECORDED by collect; applying its verb belongs to the
    /// equipment reconcile, so the pickup itself stays a pure "touch → worn".
    #[test]
    fn collecting_a_granting_row_records_it_in_the_worn_set() {
        use ambition_characters::brain::action_set::RangedActionSpec;
        let (mut app, body) = app_with_subject(ae::Vec2::ZERO);
        let row = EquipmentRow {
            id: "spark".into(),
            modifiers: Vec::new(),
            grants: vec![EquipmentGrant::Ranged(RangedActionSpec::bolt(400.0, 5))],
            on_hit: None,
        };
        app.world_mut().spawn(WorldItem::equipping(
            row,
            ae::Vec2::ZERO,
            ae::Vec2::new(12.0, 12.0),
        ));

        app.update();

        let worn = app
            .world()
            .get::<WornEquipment>(body)
            .expect("collecting a granting row still records it");
        assert!(worn.wears("spark"), "the granting row is worn");
    }
}
