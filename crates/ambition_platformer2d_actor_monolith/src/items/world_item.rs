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
use crate::features::ecs::pickups::TouchCollectorFilter;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use ambition_characters::equipment::{EquipmentRow, WornEquipment};
use ambition_platformer2d_core::{self as ae, AabbExt};

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
    // `emerging: bool` LIVED HERE and is GONE. Mary-O set it `true` when a ?-block popped a
    // reward and nothing ever set it back — the comment named a `clear_emerged_powerups` that
    // was never written — so an item finished rising, began its ordinary arc, and stayed drawn
    // BEHIND the world for the rest of its life ( of `1a05b98`, ).
    //
    // the fact was already derivable: `ItemMotion::emerging()` compares elapsed rise against
    // the authored one, every frame, and cannot go stale. `SimView` asks that instead.
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
/// returns the entity so the caller can SCOPE what it spawned. Room-scoping
/// is this function's business; whether the item is also residue of one ATTEMPT is
/// the caller's, and it could not say so while this returned `()`. See
/// `SpawnedThisAttempt` — *"one scope cannot answer both"*.
pub fn spawn_world_item(commands: &mut Commands, item: WorldItem) -> Entity {
    commands
        .spawn_room_scoped((item, Name::new("World item")))
        .id()
}

/// Spawn a `WorldItem` that MOVES — the same room-scoped pickup plus an authored
/// [`ItemMotionPlan`](super::item_motion::ItemMotionPlan) for the engine to step.
///
/// A separate entry point rather than an `Option` on [`spawn_world_item`]: a
/// still pickup and a moving one are different enough at the call site that
/// naming which one you meant is worth a second function.
pub fn spawn_moving_world_item(
    commands: &mut Commands,
    item: WorldItem,
    plan: super::item_motion::ItemMotionPlan,
) -> Entity {
    commands
        .spawn_room_scoped((
            item,
            super::item_motion::ItemMotion::new(plan),
            Name::new("World item"),
        ))
        .id()
}

/// Collect overlapping consumable `WorldItem`s for bodies in
/// [`TouchCollectorFilter`]. The payload is added to [`WornEquipment`] and the
/// consumable entity ends; durable [`GroundItem`](super::pickup::GroundItem)
/// instances instead preserve identity through [`ItemCustody`](super::pickup::ItemCustody).
/// Each body collects at most one world item per frame.
pub fn collect_world_items(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            &BodyKinematics,
            bevy::prelude::Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl>,
            Option<&mut WornEquipment>,
        ),
        (
            // A body the game is driving does not shop.
            Without<ambition_characters::control::ScriptedControl>,
            TouchCollectorFilter,
        ),
    >,
    items: Query<(Entity, &WorldItem)>,
) {
    // Snapshot eligible collector boxes before mutating equipment.
    let collectors: Vec<(Entity, ae::Aabb)> = bodies
        .iter()
        .filter(|(_, _, is_player, control, _)| {
            crate::features::ecs::pickups::body_collects_on_touch(*is_player, *control)
        })
        .map(|(entity, kin, _, _, _)| (entity, ae::Aabb::new(kin.pos, kin.size * 0.5)))
        .collect();
    if collectors.is_empty() {
        return;
    }

    // One item per body per frame, so two items landing on one bare body cannot
    // have the second `WornEquipment::new` overwrite the first.
    let mut spent: Vec<Entity> = Vec::new();
    for (item_entity, item) in &items {
        let Some(&(body, _)) = collectors
            .iter()
            .find(|(body, aabb)| !spent.contains(body) && aabb.strict_intersects(item.aabb()))
        else {
            continue;
        };
        let Ok((_, _, _, _, worn)) = bodies.get_mut(body) else {
            continue;
        };
        match &item.payload {
            // Collecting RECORDS the row and nothing else. Any verb the row grants is applied by
            // `reconcile_equipment_grants`, which derives the live action set + moveset from
            // identity + worn equipment. Now there is exactly one derivation, and a hit that spends
            // a granting row revokes its verb on the same path this pickup granted it.
            WorldItemPayload::Equip(row) => match worn {
                Some(mut worn) => worn.equip(row.clone()),
                None => {
                    commands
                        .entity(body)
                        .insert(WornEquipment::new(vec![row.clone()]));
                }
            },
        }
        spent.push(body);
        commands.entity(item_entity).despawn();
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
            exclusive_slot: None,
        }
    }

    /// A body in the touch-collect population — the marker a couch seat carries.
    fn app_with_subject(pos: ae::Vec2) -> (App, Entity) {
        let mut app = App::new();
        let body = app
            .world_mut()
            .spawn((
                kin(pos),
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ))
            .id();
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
            exclusive_slot: None,
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

    /// BOTH sides of the fork this system was on, in one test.
    ///
    /// The invariant: the population that collects a touched item is the union
    /// of the player population and any body a player is currently driving —
    /// not one body, and not `PlayerEntity` alone.
    ///
    /// it is a paired assertion because either half alone passes on the
    /// broken code. A test with only the second couch seat is green under the
    /// old `PlayerEntity`-filtered `collect_ecs_pickups`; a test with only the
    /// possessed body is green under the old `ControlledSubject` lookup. What
    /// neither population can do is BOTH, and that is what is asserted.
    ///
    /// Falsified by reverting each half in turn: with the collector filtered to
    /// a lone `ControlledSubject` the seat-two mushroom is left standing, and
    /// with it filtered to `With<PlayerEntity>` the possessed body's is.
    #[test]
    fn a_second_seat_and_a_possessed_body_both_collect() {
        let mut app = App::new();
        app.add_systems(Update, collect_world_items);

        // Seat two: in the player population, and NOT the driven subject.
        let seat_two = app
            .world_mut()
            .spawn((
                kin(ae::Vec2::new(-400.0, 0.0)),
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ))
            .id();
        // A possessed actor: no `PlayerEntity`, driven through possession.
        let possessed = app
            .world_mut()
            .spawn((
                kin(ae::Vec2::new(400.0, 0.0)),
                ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl::Player {
                    controller: ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
                },
            ))
            .id();
        // An ordinary autonomous actor is in the query's FILTER (every actor
        // carries `TemporaryControl`) and must still not collect — the poison
        // for widening the filter without applying the value test.
        let bystander = app
            .world_mut()
            .spawn((
                kin(ae::Vec2::new(0.0, 0.0)),
                ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl::Autonomous,
            ))
            .id();

        for pos in [
            ae::Vec2::new(-400.0, 0.0),
            ae::Vec2::new(400.0, 0.0),
            ae::Vec2::ZERO,
        ] {
            app.world_mut().spawn(WorldItem::equipping(
                armor_row(),
                pos,
                ae::Vec2::new(12.0, 12.0),
            ));
        }

        app.update();

        assert!(
            app.world()
                .get::<WornEquipment>(seat_two)
                .is_some_and(|w| w.wears("grow_cap")),
            "a second couch seat collects what it touches — it did not while this \
             system served a single `ControlledSubject`",
        );
        assert!(
            app.world()
                .get::<WornEquipment>(possessed)
                .is_some_and(|w| w.wears("grow_cap")),
            "a possessed body collects what it touches — it would not under a \
             `With<PlayerEntity>` population",
        );
        assert!(
            app.world().get::<WornEquipment>(bystander).is_none(),
            "an autonomous actor standing on an item must not equip it",
        );
    }
}
