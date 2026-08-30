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

use crate::features::ecs::pickups::TouchCollectorFilter;
use ambition_characters::equipment::{EquipmentRow, WornEquipment};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::prelude::SpawnScopedExt;
use ambition_platformer2d_shared_tangle::sim_selection::{in_deterministic_order, winner_by};

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
///
/// ⛔⛔ `id` IS A PARAMETER, NOT A CONVENIENCE, and it is why this signature
/// changed. [`collect_world_items`] resolves "which of two overlapping items does
/// this body get" by [`SimId`] and a CONSTANT metric, so the identity is not a
/// tie-break there — it is the entire ordering authority. Both spawn helpers used
/// to attach none, which made that sort stable-LOOKING and encounter-ordered
/// underneath: the guard's own test supplied the `SimId` production never did, so
/// it agreed with the fix by construction while every real item stayed in query
/// order. Taking the id here is what makes forgetting it a compile error.
pub fn spawn_world_item(
    commands: &mut Commands,
    id: ambition_platformer2d_shared_tangle::sim_id::SimId,
    item: WorldItem,
) -> Entity {
    commands
        .spawn_room_scoped((item, id, Name::new("World item")))
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
    id: ambition_platformer2d_shared_tangle::sim_id::SimId,
    item: WorldItem,
    plan: super::item_motion::ItemMotionPlan,
) -> Entity {
    commands
        .spawn_room_scoped((
            item,
            id,
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
    // The tie-break's authority, for both orders. A read-only lookup so a body
    // or item without one still competes — it just cannot win a tie.
    sim_ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
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
    //
    // ⛔⛔ WHICH MAKES THE OUTER ORDER A GAMEPLAY DECISION TOO, and it was Bevy
    // query order. Because the loop spends a body on its first match, the order
    // items are visited decides WHICH of two overlapping items a body receives —
    // so this system had the ordering defect twice: once choosing the body, once
    // choosing the item. Both are archetype order, which a resimulated tick can
    // present differently.
    //
    // ⭐ THE ITEM ORDER'S METRIC IS ITS OWN IDENTITY. There is no meaningful
    // distance between two items competing for one body, so the rule is the tie-
    // break alone: a constant metric plus stable `SimId`.
    let ordered_items = in_deterministic_order(
        items.iter(),
        |_| 0.0,
        |(entity, _)| sim_ids.get(*entity).ok(),
    );

    let mut spent: Vec<Entity> = Vec::new();
    for (item_entity, item) in ordered_items {
        // ⭐ AND THE NEAREST UNSPENT BODY GETS IT, not the first one the query
        // happened to yield.
        let Some(&(body, _)) = winner_by(
            collectors.iter().filter(|(body, aabb)| {
                !spent.contains(body) && aabb.strict_intersects(item.aabb())
            }),
            |(_, aabb)| aabb.center().distance_squared(item.aabb().center()),
            |(entity, _)| sim_ids.get(*entity).ok(),
        ) else {
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
    use bevy::ecs::system::RunSystemOnce;

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
    /// A distinguishable row per item, so the arm can say WHICH one was collected.
    fn row_named(id: &str) -> EquipmentRow {
        EquipmentRow {
            id: format!("row_{id}").into(),
            ..armor_row()
        }
    }

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

    /// ⛔⛔ THE ORDERING AUTHORITY PRODUCTION NEVER SUPPLIED, and the reason this
    /// arm goes through the SEAM. `collect_world_items` orders contested items by
    /// `SimId` against a CONSTANT metric, so identity is not a tie-break there —
    /// it is the whole rule. Neither spawn helper attached one, so `sort_by`
    /// compared `Equal` on every pair and Rust's stable sort preserved the input
    /// order: the Bevy query order the sort was added to remove. The guard that
    /// shipped with the fix spawned `WorldItem` and `SimId` by hand, which is the
    /// one population where the bug cannot appear.
    ///
    /// ⭐ SO THE ARM REVERSES THE SPAWN ORDER. Two items in one archetype are
    /// visited in spawn order, so if identity decides nothing the winner flips —
    /// and if it decides everything, both arrangements collect the same item.
    #[test]
    fn which_of_two_overlapping_items_a_body_gets_does_not_depend_on_spawn_order() {
        fn collected_row(first: &str, second: &str) -> String {
            let (mut app, body) = app_with_subject(ae::Vec2::ZERO);
            let (a, b) = (first.to_string(), second.to_string());
            app.world_mut()
                .run_system_once(move |mut commands: Commands| {
                    for id in [a.as_str(), b.as_str()] {
                        spawn_world_item(
                            &mut commands,
                            ambition_platformer2d_shared_tangle::sim_id::SimId::geometry(
                                &ae::GeoId::tile_layer("Blocks", id.parse().unwrap()),
                            ),
                            WorldItem::equipping(
                                row_named(id),
                                ae::Vec2::ZERO,
                                ae::Vec2::new(12.0, 12.0),
                            ),
                        );
                    }
                })
                .expect("the spawn seam runs");
            app.update();
            let worn = app
                .world()
                .get::<WornEquipment>(body)
                .expect("one of the two items was collected");
            for id in ["1", "2"] {
                if worn.wears(&format!("row_{id}")) {
                    return id.to_string();
                }
            }
            unreachable!("neither row was equipped");
        }

        let forwards = collected_row("1", "2");
        let backwards = collected_row("2", "1");
        assert_eq!(
            forwards, backwards,
            "reversing the spawn order changed which item the body collected — the \
             sort is falling back to query order, so the items carry no identity to \
             order BY (got {forwards} then {backwards})"
        );
    }

    /// And the population itself satisfies what the sort needs: every item is
    /// identified, and no two share an id. ⛔ THE WEAKER HALF ALONE IS NOT ENOUGH
    /// — two items spelling one id tie, and a tie is encounter order again.
    #[test]
    fn every_item_the_seams_spawn_is_distinctly_identified() {
        use ambition_platformer2d_shared_tangle::sim_selection::{
            every_candidate_is_identified, no_two_candidates_share_an_identity,
        };
        let mut app = App::new();
        app.world_mut()
            .run_system_once(|mut commands: Commands| {
                spawn_world_item(
                    &mut commands,
                    ambition_platformer2d_shared_tangle::sim_id::SimId::geometry(
                        &ae::GeoId::tile_layer("Blocks", 3),
                    ),
                    WorldItem::equipping(armor_row(), ae::Vec2::ZERO, ae::Vec2::splat(12.0)),
                );
                spawn_moving_world_item(
                    &mut commands,
                    ambition_platformer2d_shared_tangle::sim_id::SimId::geometry(
                        &ae::GeoId::placement(ae::PlacementId::new("block-iid"), 0),
                    ),
                    WorldItem::equipping(armor_row(), ae::Vec2::ZERO, ae::Vec2::splat(12.0)),
                    super::super::item_motion::ItemMotionPlan::still(),
                );
            })
            .expect("the spawn seams run");

        let world = app.world_mut();
        let items: Vec<(
            Entity,
            Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
        )> = world
            .query_filtered::<(
                Entity,
                Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            ), With<WorldItem>>()
            .iter(world)
            .map(|(entity, id)| (entity, id.cloned()))
            .collect();
        assert_eq!(items.len(), 2, "both seams spawned an item");
        assert!(
            every_candidate_is_identified(items.iter(), |(_, id)| id.as_ref()),
            "a world item joined the contested population with no SimId, so the \
             collection order has nothing to sort by"
        );
        assert!(
            no_two_candidates_share_an_identity(items.iter(), |(_, id)| id.as_ref()),
            "two world items spell the same SimId, so they tie and fall back to \
             query order"
        );
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

    /// ⛔⛔ THE ORDERING DEFECT TWICE OVER. `collect_world_items` spends a body
    /// on its first match — one item per body per frame — so Bevy query order
    /// decided WHICH of two overlapping items a body received, as well as which
    /// body received an item. Both are archetype order, which a resimulated tick
    /// can present differently.
    mod which_item {
        use super::*;
        use ambition_platformer2d_shared_tangle::sim_id::SimId;

        fn row(id: &str) -> EquipmentRow {
            EquipmentRow {
                id: id.into(),
                modifiers: Vec::new(),
                grants: Vec::new(),
                on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
                exclusive_slot: None,
            }
        }

        /// Two items on one body, spawned in `order`; reports which one it wore.
        fn worn_with_spawn_order(order: [&str; 2]) -> String {
            let mut app = App::new();
            app.add_systems(Update, collect_world_items);
            let here = ae::Vec2::ZERO;
            app.world_mut().spawn((
                kin(here),
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                SimId::player_slot(0),
            ));
            for id in order {
                app.world_mut().spawn((
                    WorldItem::equipping(row(id), here, ae::Vec2::new(12.0, 12.0)),
                    SimId::placement(id),
                ));
            }

            app.update();

            let world = app.world_mut();
            let mut worn_q = world.query::<&WornEquipment>();
            let worn = worn_q
                .iter(world)
                .next()
                .expect("the body collected one of the two items it is standing on");
            for id in ["alpha", "omega"] {
                if worn.wears(id) {
                    return id.to_string();
                }
            }
            panic!("the body wore neither authored row");
        }

        /// ⭐ THE PROPERTY. Reversing the order two contested items were spawned
        /// in must not change which one the body ends up wearing.
        ///
        /// ⛔⛔ AND THIS ARM PINS THE FUNCTION, NOT THE WIRING. It hands each item
        /// the `SimId` production did not, so it stays GREEN while every real
        /// world item sorts by nothing — measured: poisoning the spawn seam leaves
        /// this test passing and fails
        /// `which_of_two_overlapping_items_a_body_gets_does_not_depend_on_spawn_order`,
        /// which is the arm that goes through the seam. Keep both: this one says
        /// the rule is right, that one says the population obeys it.
        #[test]
        fn the_same_item_is_collected_whichever_order_the_two_were_spawned_in() {
            let forward = worn_with_spawn_order(["alpha", "omega"]);
            let reversed = worn_with_spawn_order(["omega", "alpha"]);
            assert_eq!(
                forward, reversed,
                "which of two overlapping items the body wore changed with the \
                 order they were spawned in, so a resimulated tick equips the other"
            );
            assert_eq!(
                forward, "alpha",
                "the order is stable SimId, so the lower id is collected first"
            );
        }
    }
}
